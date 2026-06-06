use actrpc_core::json_rpc::{
    JsonRpcMessage, JsonRpcRequest, JsonRpcResponse, JsonRpcSingleMessage,
};
use actrpc_transport::{
    JsonRpcClient, JsonRpcSession, JsonRpcSessionEvent, TransportError, TransportTarget,
};
use std::sync::Arc;

use crate::error::EndpointDoesNotSupportSessionError;

#[derive(Clone)]
pub enum EndpointConnection {
    Client(Arc<dyn JsonRpcClient<Error = TransportError>>),
    Session(Arc<dyn JsonRpcSession<Error = TransportError>>),
}

#[derive(Clone)]
pub struct JsonRpcEndpoint {
    pub name: super::config::EndpointName,
    pub target: TransportTarget,
    connection: EndpointConnection,
}

impl JsonRpcEndpoint {
    pub fn new(
        name: super::config::EndpointName,
        target: TransportTarget,
        connection: EndpointConnection,
    ) -> Self {
        Self {
            name,
            target,
            connection,
        }
    }

    pub fn request_response_capable(&self) -> bool {
        true
    }

    pub fn session_capable(&self) -> bool {
        matches!(self.connection, EndpointConnection::Session(_))
    }

    pub async fn send_request_message(
        &self,
        message: JsonRpcMessage,
    ) -> Result<JsonRpcMessage, TransportError> {
        match &self.connection {
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

    pub async fn request(
        &self,
        request: JsonRpcRequest,
    ) -> Result<JsonRpcResponse, TransportError> {
        match &self.connection {
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
    }

    pub fn subscribe(
        &self,
    ) -> Result<
        tokio::sync::broadcast::Receiver<JsonRpcSessionEvent>,
        EndpointDoesNotSupportSessionError,
    > {
        match &self.connection {
            EndpointConnection::Session(s) => Ok(s.subscribe()),
            EndpointConnection::Client(_) => Err(EndpointDoesNotSupportSessionError {
                endpoint: self.name.clone(),
            }),
        }
    }
}
