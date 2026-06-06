use crate::{
    endpoint::JsonRpcEndpoint,
    error::InterceptorRuntimeError,
    interceptor::{Interceptor, InterceptorFuture},
};
use actrpc_core::{
    ACTRPC_INTERCEPTOR_INITIALIZE_METHOD, ACTRPC_INTERCEPTOR_INTERCEPT_METHOD,
    InterceptorInitialization,
    error::CodecError,
    interception::{InterceptionRequest, InterceptionResponse},
    json_rpc::{
        JsonRpcId, JsonRpcMessage, JsonRpcParams, JsonRpcRequest, JsonRpcResponse,
        JsonRpcSingleMessage, JsonRpcVersion,
    },
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

pub struct JsonRpcBackedInterceptor {
    endpoint: Arc<JsonRpcEndpoint>,
    next_id: AtomicU64,
}

impl JsonRpcBackedInterceptor {
    pub fn new(endpoint: Arc<JsonRpcEndpoint>) -> Self {
        Self {
            endpoint,
            next_id: AtomicU64::new(1),
        }
    }

    fn next_id(&self) -> JsonRpcId {
        JsonRpcId::Number(self.next_id.fetch_add(1, Ordering::Relaxed).into())
    }
}

impl Interceptor for JsonRpcBackedInterceptor {
    fn initialize<'a>(
        &'a self,
    ) -> InterceptorFuture<'a, Result<InterceptorInitialization, InterceptorRuntimeError>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            let id = self.next_id();

            let req = JsonRpcRequest {
                jsonrpc: JsonRpcVersion::V2_0,
                id: id.clone(),
                method: ACTRPC_INTERCEPTOR_INITIALIZE_METHOD.to_owned(),
                params: None,
            };

            let resp = self
                .endpoint
                .request(req)
                .await
                .map_err(InterceptorRuntimeError::Transport)?;

            let response = JsonRpcMessage::Single(JsonRpcSingleMessage::Response(resp));
            decode_success_result::<InterceptorInitialization>(id, response)
        })
    }

    fn intercept<'a>(
        &'a self,
        request: &'a InterceptionRequest,
    ) -> InterceptorFuture<'a, Result<InterceptionResponse, InterceptorRuntimeError>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            let id = self.next_id();

            let value = serde_json::to_value(request).map_err(|source| {
                InterceptorRuntimeError::Codec(CodecError::Serialize(source.to_string()))
            })?;

            let Value::Object(params) = value else {
                return Err(InterceptorRuntimeError::Codec(
                    CodecError::InvalidFieldType {
                        field: "InterceptionRequest".to_owned(),
                    },
                ));
            };

            let rpc_req = JsonRpcRequest {
                jsonrpc: JsonRpcVersion::V2_0,
                id: id.clone(),
                method: ACTRPC_INTERCEPTOR_INTERCEPT_METHOD.to_owned(),
                params: Some(JsonRpcParams::Object(params)),
            };

            let resp = self
                .endpoint
                .request(rpc_req)
                .await
                .map_err(InterceptorRuntimeError::Transport)?;

            let response = JsonRpcMessage::Single(JsonRpcSingleMessage::Response(resp));
            decode_success_result::<InterceptionResponse>(id, response)
        })
    }
}

fn decode_success_result<T>(
    expected_id: JsonRpcId,
    message: JsonRpcMessage,
) -> Result<T, InterceptorRuntimeError>
where
    T: DeserializeOwned,
{
    let JsonRpcMessage::Single(JsonRpcSingleMessage::Response(response)) = message else {
        return Err(InterceptorRuntimeError::Codec(
            CodecError::InvalidJsonRpcStructure,
        ));
    };

    match response {
        JsonRpcResponse::Success(success) => {
            if success.id != expected_id {
                return Err(InterceptorRuntimeError::Request {
                    message: "JSON-RPC response id mismatch".to_owned(),
                });
            }

            serde_json::from_value(success.result).map_err(|source| {
                InterceptorRuntimeError::Codec(CodecError::Deserialize(source.to_string()))
            })
        }

        JsonRpcResponse::Error(error) => Err(InterceptorRuntimeError::Request {
            message: format!(
                "remote JSON-RPC error {}: {}",
                error.error.code, error.error.message
            ),
        }),
    }
}
