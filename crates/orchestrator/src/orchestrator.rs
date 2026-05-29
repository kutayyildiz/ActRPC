use crate::method::{MethodName, ProviderName};
use actrpc_core::json_rpc::{JsonRpcMessage, JsonRpcParams};
use std::{future::Future, pin::Pin};

pub type OrchestratorFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait Orchestrator {
    type Error;

    fn call<'a>(
        &'a self,
        provider: ProviderName,
        method: MethodName,
        params: Option<JsonRpcParams>,
    ) -> OrchestratorFuture<'a, Result<JsonRpcMessage, Self::Error>>
    where
        Self: 'a;
}
