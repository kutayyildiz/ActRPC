use crate::{
    action::{ActionHandlerFuture, TypedActionHandler},
    error::ActionExecutionError,
    runtime::InFlightMessageState,
};
use actrpc_core::{
    DescribeParams,
    action::{ActionSpec, NoOk, RequestedAction, ResolvedAction},
    interception::InterceptionRequest,
    json_rpc::{JsonRpcMessage, JsonRpcParams, JsonRpcSingleMessage},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, DescribeParams)]
pub struct ModifyParamsParams {
    pub params: Option<JsonRpcParams>,
}

pub struct ModifyParams;

impl ActionSpec for ModifyParams {
    type Params = ModifyParamsParams;
    type Result = NoOk;

    const KIND: &'static str = "modify_params";
}

pub struct ModifyParamsHandler {
    in_flight_message: Arc<InFlightMessageState>,
}

impl ModifyParamsHandler {
    pub fn new(in_flight_message: Arc<InFlightMessageState>) -> Self {
        Self { in_flight_message }
    }
}

impl TypedActionHandler<ModifyParams> for ModifyParamsHandler {
    fn handle_typed<'a>(
        &'a self,
        _request: &'a InterceptionRequest,
        action: RequestedAction<ModifyParams>,
    ) -> ActionHandlerFuture<'a, Result<ResolvedAction<ModifyParams>, ActionExecutionError>>
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
                JsonRpcMessage::Single(JsonRpcSingleMessage::Request(mut req)) => {
                    req.params = action.params.params.clone();
                    JsonRpcMessage::Single(JsonRpcSingleMessage::Request(req))
                }
                _ => {
                    return Err(ActionExecutionError::InvalidParams {
                        action: ModifyParams::action_kind(),
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
