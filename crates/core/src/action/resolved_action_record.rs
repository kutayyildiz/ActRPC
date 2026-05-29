use crate::{
    action::{ActionKind, ActionSpec, ResolvedAction},
    error::ProtocolError,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResolvedActionRecord {
    pub kind: ActionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    pub result: Result<Option<Value>, ProtocolError>,
}

impl<A> TryFrom<ResolvedAction<A>> for ResolvedActionRecord
where
    A: ActionSpec,
    A::Params: Serialize,
    A::Result: Serialize,
{
    type Error = serde_json::Error;

    fn try_from(value: ResolvedAction<A>) -> Result<Self, Self::Error> {
        Ok(Self {
            kind: A::action_kind(),
            params: Some(serde_json::to_value(value.params)?),
            result: match value.result {
                Ok(ok) => Ok(Some(serde_json::to_value(ok)?)),
                Err(err) => Err(err),
            },
        })
    }
}
