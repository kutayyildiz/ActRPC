use crate::interceptors::dynamic_policy::{
    error::DynamicPolicyError,
    scope::{DynamicScope, ScopeId},
};
use actrpc_core::{CallId, MethodTarget};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Default)]
struct StoreInner {
    calls: HashMap<CallId, ScopeId>,
    scopes: HashMap<ScopeId, DynamicScope>,
}

#[derive(Default)]
pub struct DynamicPolicyStore {
    inner: RwLock<StoreInner>,
}

impl DynamicPolicyStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    fn write_guard(&self) -> std::sync::RwLockWriteGuard<'_, StoreInner> {
        self.inner
            .write()
            .unwrap_or_else(|error| error.into_inner())
    }

    fn read_guard(&self) -> std::sync::RwLockReadGuard<'_, StoreInner> {
        self.inner.read().unwrap_or_else(|error| error.into_inner())
    }

    pub fn scope_for_call(&self, call_id: CallId) -> Option<ScopeId> {
        self.read_guard().calls.get(&call_id).copied()
    }

    pub fn get_scope(&self, scope_id: ScopeId) -> Option<DynamicScope> {
        self.read_guard().scopes.get(&scope_id).cloned()
    }

    pub fn bind_call(&self, call_id: CallId, scope_id: ScopeId) {
        let mut guard = self.write_guard();
        guard.calls.insert(call_id, scope_id);
    }

    pub fn create_scope_for_call(
        &self,
        creating_call_id: CallId,
        root_call_id: CallId,
        allowed_method_targets: Vec<MethodTarget>,
    ) -> Result<ScopeId, DynamicPolicyError> {
        if allowed_method_targets.is_empty() {
            return Err(DynamicPolicyError::invalid_params(
                "allowed_method_targets must not be empty",
            ));
        }

        let scope_id = ScopeId::new();
        let scope = DynamicScope {
            scope_id,
            creating_call_id,
            root_call_id,
            allowed_method_targets,
        };

        let mut guard = self.write_guard();
        guard.scopes.insert(scope_id, scope);
        Ok(scope_id)
    }

    pub fn release_call(&self, call_id: CallId) {
        let mut guard = self.write_guard();
        guard.calls.remove(&call_id);
    }

    pub fn release_scope(&self, scope_id: ScopeId) {
        let mut guard = self.write_guard();
        guard.scopes.remove(&scope_id);
        guard.calls.retain(|_, bound_scope_id| *bound_scope_id != scope_id);
    }

    pub fn release_scopes_for_root(&self, root_call_id: CallId) {
        let mut guard = self.write_guard();
        let scope_ids: Vec<ScopeId> = guard
            .scopes
            .values()
            .filter(|scope| scope.root_call_id == root_call_id)
            .map(|scope| scope.scope_id)
            .collect();

        for scope_id in scope_ids {
            guard.scopes.remove(&scope_id);
        }

        let remaining_scopes = guard.scopes.keys().copied().collect::<std::collections::HashSet<_>>();
        guard
            .calls
            .retain(|_, scope_id| remaining_scopes.contains(scope_id));
    }

    pub fn scope_created_by_call(&self, call_id: CallId) -> Option<ScopeId> {
        let guard = self.read_guard();
        guard
            .scopes
            .values()
            .find(|scope| scope.creating_call_id == call_id)
            .map(|scope| scope.scope_id)
    }

    pub fn allows_target(scope: &DynamicScope, target: &MethodTarget) -> bool {
        scope
            .allowed_method_targets
            .iter()
            .any(|allowed| allowed == target)
    }

    pub fn is_subset(requested: &[MethodTarget], parent: &[MethodTarget]) -> bool {
        requested
            .iter()
            .all(|target| parent.iter().any(|allowed| allowed == target))
    }
}