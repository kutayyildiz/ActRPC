use actrpc_core::{
    CallId, CallRelation, CurrentExecutionContext, ExecutionContextQueryResult, InterceptionId,
    MethodTarget,
    action::ResolvedActionRecord,
    participant::{Participant, ParticipantType},
};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

#[test]
fn dynamic_policy_binary_provider_and_interceptor_share_store() {
    let mut process = DynamicPolicyProcess::spawn();
    let root = CallId::new();
    let principal = CallId::new();
    let descendant = CallId::new();
    let interception_id = InterceptionId::new();

    let create_response = process.send(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "create_scope",
        "params": {
            "owner_call_id": root,
            "root_call_id": root,
            "creator": { "kind": "interceptor", "id": "planner" },
            "target_selector": { "provider": "tools", "method": "agent_x" },
            "allowed_method_targets": [
                { "provider": "demo", "method": "method_1" }
            ],
            "relation_mode": "direct_child"
        }
    }));

    assert_eq!(create_response["jsonrpc"], "2.0");
    assert_eq!(create_response["id"], 1);
    let scope_id = create_response["result"]["scope_id"]
        .as_str()
        .expect("create_scope should return scope_id");

    let list_response = process.send(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "list_scopes",
        "params": { "owner_call_id": root }
    }));

    assert_eq!(
        list_response["result"]["scopes"].as_array().unwrap().len(),
        1
    );

    let init_response = process.send(json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "actrpc.interceptor.initialize",
        "params": null
    }));

    assert_eq!(init_response["result"]["supports_outbound"], true);
    assert_eq!(init_response["result"]["supports_inbound"], false);
    assert_eq!(
        init_response["result"]["actions"]
            .as_object()
            .unwrap()
            .len(),
        2
    );

    let principal_current = CurrentExecutionContext {
        origin: Participant {
            kind: ParticipantType::Interceptor,
            id: "planner".to_owned(),
        },
        target: MethodTarget {
            provider: "tools".to_owned(),
            method: "agent_x".to_owned(),
        },
        call_id: principal,
        root_call_id: root,
        parent_call_id: Some(root),
        interception_id,
    };

    let bind_response = process.send(json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "actrpc.interceptor.intercept",
        "params": intercept_params(
            principal,
            json!({ "provider": "tools", "method": "agent_x" }),
            vec![resolved_query_current(principal_current)],
        )
    }));

    assert_eq!(bind_response["result"]["continuation"], "stop");
    assert!(result_actions(&bind_response).is_empty());

    let get_response = process.send(json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "get_scope",
        "params": { "scope_id": scope_id }
    }));

    assert_eq!(
        get_response["result"]["bound_call_id"],
        serde_json::to_value(principal).unwrap()
    );

    let descendant_current = CurrentExecutionContext {
        origin: Participant {
            kind: ParticipantType::Interceptor,
            id: "planner".to_owned(),
        },
        target: MethodTarget {
            provider: "demo".to_owned(),
            method: "method_7".to_owned(),
        },
        call_id: descendant,
        root_call_id: root,
        parent_call_id: Some(principal),
        interception_id,
    };

    let reject_response = process.send(json!({
        "jsonrpc": "2.0",
        "id": 6,
        "method": "actrpc.interceptor.intercept",
        "params": intercept_params(
            descendant,
            json!({ "provider": "demo", "method": "method_7" }),
            vec![
                resolved_query_current(descendant_current),
                resolved_query_relation(descendant, principal, CallRelation::Parent),
            ],
        )
    }));

    assert!(
        reject_response.get("error").is_none(),
        "unexpected JSON-RPC error: {reject_response:#}"
    );
    assert_eq!(reject_response["result"]["continuation"], "stop");
    let actions = result_actions(&reject_response);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0]["kind"], "reject_call");
    assert_eq!(actions[0]["params"]["error"]["code"], -32011);
}

#[test]
fn dynamic_policy_binary_method_provider_initialize_returns_snapshot() {
    let mut process = DynamicPolicyProcess::spawn();

    let response = process.send(json!({
        "jsonrpc": "2.0",
        "id": 11,
        "method": "actrpc.method_provider.initialize",
        "params": null
    }));

    assert_eq!(response["result"]["provider"], "dynamic_policy");

    let methods = response["result"]["methods"]
        .as_array()
        .expect("initialize should return methods array");
    assert_eq!(methods.len(), 4);

    let method_names: Vec<_> = methods
        .iter()
        .filter_map(|method| method.get("name").and_then(Value::as_str))
        .collect();

    for expected in ["create_scope", "release_scope", "get_scope", "list_scopes"] {
        assert!(
            method_names.contains(&expected),
            "missing method {expected}"
        );
    }
}

