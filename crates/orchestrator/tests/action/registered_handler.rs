use actrpc_core::{
    action::{ActionSpec, RequestedAction, RequestedActionRecord, ResolvedAction},
    interception::InterceptionRequest,
};
use actrpc_orchestrator::{
    action::{ActionHandlerFuture, ActionInvocationContext, ActionRegistry, TypedActionHandler},
    error::{ActionExecutionError, ActionHandlerError},
};
use serde_json::json;

use super::helpers::{dummy_request, invocation_context};

struct EchoAction;

impl ActionSpec for EchoAction {
    type Params = String;
    type Result = String;

    const KIND: &'static str = "echo_registered";
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
                result: Ok(format!("echo:{}", action.params)),
            })
        })
    }
}

#[tokio::test]
async fn registered_handler_decodes_typed_action_and_encodes_result_record() {
    let mut registry = ActionRegistry::new();
    registry.register::<EchoAction, _>(EchoHandler).unwrap();

    let action = RequestedActionRecord {
        kind: EchoAction::action_kind(),
        params: Some(json!("hello")),
    };

    let resolved = registry
        .get(&EchoAction::action_kind())
        .unwrap()
        .handle(&dummy_request(), action, &invocation_context("test"))
        .await
        .unwrap();

    assert_eq!(resolved.kind, EchoAction::action_kind());
    assert_eq!(resolved.params, Some(json!("hello")));
    assert_eq!(resolved.result, Ok(Some(json!("echo:hello"))));
}

#[tokio::test]
async fn registered_handler_rejects_malformed_params() {
    let mut registry = ActionRegistry::new();
    registry.register::<EchoAction, _>(EchoHandler).unwrap();

    let action = RequestedActionRecord {
        kind: EchoAction::action_kind(),
        params: Some(json!({ "not": "a string" })),
    };

    let err = registry
        .get(&EchoAction::action_kind())
        .unwrap()
        .handle(&dummy_request(), action, &invocation_context("test"))
        .await
        .unwrap_err();

    match err {
        ActionHandlerError::ActionCodec(_) => {}
        other => panic!("unexpected error: {other:?}"),
    }
}
