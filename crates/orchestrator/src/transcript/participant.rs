use std::fmt;

pub const ORCHESTRATOR_MAIN_ID: &str = "main";

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptParticipantKind {
    User,
    Orchestrator,
    Interceptor,
    MethodProvider,
    ReviewProvider,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TranscriptParticipant {
    pub kind: TranscriptParticipantKind,
    pub id: String,
}

impl fmt::Display for TranscriptParticipant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.kind, self.id)
    }
}

impl fmt::Display for TranscriptParticipantKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::User => "user",
            Self::Orchestrator => "orchestrator",
            Self::Interceptor => "interceptor",
            Self::MethodProvider => "method_provider",
            Self::ReviewProvider => "review_provider",
        };
        f.write_str(label)
    }
}

impl TranscriptParticipant {
    pub fn orchestrator_main() -> Self {
        Self {
            kind: TranscriptParticipantKind::Orchestrator,
            id: ORCHESTRATOR_MAIN_ID.to_owned(),
        }
    }

    pub fn interceptor(name: impl Into<String>) -> Self {
        Self {
            kind: TranscriptParticipantKind::Interceptor,
            id: name.into(),
        }
    }

    pub fn method_provider(name: impl Into<String>) -> Self {
        Self {
            kind: TranscriptParticipantKind::MethodProvider,
            id: name.into(),
        }
    }
}
