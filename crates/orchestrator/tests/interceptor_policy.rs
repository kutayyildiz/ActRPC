use actrpc_core::{
    action::{ActionKind, RequestedActionRecord},
    interception::InterceptionPhase,
};
use actrpc_orchestrator::interceptor::InterceptorPolicy;
use serde_json::json;
use std::collections::HashSet;

#[test]
fn policy_allows_actions_for_matching_phase() {
    let policy = InterceptorPolicy {
        outbound: HashSet::from([ActionKind::from("modify_params")]),
        inbound: HashSet::from([ActionKind::from("modify_result")]),
    };

    let outbound_actions = vec![RequestedActionRecord {
        kind: ActionKind::from("modify_params"),
        params: Some(json!({ "params": null })),
    }];

    let inbound_actions = vec![RequestedActionRecord {
        kind: ActionKind::from("modify_result"),
        params: Some(json!({ "result": 42 })),
    }];

    assert!(policy.allows_all(InterceptionPhase::Outbound, &outbound_actions));
    assert!(policy.allows_all(InterceptionPhase::Inbound, &inbound_actions));
}

#[test]
fn policy_rejects_actions_for_wrong_phase() {
    let policy = InterceptorPolicy {
        outbound: HashSet::from([ActionKind::from("modify_params")]),
        inbound: HashSet::from([ActionKind::from("modify_result")]),
    };

    let actions = vec![RequestedActionRecord {
        kind: ActionKind::from("modify_result"),
        params: Some(json!({ "result": 42 })),
    }];

    assert!(!policy.allows_all(InterceptionPhase::Outbound, &actions));

    let conflicts = policy.conflicting_actions(InterceptionPhase::Outbound, &actions);

    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].kind, ActionKind::from("modify_result"));
}

#[test]
fn policy_returns_all_conflicting_actions() {
    let policy = InterceptorPolicy {
        outbound: HashSet::from([ActionKind::from("allowed")]),
        inbound: HashSet::new(),
    };

    let actions = vec![
        RequestedActionRecord {
            kind: ActionKind::from("allowed"),
            params: None,
        },
        RequestedActionRecord {
            kind: ActionKind::from("denied_a"),
            params: None,
        },
        RequestedActionRecord {
            kind: ActionKind::from("denied_b"),
            params: None,
        },
    ];

    let conflicts = policy.conflicting_actions(InterceptionPhase::Outbound, &actions);

    assert_eq!(conflicts.len(), 2);
    assert_eq!(conflicts[0].kind, ActionKind::from("denied_a"));
    assert_eq!(conflicts[1].kind, ActionKind::from("denied_b"));
}
