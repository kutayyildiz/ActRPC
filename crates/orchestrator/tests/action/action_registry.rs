use actrpc_core::{
    CallId, InterceptionId, MethodTarget,
    action::{ActionKind, ActionSpec, RequestedAction, RequestedActionRecord, ResolvedAction},
    interception::InterceptionRequest,
    json_rpc::{JsonRpcId, JsonRpcMessage, JsonRpcRequest, JsonRpcSingleMessage, JsonRpcVersion},
    participant::{Participant, ParticipantType},
};
use actrpc_orchestrator::{
    action::{ActionHandlerFuture, ActionInvocationContext, ActionRegistry, TypedActionHandler},
    error::{ActionError, ActionExecutionError, ActionHandlerError, OrchestratorError},
};
use serde_json::json;

struct EchoAction;

impl ActionSpec for EchoAction {
    type Params = String;
    type Result = String;

    const KIND: &'static str = "echo";
}

struct EchoHandler;

impl TypedActionHandler<EchoAction> for EchoHandler {
    fn handle_typed<'a>(
        &'a self,
        _request: &'a InterceptionRequest,
        action: RequestedAction<EchoAction>,
        _ctx: &'a ActionInvocationContext,
    ) -> ActionHandlerFuture<'a, Result<ResolvedAction<EchoAction>, ActionExecutionError>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            Ok(ResolvedAction {
                params: action.params.clone(),
                result: Ok(action.params),
            })
        })
    }
}

#[test]
fn action_registry_starts_empty() {
    let registry = ActionRegistry::new();

    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);
    assert!(!registry.contains(&ActionKind::from("echo")));
}

#[test]
fn action_registry_registers_handler() {
    let mut registry = ActionRegistry::new();

    registry.register::<EchoAction, _>(EchoHandler).unwrap();

    let kind = EchoAction::action_kind();

    assert!(!registry.is_empty());
    assert_eq!(registry.len(), 1);
    assert!(registry.contains(&kind));

    let handler = registry.get(&kind).unwrap();
    assert_eq!(handler.kind(), kind);
}

#[test]
fn action_registry_rejects_duplicate_registration() {
    let mut registry = ActionRegistry::new();

    registry.register::<EchoAction, _>(EchoHandler).unwrap();

    let err = registry.register::<EchoAction, _>(EchoHandler).unwrap_err();

    match err {
        OrchestratorError::Action(ActionError::DuplicateRegistration { kind }) => {
            assert_eq!(kind, EchoAction::action_kind());
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn registered_action_handler_decodes_executes_and_encodes() {
    let mut registry = ActionRegistry::new();
    registry.register::<EchoAction, _>(EchoHandler).unwrap();

    let handler = registry.get(&EchoAction::action_kind()).unwrap();

    let request = dummy_request();
    let ctx = ActionInvocationContext {
        interceptor_name: "test".to_owned(),
    };

    let action = RequestedActionRecord {
        kind: EchoAction::action_kind(),
        params: Some(json!("hello")),
    };

    let resolved = handler.handle(&request, action, &ctx).await.unwrap();

    assert_eq!(resolved.kind, EchoAction::action_kind());
    assert_eq!(resolved.params, Some(json!("hello")));
    assert_eq!(resolved.result, Ok(Some(json!("hello"))));
}

#[tokio::test]
async fn registered_action_handler_rejects_wrong_action_kind() {
    let mut registry = ActionRegistry::new();
    registry.register::<EchoAction, _>(EchoHandler).unwrap();

    let handler = registry.get(&EchoAction::action_kind()).unwrap();

    let request = dummy_request();
    let ctx = ActionInvocationContext {
        interceptor_name: "test".to_owned(),
    };

    let action = RequestedActionRecord {
        kind: ActionKind::from("wrong"),
        params: Some(json!("hello")),
    };

    let err = handler.handle(&request, action, &ctx).await.unwrap_err();

    match err {
        ActionHandlerError::ActionCodec(_) => {}
        other => panic!("unexpected error: {other:?}"),
    }
}

fn dummy_request() -> InterceptionRequest {
    InterceptionRequest {
        origin: Participant {
            kind: ParticipantType::External,
            id: "test".to_owned(),
        },
        target: MethodTarget {
            provider: "test".to_owned(),
            method: "test".to_owned(),
        },
        message: JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
            jsonrpc: JsonRpcVersion::V2_0,
            id: JsonRpcId::Number(1.into()),
            method: "test".to_owned(),
            params: None,
        })),
        call_id: CallId::new(),
        interception_id: InterceptionId::new(),
        resolved_action_history: Default::default(),
        ctx: Default::default(),
    }
}
