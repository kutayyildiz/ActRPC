use crate::{
    error::{ActRpcError, CodecError, ProtocolError},
    interception::{InterceptionRequest, InterceptionResponse},
    json_rpc::{
        JsonRpcId, JsonRpcParams, JsonRpcRequest, JsonRpcResponse, JsonRpcSuccessResponse,
        JsonRpcVersion,
    },
};

pub const ACTRPC_INTERCEPTOR_INITIALIZE_METHOD: &str = "actrpc.interceptor.initialize";
pub const ACTRPC_INTERCEPTOR_INTERCEPT_METHOD: &str = "actrpc.interceptor.intercept";
pub const ACTRPC_METHOD_PROVIDER_INITIALIZE_METHOD: &str = "actrpc.method_provider.initialize";
pub const ACTRPC_METHOD_PROVIDER_REFRESH_METHOD: &str = "actrpc.method_provider.refresh";
pub const ACTRPC_METHOD_PROVIDER_CHANGED_METHOD: &str = "actrpc.method_provider.changed";

pub const INTERCEPT_METHOD: &str = ACTRPC_INTERCEPTOR_INTERCEPT_METHOD;

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
            method: INTERCEPT_METHOD.to_owned(),
            params: Some(JsonRpcParams::Object(map)),
        }
    }
}

impl TryFrom<JsonRpcResponse> for (JsonRpcId, InterceptionResponse) {
    type Error = ActRpcError;

    fn try_from(resp: JsonRpcResponse) -> Result<Self, Self::Error> {
        match resp {
            JsonRpcResponse::Success(success) => {
                let payload: InterceptionResponse = serde_json::from_value(success.result)
                    .map_err(|source| {
                        ActRpcError::Codec(CodecError::Deserialize(source.to_string()))
                    })?;

                Ok((success.id, payload))
            }

            JsonRpcResponse::Error(err) => Err(ActRpcError::RemoteJsonRpc(err.error)),
        }
    }
}

impl TryFrom<JsonRpcRequest> for (JsonRpcId, InterceptionRequest) {
    type Error = ActRpcError;

    fn try_from(req: JsonRpcRequest) -> Result<Self, Self::Error> {
        if req.method != INTERCEPT_METHOD {
            return Err(ActRpcError::Protocol(ProtocolError::UnexpectedMethod {
                expected: INTERCEPT_METHOD.to_string(),
                actual: req.method,
            }));
        }

        let params = match req.params {
            Some(JsonRpcParams::Object(map)) => map,
            Some(JsonRpcParams::Array(_)) | None => {
                return Err(ActRpcError::Protocol(ProtocolError::InvalidRequestParams));
            }
        };

        let payload = serde_json::from_value(serde_json::Value::Object(params))
            .map_err(|source| ActRpcError::Codec(CodecError::Deserialize(source.to_string())))?;

        Ok((req.id, payload))
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
