use actrpc_core::{CallId, MethodTarget};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};
use uuid::Uuid;

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
    pub creating_call_id: CallId,
    pub root_call_id: CallId,
    pub allowed_method_targets: Vec<MethodTarget>,
}
