use crate::interceptors::dynamic_policy::{
    config::{DynamicPolicyConfig, UnscopedBehavior},
    context::{DynamicPolicyContext, DynamicPolicyContextMode},
    error::DynamicPolicyError,
    scope::{DynamicScope, ScopeId},
    store::DynamicPolicyStore,
};
use actrpc_core::{
    CallId, CurrentExecutionContext, ExecutionContextQuery, ExecutionContextQueryResult,
    InterceptorInitialization, MethodTarget,
    action::{
        ActionDescriptor, ActionKind, ActionSpec, RequestedAction, RequestedActionRecord,
        ResolvedActionRecord,
    },
    interception::{InterceptionPhase, InterceptionRequest, InterceptionResponse, InterceptorContinuation},
    json_rpc::JsonRpcError,
};
use actrpc_orchestrator::{
    action::actions::{
        query_execution_context::QueryExecutionContext,
        reject_call::{RejectCall, RejectCallParams},
        request_review::{
            REVIEW_DECISION_APPROVED, REVIEW_DECISION_DENIED, RequestReview, RequestReviewParams,
            RequestReviewResult,
        },
    },
    interceptor::{Interceptor, InterceptorFuture},
};
use std::{
    collections::HashMap,
    sync::Arc,
};

const REJECT_CODE: i32 = -32011;

#[derive(Clone)]
pub struct DynamicPolicyInterceptor {
    store: Arc<DynamicPolicyStore>,
    config: DynamicPolicyConfig,
}

impl DynamicPolicyInterceptor {
    pub fn new(store: Arc<DynamicPolicyStore>, config: DynamicPolicyConfig) -> Self {
        Self { store, config }
    }

    fn intercept_inner(
        &self,
        request: &InterceptionRequest,
    ) -> Result<InterceptionResponse, DynamicPolicyError> {
        let phase = request
            .phase()
            .map_err(|error| DynamicPolicyError::InvalidQueryResult {
                message: error.to_string(),
            })?;

        match phase {
            InterceptionPhase::Outbound => self.intercept_outbound(request),
            InterceptionPhase::Inbound => self.intercept_inbound(request),
        }
    }

    fn intercept_outbound(
        &self,
        request: &InterceptionRequest,
    ) -> Result<InterceptionResponse, DynamicPolicyError> {
        let query_state = collect_query_state(request)?;

        if query_state.current.is_none() {
            return Ok(InterceptionResponse {
                actions: vec![query_current_action()?],
                continuation: InterceptorContinuation::Reinvoke,
            });
        }

        let current = query_state.current.clone().unwrap();
        let parent_scope_id = current
            .parent_call_id
            .and_then(|parent| self.store.scope_for_call(parent));

        let private_ctx = request.ctx.private.as_ref();
        let decoded_ctx = private_ctx.map(decode_dynamic_policy_context);

        match decoded_ctx {
            None => self.handle_no_ctx(request, &current, parent_scope_id),
            Some(Ok(ctx)) => {
                if ctx.mode != DynamicPolicyContextMode::Detached {
                    return self.handle_malformed_ctx(&current, parent_scope_id);
                }
                self.handle_valid_detached_ctx(request, &current, parent_scope_id, &ctx)
            }
            Some(Err(_)) => self.handle_malformed_ctx(&current, parent_scope_id),
        }
    }

    fn intercept_inbound(
        &self,
        request: &InterceptionRequest,
    ) -> Result<InterceptionResponse, DynamicPolicyError> {
        let query_state = collect_query_state(request)?;

        if query_state.current.is_none() {
            return Ok(InterceptionResponse {
                actions: vec![query_current_action()?],
                continuation: InterceptorContinuation::Reinvoke,
            });
        }

        let current = query_state.current.unwrap();

        self.store.release_call(current.call_id);

        if let Some(scope_id) = self.store.scope_created_by_call(current.call_id) {
            self.store.release_scope(scope_id);
        }

        if current.parent_call_id.is_none() {
            self.store.release_scopes_for_root(current.root_call_id);
        }

        Ok(InterceptionResponse {
            actions: vec![],
            continuation: InterceptorContinuation::Stop,
        })
    }

