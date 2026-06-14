use crate::{
    error::OrchestratorError,
    method::{MethodName, ProviderName, method_target_from_names},
    runtime::{CallExecution, CallRuntime, OrchestratorResources},
};
use actrpc_core::{
    json_rpc::{JsonRpcMessage, JsonRpcParams},
    participant::{Participant, ParticipantType},
};
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
        caller_id: impl Into<String>,
    ) -> Result<CallExecution, OrchestratorError> {
        let message = self
            .resources
            .method_catalog
            .request_message(&provider, &method, params)?;

        let transcript = Arc::new(crate::runtime::TranscriptState::new());
        let call_id = transcript.allocate_call_id();
        transcript
            .execution_tree()
            .register_root(call_id)
            .map_err(|message| OrchestratorError::Internal { message })?;

        let origin = Participant {
            kind: ParticipantType::External,
            id: caller_id.into(),
        };
        let target = method_target_from_names(&provider, &method);
        let call = Arc::new(CallRuntime::root(
            message, transcript, call_id, origin, target,
        ));

        Ok(CallExecution::new(self.clone(), call, provider, method))
    }

    pub fn create_piped(
        self: &Arc<Self>,
        provider: ProviderName,
        method: MethodName,
        params: Option<JsonRpcParams>,
        parent: &CallRuntime,
        child_origin: Participant,
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
        parent
            .transcript
            .execution_tree()
            .register_child(call_id, parent.call_id())
            .map_err(|message| OrchestratorError::Internal { message })?;

        let target = method_target_from_names(&provider, &method);
        let call = Arc::new(CallRuntime::nested(
            message,
            parent.transcript.clone(),
            call_id,
            parent.call_id(),
            parent.root_call_id(),
            child_depth,
            child_origin,
            target,
        ));

        Ok(CallExecution::new(self.clone(), call, provider, method))
    }

    pub async fn run_root(
        self: &Arc<Self>,
        provider: ProviderName,
        method: MethodName,
        params: Option<JsonRpcParams>,
        caller_id: impl Into<String>,
    ) -> Result<JsonRpcMessage, OrchestratorError> {
        let execution = self.create_root(provider, method, params, caller_id)?;
        execution.run().await
    }

    pub async fn run_piped(
        self: &Arc<Self>,
        provider: ProviderName,
        method: MethodName,
        params: Option<JsonRpcParams>,
        parent: &CallRuntime,
        child_origin: Participant,
    ) -> Result<JsonRpcMessage, OrchestratorError> {
        let execution = self.create_piped(provider, method, params, parent, child_origin)?;
        execution.run().await
    }
}
