use actrpc_core::{error::CodecError, interception::InterceptionPhase};
use actrpc_transport::TransportError;

use crate::runtime::TranscriptError;

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
pub use method::{
    MethodCallError, MethodCatalogError, MethodProviderBuildError, MethodProviderRefreshError,
};

#[derive(Debug, thiserror::Error)]
#[error("endpoint {endpoint} does not support session")]
pub struct EndpointDoesNotSupportSessionError {
    pub endpoint: crate::endpoint::EndpointName,
}

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

    #[error(
        "endpoint {endpoint} cannot support session required for watchable providers: {message}"
    )]
    WatchableUnsupportedEndpoint {
        endpoint: crate::endpoint::EndpointName,
        message: String,
    },

    #[error(transparent)]
    Transcript(#[from] TranscriptError),

    #[error(
        "max call depth exceeded: attempted depth {attempted_depth}, max {max_call_depth}: raise runtime.max_call_depth"
    )]
    MaxCallDepthExceeded {
        attempted_depth: usize,
        max_call_depth: usize,
    },

    #[error(
        "max interception reinvokes exceeded for interceptor {interceptor} in {phase} phase (limit {max_interception_reinvokes}): raise {config_hint}"
    )]
    MaxInterceptionReinvokesExceeded {
        interceptor: String,
        phase: InterceptionPhase,
        max_interception_reinvokes: usize,
        config_hint: String,
    },

    #[error(
        "interception request timed out for interceptor {interceptor} in {phase} phase after {timeout_ms} ms: raise {config_hint}"
    )]
    InterceptionRequestTimeout {
        interceptor: String,
        phase: InterceptionPhase,
        timeout_ms: u64,
        config_hint: String,
    },

    #[error(
        "max actions per interception exceeded for interceptor {interceptor} in {phase} phase (attempted {attempted_actions}, limit {max_actions_per_interception}): raise {config_hint}"
    )]
    MaxActionsPerInterceptionExceeded {
        interceptor: String,
        phase: InterceptionPhase,
        attempted_actions: usize,
        max_actions_per_interception: usize,
        config_hint: String,
    },
}
