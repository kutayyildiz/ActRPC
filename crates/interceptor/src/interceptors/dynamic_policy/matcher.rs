use crate::interceptors::dynamic_policy::{error::DynamicPolicyError, scope::TargetSelector};
use actrpc_core::MethodTarget;
use globset::{Glob, GlobMatcher};

#[derive(Debug, Clone)]
pub struct TargetSelectorMatcher {
    provider: String,
    method: GlobMatcher,
}

impl TargetSelectorMatcher {
    pub fn compile(selector: &TargetSelector) -> Result<Self, DynamicPolicyError> {
        let glob = Glob::new(&selector.method)
            .map_err(|source| DynamicPolicyError::InvalidGlob { source })?;

        Ok(Self {
            provider: selector.provider.clone(),
            method: glob.compile_matcher(),
        })
    }

    pub fn matches(&self, target: &MethodTarget) -> bool {
        self.provider == target.provider && self.method.is_match(&target.method)
    }
}
