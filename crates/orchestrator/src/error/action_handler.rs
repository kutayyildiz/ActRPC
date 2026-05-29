use actrpc_core::error::{ActionCodecError, CodecError};

use crate::error::ActionExecutionError;

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ActionHandlerError {
    #[error(transparent)]
    ActionCodec(#[from] ActionCodecError),

    #[error(transparent)]
    Execution(#[from] ActionExecutionError),

    #[error(transparent)]
    Codec(#[from] CodecError),
}
