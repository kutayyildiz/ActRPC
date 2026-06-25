use actrpc_core::{
    CallId, InterceptionId, MethodTarget,
    action::{ActionSpec, RequestedActionRecord},
    interception::InterceptionRequest,
    json_rpc::{
        JsonRpcError, JsonRpcErrorResponse, JsonRpcId, JsonRpcMessage, JsonRpcParams,
        JsonRpcRequest, JsonRpcResponse, JsonRpcSingleMessage, JsonRpcSuccessResponse,
        JsonRpcVersion,
    },
    participant::{Participant, ParticipantType},
};
use serde_json::{Value, json};

pub(super) fn action_record<A>(params: Value) -> RequestedActionRecord
where
    A: ActionSpec,
{
    RequestedActionRecord {
        kind: A::action_kind(),
        params: Some(params),
    }
}

pub(super) fn no_params_action_record<A>() -> RequestedActionRecord
where
    A: ActionSpec,
{
    RequestedActionRecord {
        kind: A::action_kind(),
        params: Some(serde_json::Value::Null),
    }
}

pub(super) fn dummy_request() -> InterceptionRequest {
    InterceptionRequest {
        origin: Participant {
            kind: ParticipantType::External,
            id: "test".to_owned(),
        },
        target: MethodTarget {
            provider: "test".to_owned(),
            method: "test".to_owned(),
        },
        message: request_message("test", None),
        call_id: CallId::new(),
        interception_id: InterceptionId::new(),
        resolved_action_history: vec![],
        ctx: Default::default(),
    }
}

pub(super) fn invocation_context(
    name: &str,
) -> actrpc_orchestrator::action::ActionInvocationContext {
    actrpc_orchestrator::action::ActionInvocationContext {
        interceptor_name: name.to_owned(),
    }
}

pub(super) fn request_message(method: &str, params: Option<JsonRpcParams>) -> JsonRpcMessage {
    JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
        jsonrpc: JsonRpcVersion::V2_0,
        id: JsonRpcId::Number(1_u64.into()),
        method: method.to_owned(),
        params,
    }))
}

pub(super) fn success_message(result: Value) -> JsonRpcMessage {
    JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Success(
        JsonRpcSuccessResponse {
            jsonrpc: JsonRpcVersion::V2_0,
            id: JsonRpcId::Number(1_u64.into()),
            result,
        },
    )))
}

pub(super) fn error_message(error: JsonRpcError) -> JsonRpcMessage {
    JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Error(
        JsonRpcErrorResponse {
            jsonrpc: JsonRpcVersion::V2_0,
            id: JsonRpcId::Number(1_u64.into()),
            error,
        },
    )))
}

pub(super) fn json_error(code: i32, message: &str) -> JsonRpcError {
    JsonRpcError {
        code,
        message: message.to_owned(),
        data: Some(json!({ "test": true })),
    }
}

pub(super) fn object_params(value: Value) -> JsonRpcParams {
    serde_json::from_value(value).unwrap()
}
