use crate::transcript::{CallId, TranscriptEntry, TranscriptEntryInput, TranscriptEntryView};
use std::{
    sync::{
        RwLock,
        atomic::{AtomicU64, Ordering},
    },
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

#[derive(Debug, Default)]
pub struct TranscriptState {
    next_call_id: AtomicU64,
    inner: RwLock<TranscriptInner>,
}

impl TranscriptState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allocate_call_id(&self) -> CallId {
        CallId(self.next_call_id.fetch_add(1, Ordering::Relaxed) + 1)
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
