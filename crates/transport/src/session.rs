use actrpc_core::json_rpc::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
use std::{future::Future, pin::Pin, sync::Arc};

pub type JsonRpcSessionFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone)]
pub enum JsonRpcSessionEvent {
    Notification(JsonRpcNotification),
    Closed,
}

pub trait JsonRpcSession: Send + Sync {
    type Error: Send + Sync + 'static;

    fn request<'a>(
        &'a self,
        request: JsonRpcRequest,
    ) -> JsonRpcSessionFuture<'a, Result<JsonRpcResponse, Self::Error>>;

    fn notify<'a>(
        &'a self,
        notification: JsonRpcNotification,
    ) -> JsonRpcSessionFuture<'a, Result<(), Self::Error>>;

    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<JsonRpcSessionEvent>;
}

impl<T> JsonRpcSession for Arc<T>
where
    T: JsonRpcSession + ?Sized,
{
    type Error = T::Error;

    fn request<'a>(
        &'a self,
        request: actrpc_core::json_rpc::JsonRpcRequest,
    ) -> JsonRpcSessionFuture<'a, Result<actrpc_core::json_rpc::JsonRpcResponse, Self::Error>> {
        (**self).request(request)
    }

    fn notify<'a>(
        &'a self,
        notification: actrpc_core::json_rpc::JsonRpcNotification,
    ) -> JsonRpcSessionFuture<'a, Result<(), Self::Error>> {
        (**self).notify(notification)
    }

    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<JsonRpcSessionEvent> {
        (**self).subscribe()
    }
}

mod default_json_rpc_session_provider;
mod json_rpc_session_provider;
mod local_ipc;
mod stdio;
mod stream_loop;
mod tcp;
mod web_socket;

pub use default_json_rpc_session_provider::DefaultJsonRpcSessionProvider;
pub use json_rpc_session_provider::{JsonRpcSessionProvider, JsonRpcSessionProviderFuture};
pub use local_ipc::LocalIpcJsonRpcSession;
pub use stdio::StdioJsonRpcSession;
pub use tcp::TcpJsonRpcSession;
pub use web_socket::WebSocketJsonRpcSession;
