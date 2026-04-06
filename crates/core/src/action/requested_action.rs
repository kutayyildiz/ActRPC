use crate::{
    action::{ActionSpec, RequestedActionRecord},
    error::ActionCodecError,
};

#[derive(Debug, Clone, PartialEq)]
pub struct RequestedAction<A: ActionSpec> {
    pub params: A::Params,
}

impl<A> RequestedAction<A>
where
    A: ActionSpec,
{
    fn decode_from_record(value: &RequestedActionRecord) -> Result<Self, ActionCodecError> {
        if value.kind != A::KIND {
            return Err(ActionCodecError::KindMismatch {
                expected: A::KIND,
                actual: value.kind.clone(),
            });
        }

        let params = serde_json::from_value(value.params.clone()).map_err(|e| {
            ActionCodecError::InvalidParams {
                action: A::KIND,
                source: e,
            }
        })?;

        Ok(Self { params })
    }
}

impl<A> TryFrom<RequestedActionRecord> for RequestedAction<A>
where
    A: ActionSpec,
{
    type Error = ActionCodecError;

    fn try_from(value: RequestedActionRecord) -> Result<Self, Self::Error> {
        Self::decode_from_record(&value)
    }
}

impl<A> TryFrom<&RequestedActionRecord> for RequestedAction<A>
where
    A: ActionSpec,
{
    type Error = ActionCodecError;

    fn try_from(value: &RequestedActionRecord) -> Result<Self, Self::Error> {
        Self::decode_from_record(value)
    }
}
