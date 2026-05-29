use crate::{
    action::{ActionHandlerFuture, TypedActionHandler},
    error::ActionExecutionError,
    interceptor::{InterceptorCatalog, InterceptorPolicy, WorkingInterceptorPipeline},
};
use actrpc_core::{
    DescribeValue,
    action::{ActionSpec, NoParams, RequestedAction, ResolvedAction},
    interception::InterceptionRequest,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeValue)]
pub struct GetWorkingInterceptorCatalogEntry {
    pub name: String,
    pub policy: InterceptorPolicy,
}

pub struct GetWorkingInterceptorCatalog;

impl ActionSpec for GetWorkingInterceptorCatalog {
    type Params = NoParams;
    type Result = Vec<GetWorkingInterceptorCatalogEntry>;

    const KIND: &'static str = "get_working_interceptor_catalog";
}

pub struct GetWorkingInterceptorCatalogHandler {
    catalog: Arc<InterceptorCatalog>,
    pipeline: Arc<WorkingInterceptorPipeline>,
}

impl GetWorkingInterceptorCatalogHandler {
    pub fn new(
        catalog: Arc<InterceptorCatalog>,
        pipeline: Arc<WorkingInterceptorPipeline>,
    ) -> Self {
        Self { catalog, pipeline }
    }
}

impl TypedActionHandler<GetWorkingInterceptorCatalog> for GetWorkingInterceptorCatalogHandler {
    fn handle_typed<'a>(
        &'a self,
        _request: &'a InterceptionRequest,
        action: RequestedAction<GetWorkingInterceptorCatalog>,
    ) -> ActionHandlerFuture<
        'a,
        Result<ResolvedAction<GetWorkingInterceptorCatalog>, ActionExecutionError>,
    >
    where
        Self: 'a,
    {
        Box::pin(async move {
            let names = self.pipeline.snapshot();
            let entries = self.catalog.entries_for_names(&names)?;

            let entries = entries
                .into_iter()
                .map(|entry| GetWorkingInterceptorCatalogEntry {
                    name: entry.name,
                    policy: entry.policy,
                })
                .collect();

            Ok(ResolvedAction {
                params: action.params,
                result: Ok(entries),
            })
        })
    }
}
