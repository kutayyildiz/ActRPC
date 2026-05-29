use actrpc_core::json_rpc::{
    JsonRpcBatch, JsonRpcId, JsonRpcMessage, JsonRpcParams, JsonRpcRequest, JsonRpcSingleMessage,
    JsonRpcVersion,
};
use serde_json::json;

#[test]
fn test_request_serde() {
    let req = JsonRpcRequest {
        jsonrpc: JsonRpcVersion::V2_0,
        id: JsonRpcId::Number(3.into()),
        method: "subtract".to_string(),
        params: Some(JsonRpcParams::Array(vec![json!(1), json!("one")])),
    };

    let ser = serde_json::to_string(&req).unwrap();
    let de: JsonRpcRequest = serde_json::from_str(&ser).unwrap();
    assert_eq!(de, req);
}

#[test]
fn test_batch_message_serde_roundtrip() {
    let msg = JsonRpcMessage::Batch(JsonRpcBatch(vec![
        JsonRpcSingleMessage::Request(JsonRpcRequest {
            jsonrpc: JsonRpcVersion::V2_0,
            id: JsonRpcId::Number(1.into()),
            method: "sum".to_string(),
            params: Some(JsonRpcParams::Array(vec![json!(1), json!(2)])),
        }),
        JsonRpcSingleMessage::Request(JsonRpcRequest {
            jsonrpc: JsonRpcVersion::V2_0,
            id: JsonRpcId::Number(2.into()),
            method: "mul".to_string(),
            params: Some(JsonRpcParams::Array(vec![json!(3), json!(4)])),
        }),
    ]));

    let ser = serde_json::to_string(&msg).unwrap();
    let de: JsonRpcMessage = serde_json::from_str(&ser).unwrap();

    assert_eq!(de, msg);
}

#[test]
fn test_empty_batch_is_rejected() {
    let err = serde_json::from_str::<JsonRpcBatch>("[]").unwrap_err();
    assert!(err.to_string().contains("must not be an empty array"));
}
