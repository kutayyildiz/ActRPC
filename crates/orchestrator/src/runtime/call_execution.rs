use crate::action::ActionInvocationContext;
use crate::error::ActionHandlerError;
use crate::{
    action::{ActionRegistry, build_builtin_action_registry},
    error::{ActionError, InterceptorError, MethodCallError, OrchestratorError},
    interceptor::{InterceptorCatalogEntry, ResolvedInterceptorRuntimeLimits},
    method::{MethodName, ProviderName},
    runtime::{CallExecutionFactory, CallRuntime, PhaseRuntime, TranscriptError, TranscriptState},
    transcript::{
        PROTOCOL_INTERCEPTOR_REQUEST, PROTOCOL_INTERCEPTOR_RESPONSE, PROTOCOL_METHOD_REQUEST,
        PROTOCOL_METHOD_RESPONSE, TranscriptParticipant, to_transcript_value,
    },
};
use actrpc_core::{
    InterceptionId,
    action::{RequestedActionRecord, ResolvedActionRecord},
    error::ProtocolError,
    interception::{InterceptionPhase, InterceptionRequest, InterceptionResponse},
    json_rpc::{
        JsonRpcErrorResponse, JsonRpcMessage, JsonRpcResponse, JsonRpcSingleMessage, JsonRpcVersion,
    },
};
use std::{sync::Arc, time::Duration};
use tokio::time::timeout;

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

    pub fn transcript(&self) -> Arc<TranscriptState> {
        self.call.transcript.clone()
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
        self.record_method_request(&outbound_message)?;

        let downstream_response = resources
            .method_catalog
            .send_message(&self.provider, &self.method, outbound_message)
            .await
            .map_err(map_method_call_error)?;

        self.record_method_response(&downstream_response)?;

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

            let limits = ResolvedInterceptorRuntimeLimits::resolve(
                &resources.runtime,
                entry.runtime_limits.as_ref(),
            );

            let interception_id = self
                .call
                .transcript
                .execution_tree()
                .allocate_interception_id();

            let mut resolved_action_history: Vec<Vec<ResolvedActionRecord>> = Vec::new();
            let mut round_index = 0usize;
            let mut total_actions = 0usize;

            loop {
                let invocation_ctx = ActionInvocationContext {
                    interceptor_name: entry.name.clone(),
                };

                let request = self.build_interception_request(
                    &resolved_action_history,
                    interception_id,
                    &invocation_ctx,
                )?;

                self.record_interceptor_request(&entry, &request)?;

                let timeout_duration =
                    Duration::from_millis(limits.interception_request_timeout_ms);
                let response = timeout(timeout_duration, entry.interceptor.intercept(&request))
                    .await
                    .map_err(|_| OrchestratorError::InterceptionRequestTimeout {
                        interceptor: entry.name.clone(),
                        phase: phase.phase,
                        timeout_ms: limits.interception_request_timeout_ms,
                        config_hint: limits.timeout_config_hint(&entry.name),
                    })?
                    .map_err(|source| {
                        OrchestratorError::Interceptor(InterceptorError::InvocationFailed {
                            name: entry.name.clone(),
                            source,
                        })
                    })?;

                self.record_interceptor_response(&entry, &response)?;

                let should_reinvoke = response.should_reinvoke();

                self.validate_policy(phase.phase, &entry, &response.actions)?;

                let next_total_actions = total_actions
                    .checked_add(response.actions.len())
                    .ok_or_else(|| OrchestratorError::MaxActionsPerInterceptionExceeded {
                        interceptor: entry.name.clone(),
                        phase: phase.phase,
                        attempted_actions: usize::MAX,
                        max_actions_per_interception: limits.max_actions_per_interception,
                        config_hint: limits.actions_config_hint(&entry.name),
                    })?;
                if next_total_actions > limits.max_actions_per_interception {
                    return Err(OrchestratorError::MaxActionsPerInterceptionExceeded {
                        interceptor: entry.name.clone(),
                        phase: phase.phase,
                        attempted_actions: next_total_actions,
                        max_actions_per_interception: limits.max_actions_per_interception,
                        config_hint: limits.actions_config_hint(&entry.name),
                    });
                }
                total_actions = next_total_actions;

                let mut round_actions = Vec::new();

                for requested_action in response.actions {
                    let action_kind = requested_action.kind.clone();

                    let handler = action_registry.get(&action_kind).ok_or_else(|| {
                        OrchestratorError::Action(ActionError::HandlerNotFound {
                            action: action_kind.clone(),
                        })
                    })?;

                    match handler
                        .handle(&request, requested_action.clone(), &invocation_ctx)
                        .await
                    {
                        Ok(resolved) => {
                            round_actions.push(resolved);
                        }
                        Err(source) => {
                            round_actions.push(failed_action_record(&requested_action, &source));
                            resolved_action_history.push(round_actions);
                            return Err(OrchestratorError::Action(ActionError::HandlerFailed {
                                interceptor: entry.name.clone(),
                                action: action_kind,
                                source,
                            }));
                        }
                    }

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

                round_index += 1;
                if round_index > limits.max_interception_reinvokes {
                    return Err(OrchestratorError::MaxInterceptionReinvokesExceeded {
                        interceptor: entry.name.clone(),
                        phase: phase.phase,
                        max_interception_reinvokes: limits.max_interception_reinvokes,
                        config_hint: limits.reinvokes_config_hint(&entry.name),
                    });
                }
            }
        }

        Ok(())
    }

    fn record_interceptor_request(
        &self,
        entry: &InterceptorCatalogEntry,
        request: &InterceptionRequest,
    ) -> Result<(), OrchestratorError> {
        let message = to_transcript_value(request).map_err(map_transcript_serialize_error)?;
        self.call.record_exchange(
            TranscriptParticipant::orchestrator_main(),
            TranscriptParticipant::interceptor(entry.name.clone()),
            PROTOCOL_INTERCEPTOR_REQUEST,
            message,
        )
    }

    fn record_interceptor_response(
        &self,
        entry: &InterceptorCatalogEntry,
        response: &InterceptionResponse,
    ) -> Result<(), OrchestratorError> {
        let message = to_transcript_value(response).map_err(map_transcript_serialize_error)?;
        self.call.record_exchange(
            TranscriptParticipant::interceptor(entry.name.clone()),
            TranscriptParticipant::orchestrator_main(),
            PROTOCOL_INTERCEPTOR_RESPONSE,
            message,
        )
    }

    fn record_method_request(&self, message: &JsonRpcMessage) -> Result<(), OrchestratorError> {
        let payload = to_transcript_value(message).map_err(map_transcript_serialize_error)?;
        self.call.record_exchange(
            TranscriptParticipant::orchestrator_main(),
            TranscriptParticipant::method_provider(self.provider.as_str()),
            PROTOCOL_METHOD_REQUEST,
            payload,
        )
    }

    fn record_method_response(&self, message: &JsonRpcMessage) -> Result<(), OrchestratorError> {
        let payload = to_transcript_value(message).map_err(map_transcript_serialize_error)?;
        self.call.record_exchange(
            TranscriptParticipant::method_provider(self.provider.as_str()),
            TranscriptParticipant::orchestrator_main(),
            PROTOCOL_METHOD_RESPONSE,
            payload,
        )
    }

    fn build_interception_request(
        &self,
        resolved_action_history: &[Vec<ResolvedActionRecord>],
        interception_id: InterceptionId,
        invocation_ctx: &ActionInvocationContext,
    ) -> Result<InterceptionRequest, OrchestratorError> {
        let ctx = self
            .call
            .call_ctx()
            .map(|call_ctx| call_ctx.filter_for_interceptor(&invocation_ctx.interceptor_name))
            .unwrap_or_default();

        Ok(InterceptionRequest {
            origin: self.call.origin().clone(),
            target: self.call.target().clone(),
            message: self.snapshot_message()?,
            call_id: self.call.call_id(),
            interception_id,
            resolved_action_history: resolved_action_history.to_vec(),
            ctx,
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

fn failed_action_record(
    requested_action: &RequestedActionRecord,
    source: &ActionHandlerError,
) -> ResolvedActionRecord {
    ResolvedActionRecord {
        kind: requested_action.kind.clone(),
        params: requested_action.params.clone(),
        result: Err(ProtocolError::InvalidMessageDirection {
            reason: source.to_string(),
        }),
    }
}

fn map_method_call_error(error: MethodCallError) -> OrchestratorError {
    OrchestratorError::MethodCall(error)
}

fn map_transcript_serialize_error(error: serde_json::Error) -> OrchestratorError {
    OrchestratorError::Transcript(TranscriptError::Serialize {
        message: error.to_string(),
    })
}
