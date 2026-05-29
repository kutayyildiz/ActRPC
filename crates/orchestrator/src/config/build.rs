use crate::{
    action::available_actions, config::OrchestratorConfig, error::OrchestratorError,
    interceptor::InterceptorCatalog, method::MethodCatalog, review::ReviewProvider,
    runtime::OrchestratorResources,
};
use actrpc_transport::{JsonRpcClient, JsonRpcClientProvider, TransportError};
use std::sync::Arc;

impl OrchestratorConfig {
    pub async fn build_resources<P>(
        self,
        provider: &P,
        review_provider: Arc<dyn ReviewProvider>,
    ) -> Result<OrchestratorResources, OrchestratorError>
    where
        P: JsonRpcClientProvider<
                Client = Arc<dyn JsonRpcClient<Error = TransportError>>,
                Error = TransportError,
            > + Send
            + Sync,
    {
        let method_catalog = MethodCatalog::from_configs(self.methods, provider).await?;

        let interceptor_catalog = InterceptorCatalog::build_from_configs(
            self.interceptors,
            self.pipelines.outbound,
            self.pipelines.inbound,
            &available_actions(),
            provider,
        )
        .await?;

        Ok(OrchestratorResources::with_review_provider(
            Arc::new(interceptor_catalog),
            Arc::new(method_catalog),
            review_provider,
        ))
    }
}
