use crate::interceptors::dynamic_policy::{
    error::DynamicPolicyError,
    matcher::TargetSelectorMatcher,
    scope::{DynamicScope, RelationMode},
    store::DynamicPolicyStore,
};
use actrpc_core::{
    CallId, CallRelation, CurrentExecutionContext, ExecutionContextQuery,
    ExecutionContextQueryResult, InterceptorInitialization,
    action::{
        ActionDescriptor, ActionKind, ActionSpec, RequestedAction, RequestedActionRecord,
        ResolvedActionRecord,
    },
    interception::{InterceptionRequest, InterceptionResponse, InterceptorContinuation},
    json_rpc::JsonRpcError,
};
use actrpc_orchestrator::{
    action::actions::{
        query_execution_context::QueryExecutionContext,
        reject_call::{RejectCall, RejectCallParams},
    },
    interceptor::{Interceptor, InterceptorFuture},
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

const REJECT_CODE: i32 = -32011;

#[derive(Clone)]
pub struct DynamicPolicyInterceptor {
    store: Arc<DynamicPolicyStore>,
}

impl DynamicPolicyInterceptor {
    pub fn new(store: Arc<DynamicPolicyStore>) -> Self {
        Self { store }
    }

    fn intercept_inner(
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

        let scopes = self.store.scopes_for_root(current.root_call_id);
        let matchers = self.load_matchers(&scopes);

        let mut pending_relations = Vec::<(CallId, CallId)>::new();
        let mut seen_relations = HashSet::<(CallId, CallId)>::new();

        for scope in &scopes {
            if let Some(bound) = scope.bound_call_id {
                push_relation(
                    &mut pending_relations,
                    &mut seen_relations,
                    (current.call_id, bound),
                );
                continue;
            }

            if scope.relation_mode != RelationMode::Descendant {
                continue;
            }

            let Some(matcher) = matchers.get(&scope.scope_id) else {
                continue;
            };

            if !matcher.matches(&current.target) {
                continue;
            }

            push_relation(
                &mut pending_relations,
                &mut seen_relations,
                (current.call_id, scope.owner_call_id),
            );
        }

        let missing_relations: Vec<_> = pending_relations
            .into_iter()
            .filter(|(subject, other)| !query_state.has_relation(*subject, *other))
            .collect();

        if !missing_relations.is_empty() {
            let actions = missing_relations
                .into_iter()
                .map(|(subject, other)| query_relation_action(subject, other))
                .collect::<Result<Vec<_>, _>>()?;

            return Ok(InterceptionResponse {
                actions,
                continuation: InterceptorContinuation::Reinvoke,
            });
        }

        self.try_bind_scopes(&current, &scopes, &matchers, &query_state);

        let applying = self.find_applying_scopes(&current, &scopes, &query_state);

        if applying.is_empty() {
            return Ok(InterceptionResponse {
                actions: vec![],
                continuation: InterceptorContinuation::Stop,
            });
        }

        if applying
            .iter()
            .all(|scope| DynamicPolicyStore::allows_target(scope, &request.target))
        {
            return Ok(InterceptionResponse {
                actions: vec![],
                continuation: InterceptorContinuation::Stop,
            });
        }

        Ok(InterceptionResponse {
            actions: vec![reject_call_action()?],
            continuation: InterceptorContinuation::Stop,
        })
    }

    fn load_matchers(
        &self,
        scopes: &[DynamicScope],
    ) -> HashMap<crate::interceptors::dynamic_policy::scope::ScopeId, TargetSelectorMatcher> {
        let mut matchers = HashMap::new();
        for scope in scopes {
            if let Some(matcher) = self
                .store
                .matcher_for(scope.scope_id)
                .or_else(|| TargetSelectorMatcher::compile(&scope.target_selector).ok())
            {
                matchers.insert(scope.scope_id, matcher);
            }
        }
        matchers
    }

    fn try_bind_scopes(
        &self,
        current: &CurrentExecutionContext,
        scopes: &[DynamicScope],
        matchers: &HashMap<
            crate::interceptors::dynamic_policy::scope::ScopeId,
            TargetSelectorMatcher,
        >,
        query_state: &QueryState,
    ) {
        for scope in scopes {
            if scope.bound_call_id.is_some() {
                continue;
            }

            let Some(matcher) = matchers.get(&scope.scope_id) else {
                continue;
            };

            if !matcher.matches(&current.target) {
                continue;
            }

            let should_bind = match scope.relation_mode {
                RelationMode::DirectChild => current.parent_call_id == Some(scope.owner_call_id),
                RelationMode::Descendant => query_state
                    .relation(current.call_id, scope.owner_call_id)
                    .is_some_and(|relation| {
                        matches!(relation, CallRelation::Parent | CallRelation::Ancestor)
                    }),
            };

            if should_bind {
                self.store.bind_scope(scope.scope_id, current.call_id);
            }
        }
    }

    fn find_applying_scopes<'a>(
        &self,
        current: &CurrentExecutionContext,
        scopes: &'a [DynamicScope],
        query_state: &QueryState,
    ) -> Vec<&'a DynamicScope> {
        scopes
            .iter()
            .filter_map(|scope| {
                let bound = scope.bound_call_id?;
                if !Self::scope_applies_for_enforcement(current.call_id, bound, query_state) {
                    return None;
                }
                Some(scope)
            })
            .collect()
    }

    fn scope_applies_for_enforcement(
        current_call_id: CallId,
        bound_call_id: CallId,
        query_state: &QueryState,
    ) -> bool {
        if current_call_id == bound_call_id {
            return false;
        }

        query_state
            .relation(current_call_id, bound_call_id)
            .is_some_and(|relation| {
                matches!(relation, CallRelation::Parent | CallRelation::Ancestor)
            })
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
                supports_inbound: false,
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

#[derive(Debug, Default, Clone)]
struct QueryState {
    current: Option<CurrentExecutionContext>,
    relations: HashMap<(CallId, CallId), CallRelation>,
}

impl QueryState {
    fn has_relation(&self, subject: CallId, other: CallId) -> bool {
        self.relations.contains_key(&(subject, other))
    }

    fn relation(&self, subject: CallId, other: CallId) -> Option<CallRelation> {
        self.relations.get(&(subject, other)).copied()
    }
}

fn push_relation(
    pending: &mut Vec<(CallId, CallId)>,
    seen: &mut HashSet<(CallId, CallId)>,
    relation: (CallId, CallId),
) {
    if seen.insert(relation) {
        pending.push(relation);
    }
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

            match (params.query, result) {
                (ExecutionContextQuery::Current, ExecutionContextQueryResult::Current(current)) => {
                    state.current = Some(current)
                }
                (
                    ExecutionContextQuery::Relation { subject, other },
                    ExecutionContextQueryResult::Relation(relation),
                ) => {
                    state.relations.insert((subject, other), relation);
                }
                _ => {}
            }
        }
    }

    Ok(state)
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

fn query_current_action() -> Result<RequestedActionRecord, DynamicPolicyError> {
    RequestedAction::<QueryExecutionContext> {
        params: actrpc_core::QueryExecutionContextParams {
            query: ExecutionContextQuery::Current,
        },
    }
    .try_into()
    .map_err(|source| DynamicPolicyError::ActionEncoding { source })
}

fn query_relation_action(
    subject: CallId,
    other: CallId,
) -> Result<RequestedActionRecord, DynamicPolicyError> {
    RequestedAction::<QueryExecutionContext> {
        params: actrpc_core::QueryExecutionContextParams {
            query: ExecutionContextQuery::Relation { subject, other },
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

fn action_descriptors() -> HashMap<ActionKind, ActionDescriptor> {
    actrpc_core::action::action_descriptor_map!(RejectCall, QueryExecutionContext)
}
