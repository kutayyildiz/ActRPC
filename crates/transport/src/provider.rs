use crate::{client::JsonRpcClient, target::TransportTarget};
use std::{future::Future, pin::Pin};

pub type JsonRpcClientProviderFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait JsonRpcClientProvider: Send + Sync {
    type Error: Send + Sync + 'static;
    type Client: JsonRpcClient<Error = Self::Error>;

    fn get_client<'a>(
        &'a self,
        target: &'a TransportTarget,
    ) -> JsonRpcClientProviderFuture<'a, Result<Self::Client, Self::Error>>;
}
