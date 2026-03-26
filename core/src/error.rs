use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Error)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum ActRpcError {
    #[error(transparent)]
    Action(#[from] ActionError),

    #[error(transparent)]
    Interceptor(#[from] InterceptorError),

    #[error("invalid JSON-RPC message: {message}")]
    InvalidMessage { message: String },

    #[error(transparent)]
    Codec(#[from] CodecError),

    #[error("internal orchestrator error: {message}")]
    Internal { message: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Error)]
#[non_exhaustive]
pub enum ActionError {
    #[error("action forbidden by policy: {action}")]
    ForbiddenCapability { action: String },

    #[error("unsupported action at dispatch: {action}")]
    UnsupportedAction { action: String },

    #[error("target '{target}' not found")]
    TargetNotFound { target: String },

    #[error("remote call failed: {message}")]
    RemoteCallFailed { message: String },

    #[error("invalid params: {params}")]
    InvalidParams { params: String },

    #[error("internal orchestrator error: {message}")]
    Internal { message: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Error)]
#[non_exhaustive]
pub enum InterceptorError {
    #[error("interceptor '{name}' decision failed: {reason}")]
    DecisionFailed { name: String, reason: String },

    #[error("interceptor '{name}' returned malformed response: {reason}")]
    MalformedResponse { name: String, reason: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Error)]
#[non_exhaustive]
pub enum CodecError {
    #[error("unsupported action: {action}")]
    UnsupportedAction { action: String },

    #[error("failed to decode requested action '{action}' params: {reason}; params={params}")]
    DecodeRequestedParams {
        action: String,
        params: Value,
        reason: String,
    },

    #[error("failed to decode resolved action '{action}' params: {reason}; params={params}")]
    DecodeResolvedParams {
        action: String,
        params: Value,
        reason: String,
    },

    #[error("failed to decode resolved action '{action}' result: {reason}; result={result}")]
    DecodeResolvedResult {
        action: String,
        result: Value,
        reason: String,
    },

    #[error("mismatched action kind: expected {expected}, got {actual}")]
    MismatchedActionKind { expected: String, actual: String },
}
