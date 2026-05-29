use actrpc_core::error::{CodecError, ProtocolError};
use actrpc_transport::TransportError;

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum InterceptorRuntimeError {
    #[error("interceptor initialization failed: {message}")]
    Initialization { message: String },

    #[error("interceptor request failed: {message}")]
    Request { message: String },

    #[error(transparent)]
    Transport(#[from] TransportError),

    #[error(transparent)]
    Protocol(#[from] ProtocolError),

    #[error(transparent)]
    Codec(#[from] CodecError),

    #[error("internal interceptor runtime error: {message}")]
    Internal { message: String },
}
