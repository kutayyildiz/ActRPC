use crate::{
    endpoint::EndpointName,
    interceptor::{InterceptorPolicy, runtime_limits::InterceptorRuntimeLimitsOverride},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterceptorConfig {
    pub name: String,
    pub policy: InterceptorPolicy,
    pub endpoint: EndpointName,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<InterceptorRuntimeLimitsOverride>,
}
