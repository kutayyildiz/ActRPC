use actrpc_core::{
    DescribeValue,
    action::{ActionKind, RequestedActionRecord},
    interception::InterceptionPhase,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeValue)]
pub struct InterceptorPolicy {
    pub outbound: HashSet<ActionKind>,
    pub inbound: HashSet<ActionKind>,
}

impl InterceptorPolicy {
    fn allowed_for_phase(&self, phase: InterceptionPhase) -> &HashSet<ActionKind> {
        match phase {
            InterceptionPhase::Outbound => &self.outbound,
            InterceptionPhase::Inbound => &self.inbound,
        }
    }

    pub fn allows_all(&self, phase: InterceptionPhase, actions: &[RequestedActionRecord]) -> bool {
        let allowed = self.allowed_for_phase(phase);
        actions.iter().all(|action| allowed.contains(&action.kind))
    }

    pub fn conflicting_actions<'a>(
        &self,
        phase: InterceptionPhase,
        actions: &'a [RequestedActionRecord],
    ) -> Vec<&'a RequestedActionRecord> {
        let allowed = self.allowed_for_phase(phase);

        actions
            .iter()
            .filter(|action| !allowed.contains(&action.kind))
            .collect()
    }
}
