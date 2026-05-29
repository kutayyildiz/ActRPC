use actrpc_core::error::CodecError;
use actrpc_transport::TransportError;

mod action;
mod action_execution;
mod action_handler;
mod config;
mod interceptor;
mod interceptor_runtime;
mod method;

pub use action::ActionError;
pub use action_execution::ActionExecutionError;
pub use action_handler::ActionHandlerError;
pub use config::ConfigError;
pub use interceptor::InterceptorError;
pub use interceptor_runtime::InterceptorRuntimeError;
pub use method::{MethodCallError, MethodCatalogError, MethodProviderBuildError};

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error(transparent)]
    Action(#[from] ActionError),

    #[error(transparent)]
    Interceptor(#[from] InterceptorError),

    #[error(transparent)]
    Transport(#[from] TransportError),

    #[error(transparent)]
    Codec(#[from] CodecError),

    #[error("internal orchestrator error: {message}")]
    Internal { message: String },

    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error(transparent)]
    MethodCatalog(#[from] MethodCatalogError),

    #[error(transparent)]
    MethodCall(#[from] MethodCallError),
}
