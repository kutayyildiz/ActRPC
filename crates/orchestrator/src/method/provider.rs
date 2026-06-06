use crate::{
    endpoint::EndpointName,
    error::MethodCallError,
    method::{MethodName, MethodProviderSnapshot, ProviderName},
};
use actrpc_core::json_rpc::{JsonRpcMessage, JsonRpcParams};
use std::{future::Future, pin::Pin};

pub type MethodProviderFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait MethodProvider: Send + Sync {
    fn name(&self) -> &ProviderName;

    fn endpoint(&self) -> Option<&EndpointName> {
        None
    }

    fn is_watchable(&self) -> bool {
        false
    }

    fn snapshot(&self) -> MethodProviderSnapshot;

    fn method(&self, name: &MethodName) -> Option<crate::method::MethodInfo> {
        self.snapshot()
            .methods
            .into_iter()
            .find(|m| &m.name == name)
    }

    fn request_message(
        &self,
        method: &MethodName,
        params: Option<JsonRpcParams>,
    ) -> Result<JsonRpcMessage, MethodCallError>;

    fn send_message<'a>(
        &'a self,
        method: &'a MethodName,
        message: JsonRpcMessage,
    ) -> MethodProviderFuture<'a, Result<JsonRpcMessage, MethodCallError>>;

    fn refresh<'a>(
        &'a self,
    ) -> MethodProviderFuture<
        'a,
        Result<MethodProviderSnapshot, crate::error::MethodProviderRefreshError>,
    >;

    fn call<'a>(
        &'a self,
        method: &'a MethodName,
        params: Option<JsonRpcParams>,
    ) -> MethodProviderFuture<'a, Result<serde_json::Value, MethodCallError>> {
        Box::pin(async move {
            let request = self.request_message(method, params)?;
            let response = self.send_message(method, request).await?;
            decode_success_value(self.name().clone(), method.clone(), response)
        })
    }
}

pub fn decode_success_value(
    provider: ProviderName,
    method: MethodName,
    message: JsonRpcMessage,
) -> Result<serde_json::Value, MethodCallError> {
    use actrpc_core::json_rpc::{JsonRpcResponse, JsonRpcSingleMessage};

    let JsonRpcMessage::Single(JsonRpcSingleMessage::Response(response)) = message else {
        return Err(MethodCallError::InvalidResponse {
            provider,
            method,
            message: "provider returned a non-response JSON-RPC message".to_owned(),
        });
    };

    match response {
        JsonRpcResponse::Success(success) => Ok(success.result),

        JsonRpcResponse::Error(error) => Err(MethodCallError::RemoteError {
            provider,
            method,
            error: error.error,
        }),
    }
}
