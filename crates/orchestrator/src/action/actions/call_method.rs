use crate::{
    action::{ActionHandlerFuture, ActionInvocationContext, TypedActionHandler},
    error::{ActionExecutionError, OrchestratorError},
    method::{MethodName, ProviderName},
    runtime::{CallExecutionFactory, CallRuntime},
};
use actrpc_core::{
    CallContext, DescribeParams,
    action::{ActionSpec, RequestedAction, ResolvedAction},
    interception::InterceptionRequest,
    json_rpc::{JsonRpcMessage, JsonRpcParams, JsonRpcResponse, JsonRpcSingleMessage},
    participant::{Participant, ParticipantType},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, DescribeParams)]
#[serde(deny_unknown_fields)]
pub struct CallMethodParams {
    pub provider: ProviderName,
    pub method: MethodName,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<JsonRpcParams>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ctx: Option<CallContext>,
}

pub struct CallMethod;

impl ActionSpec for CallMethod {
    type Params = CallMethodParams;
    type Result = JsonRpcResponse;

    const KIND: &'static str = "call_method";
}

pub struct CallMethodHandler {
    factory: Arc<CallExecutionFactory>,
    parent_call: Arc<CallRuntime>,
}

impl CallMethodHandler {
    pub fn new(factory: Arc<CallExecutionFactory>, parent_call: Arc<CallRuntime>) -> Self {
        Self {
            factory,
            parent_call,
        }
    }
}

impl TypedActionHandler<CallMethod> for CallMethodHandler {
    fn handle_typed<'a>(
        &'a self,
        _request: &'a InterceptionRequest,
        action: RequestedAction<CallMethod>,
        ctx: &'a ActionInvocationContext,
    ) -> ActionHandlerFuture<'a, Result<ResolvedAction<CallMethod>, ActionExecutionError>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            if let Some(call_ctx) = &action.params.ctx {
                crate::call_context::validate_call_context(call_ctx, CallMethod::action_kind())?;
            }

            let child_origin = Participant {
                kind: ParticipantType::Interceptor,
                id: ctx.interceptor_name.clone(),
            };

            let response = self
                .factory
                .run_piped(
                    action.params.provider.clone(),
                    action.params.method.clone(),
                    action.params.params.clone(),
                    action.params.ctx.clone(),
                    self.parent_call.as_ref(),
                    child_origin,
                )
                .await
                .map_err(map_orchestrator_error)?;

            let result = decode_response_value(response)?;

            Ok(ResolvedAction {
                params: action.params,
                result: Ok(result),
            })
        })
    }
}

fn decode_response_value(message: JsonRpcMessage) -> Result<JsonRpcResponse, ActionExecutionError> {
    match message {
        JsonRpcMessage::Single(JsonRpcSingleMessage::Response(response)) => Ok(response),

        _ => Err(ActionExecutionError::DependencyFailed {
            dependency: "call_method".to_owned(),
            message: "method did not return a single JSON-RPC response".to_owned(),
        }),
    }
}

fn map_orchestrator_error(error: OrchestratorError) -> ActionExecutionError {
    ActionExecutionError::DependencyFailed {
        dependency: "call_execution".to_owned(),
        message: error.to_string(),
    }
}