use actrpc_core::{
    action::ActionSpec,
    json_rpc::{JsonRpcMessage, JsonRpcResponse, JsonRpcSingleMessage},
};
use actrpc_orchestrator::{
    action::{
        ActionRegistry,
        actions::modify_result::{ModifyResult, ModifyResultHandler},
    },
    error::{ActionExecutionError, ActionHandlerError},
    runtime::InFlightMessageState,
};
use serde_json::json;
use std::sync::Arc;

use super::super::helpers::{action_record, dummy_request, request_message, success_message};

#[tokio::test]
async fn modify_result_replaces_success_response_result() {
    let state = Arc::new(InFlightMessageState::new());
    state.set_message(success_message(json!({ "old": true })));

    let mut registry = ActionRegistry::new();
    registry
        .register::<ModifyResult, _>(ModifyResultHandler::new(state.clone()))
        .unwrap();

    let action = action_record::<ModifyResult>(json!({
        "result": {
            "new": true
        }
    }));

    let resolved = registry
        .get(&ModifyResult::action_kind())
        .unwrap()
        .handle(&dummy_request(), action)
        .await
        .unwrap();

    assert_eq!(resolved.result, Ok(Some(json!(null))));

    let JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Success(success))) =
        state.snapshot().unwrap()
    else {
        panic!("expected success response");
    };

    assert_eq!(success.result, json!({ "new": true }));
}

#[tokio::test]
async fn modify_result_rejects_non_success_response() {
    let state = Arc::new(InFlightMessageState::new());
    state.set_message(request_message("sum", None));

    let mut registry = ActionRegistry::new();
    registry
        .register::<ModifyResult, _>(ModifyResultHandler::new(state))
        .unwrap();

    let action = action_record::<ModifyResult>(json!({
        "result": "new"
    }));

    let err = registry
        .get(&ModifyResult::action_kind())
        .unwrap()
        .handle(&dummy_request(), action)
        .await
        .unwrap_err();

    match err {
        ActionHandlerError::Execution(ActionExecutionError::InvalidParams { action }) => {
            assert_eq!(action, ModifyResult::action_kind());
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn modify_result_rejects_missing_in_flight_message() {
    let state = Arc::new(InFlightMessageState::new());

    let mut registry = ActionRegistry::new();
    registry
        .register::<ModifyResult, _>(ModifyResultHandler::new(state))
        .unwrap();

    let action = action_record::<ModifyResult>(json!({
        "result": "new"
    }));

    let err = registry
        .get(&ModifyResult::action_kind())
        .unwrap()
        .handle(&dummy_request(), action)
        .await
        .unwrap_err();

    match err {
        ActionHandlerError::Execution(ActionExecutionError::InvalidState { .. }) => {}
        other => panic!("unexpected error: {other:?}"),
    }
}
