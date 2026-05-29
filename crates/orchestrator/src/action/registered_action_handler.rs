use crate::action::{ActionHandler, ActionHandlerFuture, TypedActionHandler};
use actrpc_core::{
    action::{ActionKind, ActionSpec, RequestedActionRecord, ResolvedActionRecord},
    interception::InterceptionRequest,
};
use std::marker::PhantomData;

pub struct RegisteredActionHandler<A, H>
where
    A: ActionSpec,
    A::Params: Send + 'static,
    A::Result: Send + 'static,
    H: TypedActionHandler<A>,
{
    inner: H,
    _marker: PhantomData<A>,
}

impl<A, H> RegisteredActionHandler<A, H>
where
    A: ActionSpec,
    A::Params: Send + 'static,
    A::Result: Send + 'static,
    H: TypedActionHandler<A>,
{
    pub fn new(inner: H) -> Self {
        Self {
            inner,
            _marker: PhantomData,
        }
    }
}

impl<A, H> ActionHandler for RegisteredActionHandler<A, H>
where
    A: ActionSpec + Send + Sync + 'static,
    A::Params: Send + 'static,
    A::Result: Send + 'static,
    H: TypedActionHandler<A> + Send + Sync + 'static,
{
    fn kind(&self) -> ActionKind {
        A::action_kind()
    }

    fn handle<'a>(
        &'a self,
        request: &'a InterceptionRequest,
        action: RequestedActionRecord,
    ) -> ActionHandlerFuture<'a>
    where
        Self: 'a,
    {
        Box::pin(async move {
            let typed_action = action.try_into()?;
            let resolved = self.inner.handle_typed(request, typed_action).await?;
            let record: ResolvedActionRecord =
                resolved.try_into().map_err(map_serde_serialize_err)?;

            Ok(record)
        })
    }
}

fn map_serde_serialize_err(error: serde_json::Error) -> actrpc_core::error::CodecError {
    actrpc_core::error::CodecError::Serialize(error.to_string())
}
