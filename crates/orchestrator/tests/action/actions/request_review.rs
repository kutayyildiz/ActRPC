use actrpc_core::action::ActionSpec;
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

use super::super::helpers::{action_record, dummy_request};

struct ApprovingReviewProvider;

impl ReviewProvider for ApprovingReviewProvider {
    fn request_review<'a>(
        &'a self,
        _params: RequestReviewParams,
    ) -> ActionHandlerFuture<'a, Result<RequestReviewResult, ActionExecutionError>> {
        Box::pin(async move { Ok(RequestReviewResult::approved()) })
    }
}

struct DenyingReviewProvider;

impl ReviewProvider for DenyingReviewProvider {
    fn request_review<'a>(
        &'a self,
        _params: RequestReviewParams,
    ) -> ActionHandlerFuture<'a, Result<RequestReviewResult, ActionExecutionError>> {
        Box::pin(async move { Ok(RequestReviewResult::denied()) })
    }
}

#[tokio::test]
async fn request_review_handler_returns_approved_provider_decision() {
    let mut registry = ActionRegistry::new();

    registry
        .register::<RequestReview, _>(RequestReviewHandler::new(Arc::new(ApprovingReviewProvider)))
        .unwrap();

    let action = action_record::<RequestReview>(json!({
        "rule_name": "review_sensitive_write",
        "title": "Sensitive file write",
        "reason": "Agent wants to write inside a user-owned directory.",
        "severity": "high"
    }));

    let resolved = registry
        .get(&RequestReview::action_kind())
        .unwrap()
        .handle(&dummy_request(), action)
        .await
        .unwrap();

    assert_eq!(resolved.kind, RequestReview::action_kind());
    assert_eq!(
        resolved.params,
        Some(json!({
            "rule_name": "review_sensitive_write",
            "title": "Sensitive file write",
            "reason": "Agent wants to write inside a user-owned directory.",
            "severity": "high"
        }))
    );
    assert_eq!(
        resolved.result,
        Ok(Some(json!({
            "decision": "approved"
        })))
    );
}

#[tokio::test]
async fn request_review_handler_returns_denied_provider_decision() {
    let mut registry = ActionRegistry::new();

    registry
        .register::<RequestReview, _>(RequestReviewHandler::new(Arc::new(DenyingReviewProvider)))
        .unwrap();

    let action = action_record::<RequestReview>(json!({
        "rule_name": "dangerous_call",
        "title": "Dangerous call",
        "reason": "Policy requires user approval.",
        "severity": "medium"
    }));

    let resolved = registry
        .get(&RequestReview::action_kind())
        .unwrap()
        .handle(&dummy_request(), action)
        .await
        .unwrap();

    assert_eq!(
        resolved.result,
        Ok(Some(json!({
            "decision": "denied"
        })))
    );
}

#[test]
fn request_review_result_helpers_match_decision_strings() {
    let approved = RequestReviewResult::approved();
    let denied = RequestReviewResult::denied();

    assert!(approved.is_approved());
    assert!(!approved.is_denied());

    assert!(denied.is_denied());
    assert!(!denied.is_approved());
}
