use crate::method::ProviderName;
use actrpc_core::{ACTRPC_METHOD_PROVIDER_CHANGED_METHOD, json_rpc::JsonRpcNotification};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct MethodProviderChangedParams {
    pub provider: ProviderName,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

pub fn try_parse_method_provider_changed(
    notification: &JsonRpcNotification,
) -> Option<MethodProviderChangedParams> {
    if notification.method != ACTRPC_METHOD_PROVIDER_CHANGED_METHOD {
        return None;
    }

    let params = notification.params.as_ref()?;
    let value = match params {
        actrpc_core::json_rpc::JsonRpcParams::Object(map) => serde_json::Value::Object(map.clone()),
        _ => return None,
    };

    serde_json::from_value(value).ok()
}
