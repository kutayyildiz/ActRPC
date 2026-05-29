use crate::interceptor::InterceptorPolicy;
use actrpc_transport::TransportTarget;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterceptorConfig {
    pub name: String,
    pub policy: InterceptorPolicy,
    pub target: TransportTarget,
}
