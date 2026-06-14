mod participant;
mod protocol;

pub use participant::{TranscriptParticipant, TranscriptParticipantKind};
pub use protocol::{
    PROTOCOL_INTERCEPTOR_REQUEST, PROTOCOL_INTERCEPTOR_RESPONSE, PROTOCOL_METHOD_REQUEST,
    PROTOCOL_METHOD_RESPONSE,
};

use actrpc_core::{CallId, descriptor::traits::DescribeValue};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptEntry {
    pub seq: u64,
    pub ts_ms: u64,
    pub call_id: CallId,
    pub parent_call_id: Option<CallId>,
    pub depth: usize,
    pub from: TranscriptParticipant,
    pub to: TranscriptParticipant,
    pub protocol: String,
    pub message: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptEntryInput {
    pub call_id: CallId,
    pub parent_call_id: Option<CallId>,
    pub depth: usize,
    pub from: TranscriptParticipant,
    pub to: TranscriptParticipant,
    pub protocol: &'static str,
    pub message: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptEntryView {
    pub seq: u64,
    pub ts_ms: u64,
    pub call_id: CallId,
    pub parent_call_id: Option<CallId>,
    pub depth: usize,
    pub from: String,
    pub to: String,
    pub protocol: String,
    pub message: serde_json::Value,
}

impl From<TranscriptEntry> for TranscriptEntryView {
    fn from(value: TranscriptEntry) -> Self {
        Self {
            seq: value.seq,
            ts_ms: value.ts_ms,
            call_id: value.call_id,
            parent_call_id: value.parent_call_id,
            depth: value.depth,
            from: value.from.to_string(),
            to: value.to.to_string(),
            protocol: value.protocol,
            message: value.message,
        }
    }
}

pub fn to_transcript_value<T: Serialize>(
    value: &T,
) -> Result<serde_json::Value, serde_json::Error> {
    serde_json::to_value(value)
}

impl DescribeValue for TranscriptEntryView {
    fn describe_value() -> actrpc_core::descriptor::types::ValueDescriptor {
        use actrpc_core::descriptor::types::{
            FieldDescriptor, NestedObjectDescriptor, PrimitiveDescriptor, ValueDescriptor,
        };

        ValueDescriptor::Object(NestedObjectDescriptor {
            fields: vec![
                FieldDescriptor {
                    name: "seq".to_owned(),
                    ty: ValueDescriptor::Primitive(PrimitiveDescriptor::Integer),
                },
                FieldDescriptor {
                    name: "ts_ms".to_owned(),
                    ty: ValueDescriptor::Primitive(PrimitiveDescriptor::Integer),
                },
                FieldDescriptor {
                    name: "call_id".to_owned(),
                    ty: ValueDescriptor::Primitive(PrimitiveDescriptor::String),
                },
                FieldDescriptor {
                    name: "parent_call_id".to_owned(),
                    ty: ValueDescriptor::OneOf(vec![
                        ValueDescriptor::Primitive(PrimitiveDescriptor::Null),
                        ValueDescriptor::Primitive(PrimitiveDescriptor::String),
                    ]),
                },
                FieldDescriptor {
                    name: "depth".to_owned(),
                    ty: ValueDescriptor::Primitive(PrimitiveDescriptor::Integer),
                },
                FieldDescriptor {
                    name: "from".to_owned(),
                    ty: ValueDescriptor::Primitive(PrimitiveDescriptor::String),
                },
                FieldDescriptor {
                    name: "to".to_owned(),
                    ty: ValueDescriptor::Primitive(PrimitiveDescriptor::String),
                },
                FieldDescriptor {
                    name: "protocol".to_owned(),
                    ty: ValueDescriptor::Primitive(PrimitiveDescriptor::String),
                },
                FieldDescriptor {
                    name: "message".to_owned(),
                    ty: ValueDescriptor::Any,
                },
            ],
        })
    }
}
