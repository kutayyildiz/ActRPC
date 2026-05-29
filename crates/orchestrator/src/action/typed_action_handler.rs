use crate::{action::ActionHandlerFuture, error::ActionExecutionError};
use actrpc_core::{
    action::{ActionSpec, RequestedAction, ResolvedAction},
    interception::InterceptionRequest,
};

pub type TypedActionHandlerResult<A> = Result<ResolvedAction<A>, ActionExecutionError>;

pub trait TypedActionHandler<A>: Send + Sync
where
    A: ActionSpec,
    A::Params: Send + 'static,
    A::Result: Send + 'static,
{
    fn handle_typed<'a>(
        &'a self,
        request: &'a InterceptionRequest,
        action: RequestedAction<A>,
    ) -> ActionHandlerFuture<'a, TypedActionHandlerResult<A>>
    where
        Self: 'a;
}
