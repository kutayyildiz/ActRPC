use serde_json::{Value, json};

fn intercept_params(message: Value, resolved_action_history: Value) -> Value {
    json!({
        "origin": {
            "kind": "external",
            "id": "caller"
        },
        "target": {
            "provider": "test",
            "method": message.get("method").and_then(Value::as_str).unwrap_or("unknown")
        },
        "message": message,
        "call_id": "550e8400-e29b-41d4-a716-446655440000",
        "interception_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "resolved_action_history": resolved_action_history
    })
}
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

#[test]
fn policy_binary_initialize_smoke_test() {
    let mut process = PolicyProcess::spawn("block_sensitive_write.yaml");

    let response = process.send(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "actrpc.interceptor.initialize",
        "params": null
    }));

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert_eq!(response["result"]["supports_outbound"], true);
    assert_eq!(response["result"]["supports_inbound"], true);

    let actions = &response["result"]["actions"];

    assert!(actions.get("reject_call").is_some());
    assert!(actions.get("exclude_interceptors").is_some());
    assert!(actions.get("request_review").is_some());
    assert_eq!(actions.as_object().unwrap().len(), 3);
}

#[test]
fn policy_binary_intercept_reject_call_smoke_test() {
    let mut process = PolicyProcess::spawn("block_sensitive_write.yaml");

    let _ = process.initialize();

    let response = process.send(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "actrpc.interceptor.intercept",
        "params": intercept_params(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "write_file",
                "params": {
                    "path": "/etc/passwd"
                }
            }),
            json!([]),
        )
    }));

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);

    let actions = result_actions(&response);

    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0]["kind"], "reject_call");
    assert_eq!(actions[0]["params"]["error"]["code"], -32010);
    assert_eq!(
        actions[0]["params"]["error"]["message"],
        "write to sensitive system path denied"
    );
    assert_eq!(response["result"]["continuation"], "stop");
}

#[test]
fn policy_binary_intercept_no_match_smoke_test() {
    let mut process = PolicyProcess::spawn("block_sensitive_write.yaml");

    let _ = process.initialize();

    let response = process.send(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "actrpc.interceptor.intercept",
        "params": intercept_params(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "write_file",
                "params": {
                    "path": "/tmp/file.txt"
                }
            }),
            json!([]),
        )
    }));

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);

    let actions = result_actions(&response);

    assert!(actions.is_empty());
    assert_eq!(response["result"]["continuation"], "stop");
}

#[test]
fn policy_binary_intercept_review_request_smoke_test() {
    let mut process = PolicyProcess::spawn("simple_review.yaml");

    let _ = process.initialize();

    let response = process.send(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "actrpc.interceptor.intercept",
        "params": intercept_params(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "write_file",
                "params": {
                    "path": "/home/mortal/file.txt"
                }
            }),
            json!([]),
        )
    }));

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);

    let actions = result_actions(&response);

    assert_eq!(response["result"]["continuation"], "reinvoke");
    assert_eq!(actions.len(), 1);

    assert_eq!(actions[0]["kind"], "request_review");
    assert_eq!(actions[0]["params"]["rule_name"], "review_sensitive_write");
    assert_eq!(actions[0]["params"]["title"], "Sensitive file write");
    assert_eq!(
        actions[0]["params"]["reason"],
        "Agent wants to write inside a user-owned directory."
    );
    assert_eq!(actions[0]["params"]["severity"], "high");
}

#[test]
fn policy_binary_intercept_review_approved_effect_smoke_test() {
    let mut process = PolicyProcess::spawn("simple_review.yaml");

    let _ = process.initialize();

    let response = process.send(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "actrpc.interceptor.intercept",
        "params": intercept_params(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "write_file",
                "params": {
                    "path": "/home/mortal/file.txt"
                }
            }),
            json!([
                [
                    {
                        "kind": "request_review",
                        "params": {
                            "rule_name": "review_sensitive_write",
                            "title": "Sensitive file write",
                            "reason": "Agent wants to write inside a user-owned directory.",
                            "severity": "high"
                        },
                        "result": {
                            "Ok": {
                                "decision": "approved"
                            }
                        }
                    }
                ]
            ]),
        )
    }));

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);

    assert!(
        response.get("error").is_none(),
        "unexpected JSON-RPC error response: {response:#}"
    );

    let actions = result_actions(&response);

    assert_eq!(response["result"]["continuation"], "stop");
    assert_eq!(actions.len(), 1);

    assert_eq!(actions[0]["kind"], "exclude_interceptors");
    assert_eq!(actions[0]["params"]["names"], json!(["transcript_logger"]));
}

