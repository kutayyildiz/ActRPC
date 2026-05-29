use crate::{error::ActionExecutionError, transcript::TranscriptEntryView};
use std::sync::RwLock;

#[derive(Debug, Default)]
pub struct TranscriptState {
    entries: RwLock<Vec<TranscriptEntryView>>,
}

impl TranscriptState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(&self, entry: TranscriptEntryView) -> Result<(), ActionExecutionError> {
        let mut entries = self.entries.write().expect("poisoned transcript lock");
        entries.push(entry);
        Ok(())
    }

    pub fn snapshot(&self) -> Vec<TranscriptEntryView> {
        let entries = self.entries.read().expect("poisoned transcript lock");
        entries.clone()
    }

    pub fn len(&self) -> usize {
        let entries = self.entries.read().expect("poisoned transcript lock");
        entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
