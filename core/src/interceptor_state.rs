use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterceptorState {
    pub interceptor_name: String,
    pub enabled: bool,
    pub executed: bool,
}
