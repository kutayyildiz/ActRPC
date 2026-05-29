use actrpc_core::{
    action::ActionSpec,
    json_rpc::{JsonRpcMessage, JsonRpcSingleMessage},
};
use actrpc_orchestrator::{
    action::{
        ActionRegistry,
        actions::modify_params::{ModifyParams, ModifyParamsHandler},
    },
    error::{ActionExecutionError, ActionHandlerError},
    runtime::InFlightMessageState,
};
use serde_json::json;
use std::sync::Arc;

use super::super::helpers::{
    action_record, dummy_request, object_params, request_message, success_message,
};

#[tokio::test]
async fn modify_params_replaces_request_params() {
    let state = Arc::new(InFlightMessageState::new());
    state.set_message(request_message("sum", None));

    let mut registry = ActionRegistry::new();
    registry
        .register::<ModifyParams, _>(ModifyParamsHandler::new(state.clone()))
        .unwrap();

    let action = action_record::<ModifyParams>(json!({
        "params": {
            "x": 1,
            "y": 2
        }
    }));

    let resolved = registry
        .get(&ModifyParams::action_kind())
        .unwrap()
        .handle(&dummy_request(), action)
        .await
        .unwrap();

    assert_eq!(resolved.result, Ok(Some(json!(null))));

    let JsonRpcMessage::Single(JsonRpcSingleMessage::Request(request)) = state.snapshot().unwrap()
    else {
        panic!("expected request message");
    };

    assert_eq!(
        request.params,
        Some(object_params(json!({
            "x": 1,
            "y": 2
        })))
    );
}

#[tokio::test]
async fn modify_params_can_clear_request_params() {
    let state = Arc::new(InFlightMessageState::new());
    state.set_message(request_message(
        "sum",
        Some(object_params(json!({ "old": true }))),
    ));

    let mut registry = ActionRegistry::new();
    registry
        .register::<ModifyParams, _>(ModifyParamsHandler::new(state.clone()))
        .unwrap();

    let action = action_record::<ModifyParams>(json!({
        "params": null
    }));

    registry
        .get(&ModifyParams::action_kind())
        .unwrap()
        .handle(&dummy_request(), action)
        .await
        .unwrap();

    let JsonRpcMessage::Single(JsonRpcSingleMessage::Request(request)) = state.snapshot().unwrap()
    else {
        panic!("expected request message");
    };

    assert_eq!(request.params, None);
}

#[tokio::test]
async fn modify_params_rejects_non_request_message() {
    let state = Arc::new(InFlightMessageState::new());
    state.set_message(success_message(json!("ok")));

    let mut registry = ActionRegistry::new();
    registry
        .register::<ModifyParams, _>(ModifyParamsHandler::new(state))
        .unwrap();

    let action = action_record::<ModifyParams>(json!({
        "params": {
            "x": 1
        }
    }));

    let err = registry
        .get(&ModifyParams::action_kind())
        .unwrap()
        .handle(&dummy_request(), action)
        .await
        .unwrap_err();

    match err {
        ActionHandlerError::Execution(ActionExecutionError::InvalidParams { action }) => {
            assert_eq!(action, ModifyParams::action_kind());
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn modify_params_rejects_missing_in_flight_message() {
    let state = Arc::new(InFlightMessageState::new());

    let mut registry = ActionRegistry::new();
    registry
        .register::<ModifyParams, _>(ModifyParamsHandler::new(state))
        .unwrap();

    let action = action_record::<ModifyParams>(json!({
        "params": {
            "x": 1
        }
    }));

    let err = registry
        .get(&ModifyParams::action_kind())
        .unwrap()
        .handle(&dummy_request(), action)
        .await
        .unwrap_err();

    match err {
        ActionHandlerError::Execution(ActionExecutionError::InvalidState { .. }) => {}
        other => panic!("unexpected error: {other:?}"),
    }
}
