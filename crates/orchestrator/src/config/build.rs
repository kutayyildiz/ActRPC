use crate::{
    action::available_actions, config::OrchestratorConfig, endpoint::EndpointCatalog,
    error::OrchestratorError, interceptor::InterceptorCatalog, method::MethodCatalog,
    method::spawn_watchable_listeners, review::ReviewProvider, runtime::OrchestratorResources,
};
use actrpc_transport::{
    JsonRpcClient, JsonRpcClientProvider, JsonRpcSession, JsonRpcSessionProvider, TransportError,
};
use std::sync::Arc;

impl OrchestratorConfig {
    pub async fn build_resources<CP, SP>(
        self,
        client_provider: &CP,
        session_provider: &SP,
        review_provider: Arc<dyn ReviewProvider>,
    ) -> Result<OrchestratorResources, OrchestratorError>
    where
        CP: JsonRpcClientProvider<
                Client = Arc<dyn JsonRpcClient<Error = TransportError>>,
                Error = TransportError,
            > + Send
            + Sync,
        SP: JsonRpcSessionProvider<
                Session = Arc<dyn JsonRpcSession<Error = TransportError>>,
                Error = TransportError,
            > + Send
            + Sync,
    {
        let endpoint_catalog = EndpointCatalog::from_configs(
            self.endpoints,
            &self.methods,
            &self.interceptors,
            client_provider,
            session_provider,
        )
        .await?;

        let method_catalog =
            Arc::new(MethodCatalog::from_configs(self.methods, &endpoint_catalog).await?);

        let listener_tasks = spawn_watchable_listeners(method_catalog.clone(), &endpoint_catalog)?;

        let interceptor_catalog = InterceptorCatalog::build_from_configs(
            self.interceptors,
            self.pipelines.outbound,
            self.pipelines.inbound,
            &available_actions(),
            &endpoint_catalog,
        )
        .await?;

        let runtime = self.runtime.clone().unwrap_or_default();
        runtime.validate()?;

        Ok(OrchestratorResources::with_review_provider_and_runtime(
            Arc::new(interceptor_catalog),
            method_catalog,
            review_provider,
            listener_tasks,
            runtime,
        ))
    }
}
