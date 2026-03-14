use crate::{json_rpc::JsonRpcMessage, participant::Participant};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptEntry {
    pub from: Participant,
    pub to: Participant,
    pub seq: u64,
    pub ts: f64,
    pub message: JsonRpcMessage,
}
