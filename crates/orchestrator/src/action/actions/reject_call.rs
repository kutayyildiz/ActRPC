use crate::{
    action::{ActionHandlerFuture, TypedActionHandler},
    error::ActionExecutionError,
    runtime::CurrentCallRejection,
};
use actrpc_core::{
    DescribeParams,
    action::{ActionSpec, NoOk, RequestedAction, ResolvedAction},
    interception::InterceptionRequest,
    json_rpc::JsonRpcError,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, DescribeParams)]
pub struct RejectCallParams {
    pub error: JsonRpcError,
}

pub struct RejectCall;

impl ActionSpec for RejectCall {
    type Params = RejectCallParams;
    type Result = NoOk;

    const KIND: &'static str = "reject_call";
}

pub struct RejectCallHandler {
    rejection: Arc<CurrentCallRejection>,
}

impl RejectCallHandler {
    pub fn new(rejection: Arc<CurrentCallRejection>) -> Self {
        Self { rejection }
    }
}

impl TypedActionHandler<RejectCall> for RejectCallHandler {
    fn handle_typed<'a>(
        &'a self,
        _request: &'a InterceptionRequest,
        action: RequestedAction<RejectCall>,
    ) -> ActionHandlerFuture<'a, Result<ResolvedAction<RejectCall>, ActionExecutionError>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            self.rejection.set(action.params.error.clone());

            Ok(ResolvedAction {
                params: action.params,
                result: Ok(NoOk),
            })
        })
    }
}
