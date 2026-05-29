use crate::{
    error::InterceptorRuntimeError,
    interceptor::{Interceptor, InterceptorFuture},
};
use actrpc_core::{
    INTERCEPT_METHOD, InterceptorInitialization,
    error::CodecError,
    interception::{InterceptionRequest, InterceptionResponse},
    json_rpc::{
        JsonRpcId, JsonRpcMessage, JsonRpcParams, JsonRpcRequest, JsonRpcResponse,
        JsonRpcSingleMessage, JsonRpcVersion,
    },
};
use actrpc_transport::{JsonRpcClient, TransportError};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

pub struct JsonRpcBackedInterceptor {
    client: Arc<dyn JsonRpcClient<Error = TransportError>>,
    next_id: AtomicU64,
}

impl JsonRpcBackedInterceptor {
    pub fn new(client: Arc<dyn JsonRpcClient<Error = TransportError>>) -> Self {
        Self {
            client,
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

            let request = JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
                jsonrpc: JsonRpcVersion::V2_0,
                id: id.clone(),
                method: "initialize".to_owned(),
                params: None,
            }));

            let response = self
                .client
                .send(request)
                .await
                .map_err(InterceptorRuntimeError::Transport)?;

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

            let rpc_request =
                JsonRpcMessage::Single(JsonRpcSingleMessage::Request(JsonRpcRequest {
                    jsonrpc: JsonRpcVersion::V2_0,
                    id: id.clone(),
                    method: INTERCEPT_METHOD.to_owned(),
                    params: Some(JsonRpcParams::Object(params)),
                }));

            let response = self
                .client
                .send(rpc_request)
                .await
                .map_err(InterceptorRuntimeError::Transport)?;

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
