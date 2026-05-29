use serde::{Deserialize, Deserializer, Serialize, de::Error};
use serde_json::{Map, Number, Value};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcMessage {
    Single(JsonRpcSingleMessage),
    Batch(JsonRpcBatch),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcSingleMessage {
    Request(JsonRpcRequest),
    Notification(JsonRpcNotification),
    Response(JsonRpcResponse),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(transparent)]
pub struct JsonRpcBatch(pub Vec<JsonRpcSingleMessage>);

impl<'de> Deserialize<'de> for JsonRpcBatch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let items = Vec::<JsonRpcSingleMessage>::deserialize(deserializer)?;
        if items.is_empty() {
            return Err(Error::custom("JSON-RPC batch must not be an empty array"));
        }
        Ok(Self(items))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum JsonRpcVersion {
    #[serde(rename = "2.0")]
    #[default]
    V2_0,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcId {
    String(String),
    Number(Number),
    Null,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcParams {
    Array(Vec<Value>),
    Object(Map<String, Value>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonRpcRequest {
    #[serde(default)]
    pub jsonrpc: JsonRpcVersion,
    pub id: JsonRpcId,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<JsonRpcParams>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonRpcNotification {
    #[serde(default)]
    pub jsonrpc: JsonRpcVersion,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<JsonRpcParams>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcResponse {
    Success(JsonRpcSuccessResponse),
    Error(JsonRpcErrorResponse),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonRpcSuccessResponse {
    #[serde(default)]
    pub jsonrpc: JsonRpcVersion,
    pub id: JsonRpcId,
    pub result: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonRpcErrorResponse {
    #[serde(default)]
    pub jsonrpc: JsonRpcVersion,
    pub id: JsonRpcId,
    pub error: JsonRpcError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}
