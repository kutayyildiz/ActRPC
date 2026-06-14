use actrpc_core::{CallId, CallRelation, ExecutionContextQueryResult, action::ActionSpec};
use actrpc_orchestrator::{
    action::{
        ActionRegistry,
        actions::query_execution_context::{QueryExecutionContext, QueryExecutionContextHandler},
    },
    runtime::TranscriptState,
};
use serde_json::json;
use std::sync::Arc;

use super::super::helpers::{action_record, dummy_request, invocation_context};

#[tokio::test]
async fn query_execution_context_current_echoes_request_fields() {
    let transcript = Arc::new(TranscriptState::new());
    let request = dummy_request();

    transcript
        .execution_tree()
        .register_root(request.call_id)
        .unwrap();

    let mut registry = ActionRegistry::new();
    registry
        .register::<QueryExecutionContext, _>(QueryExecutionContextHandler::new(transcript))
        .unwrap();

    let resolved = registry
        .get(&QueryExecutionContext::action_kind())
        .unwrap()
        .handle(
            &request,
            action_record::<QueryExecutionContext>(json!({
                "query": { "kind": "current" }
            })),
            &invocation_context("test"),
        )
        .await
        .unwrap();

    let result: ExecutionContextQueryResult =
        serde_json::from_value(resolved.result.unwrap().unwrap()).unwrap();

    match result {
        ExecutionContextQueryResult::Current(current) => {
            assert_eq!(current.origin, request.origin);
            assert_eq!(current.target, request.target);
            assert_eq!(current.call_id, request.call_id);
            assert_eq!(current.interception_id, request.interception_id);
            assert_eq!(current.root_call_id, request.call_id);
            assert_eq!(current.parent_call_id, None);
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

#[tokio::test]
async fn query_execution_context_relation_uses_subject_other_semantics() {
    let transcript = Arc::new(TranscriptState::new());
    let root = CallId::new();
    let child = CallId::new();

    transcript.execution_tree().register_root(root).unwrap();
    transcript
        .execution_tree()
        .register_child(child, root)
        .unwrap();

    let mut registry = ActionRegistry::new();
    registry
        .register::<QueryExecutionContext, _>(QueryExecutionContextHandler::new(transcript))
        .unwrap();

    let request = dummy_request();
    let resolved = registry
        .get(&QueryExecutionContext::action_kind())
        .unwrap()
        .handle(
            &request,
            action_record::<QueryExecutionContext>(json!({
                "query": {
                    "kind": "relation",
                    "subject": child,
                    "other": root
                }
            })),
            &invocation_context("test"),
        )
        .await
        .unwrap();

    let result: ExecutionContextQueryResult =
        serde_json::from_value(resolved.result.unwrap().unwrap()).unwrap();

    assert_eq!(
        result,
        ExecutionContextQueryResult::Relation(CallRelation::Parent)
    );
}
