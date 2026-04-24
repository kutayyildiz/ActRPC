use crate::action::{ActionKind, action_descriptor::ActionDescriptor};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct InterceptorInitialization {
    #[serde(default)]
    pub supports_outbound: bool,
    #[serde(default)]
    pub supports_inbound: bool,
    #[serde(default)]
    pub actions: HashMap<ActionKind, ActionDescriptor>,
}
