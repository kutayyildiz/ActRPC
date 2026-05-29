use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::action::{ActionKind, ActionSpec, RequestedAction};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestedActionRecord {
    pub kind: ActionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl<A> TryFrom<RequestedAction<A>> for RequestedActionRecord
where
    A: ActionSpec,
{
    type Error = serde_json::Error;

    fn try_from(value: RequestedAction<A>) -> Result<Self, Self::Error> {
        Ok(Self {
            kind: A::KIND.into(),
            params: Some(serde_json::to_value(value.params)?),
        })
    }
}