#[test]
fn policy_binary_intercept_review_denied_effect_smoke_test() {
    let mut process = PolicyProcess::spawn("simple_review.yaml");

    let _ = process.initialize();

    let response = process.send(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "actrpc.interceptor.intercept",
        "params": intercept_params(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "write_file",
                "params": {
                    "path": "/home/mortal/file.txt"
                }
            }),
            json!([
                [
                    {
                        "kind": "request_review",
                        "params": {
                            "rule_name": "review_sensitive_write",
                            "title": "Sensitive file write",
                            "reason": "Agent wants to write inside a user-owned directory.",
                            "severity": "high"
                        },
                        "result": {
                            "Ok": {
                                "decision": "denied"
                            }
                        }
                    }
                ]
            ]),
        )
    }));

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);

    assert!(
        response.get("error").is_none(),
        "unexpected JSON-RPC error response: {response:#}"
    );

    let actions = result_actions(&response);

    assert_eq!(actions.len(), 2);

    assert_eq!(actions[0]["kind"], "exclude_interceptors");
    assert_eq!(actions[0]["params"]["names"], json!(["transcript_logger"]));

    assert_eq!(actions[1]["kind"], "reject_call");
    assert_eq!(actions[1]["params"]["error"]["code"], -32051);
    assert_eq!(
        actions[1]["params"]["error"]["message"],
        "user denied sensitive file write"
    );
}

#[test]
fn policy_binary_invalid_json_returns_parse_error() {
    let mut process = PolicyProcess::spawn("block_sensitive_write.yaml");

    let response = process.send_raw("{ invalid json");

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], Value::Null);
    assert_eq!(response["error"]["code"], -32700);
}

#[test]
fn policy_binary_rejects_non_request_message() {
    let mut process = PolicyProcess::spawn("block_sensitive_write.yaml");

    let response = process.send(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {}
    }));

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], Value::Null);
    assert_eq!(response["error"]["code"], -32600);
}

#[test]
fn policy_binary_unknown_method_returns_method_not_found() {
    let mut process = PolicyProcess::spawn("block_sensitive_write.yaml");

    let response = process.send(json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "unknown_method",
        "params": null
    }));

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 7);
    assert_eq!(response["error"]["code"], -32601);
}

#[test]
fn policy_binary_intercept_without_params_returns_invalid_params() {
    let mut process = PolicyProcess::spawn("block_sensitive_write.yaml");

    let response = process.send(json!({
        "jsonrpc": "2.0",
        "id": 8,
        "method": "actrpc.interceptor.intercept"
    }));

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 8);
    assert_eq!(response["error"]["code"], -32602);
}

fn result_actions(response: &Value) -> Vec<Value> {
    response["result"]
        .get("actions")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default()
}

struct PolicyProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl PolicyProcess {
    fn spawn(example_name: &str) -> Self {
        let binary = env!("CARGO_BIN_EXE_actrpc_policy_interceptor");

        let config_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("interceptors")
            .join("policy")
            .join("config")
            .join("examples")
            .join(example_name);

        let mut child = Command::new(binary)
            .arg("--config")
            .arg(config_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("failed to spawn actrpc_policy_interceptor");

        let stdin = child.stdin.take().expect("missing child stdin");
        let stdout = BufReader::new(child.stdout.take().expect("missing child stdout"));

        Self {
            child,
            stdin,
            stdout,
        }
    }

    fn initialize(&mut self) -> Value {
        self.send(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "actrpc.interceptor.initialize",
            "params": null
        }))
    }

    fn send(&mut self, request: Value) -> Value {
        self.send_raw(&serde_json::to_string(&request).unwrap())
    }

    fn send_raw(&mut self, request: &str) -> Value {
        writeln!(self.stdin, "{request}").unwrap();
        self.stdin.flush().unwrap();

        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();

        assert!(
            !line.trim().is_empty(),
            "policy interceptor produced empty stdout response"
        );

        serde_json::from_str(&line).unwrap()
    }
}

impl Drop for PolicyProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
