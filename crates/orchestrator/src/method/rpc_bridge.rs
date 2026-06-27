use crate::error::MethodCallError;
use crate::method::{MethodName, ProviderName};
use actrpc_core::json_rpc::{
    JsonRpcError, JsonRpcErrorResponse, JsonRpcId, JsonRpcMessage, JsonRpcResponse,
    JsonRpcSingleMessage, JsonRpcSuccessResponse, JsonRpcVersion,
};

pub fn request_internal_id(id: &JsonRpcId) -> Option<JsonRpcId> {
    match id {
        JsonRpcId::Null => None,
        other => Some(other.clone()),
    }
}

pub fn remap_json_rpc_response(
    internal_id: JsonRpcId,
    external_id: JsonRpcId,
    response: JsonRpcResponse,
) -> Result<JsonRpcResponse, String> {
    match response {
        JsonRpcResponse::Success(success) => {
            if success.id != external_id {
                return Err("JSON-RPC response id mismatch".to_owned());
            }
            Ok(JsonRpcResponse::Success(JsonRpcSuccessResponse {
                jsonrpc: success.jsonrpc,
                id: internal_id,
                result: success.result,
            }))
        }
        JsonRpcResponse::Error(error) => {
            if error.id != external_id {
                return Err("JSON-RPC response id mismatch".to_owned());
            }
            Ok(JsonRpcResponse::Error(JsonRpcErrorResponse {
                jsonrpc: error.jsonrpc,
                id: internal_id,
                error: error.error,
            }))
        }
    }
}

pub fn logical_error_response(
    internal_id: JsonRpcId,
    message: impl Into<String>,
) -> JsonRpcMessage {
    JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Error(
        JsonRpcErrorResponse {
            jsonrpc: JsonRpcVersion::V2_0,
            id: internal_id,
            error: JsonRpcError {
                code: -32603,
                message: message.into(),
                data: None,
            },
        },
    )))
}

pub fn invalid_request_message_response(
    provider: &ProviderName,
    method: &MethodName,
    message: JsonRpcMessage,
) -> Result<JsonRpcMessage, MethodCallError> {
    if let JsonRpcMessage::Single(JsonRpcSingleMessage::Request(request)) = message {
        if let Some(internal_id) = request_internal_id(&request.id) {
            return Ok(logical_error_response(
                internal_id,
                "invalid provider input message",
            ));
        }
    }

    Err(MethodCallError::InvalidResponse {
        provider: provider.clone(),
        method: method.clone(),
        message: "provider send_message expected a single JSON-RPC request with id".to_owned(),
    })
}
