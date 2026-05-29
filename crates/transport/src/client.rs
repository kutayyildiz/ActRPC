mod http;
mod local_ipc;
mod stdio;
mod tcp;
mod web_socket;

pub use http::HttpJsonRpcClient;
pub use local_ipc::LocalIpcJsonRpcClient;
pub use stdio::StdioJsonRpcClient;
pub use tcp::TcpJsonRpcClient;
pub use web_socket::WebSocketJsonRpcClient;

use actrpc_core::json_rpc::JsonRpcMessage;
use std::{future::Future, pin::Pin, sync::Arc};

pub type JsonRpcClientFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait JsonRpcClient: Send + Sync {
    type Error: Send + Sync + 'static;

    fn send<'a>(
        &'a self,
        message: JsonRpcMessage,
    ) -> JsonRpcClientFuture<'a, Result<JsonRpcMessage, Self::Error>>;
}

impl<T> JsonRpcClient for Arc<T>
where
    T: JsonRpcClient + ?Sized,
{
    type Error = T::Error;

    fn send<'a>(
        &'a self,
        message: JsonRpcMessage,
    ) -> JsonRpcClientFuture<'a, Result<JsonRpcMessage, Self::Error>> {
        (**self).send(message)
    }
}
