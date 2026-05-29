use actrpc_transport::{
    client::StdioJsonRpcClient,
    target::{StdioTarget, TransportTarget},
};
use serde_json::json;

#[cfg(unix)]
#[tokio::test]
async fn test_stdio_client_roundtrip_newline_delimited_with_cat() {
    let target = stdio_target("cat", "newline_delimited");
    let client = StdioJsonRpcClient::new(target).unwrap();

    let request = request(11, "echo");
    let response = actrpc_transport::JsonRpcClient::send(&client, request.clone())
        .await
        .unwrap();

    assert_eq!(response, request);
}

#[cfg(unix)]
#[tokio::test]
async fn test_stdio_client_roundtrip_content_length_with_cat() {
    let target = stdio_target("cat", "content_length");
    let client = StdioJsonRpcClient::new(target).unwrap();

    let request = request(12, "echo");
    let response = actrpc_transport::JsonRpcClient::send(&client, request.clone())
        .await
        .unwrap();

    assert_eq!(response, request);
}

#[cfg(windows)]
#[tokio::test]
async fn test_stdio_client_roundtrip_newline_delimited_with_powershell() {
    let target = windows_echo_stdio_target("newline_delimited");
    let client = StdioJsonRpcClient::new(target).unwrap();

    let request = request(21, "echo");
    let response = actrpc_transport::JsonRpcClient::send(&client, request.clone())
        .await
        .unwrap();

    assert_eq!(response, request);
}

#[cfg(windows)]
#[tokio::test]
async fn test_stdio_client_roundtrip_content_length_with_powershell() {
    let target = windows_echo_stdio_target("content_length");
    let client = StdioJsonRpcClient::new(target).unwrap();

    let request = request(22, "echo");
    let response = actrpc_transport::JsonRpcClient::send(&client, request.clone())
        .await
        .unwrap();

    assert_eq!(response, request);
}

#[test]
fn test_stdio_client_spawn_failure_returns_error() {
    let target = stdio_target("__actrpc_missing_stdio_program__", "newline_delimited");

    let err = StdioJsonRpcClient::new(target).unwrap_err();

    assert!(err.to_string().contains("failed to spawn stdio target"));
}

fn stdio_target(program: &str, framing: &str) -> StdioTarget {
    let target: TransportTarget = serde_json::from_value(json!({
        "stdio": {
            "program": program,
            "args": [],
            "env": [],
            "framing": framing
        }
    }))
    .unwrap();

    let TransportTarget::Stdio(target) = target else {
        panic!("expected stdio target");
    };

    target
}

#[cfg(any(unix, windows))]
fn request(id: u64, method: impl Into<String>) -> actrpc_core::json_rpc::JsonRpcMessage {
    use actrpc_core::json_rpc::{
        JsonRpcId, JsonRpcMessage, JsonRpcParams, JsonRpcRequest, JsonRpcSingleMessage,
        JsonRpcVersion,
    };

    JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
        jsonrpc: JsonRpcVersion::V2_0,
        id: JsonRpcId::Number(id.into()),
        method: method.into(),
        params: Some(JsonRpcParams::Array(vec![json!(1), json!(2)])),
    }))
}

#[cfg(windows)]
fn windows_echo_stdio_target(framing: &str) -> StdioTarget {
    let script = r#"
$stdin = [Console]::OpenStandardInput()
$stdout = [Console]::OpenStandardOutput()
$buffer = New-Object byte[] 8192

while (($read = $stdin.Read($buffer, 0, $buffer.Length)) -gt 0) {
    $stdout.Write($buffer, 0, $read)
    $stdout.Flush()
}
"#;

    let target: TransportTarget = serde_json::from_value(json!({
        "stdio": {
            "program": "powershell.exe",
            "args": [
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                script
            ],
            "env": [],
            "framing": framing
        }
    }))
    .unwrap();

    let TransportTarget::Stdio(target) = target else {
        panic!("expected stdio target");
    };

    target
}
