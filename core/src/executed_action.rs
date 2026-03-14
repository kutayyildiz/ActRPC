use crate::{action::Action, error::ActionExecutionError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExecutedAction {
    Applied(Action),
    Failed {
        action: Action,
        error: ActionExecutionError,
    },
}
