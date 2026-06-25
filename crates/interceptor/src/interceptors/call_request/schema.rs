use crate::interceptors::call_request::error::CallRequestError;
use actrpc_core::{CallContext, json_rpc::JsonRpcParams};
use actrpc_orchestrator::{method::MethodName, method::ProviderName};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallRequest {
    pub target: String,

    #[serde(default)]
    pub params: Option<Value>,

    #[serde(default)]
    pub ctx: Option<CallContext>,

    #[serde(default)]
    pub reason: Option<String>,
}

pub fn parse_target(target: &str) -> Result<(ProviderName, MethodName), CallRequestError> {
    let parts: Vec<&str> = target.split("::").collect();
    if parts.len() != 2 {
        return Err(invalid_target(target));
    }

    let provider = parts[0];
    let method = parts[1];

    if provider.is_empty() || method.is_empty() {
        return Err(invalid_target(target));
    }

    Ok((ProviderName::from(provider), MethodName::from(method)))
}

fn invalid_target(target: &str) -> CallRequestError {
    CallRequestError::InvalidCallRequest {
        message: format!(
            "target must use non-empty provider::method format, got {target:?}"
        ),
    }
}

pub fn params_to_json_rpc(params: Option<Value>) -> Result<Option<JsonRpcParams>, CallRequestError> {
    match params {
        None => Ok(None),
        Some(Value::Object(map)) => Ok(Some(JsonRpcParams::Object(map))),
        Some(Value::Array(items)) => Ok(Some(JsonRpcParams::Array(items))),
        Some(other) => Err(CallRequestError::InvalidCallRequest {
            message: format!("params must be a JSON object or array, got {other}"),
        }),
    }
}

/// Normalized executable call-request shape used for matching and output.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExecutedCallRequest {
    pub target: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ctx: Option<CallContext>,
}

pub fn executed_call_request_from_call_request(request: &CallRequest) -> ExecutedCallRequest {
    ExecutedCallRequest {
        target: request.target.clone(),
        params: request.params.clone(),
        ctx: request.ctx.clone(),
    }
}

pub fn executed_call_request_from_call_method_params(
    params: &Value,
) -> Result<ExecutedCallRequest, CallRequestError> {
    let provider = params
        .get("provider")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty());
    let method = params
        .get("method")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty());

    let (provider, method) = match (provider, method) {
        (Some(provider), Some(method)) => (provider, method),
        _ => {
            return Err(CallRequestError::InvalidCallRequest {
                message: "CallMethod params must include non-empty provider and method".to_owned(),
            });
        }
    };

    let ctx = match params.get("ctx") {
        None => None,
        Some(value) => Some(serde_json::from_value(value.clone()).map_err(|source| {
            CallRequestError::InvalidCallRequest {
                message: format!("CallMethod ctx is invalid: {source}"),
            }
        })?),
    };

    Ok(ExecutedCallRequest {
        target: format!("{provider}::{method}"),
        params: params.get("params").cloned(),
        ctx,
    })
}

pub fn canonicalize_json_value(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let sorted: BTreeMap<String, Value> = map
                .into_iter()
                .map(|(key, value)| (key, canonicalize_json_value(value)))
                .collect();
            Value::Object(sorted.into_iter().collect::<Map<String, Value>>())
        }
        Value::Array(items) => {
            Value::Array(items.into_iter().map(canonicalize_json_value).collect())
        }
        scalar => scalar,
    }
}

pub fn canonical_executed_call_request_key(
    request: &ExecutedCallRequest,
) -> Result<String, CallRequestError> {
    let value = serde_json::to_value(request).map_err(|source| CallRequestError::InvalidCallRequest {
        message: format!("failed to serialize executed call request: {source}"),
    })?;

    let canonical = canonicalize_json_value(value);

    serde_json::to_string(&canonical).map_err(|source| CallRequestError::InvalidCallRequest {
        message: format!("failed to canonicalize executed call request: {source}"),
    })
}