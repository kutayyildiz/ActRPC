use super::{
    config::EndpointName,
    kind::{EndpointCapabilities, EndpointKind},
};
use actrpc_core::json_rpc::{
    JsonRpcMessage, JsonRpcRequest, JsonRpcResponse, JsonRpcSingleMessage,
};
use actrpc_transport::{JsonRpcClient, JsonRpcSession, JsonRpcSessionEvent, TransportError};
use std::{future::Future, pin::Pin, sync::Arc};

pub type JsonRpc2EndpointFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait JsonRpc2RequestEndpoint: Send + Sync {
    fn endpoint_name(&self) -> &EndpointName;
    fn endpoint_kind(&self) -> EndpointKind;
    fn endpoint_capabilities(&self) -> EndpointCapabilities;

    fn request<'a>(
        &'a self,
        request: JsonRpcRequest,
    ) -> JsonRpc2EndpointFuture<'a, Result<JsonRpcResponse, TransportError>>;
}

pub trait JsonRpc2SessionEndpoint: Send + Sync {
    fn endpoint_name(&self) -> &EndpointName;

    fn subscribe(
        &self,
    ) -> Result<
        tokio::sync::broadcast::Receiver<JsonRpcSessionEvent>,
        crate::error::EndpointDoesNotSupportSessionError,
    >;
}

#[derive(Clone)]
enum EndpointConnection {
    Client(Arc<dyn JsonRpcClient<Error = TransportError>>),
    Session(Arc<dyn JsonRpcSession<Error = TransportError>>),
}

struct JsonRpc2EndpointInner {
    name: EndpointName,
    connection: EndpointConnection,
}

#[derive(Clone)]
pub struct JsonRpc2RequestHandle {
    inner: Arc<JsonRpc2EndpointInner>,
}

#[derive(Clone)]
pub struct JsonRpc2SessionHandle {
    inner: Arc<JsonRpc2EndpointInner>,
}

impl JsonRpc2RequestHandle {
    pub fn from_client(
        name: EndpointName,
        client: Arc<dyn JsonRpcClient<Error = TransportError>>,
    ) -> Self {
        Self {
            inner: Arc::new(JsonRpc2EndpointInner {
                name,
                connection: EndpointConnection::Client(client),
            }),
        }
    }

    pub fn from_session(
        name: EndpointName,
        session: Arc<dyn JsonRpcSession<Error = TransportError>>,
    ) -> (Self, JsonRpc2SessionHandle) {
        let inner = Arc::new(JsonRpc2EndpointInner {
            name,
            connection: EndpointConnection::Session(session),
        });
        (
            Self {
                inner: inner.clone(),
            },
            JsonRpc2SessionHandle { inner },
        )
    }

    pub async fn send_request_message(
        &self,
        message: JsonRpcMessage,
    ) -> Result<JsonRpcMessage, TransportError> {
        match &self.inner.connection {
            EndpointConnection::Client(c) => c.send(message).await,
            EndpointConnection::Session(s) => {
                if let JsonRpcMessage::Single(JsonRpcSingleMessage::Request(r)) = message {
                    let resp = s.request(r).await?;
                    Ok(JsonRpcMessage::Single(JsonRpcSingleMessage::Response(resp)))
                } else {
                    Err(TransportError::Internal {
                        message: "session endpoint send_request_message expects a request message"
                            .to_owned(),
                    })
                }
            }
        }
    }
}

impl JsonRpc2RequestEndpoint for JsonRpc2RequestHandle {
    fn endpoint_name(&self) -> &EndpointName {
        &self.inner.name
    }

    fn endpoint_kind(&self) -> EndpointKind {
        EndpointKind::JsonRpc2
    }

    fn endpoint_capabilities(&self) -> EndpointCapabilities {
        match self.inner.connection {
            EndpointConnection::Client(_) => EndpointCapabilities::REQUEST_RESPONSE,
            EndpointConnection::Session(_) => EndpointCapabilities::SESSION,
        }
    }

    fn request<'a>(
        &'a self,
        request: JsonRpcRequest,
    ) -> JsonRpc2EndpointFuture<'a, Result<JsonRpcResponse, TransportError>> {
        let this = self.clone();
        Box::pin(async move {
            match &this.inner.connection {
                EndpointConnection::Client(c) => {
                    let full = JsonRpcMessage::Single(JsonRpcSingleMessage::Request(request));
                    let full_resp = c.send(full).await?;
                    match full_resp {
                        JsonRpcMessage::Single(JsonRpcSingleMessage::Response(r)) => Ok(r),
                        _ => Err(TransportError::Internal {
                            message: "client endpoint request did not return a single response"
                                .to_owned(),
                        }),
                    }
                }
                EndpointConnection::Session(s) => s.request(request).await,
            }
        })
    }
}

impl JsonRpc2SessionEndpoint for JsonRpc2SessionHandle {
    fn endpoint_name(&self) -> &EndpointName {
        &self.inner.name
    }

    fn subscribe(
        &self,
    ) -> Result<
        tokio::sync::broadcast::Receiver<JsonRpcSessionEvent>,
        crate::error::EndpointDoesNotSupportSessionError,
    > {
        match &self.inner.connection {
            EndpointConnection::Session(s) => Ok(s.subscribe()),
            EndpointConnection::Client(_) => {
                Err(crate::error::EndpointDoesNotSupportSessionError {
                    endpoint: self.inner.name.clone(),
                })
            }
        }
    }
}
