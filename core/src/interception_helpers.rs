use crate::{
    interception::{InterceptionDecision, InterceptionRequest},
    json_rpc::{JsonRpcRequest, JsonRpcResponse},
};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Generates a collision-free JSON-RPC id (u64 → Number).
/// Thread-safe, deterministic within the process, perfect for tests + replay.
pub fn new_jsonrpc_id() -> Value {
    Value::Number(NEXT_ID.fetch_add(1, Ordering::Relaxed).into())
}

/// Orchestrator → Interceptor  
pub fn create_interception_request(
    interception: InterceptionRequest,
    id: Option<Value>,
) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id,
        method: "intercept".to_string(),
        params: serde_json::to_value(interception)
            .expect("Failed to serialize InterceptionRequest (bug in core)"),
    }
}

/// Interceptor → Orchestrator  
pub fn create_interception_response(
    decision: InterceptionDecision,
    request_id: Option<Value>,
) -> JsonRpcResponse {
    JsonRpcResponse::Success {
        jsonrpc: "2.0".to_string(),
        id: request_id,
        result: serde_json::to_value(decision)
            .expect("Failed to serialize InterceptionDecision (bug in core)"),
    }
}

impl InterceptionRequest {
    pub fn into_rpc_request(self, id: Option<Value>) -> JsonRpcRequest {
        create_interception_request(self, id)
    }
}

impl InterceptionDecision {
    /// Same as `create_inspect_response` but on the struct itself.
    pub fn into_rpc_response(self, request_id: Option<Value>) -> JsonRpcResponse {
        create_interception_response(self, request_id)
    }
}
