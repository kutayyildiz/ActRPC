use actrpc_core::json_rpc::JsonRpcRequest;
use serde_json::json;

#[test]
fn test_request_serde() {
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(42.into()),
        method: "subtract".to_string(),
        params: json!([23, 7]),
    };

    let ser = serde_json::to_string(&req).unwrap();
    let de: JsonRpcRequest = serde_json::from_str(&ser).unwrap();
    assert_eq!(de, req);
}
