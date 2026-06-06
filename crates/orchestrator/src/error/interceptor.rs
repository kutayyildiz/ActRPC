use actrpc_core::action::ActionKind;

use crate::error::InterceptorRuntimeError;

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum InterceptorError {
    #[error("interceptor initialization failed for {name}: {source}")]
    InitializationFailed {
        name: String,
        #[source]
        source: InterceptorRuntimeError,
    },

    #[error("interceptor invocation failed for {name}: {source}")]
    InvocationFailed {
        name: String,
        #[source]
        source: InterceptorRuntimeError,
    },

    #[error("duplicate interceptor registration for {name}")]
    DuplicateRegistration { name: String },

    #[error("pipeline {phase} references unknown interceptor {name}")]
    UnknownPipelineInterceptor { phase: String, name: String },

    #[error("pipeline {phase} contains duplicate interceptor {name}")]
    DuplicatePipelineEntry { phase: String, name: String },

    #[error("interceptor {interceptor} declared unsupported action descriptor for action {action}")]
    UnsupportedActionDescriptor {
        interceptor: String,
        action: ActionKind,
    },

    #[error("interceptor {interceptor} descriptor mismatch for action {action}")]
    ActionDescriptorMismatch {
        interceptor: String,
        action: ActionKind,
    },

    #[error(
        "interceptor {interceptor} policy references undeclared action {action} in {phase} phase"
    )]
    PolicyReferencesUndeclaredAction {
        interceptor: String,
        action: ActionKind,
        phase: String,
    },

    #[error("invalid interceptor initialization for {interceptor}: {message}")]
    InvalidInitialization {
        interceptor: String,
        message: String,
    },

    #[error("unknown endpoint {name} referenced by interceptor")]
    UnknownEndpoint { name: String },
}
