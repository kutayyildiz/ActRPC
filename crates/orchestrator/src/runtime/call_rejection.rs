use actrpc_core::json_rpc::JsonRpcError;
use std::sync::RwLock;

#[derive(Debug, Default)]
pub struct CurrentCallRejection {
    error: RwLock<Option<JsonRpcError>>,
}

impl CurrentCallRejection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&self, error: JsonRpcError) {
        let mut slot = self.error.write().expect("poisoned call rejection lock");
        *slot = Some(error);
    }

    pub fn clear(&self) {
        let mut slot = self.error.write().expect("poisoned call rejection lock");
        *slot = None;
    }

    pub fn snapshot(&self) -> Option<JsonRpcError> {
        let slot = self.error.read().expect("poisoned call rejection lock");
        slot.clone()
    }

    pub fn is_rejected(&self) -> bool {
        self.error
            .read()
            .expect("poisoned call rejection lock")
            .is_some()
    }
}
