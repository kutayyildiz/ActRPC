use crate::error::ActionExecutionError;
use actrpc_core::action::ActionKind;
use actrpc_core::{CallContext, MAX_CALL_CONTEXT_BYTES, MAX_INTERCEPTOR_CTX_ENTRIES};

pub fn validate_call_context(
    ctx: &CallContext,
    action: ActionKind,
) -> Result<(), ActionExecutionError> {
    if ctx.interceptors.len() > MAX_INTERCEPTOR_CTX_ENTRIES {
        return Err(ActionExecutionError::InvalidState {
            message: format!(
                "invalid call context for {action}: interceptor ctx entry count {} exceeds max {MAX_INTERCEPTOR_CTX_ENTRIES}",
                ctx.interceptors.len()
            ),
        });
    }

    let serialized = serde_json::to_vec(ctx).map_err(|error| ActionExecutionError::Internal {
        message: format!("failed to serialize call context: {error}"),
    })?;

    if serialized.len() > MAX_CALL_CONTEXT_BYTES {
        return Err(ActionExecutionError::InvalidState {
            message: format!(
                "invalid call context for {action}: serialized ctx size {} exceeds max {MAX_CALL_CONTEXT_BYTES} bytes",
                serialized.len()
            ),
        });
    }

    Ok(())
}
