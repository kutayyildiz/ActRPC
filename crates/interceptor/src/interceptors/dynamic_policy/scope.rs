use actrpc_core::{CallId, MethodTarget, participant::Participant};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};
use uuid::Uuid;

pub const PROVIDER_NAME: &str = "dynamic_policy";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationMode {
    DirectChild,
    Descendant,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TargetSelector {
    pub provider: String,
    pub method: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ScopeId(pub Uuid);

impl ScopeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ScopeId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for ScopeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for ScopeId {
    type Err = uuid::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(value)?))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DynamicScope {
    pub scope_id: ScopeId,
    pub owner_call_id: CallId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_call_id: Option<CallId>,
    pub creator: Participant,
    pub target_selector: TargetSelector,
    pub allowed_method_targets: Vec<MethodTarget>,
    pub relation_mode: RelationMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound_call_id: Option<CallId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateScopeParams {
    pub owner_call_id: CallId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_call_id: Option<CallId>,
    pub creator: Participant,
    pub target_selector: TargetSelector,
    pub allowed_method_targets: Vec<MethodTarget>,
    pub relation_mode: RelationMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateScopeResult {
    pub scope_id: ScopeId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReleaseScopeParams {
    pub scope_id: ScopeId,
    pub creator: Participant,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReleaseScopeResult {
    pub released: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetScopeParams {
    pub scope_id: ScopeId,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ListScopesParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_call_id: Option<CallId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_call_id: Option<CallId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creator: Option<Participant>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListScopesResult {
    pub scopes: Vec<DynamicScope>,
}
