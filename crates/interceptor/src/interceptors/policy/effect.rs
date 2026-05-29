use crate::interceptors::policy::{config::PolicyEffect, error::PolicyError};
use actrpc_core::action::{ActionSpec, RequestedAction, RequestedActionRecord};
use actrpc_orchestrator::action::actions::{
    exclude_interceptors::{ExcludeInterceptors, ExcludeInterceptorsParams},
    reject_call::{RejectCall, RejectCallParams},
};

pub fn effect_to_action(effect: &PolicyEffect) -> Result<RequestedActionRecord, PolicyError> {
    match effect {
        PolicyEffect::ExcludeInterceptors {
            exclude_interceptors,
        } => requested_action::<ExcludeInterceptors>(ExcludeInterceptorsParams {
            names: exclude_interceptors.names.clone(),
        }),

        PolicyEffect::RejectCall { reject_call } => {
            requested_action::<RejectCall>(RejectCallParams {
                error: reject_call.error.clone(),
            })
        }
    }
}

fn requested_action<A>(params: A::Params) -> Result<RequestedActionRecord, PolicyError>
where
    A: ActionSpec,
{
    RequestedAction::<A> { params }
        .try_into()
        .map_err(|source| PolicyError::ActionEncoding { source })
}
