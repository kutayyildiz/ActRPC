use crate::{
    action::{ActionHandlerFuture, ActionInvocationContext, TypedActionHandler},
    error::ActionExecutionError,
    runtime::TranscriptState,
};
use actrpc_core::{
    CurrentExecutionContext, ExecutionContextQuery, ExecutionContextQueryResult,
    QueryExecutionContextParams,
    action::{ActionSpec, RequestedAction, ResolvedAction},
    interception::InterceptionRequest,
};
use std::sync::Arc;

pub struct QueryExecutionContext;

impl ActionSpec for QueryExecutionContext {
    type Params = QueryExecutionContextParams;
    type Result = ExecutionContextQueryResult;

    const KIND: &'static str = "query_execution_context";
}

pub struct QueryExecutionContextHandler {
    transcript: Arc<TranscriptState>,
}

impl QueryExecutionContextHandler {
    pub fn new(transcript: Arc<TranscriptState>) -> Self {
        Self { transcript }
    }
}

impl TypedActionHandler<QueryExecutionContext> for QueryExecutionContextHandler {
    fn handle_typed<'a>(
        &'a self,
        request: &'a InterceptionRequest,
        action: RequestedAction<QueryExecutionContext>,
        _ctx: &'a ActionInvocationContext,
    ) -> ActionHandlerFuture<'a, Result<ResolvedAction<QueryExecutionContext>, ActionExecutionError>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            let tree = self.transcript.execution_tree();
            let current_call_id = request.call_id;

            let result = match action.params.query {
                ExecutionContextQuery::Current => {
                    ExecutionContextQueryResult::Current(CurrentExecutionContext {
                        origin: request.origin.clone(),
                        target: request.target.clone(),
                        call_id: request.call_id,
                        root_call_id: tree
                            .get_node(current_call_id)
                            .map(|node| node.root_call_id)
                            .unwrap_or(current_call_id),
                        parent_call_id: tree
                            .get_node(current_call_id)
                            .and_then(|node| node.parent_call_id),
                        interception_id: request.interception_id,
                    })
                }

                ExecutionContextQuery::Relation { subject, other } => {
                    ExecutionContextQueryResult::Relation(tree.relation(subject, other))
                }

                ExecutionContextQuery::Lineage { call_id } => {
                    let resolved = call_id.unwrap_or(current_call_id);
                    let lineage =
                        tree.lineage(resolved)
                            .ok_or_else(|| ActionExecutionError::NotFound {
                                target: format!("call_id:{resolved}"),
                            })?;
                    ExecutionContextQueryResult::Lineage(lineage)
                }

                ExecutionContextQuery::Children { call_id } => {
                    let resolved = call_id.unwrap_or(current_call_id);
                    let children =
                        tree.children(resolved)
                            .ok_or_else(|| ActionExecutionError::NotFound {
                                target: format!("call_id:{resolved}"),
                            })?;
                    ExecutionContextQueryResult::Children(children)
                }

                ExecutionContextQuery::Descendants { call_id, max_depth } => {
                    let resolved = call_id.unwrap_or(current_call_id);
                    let descendants = tree.descendants(resolved, max_depth).ok_or_else(|| {
                        ActionExecutionError::NotFound {
                            target: format!("call_id:{resolved}"),
                        }
                    })?;
                    ExecutionContextQueryResult::Descendants(descendants)
                }
            };

            Ok(ResolvedAction {
                params: action.params,
                result: Ok(result),
            })
        })
    }
}
