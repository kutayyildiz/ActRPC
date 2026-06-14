use actrpc_core::{
    action::{ActionKind, ActionSpec, RequestedAction, ResolvedAction},
    interception::InterceptionRequest,
};
use actrpc_orchestrator::{
    action::{ActionHandlerFuture, ActionInvocationContext, ActionRegistry, TypedActionHandler},
    error::{ActionError, ActionExecutionError, OrchestratorError},
};

struct EchoAction;

impl ActionSpec for EchoAction {
    type Params = String;
    type Result = String;

    const KIND: &'static str = "echo";
}

struct EchoHandler;

impl TypedActionHandler<EchoAction> for EchoHandler {
    fn handle_typed<'a>(
        &'a self,
        _request: &'a InterceptionRequest,
        action: RequestedAction<EchoAction>,
        _ctx: &'a ActionInvocationContext,
    ) -> ActionHandlerFuture<'a, Result<ResolvedAction<EchoAction>, ActionExecutionError>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            Ok(ResolvedAction {
                params: action.params.clone(),
                result: Ok(action.params),
            })
        })
    }
}

#[test]
fn action_registry_starts_empty() {
    let registry = ActionRegistry::new();

    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);
    assert!(!registry.contains(&ActionKind::from("echo")));
}

#[test]
fn action_registry_registers_handler() {
    let mut registry = ActionRegistry::new();

    registry.register::<EchoAction, _>(EchoHandler).unwrap();

    let kind = EchoAction::action_kind();

    assert!(!registry.is_empty());
    assert_eq!(registry.len(), 1);
    assert!(registry.contains(&kind));

    let handler = registry.get(&kind).unwrap();
    assert_eq!(handler.kind(), kind);
}

#[test]
fn action_registry_returns_none_for_missing_handler() {
    let registry = ActionRegistry::new();

    assert!(registry.get(&ActionKind::from("missing")).is_none());
}

#[test]
fn action_registry_rejects_duplicate_registration() {
    let mut registry = ActionRegistry::new();

    registry.register::<EchoAction, _>(EchoHandler).unwrap();

    let err = registry.register::<EchoAction, _>(EchoHandler).unwrap_err();

    match err {
        OrchestratorError::Action(ActionError::DuplicateRegistration { kind }) => {
            assert_eq!(kind, EchoAction::action_kind());
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
