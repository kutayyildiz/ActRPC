use crate::{
    action::{ActionRegistry, build_builtin_action_registry},
    error::{ActionError, InterceptorError, MethodCallError, OrchestratorError},
    interceptor::InterceptorCatalogEntry,
    method::{MethodName, ProviderName},
    runtime::{CallExecutionFactory, CallRuntime, PhaseRuntime},
};
use actrpc_core::{
    action::{RequestedActionRecord, ResolvedActionRecord},
    interception::{InterceptionPhase, InterceptionRequest},
    json_rpc::{
        JsonRpcErrorResponse, JsonRpcMessage, JsonRpcResponse, JsonRpcSingleMessage, JsonRpcVersion,
    },
    participant::{Participant, ParticipantType},
};
use std::sync::Arc;

pub struct CallExecution {
    factory: Arc<CallExecutionFactory>,
    call: Arc<CallRuntime>,
    provider: ProviderName,
    method: MethodName,
}

impl CallExecution {
    pub fn new(
        factory: Arc<CallExecutionFactory>,
        call: Arc<CallRuntime>,
        provider: ProviderName,
        method: MethodName,
    ) -> Self {
        Self {
            factory,
            call,
            provider,
            method,
        }
    }

    pub async fn run(&self) -> Result<JsonRpcMessage, OrchestratorError> {
        let resources = self.factory.resources();

        let outbound = PhaseRuntime::new(
            InterceptionPhase::Outbound,
            self.call.clone(),
            resources.interceptor_catalog.outbound_pipeline_snapshot(),
        );

        let outbound_actions =
            build_builtin_action_registry(self.factory.clone(), resources, &outbound)?;

        self.run_interceptor_phase(&outbound, &outbound_actions)
            .await?;

        if self.call.rejection.is_rejected() {
            return self.rejection_response();
        }

        let outbound_message = self.snapshot_message()?;

        let downstream_response = resources
            .method_catalog
            .send_message(&self.provider, &self.method, outbound_message)
            .await
            .map_err(map_method_call_error)?;

        if !self
            .call
            .in_flight_message
            .replace_message(downstream_response)
        {
            return Err(OrchestratorError::Internal {
                message: "failed to replace in-flight message after downstream call".to_owned(),
            });
        }

        let inbound = PhaseRuntime::new(
            InterceptionPhase::Inbound,
            self.call.clone(),
            resources.interceptor_catalog.inbound_pipeline_snapshot(),
        );

        let inbound_actions =
            build_builtin_action_registry(self.factory.clone(), resources, &inbound)?;

        self.run_interceptor_phase(&inbound, &inbound_actions)
            .await?;

        if self.call.rejection.is_rejected() {
            return self.rejection_response();
        }

        self.snapshot_message()
    }

    async fn run_interceptor_phase(
        &self,
        phase: &PhaseRuntime,
        action_registry: &ActionRegistry,
    ) -> Result<(), OrchestratorError> {
        let resources = self.factory.resources();

        for interceptor_name in phase.pipeline.snapshot() {
            if !phase.pipeline.contains(&interceptor_name) {
                continue;
            }

            let entry = resources
                .interceptor_catalog
                .get_entry(&interceptor_name)
                .map_err(|source| OrchestratorError::Internal {
                    message: source.to_string(),
                })?;

            let mut resolved_action_history: Vec<Vec<ResolvedActionRecord>> = Vec::new();

            loop {
                let request = self.build_interception_request(&resolved_action_history)?;

                let response = entry
                    .interceptor
                    .intercept(&request)
                    .await
                    .map_err(|source| {
                        OrchestratorError::Interceptor(InterceptorError::InvocationFailed {
                            name: entry.name.clone(),
                            source,
                        })
                    })?;

                let should_reinvoke = response.should_reinvoke();

                self.validate_policy(phase.phase, &entry, &response.actions)?;

                let mut round_actions = Vec::new();

                for requested_action in response.actions {
                    let action_kind = requested_action.kind.clone();

                    let handler = action_registry.get(&action_kind).ok_or_else(|| {
                        OrchestratorError::Action(ActionError::HandlerNotFound {
                            action: action_kind.clone(),
                        })
                    })?;

                    let resolved =
                        handler
                            .handle(&request, requested_action)
                            .await
                            .map_err(|source| {
                                OrchestratorError::Action(ActionError::HandlerFailed {
                                    interceptor: entry.name.clone(),
                                    action: action_kind,
                                    source,
                                })
                            })?;

                    round_actions.push(resolved);

                    if self.call.rejection.is_rejected() {
                        return Ok(());
                    }
                }

                if !round_actions.is_empty() {
                    resolved_action_history.push(round_actions);
                }

                if !should_reinvoke {
                    break;
                }
            }
        }

        Ok(())
    }

    fn build_interception_request(
        &self,
        resolved_action_history: &[Vec<ResolvedActionRecord>],
    ) -> Result<InterceptionRequest, OrchestratorError> {
        Ok(InterceptionRequest {
            origin: Participant {
                kind: ParticipantType::Orchestrator,
                id: "orchestrator".to_owned(),
            },
            message: self.snapshot_message()?,
            resolved_action_history: resolved_action_history.to_vec(),
        })
    }

    fn validate_policy(
        &self,
        phase: InterceptionPhase,
        entry: &InterceptorCatalogEntry,
        actions: &[RequestedActionRecord],
    ) -> Result<(), OrchestratorError> {
        let conflicts = entry.policy.conflicting_actions(phase, actions);

        let Some(conflict) = conflicts.first() else {
            return Ok(());
        };

        Err(OrchestratorError::Action(ActionError::ForbiddenByPolicy {
            interceptor: entry.name.clone(),
            action: conflict.kind.clone(),
            phase,
        }))
    }

    fn snapshot_message(&self) -> Result<JsonRpcMessage, OrchestratorError> {
        self.call
            .in_flight_message
            .snapshot()
            .ok_or_else(|| OrchestratorError::Internal {
                message: "no in-flight message is currently set".to_owned(),
            })
    }

    fn rejection_response(&self) -> Result<JsonRpcMessage, OrchestratorError> {
        let error = self
            .call
            .rejection
            .snapshot()
            .ok_or_else(|| OrchestratorError::Internal {
                message: "call rejection was set without an error".to_owned(),
            })?;

        let id = match self.snapshot_message()? {
            JsonRpcMessage::Single(JsonRpcSingleMessage::Request(request)) => request.id,

            JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Success(
                success,
            ))) => success.id,

            JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Error(
                error_response,
            ))) => error_response.id,

            JsonRpcMessage::Single(JsonRpcSingleMessage::Notification(_)) => {
                return Err(OrchestratorError::Internal {
                    message: "reject_call cannot produce a JSON-RPC response for a notification"
                        .to_owned(),
                });
            }

            JsonRpcMessage::Batch(_) => {
                return Err(OrchestratorError::Internal {
                    message: "reject_call does not support batched JSON-RPC messages yet"
                        .to_owned(),
                });
            }
        };

        Ok(JsonRpcMessage::Single(JsonRpcSingleMessage::Response(
            JsonRpcResponse::Error(JsonRpcErrorResponse {
                jsonrpc: JsonRpcVersion::V2_0,
                id,
                error,
            }),
        )))
    }
}

fn map_method_call_error(error: MethodCallError) -> OrchestratorError {
    OrchestratorError::MethodCall(error)
}