#[test]
fn dynamic_policy_binary_create_scope_empty_allowlist_returns_invalid_params() {
    let mut process = DynamicPolicyProcess::spawn();
    let root = CallId::new();

    let response = process.send(json!({
        "jsonrpc": "2.0",
        "id": 12,
        "method": "create_scope",
        "params": {
            "owner_call_id": root,
            "root_call_id": root,
            "creator": { "kind": "interceptor", "id": "planner" },
            "target_selector": { "provider": "tools", "method": "agent_x" },
            "allowed_method_targets": [],
            "relation_mode": "direct_child"
        }
    }));

    assert_eq!(response["error"]["code"], -32602);
}

#[test]
fn dynamic_policy_binary_release_scope_creator_mismatch_returns_stable_code() {
    let mut process = DynamicPolicyProcess::spawn();
    let root = CallId::new();

    let create_response = process.send(json!({
        "jsonrpc": "2.0",
        "id": 13,
        "method": "create_scope",
        "params": {
            "owner_call_id": root,
            "root_call_id": root,
            "creator": { "kind": "interceptor", "id": "planner" },
            "target_selector": { "provider": "tools", "method": "agent_x" },
            "allowed_method_targets": [
                { "provider": "demo", "method": "method_1" }
            ],
            "relation_mode": "direct_child"
        }
    }));

    let scope_id = create_response["result"]["scope_id"]
        .as_str()
        .expect("create_scope should return scope_id");

    let response = process.send(json!({
        "jsonrpc": "2.0",
        "id": 14,
        "method": "release_scope",
        "params": {
            "scope_id": scope_id,
            "creator": { "kind": "interceptor", "id": "other" }
        }
    }));

    assert_eq!(response["error"]["code"], -32013);
}

#[test]
fn dynamic_policy_binary_unknown_provider_method_returns_method_not_found() {
    let mut process = DynamicPolicyProcess::spawn();

    let response = process.send(json!({
        "jsonrpc": "2.0",
        "id": 9,
        "method": "unknown_provider_method",
        "params": null
    }));

    assert_eq!(response["error"]["code"], -32601);
}

#[test]
fn dynamic_policy_binary_scope_not_found_returns_stable_code() {
    let mut process = DynamicPolicyProcess::spawn();

    let response = process.send(json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "get_scope",
        "params": { "scope_id": "550e8400-e29b-41d4-a716-446655440000" }
    }));

    assert_eq!(response["error"]["code"], -32012);
}

fn resolved_query_current(current: CurrentExecutionContext) -> ResolvedActionRecord {
    ResolvedActionRecord {
        kind: actrpc_core::action::ActionKind::from("query_execution_context"),
        params: Some(json!({ "query": { "kind": "current" } })),
        result: Ok(Some(
            serde_json::to_value(ExecutionContextQueryResult::Current(current)).unwrap(),
        )),
    }
}

fn resolved_query_relation(
    subject: CallId,
    other: CallId,
    relation: CallRelation,
) -> ResolvedActionRecord {
    ResolvedActionRecord {
        kind: actrpc_core::action::ActionKind::from("query_execution_context"),
        params: Some(json!({
            "query": {
                "kind": "relation",
                "subject": subject,
                "other": other
            }
        })),
        result: Ok(Some(
            serde_json::to_value(ExecutionContextQueryResult::Relation(relation)).unwrap(),
        )),
    }
}

fn intercept_params(call_id: CallId, target: Value, history: Vec<ResolvedActionRecord>) -> Value {
    json!({
        "origin": { "kind": "interceptor", "id": "planner" },
        "target": target,
        "message": {
            "jsonrpc": "2.0",
            "id": 1,
            "method": target.get("method").and_then(Value::as_str).unwrap_or("noop")
        },
        "call_id": call_id,
        "interception_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "resolved_action_history": [history]
    })
}

fn result_actions(response: &Value) -> Vec<Value> {
    response["result"]
        .get("actions")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default()
}

struct DynamicPolicyProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl DynamicPolicyProcess {
    fn spawn() -> Self {
        let binary = env!("CARGO_BIN_EXE_actrpc_dynamic_policy_component");

        let mut child = Command::new(binary)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("failed to spawn actrpc_dynamic_policy_component");

        let stdin = child.stdin.take().expect("missing child stdin");
        let stdout = BufReader::new(child.stdout.take().expect("missing child stdout"));

        Self {
            child,
            stdin,
            stdout,
        }
    }

    fn send(&mut self, request: Value) -> Value {
        writeln!(self.stdin, "{}", serde_json::to_string(&request).unwrap()).unwrap();
        self.stdin.flush().unwrap();

        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();

        assert!(
            !line.trim().is_empty(),
            "dynamic policy component produced empty stdout response"
        );

        serde_json::from_str(&line).unwrap()
    }
}

impl Drop for DynamicPolicyProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
