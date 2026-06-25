use actrpc_core::{
    CallId, CurrentExecutionContext, InterceptionId, MethodTarget,
    action::{ActionKind, ResolvedActionRecord},
    interception::InterceptionRequest,
    json_rpc::JsonRpcMessage,
    participant::{Participant, ParticipantType},
};
use actrpc_interceptor::interceptors::dynamic_policy::new_component;
use serde_json::json;

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
        ctx: Default::default(),
    }
}

pub fn external_origin(id: &str) -> Participant {
    Participant {
        kind: ParticipantType::External,
        id: id.to_owned(),
    }
}

pub fn interceptor_origin(id: &str) -> Participant {
    Participant {
        kind: ParticipantType::Interceptor,
        id: id.to_owned(),
    }
}

pub fn default_target(method: &str) -> MethodTarget {
    MethodTarget {
        provider: "test-provider".to_owned(),
        method: method.to_owned(),
    }
}

pub fn method_target(provider: &str, method: &str) -> MethodTarget {
    MethodTarget {
        provider: provider.to_owned(),
        method: method.to_owned(),
    }
}

pub fn resolved_query_current(current: CurrentExecutionContext) -> ResolvedActionRecord {
    ResolvedActionRecord {
        kind: ActionKind::from("query_execution_context"),
        params: Some(json!({ "query": { "kind": "current" } })),
        result: Ok(Some(
            serde_json::to_value(actrpc_core::ExecutionContextQueryResult::Current(current))
                .unwrap(),
        )),
    }
}

pub fn resolved_query_relation(
    subject: CallId,
    other: CallId,
    relation: actrpc_core::CallRelation,
) -> ResolvedActionRecord {
    ResolvedActionRecord {
        kind: ActionKind::from("query_execution_context"),
        params: Some(json!({
            "query": {
                "kind": "relation",
                "subject": subject,
                "other": other
            }
        })),
        result: Ok(Some(
            serde_json::to_value(actrpc_core::ExecutionContextQueryResult::Relation(relation))
                .unwrap(),
        )),
    }
}

pub fn current_context(
    call_id: CallId,
    root_call_id: CallId,
    parent_call_id: Option<CallId>,
    target: MethodTarget,
) -> CurrentExecutionContext {
    CurrentExecutionContext {
        origin: interceptor_origin("planner"),
        target,
        call_id,
        root_call_id,
        parent_call_id,
        interception_id: InterceptionId::new(),
    }
}

pub fn dynamic_policy_fixture()
-> actrpc_interceptor::interceptors::dynamic_policy::DynamicPolicyComponent {
    new_component()
}
