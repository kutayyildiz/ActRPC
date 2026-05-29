pub struct OrchestratorBuilder<I, A, P> {
    interceptor_registry: I,
    action_registry: A,
    client_provider: P,
}

impl<I, A, P> OrchestratorBuilder<I, A, P> {
    pub fn with_interceptor_registry<NI>(
        self,
        interceptor_registry: NI,
    ) -> OrchestratorBuilder<NI, A, P> {
        OrchestratorBuilder {
            interceptor_registry,
            action_registry: self.action_registry,
            client_provider: self.client_provider,
        }
    }

    pub fn with_action_registry<NA>(self, action_registry: NA) -> OrchestratorBuilder<I, NA, P> {
        OrchestratorBuilder {
            interceptor_registry: self.interceptor_registry,
            action_registry,
            client_provider: self.client_provider,
        }
    }

    pub fn with_client_provider<NP>(self, client_provider: NP) -> OrchestratorBuilder<I, A, NP> {
        OrchestratorBuilder {
            interceptor_registry: self.interceptor_registry,
            action_registry: self.action_registry,
            client_provider,
        }
    }
}
