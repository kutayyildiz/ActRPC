use crate::{
    TransportError,
    client::{JsonRpcClient, JsonRpcClientFuture},
    target::WebSocketTarget,
};
use actrpc_core::{error::CodecError, json_rpc::JsonRpcMessage};
use futures_util::{SinkExt, StreamExt};
use std::{str::FromStr, time::Duration};
use tokio::{sync::Mutex, time};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{
        Message,
        client::IntoClientRequest,
        error::Error as TungsteniteError,
        http::{
            HeaderName, HeaderValue, Request,
            header::{ACCEPT, CONTENT_TYPE},
        },
    },
};

/// JSON-RPC client over WebSocket.
///
/// WebSocket is a message transport:
/// - outbound: one WebSocket text message containing one JSON-RPC message
/// - inbound: one WebSocket text/binary message containing one JSON-RPC message
///
/// Stream framing is intentionally not used here.
#[derive(Debug)]
pub struct WebSocketJsonRpcClient {
    inner: Mutex<WebSocketConnection>,
}

#[derive(Debug)]
struct WebSocketConnection {
    socket: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    read_timeout_ms: u64,
    write_timeout_ms: u64,
}

impl WebSocketJsonRpcClient {
    pub async fn new(target: WebSocketTarget) -> Result<Self, TransportError> {
        let request = build_request(&target)?;

        let (socket, _response) = time::timeout(
            Duration::from_millis(target.connect_timeout_ms),
            connect_async(request),
        )
        .await
        .map_err(|_| TransportError::Timeout)?
        .map_err(map_tungstenite_error)?;

        Ok(Self {
            inner: Mutex::new(WebSocketConnection {
                socket,
                read_timeout_ms: target.read_timeout_ms,
                write_timeout_ms: target.write_timeout_ms,
            }),
        })
    }
}

impl JsonRpcClient for WebSocketJsonRpcClient {
    type Error = TransportError;

    fn send<'a>(
        &'a self,
        message: JsonRpcMessage,
    ) -> JsonRpcClientFuture<'a, Result<JsonRpcMessage, Self::Error>> {
        Box::pin(async move {
            let mut inner = self.inner.lock().await;

            inner.write_message(&message).await?;
            inner.read_message().await
        })
    }
}

impl WebSocketConnection {
    async fn write_message(&mut self, message: &JsonRpcMessage) -> Result<(), TransportError> {
        let payload = serde_json::to_string(message)
            .map_err(|source| CodecError::Serialize(source.to_string()))?;

        time::timeout(
            Duration::from_millis(self.write_timeout_ms),
            self.socket.send(Message::Text(payload.into())),
        )
        .await
        .map_err(|_| TransportError::Timeout)?
        .map_err(map_tungstenite_error)?;

        Ok(())
    }

    async fn read_message(&mut self) -> Result<JsonRpcMessage, TransportError> {
        loop {
            let message = time::timeout(
                Duration::from_millis(self.read_timeout_ms),
                self.socket.next(),
            )
            .await
            .map_err(|_| TransportError::Timeout)?
            .ok_or_else(|| TransportError::Connection {
                message: "WebSocket stream ended before response was received".to_owned(),
            })?
            .map_err(map_tungstenite_error)?;

            match message {
                Message::Text(text) => {
                    return serde_json::from_str::<JsonRpcMessage>(text.as_ref())
                        .map_err(|source| CodecError::Deserialize(source.to_string()).into());
                }

                Message::Binary(bytes) => {
                    return serde_json::from_slice::<JsonRpcMessage>(&bytes)
                        .map_err(|source| CodecError::Deserialize(source.to_string()).into());
                }

                Message::Ping(payload) => {
                    time::timeout(
                        Duration::from_millis(self.write_timeout_ms),
                        self.socket.send(Message::Pong(payload)),
                    )
                    .await
                    .map_err(|_| TransportError::Timeout)?
                    .map_err(map_tungstenite_error)?;
                }

                Message::Pong(_) => {
                    continue;
                }

                Message::Close(frame) => {
                    return Err(TransportError::Connection {
                        message: match frame {
                            Some(frame) => format!(
                                "WebSocket peer closed connection: code={}, reason={}",
                                frame.code, frame.reason
                            ),
                            None => "WebSocket peer closed connection".to_owned(),
                        },
                    });
                }

                Message::Frame(_) => {
                    continue;
                }
            }
        }
    }
}

fn build_request(target: &WebSocketTarget) -> Result<Request<()>, TransportError> {
    let mut request = target
        .url
        .as_str()
        .into_client_request()
        .map_err(|source| TransportError::ClientInit {
            message: format!(
                "failed to build WebSocket request for '{}': {source}",
                target.url
            ),
        })?;

    {
        let headers = request.headers_mut();

        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        for (name, value) in &target.headers {
            let header_name =
                HeaderName::from_str(name).map_err(|source| TransportError::ClientInit {
                    message: format!("invalid WebSocket header name '{name}': {source}"),
                })?;

            let header_value =
                HeaderValue::from_str(value).map_err(|source| TransportError::ClientInit {
                    message: format!("invalid WebSocket header value for '{name}': {source}"),
                })?;

            headers.insert(header_name, header_value);
        }
    }

    Ok(request)
}

fn map_tungstenite_error(source: TungsteniteError) -> TransportError {
    match source {
        TungsteniteError::ConnectionClosed | TungsteniteError::AlreadyClosed => {
            TransportError::Connection {
                message: format!("WebSocket connection closed: {source}"),
            }
        }

        TungsteniteError::Io(io) => {
            if io.kind() == std::io::ErrorKind::TimedOut
                || io.kind() == std::io::ErrorKind::WouldBlock
            {
                TransportError::Timeout
            } else {
                TransportError::Io {
                    message: format!("WebSocket I/O error: {io}"),
                }
            }
        }

        TungsteniteError::Tls(tls) => TransportError::Connection {
            message: format!("WebSocket TLS error: {tls}"),
        },

        TungsteniteError::Http(response) => TransportError::HttpStatus {
            status: response.status().as_u16(),
            body: format!("WebSocket handshake failed: HTTP {}", response.status()),
        },

        TungsteniteError::HttpFormat(http) => TransportError::ClientInit {
            message: format!("invalid WebSocket HTTP request/response format: {http}"),
        },

        TungsteniteError::Url(url) => TransportError::ClientInit {
            message: format!("invalid WebSocket URL: {url}"),
        },

        TungsteniteError::Utf8(utf8) => TransportError::Codec(CodecError::Deserialize(format!(
            "invalid WebSocket UTF-8 payload: {utf8}"
        ))),

        TungsteniteError::Protocol(protocol) => TransportError::Connection {
            message: format!("WebSocket protocol error: {protocol}"),
        },

        TungsteniteError::Capacity(capacity) => TransportError::Connection {
            message: format!("WebSocket capacity error: {capacity}"),
        },

        TungsteniteError::WriteBufferFull(_) => TransportError::Io {
            message: "WebSocket write buffer is full".to_owned(),
        },

        TungsteniteError::AttackAttempt => TransportError::Connection {
            message: "WebSocket attack attempt detected".to_owned(),
        },
    }
}
