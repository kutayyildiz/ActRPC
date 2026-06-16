use crate::interceptors::dynamic_policy::{
    error::DynamicPolicyError,
    matcher::TargetSelectorMatcher,
    scope::{
        CreateScopeParams, CreateScopeResult, DynamicScope, GetScopeParams, ListScopesParams,
        ListScopesResult, ReleaseScopeParams, ReleaseScopeResult, ScopeId,
    },
};
use actrpc_core::{CallId, MethodTarget};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

struct StoredScope {
    scope: DynamicScope,
    matcher: TargetSelectorMatcher,
}

#[derive(Default)]
pub struct DynamicPolicyStore {
    inner: RwLock<HashMap<ScopeId, StoredScope>>,
}

impl DynamicPolicyStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    fn write_guard(&self) -> std::sync::RwLockWriteGuard<'_, HashMap<ScopeId, StoredScope>> {
        self.inner
            .write()
            .unwrap_or_else(|error| error.into_inner())
    }

    fn read_guard(&self) -> std::sync::RwLockReadGuard<'_, HashMap<ScopeId, StoredScope>> {
        self.inner.read().unwrap_or_else(|error| error.into_inner())
    }

    pub fn create_scope(
        &self,
        params: CreateScopeParams,
    ) -> Result<CreateScopeResult, DynamicPolicyError> {
        if params.allowed_method_targets.is_empty() {
            return Err(DynamicPolicyError::invalid_params(
                "allowed_method_targets must not be empty",
            ));
        }

        let matcher = TargetSelectorMatcher::compile(&params.target_selector)?;

        let scope_id = ScopeId::new();
        let scope = DynamicScope {
            scope_id,
            owner_call_id: params.owner_call_id,
            root_call_id: params.root_call_id,
            creator: params.creator,
            target_selector: params.target_selector,
            allowed_method_targets: params.allowed_method_targets,
            relation_mode: params.relation_mode,
            label: params.label,
            bound_call_id: None,
        };

        let mut guard = self.write_guard();
        guard.insert(scope_id, StoredScope { scope, matcher });

        Ok(CreateScopeResult { scope_id })
    }

    pub fn release_scope(
        &self,
        params: ReleaseScopeParams,
    ) -> Result<ReleaseScopeResult, DynamicPolicyError> {
        let mut guard = self.write_guard();

        let Some(stored) = guard.get(&params.scope_id) else {
            return Err(DynamicPolicyError::ScopeNotFound {
                scope_id: params.scope_id,
            });
        };

        if stored.scope.creator != params.creator {
            return Err(DynamicPolicyError::CreatorMismatch {
                scope_id: params.scope_id,
            });
        }

        guard.remove(&params.scope_id);
        Ok(ReleaseScopeResult { released: true })
    }

    pub fn get_scope(&self, params: GetScopeParams) -> Result<DynamicScope, DynamicPolicyError> {
        let guard = self.read_guard();
        let stored = guard
            .get(&params.scope_id)
            .ok_or(DynamicPolicyError::ScopeNotFound {
                scope_id: params.scope_id,
            })?;

        Ok(stored.scope.clone())
    }

    pub fn list_scopes(
        &self,
        params: ListScopesParams,
    ) -> Result<ListScopesResult, DynamicPolicyError> {
        let guard = self.read_guard();
        let scopes = guard
            .values()
            .map(|stored| &stored.scope)
            .filter(|scope| {
                params
                    .root_call_id
                    .is_none_or(|root| scope.root_call_id == Some(root))
            })
            .filter(|scope| {
                params
                    .owner_call_id
                    .is_none_or(|owner| scope.owner_call_id == owner)
            })
            .filter(|scope| {
                params
                    .label
                    .as_ref()
                    .is_none_or(|label| scope.label.as_deref() == Some(label.as_str()))
            })
            .filter(|scope| {
                params
                    .creator
                    .as_ref()
                    .is_none_or(|creator| &scope.creator == creator)
            })
            .cloned()
            .collect();

        Ok(ListScopesResult { scopes })
    }

    pub fn bind_scope(&self, scope_id: ScopeId, bound_call_id: CallId) {
        let mut guard = self.write_guard();
        if let Some(stored) = guard.get_mut(&scope_id) {
            if stored.scope.bound_call_id.is_none() {
                stored.scope.bound_call_id = Some(bound_call_id);
            }
        }
    }

    pub fn scopes_for_root(&self, root_call_id: CallId) -> Vec<DynamicScope> {
        let guard = self.read_guard();
        guard
            .values()
            .map(|stored| &stored.scope)
            .filter(|scope| scope.root_call_id.is_none_or(|root| root == root_call_id))
            .cloned()
            .collect()
    }

    pub fn matcher_for(&self, scope_id: ScopeId) -> Option<TargetSelectorMatcher> {
        let guard = self.read_guard();
        guard.get(&scope_id).map(|stored| stored.matcher.clone())
    }

    pub fn allows_target(scope: &DynamicScope, target: &MethodTarget) -> bool {
        scope
            .allowed_method_targets
            .iter()
            .any(|allowed| allowed == target)
    }
}
