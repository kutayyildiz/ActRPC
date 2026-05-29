use actrpc_core::{DescribeValue, json_rpc::JsonRpcMessage, participant::Participant};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptEntry {
    pub from: Participant,
    pub to: Participant,
    pub seq: u64,
    pub ts: f64,
    pub message: JsonRpcMessage,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeValue)]
pub struct TranscriptEntryView {
    pub from: String,
    pub to: String,
    pub seq: u64,
    pub ts: f64,
    pub message: serde_json::Value,
}

impl From<TranscriptEntry> for TranscriptEntryView {
    fn from(value: TranscriptEntry) -> Self {
        Self {
            from: value.from.to_string(),
            to: value.to.to_string(),
            seq: value.seq,
            ts: value.ts,
            message: serde_json::to_value(value.message).expect("JsonRpcMessage should serialize"),
        }
    }
}
