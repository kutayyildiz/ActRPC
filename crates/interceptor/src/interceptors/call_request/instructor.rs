use crate::interceptors::call_request::{
    config::{InstructorConfig, PromptInjection},
    error::CallRequestError,
};
use actrpc_core::{
    InterceptorInitialization,
    action::{ActionDescriptor, ActionKind, RequestedAction, RequestedActionRecord},
    interception::{
        InterceptionPhase, InterceptionRequest, InterceptionResponse, InterceptorContinuation,
    },
    json_rpc::{JsonRpcMessage, JsonRpcParams, JsonRpcSingleMessage},
};
use actrpc_orchestrator::{
    action::actions::modify_params::{ModifyParams, ModifyParamsParams},
    interceptor::{Interceptor, InterceptorFuture},
};
use std::collections::HashMap;

pub struct CallRequestInstructor {
    config: InstructorConfig,
}

impl CallRequestInstructor {
    pub fn new(config: InstructorConfig) -> Self {
        Self { config }
    }

    fn intercept_inner(
        &self,
        request: &InterceptionRequest,
    ) -> Result<InterceptionResponse, CallRequestError> {
        if request.phase()? != InterceptionPhase::Outbound {
            return Ok(no_op());
        }

        let Some(rule) = self.config.rules.iter().find(|rule| {
            rule.provider == request.target.provider && rule.method == request.target.method
        }) else {
            return Ok(no_op());
        };

        let JsonRpcMessage::Single(JsonRpcSingleMessage::Request(rpc_request)) = &request.message
        else {
            return Ok(no_op());
        };

        let Some(params) = &rpc_request.params else {
            return Ok(no_op());
        };

        let JsonRpcParams::Object(map) = params else {
            return Ok(no_op());
        };

        let Some(current_prompt) = map.get(&rule.prompt_field).and_then(|v| v.as_str()) else {
            return Ok(no_op());
        };

        let updated_prompt = format_prompt(current_prompt, &rule.injection);
        if updated_prompt == current_prompt {
            return Ok(no_op());
        }

        let mut updated_map = map.clone();
        updated_map.insert(
            rule.prompt_field.clone(),
            serde_json::Value::String(updated_prompt),
        );

        Ok(InterceptionResponse {
            actions: vec![modify_params_action(Some(JsonRpcParams::Object(
                updated_map,
            )))?],
            continuation: InterceptorContinuation::Stop,
        })
    }
}

impl Interceptor for CallRequestInstructor {
    fn initialize<'a>(
        &'a self,
    ) -> InterceptorFuture<
        'a,
        Result<InterceptorInitialization, actrpc_orchestrator::error::InterceptorRuntimeError>,
    >
    where
        Self: 'a,
    {
        Box::pin(async move {
            Ok(InterceptorInitialization {
                supports_outbound: true,
                supports_inbound: false,
                actions: action_descriptors(),
            })
        })
    }

    fn intercept<'a>(
        &'a self,
        request: &'a InterceptionRequest,
    ) -> InterceptorFuture<
        'a,
        Result<InterceptionResponse, actrpc_orchestrator::error::InterceptorRuntimeError>,
    >
    where
        Self: 'a,
    {
        Box::pin(async move {
            self.intercept_inner(request)
                .map_err(actrpc_orchestrator::error::InterceptorRuntimeError::from)
        })
    }
}

fn format_prompt(original: &str, injection: &PromptInjection) -> String {
    match (&injection.prepend, &injection.append) {
        (None, None) => original.to_owned(),
        (Some(prepend), None) => format!("{prepend}\n\n{original}"),
        (None, Some(append)) => format!("{original}\n\n{append}"),
        (Some(prepend), Some(append)) => format!("{prepend}\n\n{original}\n\n{append}"),
    }
}

fn no_op() -> InterceptionResponse {
    InterceptionResponse {
        actions: vec![],
        continuation: InterceptorContinuation::Stop,
    }
}

fn modify_params_action(
    params: Option<JsonRpcParams>,
) -> Result<RequestedActionRecord, CallRequestError> {
    RequestedAction::<ModifyParams> {
        params: ModifyParamsParams { params },
    }
    .try_into()
    .map_err(|source| CallRequestError::ActionEncoding { source })
}

fn action_descriptors() -> HashMap<ActionKind, ActionDescriptor> {
    actrpc_core::action::action_descriptor_map!(ModifyParams)
}
