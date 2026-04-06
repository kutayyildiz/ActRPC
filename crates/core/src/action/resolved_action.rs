use crate::{
    action::{ActionSpec, ResolvedActionRecord},
    error::{ActionCodecError, ActionExecutionError},
};

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedAction<A: ActionSpec> {
    pub params: A::Params,
    pub result: Result<A::Result, ActionExecutionError>,
}

impl<A> ResolvedAction<A>
where
    A: ActionSpec,
{
    fn decode_from_record(value: &ResolvedActionRecord) -> Result<Self, ActionCodecError> {
        if value.kind != A::KIND {
            return Err(ActionCodecError::KindMismatch {
                expected: A::KIND,
                actual: value.kind.clone(),
            });
        }

        let params = serde_json::from_value(value.params.clone()).map_err(|e| {
            ActionCodecError::InvalidParams {
                action: value.kind.clone(),
                source: e,
            }
        })?;

        let result = match &value.result {
            Ok(v) => Ok(serde_json::from_value(v.clone()).map_err(|e| {
                ActionCodecError::InvalidResult {
                    action: value.kind.clone(),
                    source: e,
                }
            })?),
            Err(err) => Err(err.clone()),
        };

        Ok(Self { params, result })
    }
}

impl<A> TryFrom<&ResolvedActionRecord> for ResolvedAction<A>
where
    A: ActionSpec,
{
    type Error = ActionCodecError;

    fn try_from(value: &ResolvedActionRecord) -> Result<Self, Self::Error> {
        Self::decode_from_record(value)
    }
}

impl<A> TryFrom<ResolvedActionRecord> for ResolvedAction<A>
where
    A: ActionSpec,
{
    type Error = ActionCodecError;

    fn try_from(value: ResolvedActionRecord) -> Result<Self, Self::Error> {
        Self::decode_from_record(&value)
    }
}
