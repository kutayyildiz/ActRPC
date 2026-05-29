use actrpc_core::json_rpc::JsonRpcMessage;
use std::sync::RwLock;

#[derive(Debug, Default)]
pub struct InFlightMessageState {
    message: RwLock<Option<JsonRpcMessage>>,
}

impl InFlightMessageState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_message(&self, message: JsonRpcMessage) {
        let mut slot = self
            .message
            .write()
            .expect("poisoned in-flight message lock");
        *slot = Some(message);
    }

    pub fn clear(&self) {
        let mut slot = self
            .message
            .write()
            .expect("poisoned in-flight message lock");
        *slot = None;
    }

    pub fn snapshot(&self) -> Option<JsonRpcMessage> {
        let slot = self
            .message
            .read()
            .expect("poisoned in-flight message lock");
        slot.clone()
    }

    pub fn replace_message(&self, message: JsonRpcMessage) -> bool {
        let mut slot = self
            .message
            .write()
            .expect("poisoned in-flight message lock");

        if slot.is_none() {
            return false;
        }

        *slot = Some(message);
        true
    }
}
