use crate::descriptor::traits::DescribeValue;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const MAX_CALL_CONTEXT_BYTES: usize = 16 * 1024;
pub const MAX_INTERCEPTOR_CTX_ENTRIES: usize = 16;

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shared: Option<Value>,

    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub interceptors: BTreeMap<String, Value>,
}

impl DescribeValue for CallContext {
    fn describe_value() -> crate::descriptor::types::ValueDescriptor {
        crate::descriptor::types::ValueDescriptor::Any
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterceptionContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shared: Option<Value>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub private: Option<Value>,
}

impl DescribeValue for InterceptionContext {
    fn describe_value() -> crate::descriptor::types::ValueDescriptor {
        crate::descriptor::types::ValueDescriptor::Any
    }
}

impl InterceptionContext {
    pub fn is_empty(&self) -> bool {
        self.shared.is_none() && self.private.is_none()
    }
}

impl CallContext {
    pub fn filter_for_interceptor(&self, interceptor_name: &str) -> InterceptionContext {
        InterceptionContext {
            shared: self.shared.clone(),
            private: self.interceptors.get(interceptor_name).cloned(),
        }
    }
}