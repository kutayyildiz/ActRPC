use actrpc_core::{CallId, CallLineage, CallRelation, InterceptionId};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::RwLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallNode {
    pub call_id: CallId,
    pub parent_call_id: Option<CallId>,
    pub root_call_id: CallId,
}

#[derive(Debug, Default)]
struct ExecutionTreeInner {
    nodes: HashMap<CallId, CallNode>,
    children: HashMap<CallId, Vec<CallId>>,
}

#[derive(Debug, Default)]
pub struct ExecutionTreeState {
    inner: RwLock<ExecutionTreeInner>,
}

impl ExecutionTreeState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_root(&self, call_id: CallId) -> Result<(), String> {
        let mut inner = self.inner.write().expect("execution tree lock poisoned");

        if inner.nodes.contains_key(&call_id) {
            return Err(format!("call_id:{call_id} already registered"));
        }

        let node = CallNode {
            call_id,
            parent_call_id: None,
            root_call_id: call_id,
        };

        inner.nodes.insert(call_id, node);
        inner.children.entry(call_id).or_default();

        Ok(())
    }

    pub fn register_child(&self, call_id: CallId, parent_call_id: CallId) -> Result<(), String> {
        let mut inner = self.inner.write().expect("execution tree lock poisoned");

        if inner.nodes.contains_key(&call_id) {
            return Err(format!("call_id:{call_id} already registered"));
        }

        let root_call_id = inner
            .nodes
            .get(&parent_call_id)
            .ok_or_else(|| format!("parent call_id:{parent_call_id} not found"))?
            .root_call_id;

        let node = CallNode {
            call_id,
            parent_call_id: Some(parent_call_id),
            root_call_id,
        };

        inner.nodes.insert(call_id, node);
        inner
            .children
            .entry(parent_call_id)
            .or_default()
            .push(call_id);
        inner.children.entry(call_id).or_default();

        Ok(())
    }

    pub fn allocate_interception_id(&self) -> InterceptionId {
        InterceptionId::new()
    }

    pub fn contains(&self, call_id: CallId) -> bool {
        self.inner
            .read()
            .expect("execution tree lock poisoned")
            .nodes
            .contains_key(&call_id)
    }

    pub fn get_node(&self, call_id: CallId) -> Option<CallNode> {
        self.inner
            .read()
            .expect("execution tree lock poisoned")
            .nodes
            .get(&call_id)
            .cloned()
    }

    pub fn relation(&self, subject: CallId, other: CallId) -> CallRelation {
        if subject == other {
            return CallRelation::Same;
        }

        let inner = self.inner.read().expect("execution tree lock poisoned");

        let Some(subject_node) = inner.nodes.get(&subject) else {
            return CallRelation::Unknown;
        };

        let Some(other_node) = inner.nodes.get(&other) else {
            return CallRelation::Unknown;
        };

        if subject_node.root_call_id != other_node.root_call_id {
            return CallRelation::Unrelated;
        }

        // Semantics preserved from the original implementation:
        // relation(subject, other) returns what `other` is relative to `subject`.
        if subject_node.parent_call_id == Some(other_node.call_id) {
            return CallRelation::Parent;
        }

        if other_node.parent_call_id == Some(subject_node.call_id) {
            return CallRelation::Child;
        }

        if Self::is_proper_ancestor_inner(&inner, other, subject) {
            return CallRelation::Ancestor;
        }

        if Self::is_proper_ancestor_inner(&inner, subject, other) {
            return CallRelation::Descendant;
        }

        if let (Some(subject_parent), Some(other_parent)) =
            (subject_node.parent_call_id, other_node.parent_call_id)
        {
            if subject_parent == other_parent {
                return CallRelation::Sibling;
            }
        }

        CallRelation::SameRoot
    }

    pub fn lineage(&self, call_id: CallId) -> Option<CallLineage> {
        let inner = self.inner.read().expect("execution tree lock poisoned");

        let node = inner.nodes.get(&call_id)?;

        let mut ancestors = Vec::new();
        let mut current = node.parent_call_id;
        let mut visited = HashSet::new();

        while let Some(parent_id) = current {
            // Defensive guard against corrupted/cyclic state.
            if !visited.insert(parent_id) {
                return None;
            }

            ancestors.push(parent_id);
            current = inner.nodes.get(&parent_id)?.parent_call_id;
        }

        ancestors.reverse();

        Some(CallLineage {
            root_call_id: node.root_call_id,
            call_id,
            ancestors,
        })
    }

    pub fn children(&self, call_id: CallId) -> Option<Vec<CallId>> {
        let inner = self.inner.read().expect("execution tree lock poisoned");

        if !inner.nodes.contains_key(&call_id) {
            return None;
        }

        Some(inner.children.get(&call_id).cloned().unwrap_or_default())
    }

    pub fn descendants(&self, call_id: CallId, max_depth: Option<u32>) -> Option<Vec<CallId>> {
        let inner = self.inner.read().expect("execution tree lock poisoned");

        if !inner.nodes.contains_key(&call_id) {
            return None;
        }

        if max_depth == Some(0) {
            return Some(Vec::new());
        }

        let mut result = Vec::new();
        let mut frontier = VecDeque::from([(call_id, 0u32)]);
        let mut visited = HashSet::new();

        visited.insert(call_id);

        while let Some((current, depth)) = frontier.pop_front() {
            let child_ids = inner.children.get(&current).cloned().unwrap_or_default();

            for child_id in child_ids {
                let child_depth = depth.saturating_add(1);

                if let Some(limit) = max_depth {
                    if child_depth > limit {
                        continue;
                    }
                }

                // Defensive guard against duplicate/cyclic edges.
                if !visited.insert(child_id) {
                    continue;
                }

                result.push(child_id);
                frontier.push_back((child_id, child_depth));
            }
        }

        Some(result)
    }

    fn is_proper_ancestor_inner(
        inner: &ExecutionTreeInner,
        ancestor: CallId,
        descendant: CallId,
    ) -> bool {
        let mut current = inner
            .nodes
            .get(&descendant)
            .and_then(|node| node.parent_call_id);

        let mut visited = HashSet::new();

        while let Some(parent_id) = current {
            if parent_id == ancestor {
                return true;
            }

            // Defensive guard against corrupted/cyclic state.
            if !visited.insert(parent_id) {
                return false;
            }

            current = inner
                .nodes
                .get(&parent_id)
                .and_then(|node| node.parent_call_id);
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_has_empty_lineage() {
        let tree = ExecutionTreeState::new();
        let root = CallId::new();

        tree.register_root(root).unwrap();

        let lineage = tree.lineage(root).unwrap();
        assert_eq!(lineage.root_call_id, root);
        assert_eq!(lineage.call_id, root);
        assert!(lineage.ancestors.is_empty());
    }

    #[test]
    fn nested_lineage_is_root_first() {
        let tree = ExecutionTreeState::new();
        let root = CallId::new();
        let child = CallId::new();
        let grandchild = CallId::new();

        tree.register_root(root).unwrap();
        tree.register_child(child, root).unwrap();
        tree.register_child(grandchild, child).unwrap();

        let lineage = tree.lineage(grandchild).unwrap();
        assert_eq!(lineage.ancestors, vec![root, child]);
    }

    #[test]
    fn relation_subject_other_semantics() {
        let tree = ExecutionTreeState::new();
        let root = CallId::new();
        let child = CallId::new();

        tree.register_root(root).unwrap();
        tree.register_child(child, root).unwrap();

        assert_eq!(tree.relation(child, root), CallRelation::Parent);
        assert_eq!(tree.relation(root, child), CallRelation::Child);
        assert_eq!(tree.relation(child, child), CallRelation::Same);
    }

    #[test]
    fn descendants_respects_max_depth_zero() {
        let tree = ExecutionTreeState::new();
        let root = CallId::new();
        let child = CallId::new();

        tree.register_root(root).unwrap();
        tree.register_child(child, root).unwrap();

        assert_eq!(tree.descendants(root, Some(0)), Some(vec![]));
        assert_eq!(tree.descendants(root, Some(1)), Some(vec![child]));
    }

    #[test]
    fn unknown_relation_returns_unknown() {
        let tree = ExecutionTreeState::new();
        let root = CallId::new();

        tree.register_root(root).unwrap();

        assert_eq!(tree.relation(root, CallId::new()), CallRelation::Unknown);
    }

    #[test]
    fn duplicate_root_registration_is_rejected() {
        let tree = ExecutionTreeState::new();
        let root = CallId::new();

        tree.register_root(root).unwrap();

        let err = tree.register_root(root).unwrap_err();
        assert!(err.contains("already registered"));

        let root_node = tree.get_node(root).unwrap();
        assert_eq!(root_node.parent_call_id, None);
        assert_eq!(root_node.root_call_id, root);
    }

    #[test]
    fn duplicate_child_registration_is_rejected() {
        let tree = ExecutionTreeState::new();
        let root = CallId::new();
        let child = CallId::new();

        tree.register_root(root).unwrap();
        tree.register_child(child, root).unwrap();

        let err = tree.register_child(child, root).unwrap_err();
        assert!(err.contains("already registered"));

        assert_eq!(tree.children(root), Some(vec![child]));
        assert_eq!(tree.descendants(root, None), Some(vec![child]));
    }

    #[test]
    fn existing_root_cannot_be_reparented_as_child() {
        let tree = ExecutionTreeState::new();
        let root = CallId::new();
        let other = CallId::new();

        tree.register_root(root).unwrap();
        tree.register_root(other).unwrap();

        let err = tree.register_child(root, other).unwrap_err();
        assert!(err.contains("already registered"));

        let root_node = tree.get_node(root).unwrap();
        assert_eq!(root_node.parent_call_id, None);
        assert_eq!(root_node.root_call_id, root);
    }

    #[test]
    fn existing_child_cannot_be_promoted_to_root() {
        let tree = ExecutionTreeState::new();
        let root = CallId::new();
        let child = CallId::new();

        tree.register_root(root).unwrap();
        tree.register_child(child, root).unwrap();

        let err = tree.register_root(child).unwrap_err();
        assert!(err.contains("already registered"));

        let child_node = tree.get_node(child).unwrap();
        assert_eq!(child_node.parent_call_id, Some(root));
        assert_eq!(child_node.root_call_id, root);
        assert_eq!(tree.children(root), Some(vec![child]));
    }

    #[test]
    fn descendants_are_breadth_first_and_in_insertion_order() {
        let tree = ExecutionTreeState::new();

        let root = CallId::new();
        let a = CallId::new();
        let b = CallId::new();
        let a1 = CallId::new();
        let b1 = CallId::new();

        tree.register_root(root).unwrap();
        tree.register_child(a, root).unwrap();
        tree.register_child(b, root).unwrap();
        tree.register_child(a1, a).unwrap();
        tree.register_child(b1, b).unwrap();

        assert_eq!(tree.descendants(root, None), Some(vec![a, b, a1, b1]));
    }
}
