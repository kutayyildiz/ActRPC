use crate::{
    runtime::execution_tree::ExecutionTreeState,
    transcript::{TranscriptEntry, TranscriptEntryInput, TranscriptEntryView},
};
use actrpc_core::CallId;
use std::{
    sync::RwLock,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TranscriptError {
    #[error("system time is before Unix epoch")]
    TimeBeforeUnixEpoch,
    #[error("transcript storage unavailable: {message}")]
    StorageUnavailable { message: String },
    #[error("failed to serialize transcript payload: {message}")]
    Serialize { message: String },
}

#[derive(Debug, Default)]
struct TranscriptInner {
    next_seq: u64,
    entries: Vec<TranscriptEntry>,
}

#[derive(Debug)]
pub struct TranscriptState {
    inner: RwLock<TranscriptInner>,
    execution_tree: ExecutionTreeState,
}

impl Default for TranscriptState {
    fn default() -> Self {
        Self {
            inner: RwLock::new(TranscriptInner::default()),
            execution_tree: ExecutionTreeState::new(),
        }
    }
}

impl TranscriptState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn execution_tree(&self) -> &ExecutionTreeState {
        &self.execution_tree
    }

    pub fn allocate_call_id(&self) -> CallId {
        CallId::new()
    }

    pub fn append(&self, input: TranscriptEntryInput) -> Result<(), TranscriptError> {
        let ts_ms = current_ts_ms()?;

        let mut inner = self
            .inner
            .write()
            .map_err(|err| TranscriptError::StorageUnavailable {
                message: err.to_string(),
            })?;

        inner.next_seq += 1;
        let seq = inner.next_seq;

        inner.entries.push(TranscriptEntry {
            seq,
            ts_ms,
            call_id: input.call_id,
            parent_call_id: input.parent_call_id,
            depth: input.depth,
            from: input.from,
            to: input.to,
            protocol: input.protocol.to_owned(),
            message: input.message,
        });

        Ok(())
    }

    pub fn snapshot(&self) -> Result<Vec<TranscriptEntryView>, TranscriptError> {
        let inner = self
            .inner
            .read()
            .map_err(|err| TranscriptError::StorageUnavailable {
                message: err.to_string(),
            })?;
        Ok(inner
            .entries
            .iter()
            .cloned()
            .map(TranscriptEntryView::from)
            .collect())
    }

    pub fn len(&self) -> Result<usize, TranscriptError> {
        let inner = self
            .inner
            .read()
            .map_err(|err| TranscriptError::StorageUnavailable {
                message: err.to_string(),
            })?;
        Ok(inner.entries.len())
    }

    pub fn is_empty(&self) -> Result<bool, TranscriptError> {
        Ok(self.len()? == 0)
    }
}

fn current_ts_ms() -> Result<u64, TranscriptError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| TranscriptError::TimeBeforeUnixEpoch)?;
    Ok(duration.as_millis() as u64)
}
