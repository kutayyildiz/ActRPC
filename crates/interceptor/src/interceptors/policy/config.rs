//! Policy Interceptor config.
//!
//! Primary documented configuration format: **YAML**.
//!
//! # Example: simple review
//!
//! ```yaml
#![doc = include_str!("config/examples/simple_review.yaml")]
//! ```
//!
//! # Example: block sensitive write
//!
//! ```yaml
#![doc = include_str!("config/examples/block_sensitive_write.yaml")]
//! ```
//!
//! # Example: exclude loggers
//!
//! ```yaml
#![doc = include_str!("config/examples/exclude_loggers.yaml")]
//! ```

mod config;
mod loader;

pub use config::*;
pub use loader::*;
