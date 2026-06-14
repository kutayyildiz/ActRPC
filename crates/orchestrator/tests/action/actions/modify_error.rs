use actrpc_core::{
    action::ActionSpec,
    json_rpc::{JsonRpcMessage, JsonRpcResponse, JsonRpcSingleMessage},
};
use actrpc_orchestrator::{
    action::{
        ActionRegistry,
        actions::modify_error::{ModifyError, ModifyErrorHandler},
    },
    error::{ActionExecutionError, ActionHandlerError},
    runtime::InFlightMessageState,
};
use serde_json::json;
use std::sync::Arc;

use super::super::helpers::{
    action_record, dummy_request, error_message, invocation_context, json_error, success_message,
};

#[tokio::test]
async fn modify_error_replaces_error_response_error() {
    let state = Arc::new(InFlightMessageState::new());
    state.set_message(error_message(json_error(-32000, "old")));

    let mut registry = ActionRegistry::new();
    registry
        .register::<ModifyError, _>(ModifyErrorHandler::new(state.clone()))
        .unwrap();

    let action = action_record::<ModifyError>(json!({
        "error": {
            "code": -32001,
            "message": "new",
            "data": {
                "reason": "changed"
            }
        }
    }));

    let resolved = registry
        .get(&ModifyError::action_kind())
        .unwrap()
        .handle(&dummy_request(), action, &invocation_context("test"))
        .await
        .unwrap();

    assert_eq!(resolved.result, Ok(Some(json!(null))));

    let JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Error(
        error_response,
    ))) = state.snapshot().unwrap()
    else {
        panic!("expected error response");
    };

    assert_eq!(error_response.error.code, -32001);
    assert_eq!(error_response.error.message, "new");
    assert_eq!(
        error_response.error.data,
        Some(json!({ "reason": "changed" }))
    );
}

#[tokio::test]
async fn modify_error_rejects_non_error_response() {
    let state = Arc::new(InFlightMessageState::new());
    state.set_message(success_message(json!("ok")));

    let mut registry = ActionRegistry::new();
    registry
        .register::<ModifyError, _>(ModifyErrorHandler::new(state))
        .unwrap();

    let action = action_record::<ModifyError>(json!({
        "error": {
            "code": -32001,
            "message": "new"
        }
    }));

    let err = registry
        .get(&ModifyError::action_kind())
        .unwrap()
        .handle(&dummy_request(), action, &invocation_context("test"))
        .await
        .unwrap_err();

    match err {
        ActionHandlerError::Execution(ActionExecutionError::InvalidParams { action }) => {
            assert_eq!(action, ModifyError::action_kind());
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn modify_error_rejects_missing_in_flight_message() {
    let state = Arc::new(InFlightMessageState::new());

    let mut registry = ActionRegistry::new();
    registry
        .register::<ModifyError, _>(ModifyErrorHandler::new(state))
        .unwrap();

    let action = action_record::<ModifyError>(json!({
        "error": {
            "code": -32001,
            "message": "new"
        }
    }));

    let err = registry
        .get(&ModifyError::action_kind())
        .unwrap()
        .handle(&dummy_request(), action, &invocation_context("test"))
        .await
        .unwrap_err();

    match err {
        ActionHandlerError::Execution(ActionExecutionError::InvalidState { .. }) => {}
        other => panic!("unexpected error: {other:?}"),
    }
}
