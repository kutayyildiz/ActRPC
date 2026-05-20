use crate::{
    external_method::ExternalMethodCatalog,
    interceptor::InterceptorCatalog,
    review::{ReviewProvider, UnavailableReviewProvider},
};
use std::sync::Arc;

#[derive(Clone)]
pub struct OrchestratorResources {
    pub interceptor_catalog: Arc<InterceptorCatalog>,
    pub external_method_catalog: Arc<ExternalMethodCatalog>,
    pub review_provider: Arc<dyn ReviewProvider>,
}

impl OrchestratorResources {
    pub fn new(
        interceptor_catalog: Arc<InterceptorCatalog>,
        external_method_catalog: Arc<ExternalMethodCatalog>,
    ) -> Self {
        Self {
            interceptor_catalog,
            external_method_catalog,
            review_provider: Arc::new(UnavailableReviewProvider),
        }
    }

    pub fn with_review_provider(
        interceptor_catalog: Arc<InterceptorCatalog>,
        external_method_catalog: Arc<ExternalMethodCatalog>,
        review_provider: Arc<dyn ReviewProvider>,
    ) -> Self {
        Self {
            interceptor_catalog,
            external_method_catalog,
            review_provider,
        }
    }
}
