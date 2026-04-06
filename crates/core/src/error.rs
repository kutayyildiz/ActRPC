mod action_codec;
mod action_execution;
mod codec;
mod interception;
mod policy;
mod protocol;

pub use action_codec::ActionCodecError;
pub use action_execution::ActionExecutionError;
pub use codec::CodecError;
pub use interception::InterceptionError;
pub use policy::PolicyError;
pub use protocol::ProtocolError;

use crate::json_rpc::JsonRpcError;

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    ActionCodec(#[from] ActionCodecError),

    #[error(transparent)]
    ActionExecution(#[from] ActionExecutionError),

    #[error(transparent)]
    Codec(#[from] CodecError),

    #[error(transparent)]
    Interception(#[from] InterceptionError),

    #[error(transparent)]
    Policy(#[from] PolicyError),

    #[error("remote JSON-RPC error {}: {}", ._0.code, ._0.message)]
    RemoteJsonRpc(JsonRpcError),

    #[error(transparent)]
    Protocol(#[from] ProtocolError),
}

pub type ActRpcError = Error;
