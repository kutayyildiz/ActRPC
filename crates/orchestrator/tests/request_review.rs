use actrpc_core::{
    action::{ActionSpec, RequestedActionRecord},
    interception::InterceptionRequest,
    json_rpc::{JsonRpcId, JsonRpcMessage, JsonRpcRequest, JsonRpcSingleMessage, JsonRpcVersion},
    participant::{Participant, ParticipantType},
};
use actrpc_orchestrator::{
    action::{
        ActionHandlerFuture, ActionRegistry,
        actions::request_review::{
            RequestReview, RequestReviewHandler, RequestReviewParams, RequestReviewResult,
        },
    },
    error::ActionExecutionError,
    review::ReviewProvider,
};
use serde_json::json;
use std::sync::Arc;

struct ApprovingReviewProvider;

impl ReviewProvider for ApprovingReviewProvider {
    fn request_review<'a>(
        &'a self,
        _params: RequestReviewParams,
    ) -> ActionHandlerFuture<'a, Result<RequestReviewResult, ActionExecutionError>> {
        Box::pin(async move { Ok(RequestReviewResult::approved()) })
    }
}

#[tokio::test]
async fn request_review_handler_returns_provider_decision() {
    let mut registry = ActionRegistry::new();

    registry
        .register::<RequestReview, _>(RequestReviewHandler::new(Arc::new(ApprovingReviewProvider)))
        .unwrap();

    let request = dummy_request();

    let action = RequestedActionRecord {
        kind: RequestReview::action_kind(),
        params: Some(json!({
            "title": "Sensitive file write",
            "reason": "Agent wants to write inside a user-owned directory.",
            "severity": "high"
        })),
    };

    let resolved = registry
        .get(&RequestReview::action_kind())
        .unwrap()
        .handle(&request, action)
        .await
        .unwrap();

    assert_eq!(resolved.kind, RequestReview::action_kind());
    assert_eq!(
        resolved.result,
        Ok(Some(json!({
            "decision": "approved"
        })))
    );
}

fn dummy_request() -> InterceptionRequest {
    InterceptionRequest {
        origin: Participant {
            kind: ParticipantType::Orchestrator,
            id: "test".to_owned(),
        },
        message: JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
            jsonrpc: JsonRpcVersion::V2_0,
            id: JsonRpcId::Number(1.into()),
            method: "test".to_owned(),
            params: None,
        })),
        resolved_action_history: vec![],
    }
}
