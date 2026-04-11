use actrpc_core::{
    action::{RequestedActionRecord, ResolvedActionRecord},
    error::ActionExecutionError,
    interception::InterceptionRequest,
};

pub trait ActionExecutor: Send + Sync {
    fn kind(&self) -> &str;

    fn execute(
        &self,
        request: &InterceptionRequest,
        action: RequestedActionRecord,
    ) -> Result<ResolvedActionRecord, ActionExecutionError>;
}

pub trait ActionRegistry {
    fn get_executor<'a>(&'a self, kind: &str) -> Option<&'a dyn ActionExecutor>;
}
