use crate::interceptors::dynamic_policy::{
    interceptor::DynamicPolicyInterceptor, provider::DynamicPolicyMethodProvider,
    store::DynamicPolicyStore,
};
use std::sync::Arc;

pub struct DynamicPolicyComponent {
    pub store: Arc<DynamicPolicyStore>,
    pub provider: DynamicPolicyMethodProvider,
    pub interceptor: DynamicPolicyInterceptor,
}

impl DynamicPolicyComponent {
    pub fn new(store: Arc<DynamicPolicyStore>) -> Self {
        Self {
            provider: DynamicPolicyMethodProvider::new(store.clone()),
            interceptor: DynamicPolicyInterceptor::new(store.clone()),
            store,
        }
    }
}

pub fn new_component() -> DynamicPolicyComponent {
    DynamicPolicyComponent::new(DynamicPolicyStore::shared())
}
