use crate::{
    action::{ActionKind, ActionSpec, ResolvedAction},
    error::ActionExecutionError,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResolvedActionRecord {
    pub kind: ActionKind,
    pub params: Value,
    pub result: Result<Value, ActionExecutionError>,
}

impl<A> TryFrom<&ResolvedAction<A>> for ResolvedActionRecord
where
    A: ActionSpec,
{
    type Error = serde_json::Error;

    fn try_from(value: &ResolvedAction<A>) -> Result<Self, Self::Error> {
        Ok(Self {
            kind: A::KIND,
            params: serde_json::to_value(&value.params)?,
            result: match &value.result {
                Ok(ok) => Ok(serde_json::to_value(ok)?),
                Err(err) => Err(err.clone()),
            },
        })
    }
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
            kind: A::KIND,
            params: serde_json::to_value(value.params)?,
            result: match value.result {
                Ok(ok) => Ok(serde_json::to_value(ok)?),
                Err(err) => Err(err),
            },
        })
    }
}