    fn handle_no_ctx(
        &self,
        _request: &InterceptionRequest,
        current: &CurrentExecutionContext,
        parent_scope_id: Option<ScopeId>,
    ) -> Result<InterceptionResponse, DynamicPolicyError> {
        if let Some(parent_scope_id) = parent_scope_id {
            return self.enforce_parent_scope(current, parent_scope_id);
        }

        match self.config.unscoped_policy.on_unscoped {
            UnscopedBehavior::Allow => Ok(allow_response()),
            UnscopedBehavior::Reject => Ok(reject_response()?),
            UnscopedBehavior::ScopeRoot => {
                let scope_id = self.store.create_scope_for_call(
                    current.call_id,
                    current.root_call_id,
                    self.config.unscoped_policy.allowed_method_targets.clone(),
                )?;
                self.store.bind_call(current.call_id, scope_id);
                Ok(allow_response())
            }
        }
    }

    fn handle_valid_detached_ctx(
        &self,
        request: &InterceptionRequest,
        current: &CurrentExecutionContext,
        parent_scope_id: Option<ScopeId>,
        ctx: &DynamicPolicyContext,
    ) -> Result<InterceptionResponse, DynamicPolicyError> {
        if ctx.allowed_method_targets.is_empty() {
            return self.handle_malformed_ctx(current, parent_scope_id);
        }

        let review_key = review_key_for_call(current.call_id);
        let review_results = collect_request_review_results(request)?;

        if let Some(parent_scope_id) = parent_scope_id {
            let parent_scope = self
                .store
                .get_scope(parent_scope_id)
                .ok_or(DynamicPolicyError::ScopeNotFound {
                    scope_id: parent_scope_id,
                })?;

            if !DynamicPolicyStore::allows_target(&parent_scope, &current.target) {
                return Ok(reject_response()?);
            }

            let subset = DynamicPolicyStore::is_subset(
                &ctx.allowed_method_targets,
                &parent_scope.allowed_method_targets,
            );

            if subset {
                let scope_id = self.store.create_scope_for_call(
                    current.call_id,
                    current.root_call_id,
                    ctx.allowed_method_targets.clone(),
                )?;
                self.store.bind_call(current.call_id, scope_id);
                return Ok(allow_response());
            }

            if let Some(review_result) = review_results.get(&review_key) {
                return self.finalize_review(
                    review_result,
                    current,
                    &ctx.allowed_method_targets,
                );
            }

            return Ok(InterceptionResponse {
                actions: vec![request_review_action(
                    &review_key,
                    current,
                    &ctx.allowed_method_targets,
                    &parent_scope,
                )?],
                continuation: InterceptorContinuation::Reinvoke,
            });
        }

        let scope_id = self.store.create_scope_for_call(
            current.call_id,
            current.root_call_id,
            ctx.allowed_method_targets.clone(),
        )?;
        self.store.bind_call(current.call_id, scope_id);
        Ok(allow_response())
    }

    fn handle_malformed_ctx(
        &self,
        current: &CurrentExecutionContext,
        parent_scope_id: Option<ScopeId>,
    ) -> Result<InterceptionResponse, DynamicPolicyError> {
        if let Some(parent_scope_id) = parent_scope_id {
            return self.enforce_parent_scope(current, parent_scope_id);
        }

        Ok(reject_response()?)
    }

    fn enforce_parent_scope(
        &self,
        current: &CurrentExecutionContext,
        parent_scope_id: ScopeId,
    ) -> Result<InterceptionResponse, DynamicPolicyError> {
        let parent_scope = self
            .store
            .get_scope(parent_scope_id)
            .ok_or(DynamicPolicyError::ScopeNotFound {
                scope_id: parent_scope_id,
            })?;

        if DynamicPolicyStore::allows_target(&parent_scope, &current.target) {
            self.store.bind_call(current.call_id, parent_scope_id);
            Ok(allow_response())
        } else {
            Ok(reject_response()?)
        }
    }

