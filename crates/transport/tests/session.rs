use actrpc_core::json_rpc::{
    JsonRpcId, JsonRpcMessage, JsonRpcNotification, JsonRpcParams, JsonRpcRequest, JsonRpcResponse,
    JsonRpcSingleMessage, JsonRpcSuccessResponse, JsonRpcVersion,
};
use actrpc_transport::{
    DefaultJsonRpcSessionProvider, JsonRpcSession, JsonRpcSessionProvider, TransportError,
    session::{JsonRpcSessionEvent, TcpJsonRpcSession},
    target::{HttpTarget, TcpTarget, TransportTarget},
};
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::broadcast,
};

#[derive(Debug)]
struct TestSession {
    event_tx: broadcast::Sender<JsonRpcSessionEvent>,
}

impl TestSession {
    fn new() -> (Self, broadcast::Receiver<JsonRpcSessionEvent>) {
        let (event_tx, rx) = broadcast::channel(8);
        (Self { event_tx }, rx)
    }

    fn inject_notification(&self, n: JsonRpcNotification) {
        let _ = self.event_tx.send(JsonRpcSessionEvent::Notification(n));
    }
}

impl JsonRpcSession for TestSession {
    type Error = TransportError;

    fn request<'a>(
        &'a self,
        request: JsonRpcRequest,
    ) -> actrpc_transport::session::JsonRpcSessionFuture<'a, Result<JsonRpcResponse, Self::Error>>
    {
        let id = request.id.clone();
        Box::pin(async move {
            Ok(JsonRpcResponse::Success(JsonRpcSuccessResponse {
                jsonrpc: JsonRpcVersion::V2_0,
                id,
                result: json!({"echo": "ok"}),
            }))
        })
    }

    fn notify<'a>(
        &'a self,
        _notification: JsonRpcNotification,
    ) -> actrpc_transport::session::JsonRpcSessionFuture<'a, Result<(), Self::Error>> {
        Box::pin(async { Ok(()) })
    }

    fn subscribe(&self) -> broadcast::Receiver<JsonRpcSessionEvent> {
        self.event_tx.subscribe()
    }
}

#[tokio::test]
async fn test_session_request_routes_response_by_id() {
    let (sess, _rx) = TestSession::new();
    let req = JsonRpcRequest {
        jsonrpc: JsonRpcVersion::V2_0,
        id: JsonRpcId::Number(42u64.into()),
        method: "foo".to_owned(),
        params: None,
    };
    let resp = sess.request(req).await.unwrap();
    match resp {
        JsonRpcResponse::Success(s) => {
            assert_eq!(s.id, JsonRpcId::Number(42u64.into()));
        }
        _ => panic!("expected success"),
    }
}

#[tokio::test]
async fn test_session_receives_notification_without_stealing_request_response() {
    let (sess, mut rx) = TestSession::new();
    sess.inject_notification(JsonRpcNotification {
        jsonrpc: JsonRpcVersion::V2_0,
        method: "bar".to_owned(),
        params: None,
    });
    let req = JsonRpcRequest {
        jsonrpc: JsonRpcVersion::V2_0,
        id: JsonRpcId::Number(7u64.into()),
        method: "baz".to_owned(),
        params: None,
    };
    let resp = sess.request(req).await.unwrap();
    let evt = rx.recv().await.unwrap();
    match evt {
        JsonRpcSessionEvent::Notification(n) => assert_eq!(n.method, "bar"),
        _ => panic!("expected notif"),
    }
    match resp {
        JsonRpcResponse::Success(s) => assert_eq!(s.id, JsonRpcId::Number(7u64.into())),
        _ => panic!("expected resp"),
    }
}

#[tokio::test]
async fn test_default_session_provider_http_returns_unsupported() {
    let provider = DefaultJsonRpcSessionProvider::new();
    let target = TransportTarget::Http(HttpTarget {
        url: "http://example.invalid".to_owned(),
        headers: actrpc_transport::HeaderPairs::default(),
        timeout_ms: 1000,
    });
    match provider.get_session(&target).await {
        Ok(_) => panic!("expected unsupported for http"),
        Err(e) => assert!(matches!(e, TransportError::UnsupportedTarget { .. })),
    }
}

#[tokio::test]
async fn test_tcp_session_out_of_order_response_correlation() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let received = Arc::new(AtomicUsize::new(0));

    let server_received = received.clone();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        serve_out_of_order(stream, server_received).await.unwrap();
    });

    let target = tcp_target(addr.to_string());
    let session = TcpJsonRpcSession::new(target).await.unwrap();

    let first = session.request(request_with_id(1, "first"));
    let second = session.request(request_with_id(2, "second"));
    let (resp_first, resp_second) = tokio::join!(first, second);

    assert_success_id(resp_first.unwrap(), 1);
    assert_success_id(resp_second.unwrap(), 2);

    assert_eq!(received.load(Ordering::SeqCst), 2);
    server.await.unwrap();
}

#[tokio::test]
async fn test_tcp_session_notification_does_not_steal_response() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        serve_notification_then_response(stream).await.unwrap();
    });

    let target = tcp_target(addr.to_string());
    let session = TcpJsonRpcSession::new(target).await.unwrap();
    let mut events = session.subscribe();

    let response = session.request(request_with_id(9, "watch")).await.unwrap();

    let event = events.recv().await.unwrap();
    match event {
        JsonRpcSessionEvent::Notification(notification) => {
            assert_eq!(notification.method, "server-event");
        }
        other => panic!("expected notification, got {other:?}"),
    }

    assert_success_id(response, 9);
    server.await.unwrap();
}

