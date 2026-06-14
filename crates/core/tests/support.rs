use actrpc_core::{
    CallId, InterceptionId, MethodTarget,
    interception::InterceptionRequest,
    json_rpc::{
        JsonRpcId, JsonRpcMessage, JsonRpcParams, JsonRpcRequest, JsonRpcSingleMessage,
        JsonRpcVersion,
    },
    participant::{Participant, ParticipantType},
};

pub fn sample_request(
    origin: Participant,
    message: JsonRpcMessage,
    resolved_action_history: Vec<Vec<actrpc_core::action::ResolvedActionRecord>>,
) -> InterceptionRequest {
    InterceptionRequest {
        origin,
        target: MethodTarget {
            provider: "test-provider".to_owned(),
            method: "test-method".to_owned(),
        },
        message,
        call_id: CallId::new(),
        interception_id: InterceptionId::new(),
        resolved_action_history,
    }
}

pub fn external_origin(id: &str) -> Participant {
    Participant {
        kind: ParticipantType::External,
        id: id.to_owned(),
    }
}

#[allow(dead_code)]
pub fn request_message(method: &str, params: Option<JsonRpcParams>) -> JsonRpcMessage {
    JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
        jsonrpc: JsonRpcVersion::V2_0,
        id: JsonRpcId::Number(1.into()),
        method: method.to_owned(),
        params,
    }))
}