    fn finalize_review(
        &self,
        review_result: &RequestReviewResult,
        current: &CurrentExecutionContext,
        allowed_method_targets: &[MethodTarget],
    ) -> Result<InterceptionResponse, DynamicPolicyError> {
        if review_result.decision == REVIEW_DECISION_APPROVED {
            let scope_id = if let Some(existing) = self.store.scope_created_by_call(current.call_id)
            {
                existing
            } else {
                self.store.create_scope_for_call(
                    current.call_id,
                    current.root_call_id,
                    allowed_method_targets.to_vec(),
                )?
            };
            self.store.bind_call(current.call_id, scope_id);
            Ok(allow_response())
        } else if review_result.decision == REVIEW_DECISION_DENIED {
            Ok(reject_response()?)
        } else {
            Err(DynamicPolicyError::InvalidReviewResult {
                message: format!(
                    "unknown review decision for {}: {}",
                    review_key_for_call(current.call_id),
                    review_result.decision
                ),
            })
        }
    }
}

impl Interceptor for DynamicPolicyInterceptor {
    fn initialize<'a>(
        &'a self,
    ) -> InterceptorFuture<
        'a,
        Result<InterceptorInitialization, actrpc_orchestrator::error::InterceptorRuntimeError>,
    >
    where
        Self: 'a,
    {
        Box::pin(async move {
            Ok(InterceptorInitialization {
                supports_outbound: true,
                supports_inbound: true,
                actions: action_descriptors(),
            })
        })
    }

    fn intercept<'a>(
        &'a self,
        request: &'a InterceptionRequest,
    ) -> InterceptorFuture<
        'a,
        Result<InterceptionResponse, actrpc_orchestrator::error::InterceptorRuntimeError>,
    >
    where
        Self: 'a,
    {
        Box::pin(async move {
            self.intercept_inner(request)
                .map_err(actrpc_orchestrator::error::InterceptorRuntimeError::from)
        })
    }
}

fn allow_response() -> InterceptionResponse {
    InterceptionResponse {
        actions: vec![],
        continuation: InterceptorContinuation::Stop,
    }
}

fn reject_response() -> Result<InterceptionResponse, DynamicPolicyError> {
    Ok(InterceptionResponse {
        actions: vec![reject_call_action()?],
        continuation: InterceptorContinuation::Stop,
    })
}

fn review_key_for_call(call_id: CallId) -> String {
    format!("dynamic_policy:detached:{call_id}")
}

fn decode_dynamic_policy_context(value: &serde_json::Value) -> Result<DynamicPolicyContext, DynamicPolicyError> {
    serde_json::from_value(value.clone()).map_err(|source| DynamicPolicyError::InvalidContext {
        message: source.to_string(),
    })
}

#[derive(Debug, Default, Clone)]
struct QueryState {
    current: Option<CurrentExecutionContext>,
}

fn collect_query_state(request: &InterceptionRequest) -> Result<QueryState, DynamicPolicyError> {
    let mut state = QueryState::default();

    for actions in request.iter_resolved_action_rounds() {
        for action in actions {
            if action.kind != QueryExecutionContext::action_kind() {
                continue;
            }

            let params = decode_query_params(action)?;
            let result = decode_query_result(action)?;

            if let (
                ExecutionContextQuery::Current,
                ExecutionContextQueryResult::Current(current),
            ) = (params.query, result)
            {
                state.current = Some(current);
            }
        }
    }

    Ok(state)
}

fn collect_request_review_results(
    request: &InterceptionRequest,
) -> Result<HashMap<String, RequestReviewResult>, DynamicPolicyError> {
    let mut results = HashMap::new();

    for actions in request.iter_resolved_action_rounds() {
        for action in actions {
            if action.kind != RequestReview::action_kind() {
                continue;
            }

            let params = decode_review_params(action)?;
            let result = decode_review_result(action)?;
            results.insert(params.rule_name, result);
        }
    }

    Ok(results)
}

