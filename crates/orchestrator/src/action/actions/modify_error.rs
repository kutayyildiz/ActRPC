use crate::{
    action::{ActionHandlerFuture, TypedActionHandler},
    error::ActionExecutionError,
    runtime::InFlightMessageState,
};
use actrpc_core::{
    DescribeParams,
    action::{ActionSpec, NoOk, RequestedAction, ResolvedAction},
    interception::InterceptionRequest,
    json_rpc::{JsonRpcError, JsonRpcMessage, JsonRpcResponse, JsonRpcSingleMessage},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, DescribeParams)]
pub struct ModifyErrorParams {
    pub error: JsonRpcError,
}

pub struct ModifyError;

impl ActionSpec for ModifyError {
    type Params = ModifyErrorParams;
    type Result = NoOk;

    const KIND: &'static str = "modify_error";
}

pub struct ModifyErrorHandler {
    in_flight_message: Arc<InFlightMessageState>,
}

impl ModifyErrorHandler {
    pub fn new(in_flight_message: Arc<InFlightMessageState>) -> Self {
        Self { in_flight_message }
    }
}

impl TypedActionHandler<ModifyError> for ModifyErrorHandler {
    fn handle_typed<'a>(
        &'a self,
        _request: &'a InterceptionRequest,
        action: RequestedAction<ModifyError>,
    ) -> ActionHandlerFuture<'a, Result<ResolvedAction<ModifyError>, ActionExecutionError>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            let current = self.in_flight_message.snapshot().ok_or_else(|| {
                ActionExecutionError::InvalidState {
                    message: "no in-flight message is currently set".to_owned(),
                }
            })?;

            let updated = match current {
                JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Error(
                    mut error_response,
                ))) => {
                    error_response.error = action.params.error.clone();
                    JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Error(
                        error_response,
                    )))
                }
                _ => {
                    return Err(ActionExecutionError::InvalidParams {
                        action: ModifyError::action_kind(),
                    });
                }
            };

            if !self.in_flight_message.replace_message(updated) {
                return Err(ActionExecutionError::InvalidState {
                    message: "no in-flight message is currently set".to_owned(),
                });
            }

            Ok(ResolvedAction {
                params: action.params,
                result: Ok(NoOk),
            })
        })
    }
}
