```rust
use actrpc_core::{
    convert::INTERCEPT_METHOD,
    interception::{InterceptionPhase, InterceptionRequest},
    json_rpc::JsonRpcMessage,
    participant::{Participant, ParticipantType},
};
use actrpc_transport::TransportTarget;

impl<I, A, P> DefaultOrchestrator<I, A, P>
where
    I: crate::InterceptorRegistry,
    A: crate::ActionRegistry,
    P: actrpc_transport::JsonRpcClientProvider,
{
    pub fn handle(
        &self,
        mut message: JsonRpcMessage,
        destination: TransportTarget,
    ) -> Result<JsonRpcMessage, OrchestratorError> {
        let mut prior_actions = Vec::new();

        // outbound
        for interceptor in self
            .interceptor_registry
            .interceptors_for_phase(InterceptionPhase::Outbound)?
        {
            let request = InterceptionRequest {
                origin: Participant {
                    kind: ParticipantType::Orchestrator,
                    id: "orchestrator".to_string(),
                },
                message: message.clone(),
                prior_actions: prior_actions.clone(),
            };

            let client = self.client_provider.get_client(&interceptor.target)?;
            let rpc_request = encode_interception_request(request)?;
            let rpc_response = client.send(rpc_request)?;
            let response = decode_interception_response(rpc_response)?;

            for requested_action in response.actions {
                let executor = self
                    .action_registry
                    .get_executor(requested_action.kind.as_str())
                    .ok_or_else(|| OrchestratorError::UnknownAction {
                        kind: requested_action.kind.clone(),
                    })?;

                let resolved = executor.execute(&request, requested_action)?;
                prior_actions.push(resolved);
            }

            if response.should_stop() {
                break;
            }
        }

        // downstream forward
        {
            let client = self.client_provider.get_client(&destination)?;
            message = client.send(message)?;
        }

        // inbound
        for interceptor in self
            .interceptor_registry
            .interceptors_for_phase(InterceptionPhase::Inbound)?
        {
            let request = InterceptionRequest {
                origin: Participant {
                    kind: ParticipantType::Orchestrator,
                    id: "orchestrator".to_string(),
                },
                message: message.clone(),
                prior_actions: prior_actions.clone(),
            };

            let client = self.client_provider.get_client(&interceptor.target)?;
            let rpc_request = encode_interception_request(request)?;
            let rpc_response = client.send(rpc_request)?;
            let response = decode_interception_response(rpc_response)?;

            for requested_action in response.actions {
                let executor = self
                    .action_registry
                    .get_executor(requested_action.kind.as_str())
                    .ok_or_else(|| OrchestratorError::UnknownAction {
                        kind: requested_action.kind.clone(),
                    })?;

                let resolved = executor.execute(&request, requested_action)?;
                prior_actions.push(resolved);
            }

            if response.should_stop() {
                break;
            }
        }

        Ok(message)
    }
}
```
