use crate::{
    JsonRpcClient, JsonRpcClientProvider, TransportError, TransportTarget,
    client::{
        HttpJsonRpcClient, LocalIpcJsonRpcClient, StdioJsonRpcClient, TcpJsonRpcClient,
        WebSocketJsonRpcClient,
    },
    factory::{JsonRpcClientFactory, JsonRpcClientFactoryFuture},
    provider::JsonRpcClientProviderFuture,
};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultJsonRpcClientFactory;

impl DefaultJsonRpcClientFactory {
    pub fn new() -> Self {
        Self
    }
}

impl JsonRpcClientFactory for DefaultJsonRpcClientFactory {
    fn create_client<'a>(
        &'a self,
        target: &'a TransportTarget,
    ) -> JsonRpcClientFactoryFuture<
        'a,
        Result<Arc<dyn JsonRpcClient<Error = TransportError>>, TransportError>,
    > {
        Box::pin(async move {
            let client: Arc<dyn JsonRpcClient<Error = TransportError>> = match target {
                TransportTarget::Stdio(target) => {
                    Arc::new(StdioJsonRpcClient::new(target.clone())?)
                }

                TransportTarget::Tcp(target) => {
                    Arc::new(TcpJsonRpcClient::new(target.clone()).await?)
                }

                TransportTarget::LocalIpc(target) => {
                    Arc::new(LocalIpcJsonRpcClient::new(target.clone()).await?)
                }

                TransportTarget::Http(target) => Arc::new(HttpJsonRpcClient::new(target.clone())?),

                TransportTarget::WebSocket(target) => {
                    Arc::new(WebSocketJsonRpcClient::new(target.clone()).await?)
                }
            };

            Ok(client)
        })
    }
}

pub struct DefaultJsonRpcClientProvider<F = DefaultJsonRpcClientFactory>
where
    F: JsonRpcClientFactory,
{
    factory: F,
    cache: RwLock<HashMap<TransportTarget, Arc<dyn JsonRpcClient<Error = TransportError>>>>,
}

impl DefaultJsonRpcClientProvider<DefaultJsonRpcClientFactory> {
    pub fn new() -> Self {
        Self::with_factory(DefaultJsonRpcClientFactory::new())
    }
}

impl Default for DefaultJsonRpcClientProvider<DefaultJsonRpcClientFactory> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F> DefaultJsonRpcClientProvider<F>
where
    F: JsonRpcClientFactory,
{
    pub fn with_factory(factory: F) -> Self {
        Self {
            factory,
            cache: RwLock::new(HashMap::new()),
        }
    }

    pub fn clear_cache(&self) {
        self.cache
            .write()
            .expect("poisoned JSON-RPC client cache lock")
            .clear();
    }

    pub fn remove_cached_client(
        &self,
        target: &TransportTarget,
    ) -> Option<Arc<dyn JsonRpcClient<Error = TransportError>>> {
        self.cache
            .write()
            .expect("poisoned JSON-RPC client cache lock")
            .remove(target)
    }
}

impl<F> JsonRpcClientProvider for DefaultJsonRpcClientProvider<F>
where
    F: JsonRpcClientFactory,
{
    type Error = TransportError;
    type Client = Arc<dyn JsonRpcClient<Error = TransportError>>;

    fn get_client<'a>(
        &'a self,
        target: &'a TransportTarget,
    ) -> JsonRpcClientProviderFuture<'a, Result<Self::Client, Self::Error>> {
        Box::pin(async move {
            if let Some(client) = {
                let cache = self
                    .cache
                    .read()
                    .expect("poisoned JSON-RPC client cache lock");

                cache.get(target).cloned()
            } {
                return Ok(client);
            }

            let client = self.factory.create_client(target).await?;

            {
                let mut cache = self
                    .cache
                    .write()
                    .expect("poisoned JSON-RPC client cache lock");

                let entry = cache
                    .entry(target.clone())
                    .or_insert_with(|| client.clone());

                Ok(entry.clone())
            }
        })
    }
}
