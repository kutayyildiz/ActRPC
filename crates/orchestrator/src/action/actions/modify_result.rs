use crate::{
    action::{ActionHandlerFuture, TypedActionHandler},
    error::ActionExecutionError,
    runtime::InFlightMessageState,
};
use actrpc_core::{
    DescribeParams,
    action::{ActionSpec, NoOk, RequestedAction, ResolvedAction},
    interception::InterceptionRequest,
    json_rpc::{JsonRpcMessage, JsonRpcResponse, JsonRpcSingleMessage},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, DescribeParams)]
pub struct ModifyResultParams {
    pub result: serde_json::Value,
}

pub struct ModifyResult;

impl ActionSpec for ModifyResult {
    type Params = ModifyResultParams;
    type Result = NoOk;

    const KIND: &'static str = "modify_result";
}

pub struct ModifyResultHandler {
    in_flight_message: Arc<InFlightMessageState>,
}

impl ModifyResultHandler {
    pub fn new(in_flight_message: Arc<InFlightMessageState>) -> Self {
        Self { in_flight_message }
    }
}

impl TypedActionHandler<ModifyResult> for ModifyResultHandler {
    fn handle_typed<'a>(
        &'a self,
        _request: &'a InterceptionRequest,
        action: RequestedAction<ModifyResult>,
    ) -> ActionHandlerFuture<'a, Result<ResolvedAction<ModifyResult>, ActionExecutionError>>
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
                JsonRpcMessage::Single(JsonRpcSingleMessage::Response(
                    JsonRpcResponse::Success(mut success),
                )) => {
                    success.result = action.params.result.clone();
                    JsonRpcMessage::Single(JsonRpcSingleMessage::Response(
                        JsonRpcResponse::Success(success),
                    ))
                }
                _ => {
                    return Err(ActionExecutionError::InvalidParams {
                        action: ModifyResult::action_kind(),
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
