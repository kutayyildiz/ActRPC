use actrpc_core::{
    CallId, InterceptionId, MethodTarget,
    interception::InterceptionRequest,
    json_rpc::JsonRpcMessage,
    participant::{Participant, ParticipantType},
};

pub fn sample_request(
    origin: Participant,
    target: MethodTarget,
    message: JsonRpcMessage,
    resolved_action_history: Vec<Vec<actrpc_core::action::ResolvedActionRecord>>,
) -> InterceptionRequest {
    InterceptionRequest {
        origin,
        target,
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

pub fn default_target(method: &str) -> MethodTarget {
    MethodTarget {
        provider: "test-provider".to_owned(),
        method: method.to_owned(),
    }
}
