use super::{
    config::EndpointName,
    kind::{EndpointCapabilities, EndpointKind},
};
use actrpc_transport::{
    HttpRestClient, RestHttpExecuteRequest, RestHttpExecuteResponse, TransportError,
    TransportTarget,
};
use std::{future::Future, pin::Pin, sync::Arc};

pub type RestHttpEndpointFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait RestHttpEndpoint: Send + Sync {
    fn endpoint_name(&self) -> &EndpointName;
    fn endpoint_kind(&self) -> EndpointKind;
    fn endpoint_capabilities(&self) -> EndpointCapabilities;

    fn execute<'a>(
        &'a self,
        request: RestHttpExecuteRequest,
    ) -> RestHttpEndpointFuture<'a, Result<RestHttpExecuteResponse, TransportError>>;
}

#[derive(Clone)]
pub struct RestHttpEndpointImpl {
    name: EndpointName,
    target: TransportTarget,
    client: Arc<HttpRestClient>,
}

impl std::fmt::Debug for RestHttpEndpointImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RestHttpEndpointImpl")
            .field("name", &self.name)
            .field("target", &self.target)
            .finish_non_exhaustive()
    }
}

impl RestHttpEndpointImpl {
    pub fn new(name: EndpointName, target: TransportTarget, client: HttpRestClient) -> Self {
        Self {
            name,
            target,
            client: Arc::new(client),
        }
    }
}

impl RestHttpEndpoint for RestHttpEndpointImpl {
    fn endpoint_name(&self) -> &EndpointName {
        &self.name
    }

    fn endpoint_kind(&self) -> EndpointKind {
        EndpointKind::RestHttp
    }

    fn endpoint_capabilities(&self) -> EndpointCapabilities {
        EndpointCapabilities::REST_HTTP
    }

    fn execute<'a>(
        &'a self,
        request: RestHttpExecuteRequest,
    ) -> RestHttpEndpointFuture<'a, Result<RestHttpExecuteResponse, TransportError>> {
        let client = self.client.clone();
        Box::pin(async move { client.execute(request).await })
    }
}
