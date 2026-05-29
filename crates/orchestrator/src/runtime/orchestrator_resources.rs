use crate::{
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
        }
    }

    pub fn with_review_provider(
        interceptor_catalog: Arc<InterceptorCatalog>,
        method_catalog: Arc<MethodCatalog>,
        review_provider: Arc<dyn ReviewProvider>,
    ) -> Self {
        Self {
            interceptor_catalog,
            method_catalog,
            review_provider,
        }
    }
}
