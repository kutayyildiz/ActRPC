use crate::{
    action::{ActionHandlerFuture, TypedActionHandler},
    error::ActionExecutionError,
    interceptor::{InterceptorCatalog, InterceptorPolicy},
};
use actrpc_core::{
    DescribeValue,
    action::{ActionSpec, NoParams, RequestedAction, ResolvedAction},
    interception::InterceptionRequest,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeValue)]
pub struct GetInterceptorCatalogEntry {
    pub name: String,
    pub policy: InterceptorPolicy,
}

pub struct GetInterceptorCatalog;

impl ActionSpec for GetInterceptorCatalog {
    type Params = NoParams;
    type Result = Vec<GetInterceptorCatalogEntry>;

    const KIND: &'static str = "get_interceptor_catalog";
}

pub struct GetInterceptorCatalogHandler {
    catalog: Arc<InterceptorCatalog>,
}

impl GetInterceptorCatalogHandler {
    pub fn new(catalog: Arc<InterceptorCatalog>) -> Self {
        Self { catalog }
    }
}

impl TypedActionHandler<GetInterceptorCatalog> for GetInterceptorCatalogHandler {
    fn handle_typed<'a>(
        &'a self,
        _request: &'a InterceptionRequest,
        action: RequestedAction<GetInterceptorCatalog>,
    ) -> ActionHandlerFuture<'a, Result<ResolvedAction<GetInterceptorCatalog>, ActionExecutionError>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            let entries = self
                .catalog
                .entries()
                .into_iter()
                .map(|entry| GetInterceptorCatalogEntry {
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
