use crate::{
    action::{ActionSpec, ResolvedActionRecord},
    error::ActionCodecError,
};

/// The orchestrator’s recorded outcome for a previously requested action.
/// If the interceptor is reinvoked, this is fed back into subsequent
/// interceptor invocations.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedAction<A: ActionSpec> {
    pub params: A::Params,
    pub result: Result<A::Result, crate::error::ProtocolError>,
}

impl<A> ResolvedAction<A>
where
    A: ActionSpec,
{
    fn decode_from_record(value: &ResolvedActionRecord) -> Result<Self, ActionCodecError> {
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

        let result = match &value.result {
            Ok(Some(v)) => Ok(serde_json::from_value(v.clone()).map_err(|source| {
                ActionCodecError::InvalidResult {
                    action: value.kind.clone(),
                    source,
                }
            })?),
            Ok(None) => {
                return Err(ActionCodecError::MissingOkResult {
                    action: value.kind.clone(),
                });
            }
            Err(err) => Err(err.clone()),
        };

        Ok(Self { params, result })
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
