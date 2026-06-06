extern crate self as actrpc_core;

mod convert;
mod interceptor_capabilities;
mod interceptor_initialization;

pub mod action;
pub mod descriptor;
pub mod error;
pub mod interception;
pub mod json_rpc;
pub mod participant;

pub use convert::{
    ACTRPC_INTERCEPTOR_INITIALIZE_METHOD, ACTRPC_INTERCEPTOR_INTERCEPT_METHOD,
    ACTRPC_METHOD_PROVIDER_CHANGED_METHOD, ACTRPC_METHOD_PROVIDER_INITIALIZE_METHOD,
    ACTRPC_METHOD_PROVIDER_REFRESH_METHOD, INTERCEPT_METHOD,
};

pub use actrpc_core_macros::{DescribeOk, DescribeParams, DescribeValue};

pub use interceptor_capabilities::InterceptorCapabilities;
pub use interceptor_initialization::InterceptorInitialization;
