use serde_json::json;
use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};

#[test]
fn dynamic_policy_binary_initialize_returns_actions() {
    let mut child = spawn_binary();

    let response = send_request(
        &mut child,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "actrpc.interceptor.initialize",
            "params": null
        }),
    );

    assert_eq!(
        response["result"]["supports_outbound"], true,
        "unexpected initialize response: {response}"
    );
    assert_eq!(response["result"]["supports_inbound"], true);
    assert!(response["result"]["actions"].is_object());
}

fn spawn_binary() -> std::process::Child {
    let binary = env!("CARGO_BIN_EXE_actrpc_dynamic_policy_interceptor");
    Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn actrpc_dynamic_policy_interceptor")
}

fn send_request(child: &mut std::process::Child, request: serde_json::Value) -> serde_json::Value {
    let stdin = child.stdin.as_mut().expect("stdin");
    writeln!(stdin, "{request}").expect("write stdin");
    stdin.flush().expect("flush stdin");

    let stdout = child.stdout.as_mut().expect("stdout");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line).expect("read stdout");

    serde_json::from_str(&line).expect("parse response")
}