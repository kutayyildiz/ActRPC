use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::json_rpc::JsonRpcError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", content = "action_params", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Action {
    /// Replaces or rewrites params of the currently intercepted message.
    ModifyParams(Value),

    /// Replaces or rewrites result of the currently intercepted message.
    ModifyResult(Value),

    /// Replaces or rewrites errorof the currently intercepted message.
    ModifyError(JsonRpcError),

    /// Calls an orchestrator-known external method.
    CallExternalMethod {
        method_name: String,
        method_params: Value,
    },

    /// Returns available external methods.
    ListExternalMethods,

    /// Returns current interceptor order.
    GetInterceptorOrder,

    /// Returns current interceptor enabled/executed state.
    GetInterceptorState,

    /// Returns current interceptor policy.
    GetInterceptorPolicy,

    /// Enables the named interceptors.
    EnableInterceptors(Vec<String>),

    /// Disables the named interceptors.
    DisableInterceptors(Vec<String>),

    /// Returns transcript records.
    GetTranscript,
}
