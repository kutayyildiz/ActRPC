use actrpc_core::action::ActionSpec;
use actrpc_orchestrator::{
    action::{
        ActionRegistry,
        actions::reject_call::{RejectCall, RejectCallHandler},
    },
    runtime::CurrentCallRejection,
};
use serde_json::json;
use std::sync::Arc;

use super::super::helpers::{action_record, dummy_request};

#[tokio::test]
async fn reject_call_sets_current_call_rejection() {
    let rejection = Arc::new(CurrentCallRejection::new());

    let mut registry = ActionRegistry::new();
    registry
        .register::<RejectCall, _>(RejectCallHandler::new(rejection.clone()))
        .unwrap();

    let action = action_record::<RejectCall>(json!({
        "error": {
            "code": -32000,
            "message": "blocked",
            "data": {
                "reason": "policy"
            }
        }
    }));

    let resolved = registry
        .get(&RejectCall::action_kind())
        .unwrap()
        .handle(&dummy_request(), action)
        .await
        .unwrap();

    assert_eq!(resolved.result, Ok(Some(json!(null))));
    assert!(rejection.is_rejected());

    let error = rejection.snapshot().unwrap();
    assert_eq!(error.code, -32000);
    assert_eq!(error.message, "blocked");
    assert_eq!(error.data, Some(json!({ "reason": "policy" })));
}
