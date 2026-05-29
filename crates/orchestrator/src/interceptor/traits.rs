use crate::error::InterceptorRuntimeError;
use actrpc_core::{
    InterceptorInitialization,
    interception::{InterceptionRequest, InterceptionResponse},
};
use std::{future::Future, pin::Pin};

pub type InterceptorFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait Interceptor: Send + Sync {
    fn initialize<'a>(
        &'a self,
    ) -> InterceptorFuture<'a, Result<InterceptorInitialization, InterceptorRuntimeError>>
    where
        Self: 'a;

    fn intercept<'a>(
        &'a self,
        request: &'a InterceptionRequest,
    ) -> InterceptorFuture<'a, Result<InterceptionResponse, InterceptorRuntimeError>>
    where
        Self: 'a;
}
