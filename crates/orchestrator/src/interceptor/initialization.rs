use crate::{
    error::{InterceptorError, OrchestratorError},
    interceptor::InterceptorPolicy,
};
use actrpc_core::{
    InterceptorInitialization,
    action::{ActionDescriptor, ActionKind},
    descriptor::Accepts,
};
use std::collections::HashMap;

pub fn validate_action_descriptors(
    interceptor_name: &str,
    initialization: &InterceptorInitialization,
    available_actions: &HashMap<ActionKind, ActionDescriptor>,
) -> Result<(), OrchestratorError> {
    for (kind, interceptor_descriptor) in &initialization.actions {
        let Some(orchestrator_descriptor) = available_actions.get(kind) else {
            return Err(OrchestratorError::Interceptor(
                InterceptorError::UnsupportedActionDescriptor {
                    interceptor: interceptor_name.to_owned(),
                    action: kind.clone(),
                },
            ));
        };

        if !orchestrator_descriptor.accepts(interceptor_descriptor) {
            return Err(OrchestratorError::Interceptor(
                InterceptorError::ActionDescriptorMismatch {
                    interceptor: interceptor_name.to_owned(),
                    action: kind.clone(),
                },
            ));
        }
    }

    Ok(())
}

pub fn validate_phase_policy(
    interceptor_name: &str,
    policy: &InterceptorPolicy,
    initialization: &InterceptorInitialization,
) -> Result<(), OrchestratorError> {
    if !policy.outbound.is_empty() && !initialization.supports_outbound {
        return Err(OrchestratorError::Interceptor(
            InterceptorError::InvalidInitialization {
                interceptor: interceptor_name.to_owned(),
                message: "policy contains outbound actions but interceptor does not support outbound phase"
                    .to_owned(),
            },
        ));
    }

    if !policy.inbound.is_empty() && !initialization.supports_inbound {
        return Err(OrchestratorError::Interceptor(
            InterceptorError::InvalidInitialization {
                interceptor: interceptor_name.to_owned(),
                message:
                    "policy contains inbound actions but interceptor does not support inbound phase"
                        .to_owned(),
            },
        ));
    }

    for action in &policy.outbound {
        if !initialization.actions.contains_key(action) {
            return Err(OrchestratorError::Interceptor(
                InterceptorError::PolicyReferencesUndeclaredAction {
                    interceptor: interceptor_name.to_owned(),
                    action: action.clone(),
                    phase: "outbound".to_owned(),
                },
            ));
        }
    }

    for action in &policy.inbound {
        if !initialization.actions.contains_key(action) {
            return Err(OrchestratorError::Interceptor(
                InterceptorError::PolicyReferencesUndeclaredAction {
                    interceptor: interceptor_name.to_owned(),
                    action: action.clone(),
                    phase: "inbound".to_owned(),
                },
            ));
        }
    }

    Ok(())
}

pub fn validate_interceptor_registration(
    interceptor_name: &str,
    policy: &InterceptorPolicy,
    initialization: &InterceptorInitialization,
    available_actions: &HashMap<ActionKind, ActionDescriptor>,
) -> Result<(), OrchestratorError> {
    validate_action_descriptors(interceptor_name, initialization, available_actions)?;
    validate_phase_policy(interceptor_name, policy, initialization)?;
    Ok(())
}
