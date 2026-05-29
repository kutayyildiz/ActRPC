pub mod config;

mod effect;
mod engine;
mod error;
mod fact;
mod interceptor;
mod matcher;

pub use effect::effect_to_action;
pub use engine::{PolicyDecision, PolicyEngine};
pub use error::PolicyError;
pub use fact::PolicyFacts;
pub use interceptor::PolicyInterceptor;
pub use matcher::CompiledPolicyMatcher;
