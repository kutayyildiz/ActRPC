use crate::{
    interception::{InterceptionRequest, InterceptionResponse},
    json_rpc::{
        JsonRpcId, JsonRpcParams, JsonRpcRequest, JsonRpcResponse, JsonRpcSuccessResponse,
        JsonRpcVersion,
    },
};

impl From<(JsonRpcId, InterceptionRequest)> for JsonRpcRequest {
    fn from((id, req): (JsonRpcId, InterceptionRequest)) -> Self {
        let value =
            serde_json::to_value(req).expect("InterceptionRequest must always serialize to JSON");

        let serde_json::Value::Object(map) = value else {
            unreachable!("InterceptionRequest must always serialize to a JSON object");
        };

        JsonRpcRequest {
            jsonrpc: JsonRpcVersion::V2_0,
            id,
            method: "intercept".to_string(),
            params: Some(JsonRpcParams::Object(map)),
        }
    }
}

impl From<(JsonRpcId, InterceptionResponse)> for JsonRpcResponse {
    fn from((id, resp): (JsonRpcId, InterceptionResponse)) -> Self {
        let result =
            serde_json::to_value(resp).expect("InterceptionResponse must always serialize to JSON");

        JsonRpcResponse::Success(JsonRpcSuccessResponse {
            jsonrpc: JsonRpcVersion::V2_0,
            id,
            result,
        })
    }
}
