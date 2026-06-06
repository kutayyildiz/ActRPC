use crate::{session::JsonRpcSession, target::TransportTarget};
use std::{future::Future, pin::Pin};

pub type JsonRpcSessionProviderFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait JsonRpcSessionProvider: Send + Sync {
    type Error: Send + Sync + 'static;
    type Session: JsonRpcSession<Error = Self::Error>;

    fn get_session<'a>(
        &'a self,
        target: &'a TransportTarget,
    ) -> JsonRpcSessionProviderFuture<'a, Result<Self::Session, Self::Error>>;
}
