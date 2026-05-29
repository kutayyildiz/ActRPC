use actrpc_core::json_rpc::{
    JsonRpcId, JsonRpcMessage, JsonRpcParams, JsonRpcRequest, JsonRpcResponse,
    JsonRpcSingleMessage, JsonRpcSuccessResponse, JsonRpcVersion,
};
use actrpc_transport::{
    TransportError,
    client::TcpJsonRpcClient,
    target::{TcpTarget, TransportTarget},
};
use serde_json::{Value, json};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

#[tokio::test]
async fn test_tcp_client_roundtrip_newline_delimited() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        handle_newline_delimited_once(stream).await.unwrap();
    });

    let target = tcp_target(addr.to_string(), "newline_delimited");
    let client = TcpJsonRpcClient::new(target).await.unwrap();

    let response = actrpc_transport::JsonRpcClient::send(&client, request(7, "sum"))
        .await
        .unwrap();

    assert_success_result(response, 7, "sum");

    server.await.unwrap();
}

#[tokio::test]
async fn test_tcp_client_roundtrip_content_length() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        handle_content_length_once(stream).await.unwrap();
    });

    let target = tcp_target(addr.to_string(), "content_length");
    let client = TcpJsonRpcClient::new(target).await.unwrap();

    let response = actrpc_transport::JsonRpcClient::send(&client, request(9, "mul"))
        .await
        .unwrap();

    assert_success_result(response, 9, "mul");

    server.await.unwrap();
}

#[tokio::test]
async fn test_tcp_client_connection_failure_returns_connection_error() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    drop(listener);

    let target = tcp_target(addr.to_string(), "newline_delimited");

    let err = TcpJsonRpcClient::new(target).await.unwrap_err();

    assert!(matches!(
        err,
        TransportError::Connection { .. } | TransportError::Timeout
    ));
}

fn tcp_target(addr: String, framing: &str) -> TcpTarget {
    let target: TransportTarget = serde_json::from_value(json!({
        "tcp": {
            "addr": addr,
            "framing": framing,
            "connect_timeout_ms": 1_000,
            "read_timeout_ms": 1_000,
            "write_timeout_ms": 1_000,
            "nodelay": true
        }
    }))
    .unwrap();

    let TransportTarget::Tcp(target) = target else {
        panic!("expected TCP target");
    };

    target
}

async fn handle_newline_delimited_once(stream: TcpStream) -> Result<(), TransportError> {
    let mut stream = BufReader::new(stream);

    let mut line = String::new();

    stream
        .read_line(&mut line)
        .await
        .map_err(|source| TransportError::Io {
            message: source.to_string(),
        })?;

    let request: JsonRpcMessage = serde_json::from_str(line.trim())
        .map_err(|source| actrpc_core::error::CodecError::Deserialize(source.to_string()))?;

    let response = response_for(&request);

    let payload = serde_json::to_vec(&response)
        .map_err(|source| actrpc_core::error::CodecError::Serialize(source.to_string()))?;

    let stream = stream.get_mut();

    stream
        .write_all(&payload)
        .await
        .map_err(|source| TransportError::Io {
            message: source.to_string(),
        })?;

    stream
        .write_all(b"\n")
        .await
        .map_err(|source| TransportError::Io {
            message: source.to_string(),
        })?;

    stream.flush().await.map_err(|source| TransportError::Io {
        message: source.to_string(),
    })?;

    Ok(())
}

async fn handle_content_length_once(stream: TcpStream) -> Result<(), TransportError> {
    let mut stream = BufReader::new(stream);

    let content_length = read_content_length(&mut stream).await?;

    let mut payload = vec![0u8; content_length];

    stream
        .read_exact(&mut payload)
        .await
        .map_err(|source| TransportError::Io {
            message: source.to_string(),
        })?;

    let request: JsonRpcMessage = serde_json::from_slice(&payload)
        .map_err(|source| actrpc_core::error::CodecError::Deserialize(source.to_string()))?;

    let response = response_for(&request);

    let response_payload = serde_json::to_vec(&response)
        .map_err(|source| actrpc_core::error::CodecError::Serialize(source.to_string()))?;

    let header = format!("Content-Length: {}\r\n\r\n", response_payload.len());

    let stream = stream.get_mut();

    stream
        .write_all(header.as_bytes())
        .await
        .map_err(|source| TransportError::Io {
            message: source.to_string(),
        })?;

    stream
        .write_all(&response_payload)
        .await
        .map_err(|source| TransportError::Io {
            message: source.to_string(),
        })?;

    stream.flush().await.map_err(|source| TransportError::Io {
        message: source.to_string(),
    })?;

    Ok(())
}

async fn read_content_length<R>(reader: &mut R) -> Result<usize, TransportError>
where
    R: AsyncBufReadExt + Unpin,
{
    let mut content_length = None;

    loop {
        let mut line = String::new();

        let bytes_read =
            reader
                .read_line(&mut line)
                .await
                .map_err(|source| TransportError::Io {
                    message: source.to_string(),
                })?;

        if bytes_read == 0 {
            return Err(TransportError::Connection {
                message: "peer closed while reading content-length headers".to_owned(),
            });
        }

        let line = line.trim_end_matches(['\r', '\n']);

        if line.is_empty() {
            break;
        }

        let Some((name, value)) = line.split_once(':') else {
            continue;
        };

        if name.trim().eq_ignore_ascii_case("content-length") {
            let parsed = value.trim().parse::<usize>().map_err(|source| {
                actrpc_core::error::CodecError::Deserialize(source.to_string())
            })?;

            content_length = Some(parsed);
        }
    }

    content_length.ok_or_else(|| {
        TransportError::Codec(actrpc_core::error::CodecError::Deserialize(
            "missing Content-Length header".to_owned(),
        ))
    })
}

fn request(id: u64, method: impl Into<String>) -> JsonRpcMessage {
    JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
        jsonrpc: JsonRpcVersion::V2_0,
        id: JsonRpcId::Number(id.into()),
        method: method.into(),
        params: Some(JsonRpcParams::Array(vec![json!(1), json!(2)])),
    }))
}

fn response_for(message: &JsonRpcMessage) -> JsonRpcMessage {
    let JsonRpcMessage::Single(JsonRpcSingleMessage::Request(req)) = message else {
        panic!("test server expected a single JSON-RPC request");
    };

    JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Success(
        JsonRpcSuccessResponse {
            jsonrpc: JsonRpcVersion::V2_0,
            id: req.id.clone(),
            result: json!({
                "method": req.method,
                "params": req.params,
            }),
        },
    )))
}

fn assert_success_result(message: JsonRpcMessage, expected_id: u64, expected_method: &str) {
    let JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Success(resp))) =
        message
    else {
        panic!("expected single JSON-RPC success response");
    };

    assert_eq!(resp.id, JsonRpcId::Number(expected_id.into()));

    let Value::Object(result) = resp.result else {
        panic!("expected object result");
    };

    assert_eq!(result.get("method"), Some(&json!(expected_method)));
}
