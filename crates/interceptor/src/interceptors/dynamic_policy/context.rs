use actrpc_core::MethodTarget;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DynamicPolicyContext {
    pub mode: DynamicPolicyContextMode,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_method_targets: Vec<MethodTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DynamicPolicyContextMode {
    Detached,
}
