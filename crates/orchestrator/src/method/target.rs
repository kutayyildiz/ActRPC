use actrpc_core::MethodTarget;

use super::{MethodName, ProviderName};

pub fn method_target_from_names(provider: &ProviderName, method: &MethodName) -> MethodTarget {
    MethodTarget {
        provider: provider.as_str().to_owned(),
        method: method.as_str().to_owned(),
    }
}
