use crate::interceptors::dynamic_policy::{
    config::DynamicPolicyConfig,
    interceptor::DynamicPolicyInterceptor,
    store::DynamicPolicyStore,
};
use std::sync::Arc;

pub struct DynamicPolicyComponent {
    pub store: Arc<DynamicPolicyStore>,
    pub interceptor: DynamicPolicyInterceptor,
}

impl DynamicPolicyComponent {
    pub fn new(store: Arc<DynamicPolicyStore>, config: DynamicPolicyConfig) -> Self {
        Self {
            interceptor: DynamicPolicyInterceptor::new(store.clone(), config),
            store,
        }
    }
}

pub fn new_component() -> DynamicPolicyComponent {
    DynamicPolicyComponent::new(DynamicPolicyStore::shared(), DynamicPolicyConfig::default())
}

pub fn new_component_with_config(config: DynamicPolicyConfig) -> DynamicPolicyComponent {
    DynamicPolicyComponent::new(DynamicPolicyStore::shared(), config)
}