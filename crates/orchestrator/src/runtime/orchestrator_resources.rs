use crate::{
    config::RuntimeConfig,
    interceptor::InterceptorCatalog,
    method::MethodCatalog,
    review::{ReviewProvider, UnavailableReviewProvider},
};
use std::sync::Arc;

#[derive(Clone)]
pub struct OrchestratorResources {
    pub interceptor_catalog: Arc<InterceptorCatalog>,
    pub method_catalog: Arc<MethodCatalog>,
    pub review_provider: Arc<dyn ReviewProvider>,
    pub runtime: RuntimeConfig,
    _listener_tasks: Arc<Vec<tokio::task::JoinHandle<()>>>,
}

impl OrchestratorResources {
    pub fn new(
        interceptor_catalog: Arc<InterceptorCatalog>,
        method_catalog: Arc<MethodCatalog>,
    ) -> Self {
        Self {
            interceptor_catalog,
            method_catalog,
            review_provider: Arc::new(UnavailableReviewProvider),
            runtime: RuntimeConfig::default(),
            _listener_tasks: Arc::new(Vec::new()),
        }
    }

    pub fn with_review_provider(
        interceptor_catalog: Arc<InterceptorCatalog>,
        method_catalog: Arc<MethodCatalog>,
        review_provider: Arc<dyn ReviewProvider>,
        listener_tasks: Vec<tokio::task::JoinHandle<()>>,
    ) -> Self {
        Self::with_review_provider_and_runtime(
            interceptor_catalog,
            method_catalog,
            review_provider,
            listener_tasks,
            RuntimeConfig::default(),
        )
    }

    pub fn with_review_provider_and_runtime(
        interceptor_catalog: Arc<InterceptorCatalog>,
        method_catalog: Arc<MethodCatalog>,
        review_provider: Arc<dyn ReviewProvider>,
        listener_tasks: Vec<tokio::task::JoinHandle<()>>,
        runtime: RuntimeConfig,
    ) -> Self {
        Self {
            interceptor_catalog,
            method_catalog,
            review_provider,
            runtime,
            _listener_tasks: Arc::new(listener_tasks),
        }
    }
}
