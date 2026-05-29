use crate::{
    action::{ActionSpec, RequestedActionRecord},
    error::ActionCodecError,
};

/// A request from an interceptor for the orchestrator to consider executing.
#[derive(Debug, Clone, PartialEq)]
pub struct RequestedAction<A: ActionSpec> {
    pub params: A::Params,
}

impl<A> RequestedAction<A>
where
    A: ActionSpec,
{
    fn decode_from_record(value: &RequestedActionRecord) -> Result<Self, ActionCodecError> {
        let expected = A::action_kind();

        if value.kind != expected {
            return Err(ActionCodecError::KindMismatch {
                expected,
                actual: value.kind.clone(),
            });
        }

        let raw_params = value
            .params
            .clone()
            .ok_or_else(|| ActionCodecError::MissingParams {
                action: value.kind.clone(),
            })?;

        let params = serde_json::from_value(raw_params).map_err(|source| {
            ActionCodecError::InvalidParams {
                action: value.kind.clone(),
                source,
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
