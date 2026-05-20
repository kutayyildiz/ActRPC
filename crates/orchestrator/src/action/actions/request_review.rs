use crate::{
    action::{ActionHandlerFuture, TypedActionHandler},
    error::ActionExecutionError,
    review::ReviewProvider,
};
use actrpc_core::{
    DescribeOk, DescribeParams,
    action::{ActionSpec, RequestedAction, ResolvedAction},
    interception::InterceptionRequest,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const REVIEW_SEVERITY_LOW: &str = "low";
pub const REVIEW_SEVERITY_MEDIUM: &str = "medium";
pub const REVIEW_SEVERITY_HIGH: &str = "high";

pub const REVIEW_DECISION_APPROVED: &str = "approved";
pub const REVIEW_DECISION_DENIED: &str = "denied";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeParams)]
#[serde(deny_unknown_fields)]
pub struct RequestReviewParams {
    pub title: String,
    pub reason: String,
    pub severity: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeOk)]
#[serde(deny_unknown_fields)]
pub struct RequestReviewResult {
    pub decision: String,
}

impl RequestReviewResult {
    pub fn approved() -> Self {
        Self {
            decision: REVIEW_DECISION_APPROVED.to_owned(),
        }
    }

    pub fn denied() -> Self {
        Self {
            decision: REVIEW_DECISION_DENIED.to_owned(),
        }
    }

    pub fn is_approved(&self) -> bool {
        self.decision == REVIEW_DECISION_APPROVED
    }

    pub fn is_denied(&self) -> bool {
        self.decision == REVIEW_DECISION_DENIED
    }
}

pub struct RequestReview;

impl ActionSpec for RequestReview {
    type Params = RequestReviewParams;
    type Result = RequestReviewResult;

    const KIND: &'static str = "request_review";
}

pub struct RequestReviewHandler {
    provider: Arc<dyn ReviewProvider>,
}

impl RequestReviewHandler {
    pub fn new(provider: Arc<dyn ReviewProvider>) -> Self {
        Self { provider }
    }
}

impl TypedActionHandler<RequestReview> for RequestReviewHandler {
    fn handle_typed<'a>(
        &'a self,
        _request: &'a InterceptionRequest,
        action: RequestedAction<RequestReview>,
    ) -> ActionHandlerFuture<'a, Result<ResolvedAction<RequestReview>, ActionExecutionError>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            let result = self.provider.request_review(action.params.clone()).await?;

            Ok(ResolvedAction {
                params: action.params,
                result: Ok(result),
            })
        })
    }
}
