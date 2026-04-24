extern crate self as actrpc_core;

mod convert;
mod interceptor_capabilities;
mod interceptor_initialization;

pub mod action;
pub mod error;
pub mod interception;
pub mod json_rpc;
pub mod participant;

pub use convert::INTERCEPT_METHOD;

pub use actrpc_core_macros::{DescribeOk, DescribeParams, DescribeValue};

pub use interceptor_capabilities::InterceptorCapabilities;
pub use interceptor_initialization::InterceptorInitialization;
