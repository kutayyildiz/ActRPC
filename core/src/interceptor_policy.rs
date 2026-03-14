use serde::{Deserialize, Serialize};

use crate::action::Action;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterceptorPolicy {
    pub interceptor_name: String,
    pub allowed_actions: Vec<Action>,
}
