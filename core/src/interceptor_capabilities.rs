use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct InterceptorCapabilities {
    #[serde(default)]
    pub supports_outbound: bool,

    #[serde(default)]
    pub supports_inbound: bool,

    #[serde(default)]
    pub supports_batch: bool,

    #[serde(default)]
    pub supports_notifications: bool,
}
