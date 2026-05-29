use crate::{JsonRpcClient, TransportError, TransportTarget};
use std::{future::Future, pin::Pin, sync::Arc};

pub type JsonRpcClientFactoryFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait JsonRpcClientFactory: Send + Sync {
    fn create_client<'a>(
        &'a self,
        target: &'a TransportTarget,
    ) -> JsonRpcClientFactoryFuture<
        'a,
        Result<Arc<dyn JsonRpcClient<Error = TransportError>>, TransportError>,
    >;
}
