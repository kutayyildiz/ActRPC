use crate::{
    action::{ActionHandlerFuture, TypedActionHandler},
    error::ActionExecutionError,
    interceptor::WorkingInterceptorPipeline,
};
use actrpc_core::{
    DescribeValue,
    action::{ActionSpec, NoParams, RequestedAction, ResolvedAction},
    interception::InterceptionRequest,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, DescribeValue)]
pub struct GetWorkingPipelineEntry {
    pub name: String,
}

pub struct GetWorkingPipeline;

impl ActionSpec for GetWorkingPipeline {
    type Params = NoParams;
    type Result = Vec<GetWorkingPipelineEntry>;

    const KIND: &'static str = "get_working_pipeline";
}

pub struct GetWorkingPipelineHandler {
    pipeline: Arc<WorkingInterceptorPipeline>,
}

impl GetWorkingPipelineHandler {
    pub fn new(pipeline: Arc<WorkingInterceptorPipeline>) -> Self {
        Self { pipeline }
    }
}

impl TypedActionHandler<GetWorkingPipeline> for GetWorkingPipelineHandler {
    fn handle_typed<'a>(
        &'a self,
        _request: &'a InterceptionRequest,
        action: RequestedAction<GetWorkingPipeline>,
    ) -> ActionHandlerFuture<'a, Result<ResolvedAction<GetWorkingPipeline>, ActionExecutionError>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            let entries = self
                .pipeline
                .snapshot()
                .into_iter()
                .map(|name| GetWorkingPipelineEntry { name })
                .collect();

            Ok(ResolvedAction {
                params: action.params,
                result: Ok(entries),
            })
        })
    }
}
