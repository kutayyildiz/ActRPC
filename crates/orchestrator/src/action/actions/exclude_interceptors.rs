use crate::{
    action::{ActionHandlerFuture, TypedActionHandler},
    error::ActionExecutionError,
    interceptor::WorkingInterceptorPipeline,
};
use actrpc_core::{
    DescribeParams,
    action::{ActionSpec, NoOk, RequestedAction, ResolvedAction},
    interception::InterceptionRequest,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};

#[derive(Debug, Clone, Serialize, Deserialize, DescribeParams)]
pub struct ExcludeInterceptorsParams {
    pub names: Vec<String>,
}

pub struct ExcludeInterceptors;

impl ActionSpec for ExcludeInterceptors {
    type Params = ExcludeInterceptorsParams;
    type Result = NoOk;

    const KIND: &'static str = "exclude_interceptors";
}

pub struct ExcludeInterceptorsHandler {
    pipeline: Arc<WorkingInterceptorPipeline>,
}

impl ExcludeInterceptorsHandler {
    pub fn new(pipeline: Arc<WorkingInterceptorPipeline>) -> Self {
        Self { pipeline }
    }
}

impl TypedActionHandler<ExcludeInterceptors> for ExcludeInterceptorsHandler {
    fn handle_typed<'a>(
        &'a self,
        _request: &'a InterceptionRequest,
        action: RequestedAction<ExcludeInterceptors>,
    ) -> ActionHandlerFuture<'a, Result<ResolvedAction<ExcludeInterceptors>, ActionExecutionError>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            if action.params.names.is_empty() {
                return Err(ActionExecutionError::InvalidParams {
                    action: ExcludeInterceptors::action_kind(),
                });
            }

            let mut seen = HashSet::new();
            let mut names = Vec::new();

            for name in &action.params.names {
                let trimmed = name.trim();

                if trimmed.is_empty() {
                    return Err(ActionExecutionError::InvalidParams {
                        action: ExcludeInterceptors::action_kind(),
                    });
                }

                if seen.insert(trimmed.to_owned()) {
                    names.push(trimmed.to_owned());
                }
            }

            self.pipeline.exclude_named(&names);

            Ok(ResolvedAction {
                params: action.params,
                result: Ok(NoOk),
            })
        })
    }
}
