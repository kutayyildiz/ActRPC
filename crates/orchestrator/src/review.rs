use crate::{
    action::{
        ActionHandlerFuture,
        actions::request_review::{RequestReviewParams, RequestReviewResult},
    },
    error::ActionExecutionError,
};

pub trait ReviewProvider: Send + Sync {
    fn request_review<'a>(
        &'a self,
        params: RequestReviewParams,
    ) -> ActionHandlerFuture<'a, Result<RequestReviewResult, ActionExecutionError>>;
}

#[derive(Debug, Default)]
pub struct UnavailableReviewProvider;

impl ReviewProvider for UnavailableReviewProvider {
    fn request_review<'a>(
        &'a self,
        _params: RequestReviewParams,
    ) -> ActionHandlerFuture<'a, Result<RequestReviewResult, ActionExecutionError>> {
        Box::pin(async move {
            Err(ActionExecutionError::InvalidState {
                message: "request_review action was invoked, but no ReviewProvider is configured"
                    .to_owned(),
            })
        })
    }
}
