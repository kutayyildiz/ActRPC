use crate::{
    error::ProtocolError,
    json_rpc::{JsonRpcMessage, JsonRpcSingleMessage},
};
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use strum::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display)]
#[serde(rename_all = "lowercase")]
pub enum InterceptionPhase {
    Outbound,
    Inbound,
}

impl InterceptionPhase {
    pub fn is_outbound(self) -> bool {
        matches!(self, Self::Outbound)
    }

    pub fn is_inbound(self) -> bool {
        matches!(self, Self::Inbound)
    }
}

impl JsonRpcMessage {
    pub fn phase(&self) -> Result<InterceptionPhase, ProtocolError> {
        match self {
            JsonRpcMessage::Single(JsonRpcSingleMessage::Response(_)) => {
                Ok(InterceptionPhase::Inbound)
            }

            JsonRpcMessage::Single(
                JsonRpcSingleMessage::Request(_) | JsonRpcSingleMessage::Notification(_),
            ) => Ok(InterceptionPhase::Outbound),

            JsonRpcMessage::Batch(batch) => {
                let all_inbound = batch
                    .0
                    .iter()
                    .all(|msg| matches!(msg, JsonRpcSingleMessage::Response(_)));

                let all_outbound = batch.0.iter().all(|msg| {
                    matches!(
                        msg,
                        JsonRpcSingleMessage::Request(_) | JsonRpcSingleMessage::Notification(_)
                    )
                });

                match (all_inbound, all_outbound) {
                    (true, false) => Ok(InterceptionPhase::Inbound),
                    (false, true) => Ok(InterceptionPhase::Outbound),
                    _ => Err(ProtocolError::MixedBatch),
                }
            }
        }
    }
}
