use actrpc_transport::TransportTarget;
use serde_json::json;

#[test]
fn test_tcp_target_deserializes_defaults() {
    let target: TransportTarget = serde_json::from_value(json!({
        "tcp": {
            "addr": "127.0.0.1:12345"
        }
    }))
    .unwrap();

    let ser = serde_json::to_value(target).unwrap();

    assert_eq!(ser["tcp"]["addr"], "127.0.0.1:12345");
    assert_eq!(ser["tcp"]["framing"], "newline_delimited");
    assert_eq!(ser["tcp"]["connect_timeout_ms"], 10_000);
    assert_eq!(ser["tcp"]["read_timeout_ms"], 30_000);
    assert_eq!(ser["tcp"]["write_timeout_ms"], 30_000);
    assert_eq!(ser["tcp"]["nodelay"], true);
}

#[test]
fn test_tcp_target_accepts_content_length_framing() {
    let target: TransportTarget = serde_json::from_value(json!({
        "tcp": {
            "addr": "127.0.0.1:12345",
            "framing": "content_length"
        }
    }))
    .unwrap();

    let ser = serde_json::to_value(target).unwrap();

    assert_eq!(ser["tcp"]["framing"], "content_length");
}

#[test]
fn test_stdio_target_deserializes_defaults() {
    let target: TransportTarget = serde_json::from_value(json!({
        "stdio": {
            "program": "cat"
        }
    }))
    .unwrap();

    let ser = serde_json::to_value(target).unwrap();

    assert_eq!(ser["stdio"]["program"], "cat");
    assert_eq!(ser["stdio"]["args"], json!([]));
    assert_eq!(ser["stdio"]["env"], json!([]));
    assert_eq!(ser["stdio"]["framing"], "newline_delimited");
}

#[test]
fn test_http_target_deserializes_defaults() {
    let target: TransportTarget = serde_json::from_value(json!({
        "http": {
            "url": "http://127.0.0.1:8080/rpc"
        }
    }))
    .unwrap();

    let ser = serde_json::to_value(target).unwrap();

    assert_eq!(ser["http"]["url"], "http://127.0.0.1:8080/rpc");
    assert_eq!(ser["http"]["headers"], json!([]));
    assert_eq!(ser["http"]["timeout_ms"], 30_000);
}

#[test]
fn test_web_socket_target_deserializes_defaults() {
    let target: TransportTarget = serde_json::from_value(json!({
        "web_socket": {
            "url": "ws://127.0.0.1:8080/rpc"
        }
    }))
    .unwrap();

    let ser = serde_json::to_value(target).unwrap();

    assert_eq!(ser["web_socket"]["url"], "ws://127.0.0.1:8080/rpc");
    assert_eq!(ser["web_socket"]["connect_timeout_ms"], 10_000);
    assert_eq!(ser["web_socket"]["read_timeout_ms"], 30_000);
    assert_eq!(ser["web_socket"]["write_timeout_ms"], 30_000);
}

#[test]
fn test_local_ipc_target_deserializes_defaults() {
    let target: TransportTarget = serde_json::from_value(json!({
        "local_ipc": {
            "name": "actrpc-test"
        }
    }))
    .unwrap();

    let ser = serde_json::to_value(target).unwrap();

    assert_eq!(ser["local_ipc"]["name"], "actrpc-test");
    assert_eq!(ser["local_ipc"]["namespace"], "generic");
    assert_eq!(ser["local_ipc"]["framing"], "newline_delimited");
    assert_eq!(ser["local_ipc"]["connect_timeout_ms"], 10_000);
    assert_eq!(ser["local_ipc"]["read_timeout_ms"], 30_000);
    assert_eq!(ser["local_ipc"]["write_timeout_ms"], 30_000);
}

#[test]
fn test_target_rejects_unknown_fields() {
    let err = serde_json::from_value::<TransportTarget>(json!({
        "tcp": {
            "addr": "127.0.0.1:12345",
            "unknown": true
        }
    }))
    .unwrap_err();

    assert!(err.to_string().contains("unknown field"));
}