fn decode_query_params(
    action: &ResolvedActionRecord,
) -> Result<actrpc_core::QueryExecutionContextParams, DynamicPolicyError> {
    let Some(value) = &action.params else {
        return Err(DynamicPolicyError::InvalidQueryResult {
            message: "query_execution_context action is missing params".to_owned(),
        });
    };

    serde_json::from_value(value.clone()).map_err(|source| DynamicPolicyError::InvalidQueryResult {
        message: format!("failed to decode query params: {source}"),
    })
}

fn decode_query_result(
    action: &ResolvedActionRecord,
) -> Result<ExecutionContextQueryResult, DynamicPolicyError> {
    let Ok(Some(value)) = &action.result else {
        return Err(DynamicPolicyError::InvalidQueryResult {
            message: "query_execution_context action did not resolve successfully".to_owned(),
        });
    };

    serde_json::from_value(value.clone()).map_err(|source| DynamicPolicyError::InvalidQueryResult {
        message: format!("failed to decode query result: {source}"),
    })
}

fn decode_review_params(action: &ResolvedActionRecord) -> Result<RequestReviewParams, DynamicPolicyError> {
    let Some(value) = &action.params else {
        return Err(DynamicPolicyError::InvalidReviewResult {
            message: "request_review action is missing params".to_owned(),
        });
    };

    serde_json::from_value(value.clone()).map_err(|source| DynamicPolicyError::InvalidReviewResult {
        message: format!("failed to decode request_review params: {source}"),
    })
}

fn decode_review_result(action: &ResolvedActionRecord) -> Result<RequestReviewResult, DynamicPolicyError> {
    let Ok(Some(value)) = &action.result else {
        return Err(DynamicPolicyError::InvalidReviewResult {
            message: "request_review action did not resolve successfully".to_owned(),
        });
    };

    serde_json::from_value(value.clone()).map_err(|source| DynamicPolicyError::InvalidReviewResult {
        message: format!("failed to decode request_review result: {source}"),
    })
}

fn query_current_action() -> Result<RequestedActionRecord, DynamicPolicyError> {
    RequestedAction::<QueryExecutionContext> {
        params: actrpc_core::QueryExecutionContextParams {
            query: ExecutionContextQuery::Current,
        },
    }
    .try_into()
    .map_err(|source| DynamicPolicyError::ActionEncoding { source })
}

fn reject_call_action() -> Result<RequestedActionRecord, DynamicPolicyError> {
    RequestedAction::<RejectCall> {
        params: RejectCallParams {
            error: JsonRpcError {
                code: REJECT_CODE,
                message: "method not allowed by dynamic scope".to_owned(),
                data: None,
            },
        },
    }
    .try_into()
    .map_err(|source| DynamicPolicyError::ActionEncoding { source })
}

fn request_review_action(
    rule_name: &str,
    current: &CurrentExecutionContext,
    requested: &[MethodTarget],
    parent_scope: &DynamicScope,
) -> Result<RequestedActionRecord, DynamicPolicyError> {
    let reason = format!(
        "dynamic policy detached scope review for call {}: current target={}/{}, requested allowlist={:?}, parent allowlist={:?}",
        current.call_id,
        current.target.provider,
        current.target.method,
        requested,
        parent_scope.allowed_method_targets
    );

    RequestedAction::<RequestReview> {
        params: RequestReviewParams {
            rule_name: rule_name.to_owned(),
            title: "Dynamic policy detached scope expansion".to_owned(),
            reason,
            severity: "medium".to_owned(),
        },
    }
    .try_into()
    .map_err(|source| DynamicPolicyError::ActionEncoding { source })
}

fn action_descriptors() -> HashMap<ActionKind, ActionDescriptor> {
    actrpc_core::action::action_descriptor_map!(RejectCall, QueryExecutionContext, RequestReview)
}