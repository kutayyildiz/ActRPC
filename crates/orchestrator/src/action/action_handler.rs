use crate::error::ActionHandlerError;
use actrpc_core::{
    action::{ActionKind, RequestedActionRecord, ResolvedActionRecord},
    interception::InterceptionRequest,
};
use std::{future::Future, pin::Pin};

pub type ActionHandlerResult = Result<ResolvedActionRecord, ActionHandlerError>;

pub type ActionHandlerFuture<'a, T = ActionHandlerResult> =
    Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait ActionHandler: Send + Sync {
    fn kind(&self) -> ActionKind;

    fn handle<'a>(
        &'a self,
        request: &'a InterceptionRequest,
        action: RequestedActionRecord,
    ) -> ActionHandlerFuture<'a>
    where
        Self: 'a;
}