#[tokio::test]
async fn test_default_session_provider_caches_tcp_session() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let target = TransportTarget::Tcp(tcp_target(addr.to_string()));
    let provider = DefaultJsonRpcSessionProvider::new();

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = BufReader::new(stream);
        let message = read_newline_message(&mut stream).await.unwrap();
        let JsonRpcMessage::Single(JsonRpcSingleMessage::Request(request)) = message else {
            panic!("expected request");
        };
        write_newline_message(
            stream.get_mut(),
            &success_response(request.id, json!({"ok": true})),
        )
        .await
        .unwrap();
    });

    let first = provider.get_session(&target).await.unwrap();
    let second = provider.get_session(&target).await.unwrap();

    assert!(Arc::ptr_eq(&first, &second));

    let _ = first.request(request_with_id(1, "cached")).await.unwrap();

    server.await.unwrap();
}

async fn serve_out_of_order(
    stream: TcpStream,
    received: Arc<AtomicUsize>,
) -> Result<(), TransportError> {
    let mut stream = BufReader::new(stream);
    let mut requests = Vec::new();

    while received.load(Ordering::SeqCst) < 2 {
        let message = read_newline_message(&mut stream).await?;
        let JsonRpcMessage::Single(JsonRpcSingleMessage::Request(request)) = message else {
            continue;
        };
        requests.push(request);
        received.fetch_add(1, Ordering::SeqCst);
    }

    let response_for = |id: u64| {
        requests
            .iter()
            .find(|request| request_id(request) == id)
            .map(|request| success_response(request.id.clone(), json!({"id": id})))
            .expect("expected request id")
    };

    write_newline_message(stream.get_mut(), &response_for(2)).await?;
    write_newline_message(stream.get_mut(), &response_for(1)).await?;

    Ok(())
}

async fn serve_notification_then_response(stream: TcpStream) -> Result<(), TransportError> {
    let mut stream = BufReader::new(stream);
    let message = read_newline_message(&mut stream).await?;
    let JsonRpcMessage::Single(JsonRpcSingleMessage::Request(request)) = message else {
        panic!("expected request");
    };

    write_newline_message(
        stream.get_mut(),
        &JsonRpcMessage::Single(JsonRpcSingleMessage::Notification(JsonRpcNotification {
            jsonrpc: JsonRpcVersion::V2_0,
            method: "server-event".to_owned(),
            params: None,
        })),
    )
    .await?;

    write_newline_message(
        stream.get_mut(),
        &success_response(request.id, json!({"ok": true})),
    )
    .await?;

    Ok(())
}

fn tcp_target(addr: String) -> TcpTarget {
    let target: TransportTarget = serde_json::from_value(json!({
        "tcp": {
            "addr": addr,
            "framing": "newline_delimited",
            "connect_timeout_ms": 1_000,
            "read_timeout_ms": 5_000,
            "write_timeout_ms": 5_000,
            "nodelay": true
        }
    }))
    .expect("tcp target json");

    let TransportTarget::Tcp(target) = target else {
        panic!("expected tcp transport target");
    };

    target
}

fn request_with_id(id: u64, method: &str) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: JsonRpcVersion::V2_0,
        id: JsonRpcId::Number(id.into()),
        method: method.to_owned(),
        params: Some(JsonRpcParams::Array(vec![json!(1)])),
    }
}

fn success_response(id: JsonRpcId, result: serde_json::Value) -> JsonRpcMessage {
    JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Success(
        JsonRpcSuccessResponse {
            jsonrpc: JsonRpcVersion::V2_0,
            id,
            result,
        },
    )))
}

fn request_id(request: &JsonRpcRequest) -> u64 {
    match &request.id {
        JsonRpcId::Number(number) => number
            .as_u64()
            .expect("test requests use unsigned integer ids"),
        other => panic!("unexpected id type: {other:?}"),
    }
}

fn assert_success_id(response: JsonRpcResponse, expected_id: u64) {
    match response {
        JsonRpcResponse::Success(success) => {
            assert_eq!(success.id, JsonRpcId::Number(expected_id.into()));
        }
        other => panic!("expected success response, got {other:?}"),
    }
}

async fn read_newline_message<R>(reader: &mut R) -> Result<JsonRpcMessage, TransportError>
where
    R: AsyncBufRead + Unpin,
{
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .await
        .map_err(|source| TransportError::Io {
            message: source.to_string(),
        })?;

    serde_json::from_str(line.trim()).map_err(|source| {
        TransportError::Codec(actrpc_core::error::CodecError::Deserialize(
            source.to_string(),
        ))
    })
}

async fn write_newline_message<W>(
    writer: &mut W,
    message: &JsonRpcMessage,
) -> Result<(), TransportError>
where
    W: AsyncWrite + Unpin,
{
    let payload = serde_json::to_vec(message)
        .map_err(|source| actrpc_core::error::CodecError::Serialize(source.to_string()))?;

    writer
        .write_all(&payload)
        .await
        .map_err(|source| TransportError::Io {
            message: source.to_string(),
        })?;
    writer
        .write_all(b"\n")
        .await
        .map_err(|source| TransportError::Io {
            message: source.to_string(),
        })?;
    writer.flush().await.map_err(|source| TransportError::Io {
        message: source.to_string(),
    })?;

    Ok(())
}
