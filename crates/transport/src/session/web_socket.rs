use crate::{
    TransportError,
    session::{
        JsonRpcSession, JsonRpcSessionEvent, JsonRpcSessionFuture,
        stream_loop::{SessionCore, session_notify, session_request},
    },
    target::WebSocketTarget,
};
use actrpc_core::json_rpc::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
use actrpc_core::{error::CodecError, json_rpc::JsonRpcMessage};
use futures_util::{SinkExt, StreamExt};
use std::{str::FromStr, sync::Arc, time::Duration};
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

pub struct WebSocketJsonRpcSession {
    core: Arc<SessionCore>,
    writer: Arc<
        Mutex<
            futures_util::stream::SplitSink<
                tokio_tungstenite::WebSocketStream<
                    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                >,
                Message,
            >,
        >,
    >,
    write_timeout_ms: u64,
    _reader: tokio::task::JoinHandle<()>,
}

impl WebSocketJsonRpcSession {
    pub async fn new(target: WebSocketTarget) -> Result<Self, TransportError> {
        let request = build_request(&target)?;

        let (socket, _response) = time::timeout(
            Duration::from_millis(target.connect_timeout_ms),
            connect_async(request),
        )
        .await
        .map_err(|_| TransportError::Timeout)?
        .map_err(map_tungstenite_error)?;

        let (writer, reader) = socket.split();
        let read_timeout_ms = target.read_timeout_ms;
        let write_timeout_ms = target.write_timeout_ms;

        let (event_tx, _) = tokio::sync::broadcast::channel(64);
        let core = Arc::new(SessionCore::new(event_tx));
        let writer = Arc::new(Mutex::new(writer));

        let reader_core = core.clone();
        let reader_writer = writer.clone();
        let reader = tokio::spawn(async move {
            run_reader(
                reader,
                reader_core,
                reader_writer,
                read_timeout_ms,
                write_timeout_ms,
            )
            .await;
        });

        Ok(Self {
            core,
            writer,
            write_timeout_ms,
            _reader: reader,
        })
    }
}

impl JsonRpcSession for WebSocketJsonRpcSession {
    type Error = TransportError;

    fn request<'a>(
        &'a self,
        request: JsonRpcRequest,
    ) -> JsonRpcSessionFuture<'a, Result<JsonRpcResponse, Self::Error>> {
        let core = self.core.clone();
        let writer = self.writer.clone();
        let write_timeout_ms = self.write_timeout_ms;

        Box::pin(async move {
            session_request(&core, request, |message| {
                let writer = writer.clone();
                async move { write_message(writer, write_timeout_ms, &message).await }
            })
            .await
        })
    }

    fn notify<'a>(
        &'a self,
        notification: JsonRpcNotification,
    ) -> JsonRpcSessionFuture<'a, Result<(), Self::Error>> {
        let writer = self.writer.clone();
        let write_timeout_ms = self.write_timeout_ms;

        Box::pin(async move {
            session_notify(
                |message| {
                    let writer = writer.clone();
                    async move { write_message(writer, write_timeout_ms, &message).await }
                },
                notification,
            )
            .await
        })
    }

    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<JsonRpcSessionEvent> {
        self.core.subscribe()
    }
}

async fn run_reader(
    mut reader: futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    core: Arc<SessionCore>,
    writer: Arc<
        Mutex<
            futures_util::stream::SplitSink<
                tokio_tungstenite::WebSocketStream<
                    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                >,
                Message,
            >,
        >,
    >,
    read_timeout_ms: u64,
    write_timeout_ms: u64,
) {
    let close_message = loop {
        let message =
            match time::timeout(Duration::from_millis(read_timeout_ms), reader.next()).await {
                Ok(Some(Ok(message))) => message,
                Ok(Some(Err(error))) => break map_tungstenite_error(error).to_string(),
                Ok(None) => break "WebSocket stream ended".to_owned(),
                Err(_) => break "WebSocket session read timed out".to_owned(),
            };

        match message {
            Message::Text(text) => {
                let decoded = match serde_json::from_str::<JsonRpcMessage>(text.as_ref()) {
                    Ok(message) => message,
                    Err(_) => continue,
                };
                core.dispatch_message(decoded).await;
            }
            Message::Binary(bytes) => {
                let decoded = match serde_json::from_slice::<JsonRpcMessage>(&bytes) {
                    Ok(message) => message,
                    Err(_) => continue,
                };
                core.dispatch_message(decoded).await;
            }
            Message::Ping(payload) => {
                let writer = writer.clone();
                let pong_result = time::timeout(Duration::from_millis(write_timeout_ms), async {
                    let mut sink = writer.lock().await;
                    sink.send(Message::Pong(payload)).await
                })
                .await;

                match pong_result {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => break map_tungstenite_error(error).to_string(),
                    Err(_) => break "WebSocket pong write timed out".to_owned(),
                }
            }
            Message::Pong(_) | Message::Frame(_) => {}
            Message::Close(frame) => {
                break match frame {
                    Some(frame) => format!(
                        "WebSocket peer closed connection: code={}, reason={}",
                        frame.code, frame.reason
                    ),
                    None => "WebSocket peer closed connection".to_owned(),
                };
            }
        }
    };

    core.close_with_error(close_message).await;
}

async fn write_message(
    writer: Arc<
        Mutex<
            futures_util::stream::SplitSink<
                tokio_tungstenite::WebSocketStream<
                    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                >,
                Message,
            >,
        >,
    >,
    write_timeout_ms: u64,
    message: &JsonRpcMessage,
) -> Result<(), TransportError> {
    let payload = serde_json::to_string(message)
        .map_err(|source| CodecError::Serialize(source.to_string()))?;

    let mut writer = writer.lock().await;

    time::timeout(
        Duration::from_millis(write_timeout_ms),
        writer.send(Message::Text(payload.into())),
    )
    .await
    .map_err(|_| TransportError::Timeout)?
    .map_err(map_tungstenite_error)?;

    Ok(())
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
