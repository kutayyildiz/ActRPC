use crate::{
    action::{ActionHandler, RegisteredActionHandler, TypedActionHandler},
    error::{ActionError, OrchestratorError},
};
use actrpc_core::action::{ActionKind, ActionSpec};
use std::collections::HashMap;

pub struct ActionRegistry {
    handlers: HashMap<ActionKind, Box<dyn ActionHandler>>,
}

impl ActionRegistry {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register<A, H>(&mut self, handler: H) -> Result<(), OrchestratorError>
    where
        A: ActionSpec + Send + Sync + 'static,
        A::Params: Send + 'static,
        A::Result: Send + 'static,
        H: TypedActionHandler<A> + Send + Sync + 'static,
    {
        let kind = A::action_kind();

        if self.handlers.contains_key(&kind) {
            return Err(OrchestratorError::Action(
                ActionError::DuplicateRegistration { kind },
            ));
        }

        self.handlers.insert(
            kind,
            Box::new(RegisteredActionHandler::<A, H>::new(handler)),
        );

        Ok(())
    }

    pub fn get(&self, kind: &ActionKind) -> Option<&dyn ActionHandler> {
        self.handlers.get(kind).map(|handler| handler.as_ref())
    }

    pub fn contains(&self, kind: &ActionKind) -> bool {
        self.handlers.contains_key(kind)
    }

    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }
}

impl Default for ActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}
