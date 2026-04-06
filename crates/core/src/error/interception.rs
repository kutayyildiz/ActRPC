use super::{action_codec::ActionCodecError, policy::PolicyError};

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum InterceptionError {
    #[error("interception rejected: {reason}")]
    Rejection { reason: String },

    #[error(transparent)]
    Policy(#[from] PolicyError),

    #[error(transparent)]
    ActionCodec(#[from] ActionCodecError),

    #[error("internal interception error: {message}")]
    Internal { message: String },
}
