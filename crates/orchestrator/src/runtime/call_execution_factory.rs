use crate::{
    error::OrchestratorError,
    method::{MethodName, ProviderName},
    runtime::{CallExecution, CallRuntime, OrchestratorResources},
};
use actrpc_core::json_rpc::{JsonRpcMessage, JsonRpcParams};
use std::sync::Arc;

pub struct CallExecutionFactory {
    resources: Arc<OrchestratorResources>,
}

impl CallExecutionFactory {
    pub fn new(resources: Arc<OrchestratorResources>) -> Self {
        Self { resources }
    }

    pub fn resources(&self) -> &OrchestratorResources {
        &self.resources
    }

    pub fn create_root(
        self: &Arc<Self>,
        provider: ProviderName,
        method: MethodName,
        params: Option<JsonRpcParams>,
    ) -> Result<CallExecution, OrchestratorError> {
        let message = self
            .resources
            .method_catalog
            .request_message(&provider, &method, params)?;

        let transcript = Arc::new(crate::runtime::TranscriptState::new());
        let call_id = transcript.allocate_call_id();
        let call = Arc::new(CallRuntime::root(message, transcript, call_id));

        Ok(CallExecution::new(self.clone(), call, provider, method))
    }

    pub fn create_piped(
        self: &Arc<Self>,
        provider: ProviderName,
        method: MethodName,
        params: Option<JsonRpcParams>,
        parent: &CallRuntime,
    ) -> Result<CallExecution, OrchestratorError> {
        let child_depth = parent.depth() + 1;
        if child_depth > self.resources.runtime.max_call_depth {
            return Err(OrchestratorError::MaxCallDepthExceeded {
                attempted_depth: child_depth,
                max_call_depth: self.resources.runtime.max_call_depth,
            });
        }

        let message = self
            .resources
            .method_catalog
            .request_message(&provider, &method, params)?;

        let call_id = parent.transcript.allocate_call_id();
        let call = Arc::new(CallRuntime::nested(
            message,
            parent.transcript.clone(),
            call_id,
            parent.call_id(),
            child_depth,
        ));

        Ok(CallExecution::new(self.clone(), call, provider, method))
    }

    pub async fn run_root(
        self: &Arc<Self>,
        provider: ProviderName,
        method: MethodName,
        params: Option<JsonRpcParams>,
    ) -> Result<JsonRpcMessage, OrchestratorError> {
        let execution = self.create_root(provider, method, params)?;
        execution.run().await
    }

    pub async fn run_piped(
        self: &Arc<Self>,
        provider: ProviderName,
        method: MethodName,
        params: Option<JsonRpcParams>,
        parent: &CallRuntime,
    ) -> Result<JsonRpcMessage, OrchestratorError> {
        let execution = self.create_piped(provider, method, params, parent)?;
        execution.run().await
    }
}
