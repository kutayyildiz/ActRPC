use crate::{InterceptorInitialization, action::ActionKind};
use actrpc_core_macros::DescribeValue;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, DescribeValue)]
#[serde(deny_unknown_fields)]
pub struct InterceptorCapabilities {
    #[serde(default)]
    pub supports_outbound: bool,

    #[serde(default)]
    pub supports_inbound: bool,

    #[serde(default)]
    pub supported_actions: HashSet<ActionKind>,
}

impl From<&InterceptorInitialization> for InterceptorCapabilities {
    fn from(value: &InterceptorInitialization) -> Self {
        Self {
            supports_outbound: value.supports_outbound,
            supports_inbound: value.supports_inbound,
            supported_actions: value.actions.keys().cloned().collect(),
        }
    }
}

impl From<InterceptorInitialization> for InterceptorCapabilities {
    fn from(value: InterceptorInitialization) -> Self {
        Self {
            supports_outbound: value.supports_outbound,
            supports_inbound: value.supports_inbound,
            supported_actions: value.actions.into_keys().collect(),
        }
    }
}
