use crate::{
    action::{
        ActionKind, ActionSpec,
        types::{CallExternalMethodParams, ExternalMethodInfo, NoOutput, NoParams},
    },
    interceptor::{InterceptorPolicy, InterceptorState},
    json_rpc::{JsonRpcError, JsonRpcParams, JsonRpcResponse},
    transcript::TranscriptEntry,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[macro_export]
macro_rules! impl_action_spec {
    ($action_name:ident, $params:ty, $result:ty) => {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub struct $action_name;

        impl ActionSpec for $action_name {
            type Params = $params;
            type Result = $result;
            const KIND: ActionKind = ActionKind::$action_name;
        }
    };

    ($($action_name:ident => ($params:ty, $result:ty)),* $(,)?) => {
        $(
            impl_action_spec!($action_name, $params, $result);
        )*
    };
}

impl_action_spec! {
    ModifyParams => (JsonRpcParams, NoOutput),
    ModifyResult => (Value, NoOutput),
    ModifyError => (JsonRpcError, NoOutput),
    CallExternalMethod => (CallExternalMethodParams, JsonRpcResponse),
    ListExternalMethods => (NoParams,Vec<ExternalMethodInfo>),
    GetInterceptorOrder=> (NoParams,Vec<String>),
    GetInterceptorState=> (NoParams,Vec<InterceptorState>),
    GetInterceptorPolicy=> (NoParams,Vec<InterceptorPolicy>),
    EnableInterceptors=> (Vec<String>,NoOutput),
    DisableInterceptors=> (Vec<String>,NoOutput),
    GetTranscript=> (NoParams,Vec<TranscriptEntry>),
}
