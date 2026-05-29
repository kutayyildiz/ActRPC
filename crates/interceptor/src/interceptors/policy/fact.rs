use crate::interceptors::policy::config::PolicyFactPath;
use actrpc_core::{
    interception::{InterceptionPhase, InterceptionRequest},
    json_rpc::{JsonRpcMessage, JsonRpcParams, JsonRpcResponse, JsonRpcSingleMessage},
};

pub struct PolicyFacts<'a> {
    request: &'a InterceptionRequest,
}

impl<'a> PolicyFacts<'a> {
    pub fn new(request: &'a InterceptionRequest) -> Self {
        Self { request }
    }

    pub fn get_string(&self, path: &PolicyFactPath) -> Option<String> {
        let path = path.as_str();

        match path {
            "phase" => return self.phase(),
            "origin.kind" => return Some(self.request.origin.kind.to_string()),
            "origin.id" => return Some(self.request.origin.id.clone()),
            "message.method" => return self.message_method(),
            "message.error.code" => return self.message_error_code(),
            "message.error.message" => return self.message_error_message(),
            _ => {}
        }

        if let Some(rest) = path.strip_prefix("message.params.") {
            return self.message_params_path(rest);
        }

        if let Some(rest) = path.strip_prefix("message.result.") {
            return self.message_result_path(rest);
        }

        if let Some(rest) = path.strip_prefix("message.error.data.") {
            return self.message_error_data_path(rest);
        }

        None
    }

    fn phase(&self) -> Option<String> {
        match self.request.phase().ok()? {
            InterceptionPhase::Outbound => Some("outbound".to_owned()),
            InterceptionPhase::Inbound => Some("inbound".to_owned()),
        }
    }

    fn message_method(&self) -> Option<String> {
        match &self.request.message {
            JsonRpcMessage::Single(JsonRpcSingleMessage::Request(request)) => {
                Some(request.method.clone())
            }

            JsonRpcMessage::Single(JsonRpcSingleMessage::Notification(notification)) => {
                Some(notification.method.clone())
            }

            _ => None,
        }
    }

    fn message_params_path(&self, path: &str) -> Option<String> {
        let value = match &self.request.message {
            JsonRpcMessage::Single(JsonRpcSingleMessage::Request(request)) => {
                params_to_value(request.params.as_ref()?)
            }

            JsonRpcMessage::Single(JsonRpcSingleMessage::Notification(notification)) => {
                params_to_value(notification.params.as_ref()?)
            }

            _ => return None,
        };

        json_path_to_string(&value, path)
    }

    fn message_result_path(&self, path: &str) -> Option<String> {
        let value = match &self.request.message {
            JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Success(
                success,
            ))) => &success.result,

            _ => return None,
        };

        json_path_to_string(value, path)
    }

    fn message_error_code(&self) -> Option<String> {
        match &self.request.message {
            JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Error(
                error,
            ))) => Some(error.error.code.to_string()),

            _ => None,
        }
    }

    fn message_error_message(&self) -> Option<String> {
        match &self.request.message {
            JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Error(
                error,
            ))) => Some(error.error.message.clone()),

            _ => None,
        }
    }

    fn message_error_data_path(&self, path: &str) -> Option<String> {
        let value = match &self.request.message {
            JsonRpcMessage::Single(JsonRpcSingleMessage::Response(JsonRpcResponse::Error(
                error,
            ))) => error.error.data.as_ref()?,

            _ => return None,
        };

        json_path_to_string(value, path)
    }
}

fn params_to_value(params: &JsonRpcParams) -> serde_json::Value {
    match params {
        JsonRpcParams::Array(values) => serde_json::Value::Array(values.clone()),
        JsonRpcParams::Object(map) => serde_json::Value::Object(map.clone()),
    }
}

fn json_path_to_string(value: &serde_json::Value, path: &str) -> Option<String> {
    let mut current = value;

    for segment in path.split('.') {
        if segment.is_empty() {
            return None;
        }

        current = current.get(segment)?;
    }

    json_value_to_string(current)
}

fn json_value_to_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Number(value) => Some(value.to_string()),
        serde_json::Value::Bool(value) => Some(value.to_string()),
        serde_json::Value::Null => Some("null".to_owned()),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => Some(value.to_string()),
    }
}
