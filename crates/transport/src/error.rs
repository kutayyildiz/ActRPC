#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("failed to initialize client: {message}")]
    ClientInit { message: String },

    #[error("connection failed: {message}")]
    Connection { message: String },

    #[error("request timed out")]
    Timeout,

    #[error("transport i/o error: {message}")]
    Io { message: String },

    #[error("unsupported transport target {target}: {message}")]
    UnsupportedTarget { target: String, message: String },

    #[error("HTTP request failed with status {status}: {body}")]
    HttpStatus { status: u16, body: String },

    #[error(transparent)]
    Codec(#[from] actrpc_core::error::CodecError),

    #[error(transparent)]
    Protocol(#[from] actrpc_core::error::ProtocolError),

    #[error("internal transport error: {message}")]
    Internal { message: String },
}
