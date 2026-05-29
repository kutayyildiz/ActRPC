mod action_codec;
mod codec;
mod protocol;

pub use action_codec::ActionCodecError;
pub use codec::CodecError;
pub use protocol::ProtocolError;

use crate::json_rpc::JsonRpcError;

/// Umbrella error for operations implemented inside `actrpc-core`.
///
/// This crate only owns protocol/model/codec concerns.
/// Runtime crates should define their own execution, policy, transport,
/// and orchestration errors locally.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    ActionCodec(#[from] ActionCodecError),

    #[error(transparent)]
    Codec(#[from] CodecError),

    #[error(transparent)]
    Protocol(#[from] ProtocolError),

    #[error("remote JSON-RPC error {code}: {message}", code = .0.code, message = .0.message)]
    RemoteJsonRpc(JsonRpcError),
}

pub type ActRpcError = Error;
