use crate::interceptors::policy::{
    config::{PolicyConfig, PolicyEffect, PolicyReviewSeverity},
    effect::effect_to_action,
    engine::{MatchedPolicyRule, PolicyEngine},
    error::PolicyError,
};
use actrpc_core::{
    InterceptorInitialization,
    action::{
        ActionDescriptor, ActionKind, ActionSpec, RequestedAction, RequestedActionRecord,
        ResolvedActionRecord,
    },
    interception::{InterceptionRequest, InterceptionResponse, InterceptorContinuation},
};
use actrpc_orchestrator::{
    action::actions::{
        exclude_interceptors::ExcludeInterceptors,
        reject_call::RejectCall,
        request_review::{
            REVIEW_DECISION_APPROVED, REVIEW_DECISION_DENIED, RequestReview, RequestReviewParams,
            RequestReviewResult,
        },
    },
    interceptor::{Interceptor, InterceptorFuture},
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PolicyInterceptor {
    engine: PolicyEngine,
}

impl PolicyInterceptor {
    pub fn new(config: PolicyConfig) -> Result<Self, PolicyError> {
        Ok(Self {
            engine: PolicyEngine::compile(config)?,
        })
    }
}

impl Interceptor for PolicyInterceptor {
    fn initialize<'a>(
        &'a self,
    ) -> InterceptorFuture<
        'a,
        Result<InterceptorInitialization, actrpc_orchestrator::error::InterceptorRuntimeError>,
    >
    where
        Self: 'a,
    {
        Box::pin(async move {
            Ok(InterceptorInitialization {
                supports_outbound: true,
                supports_inbound: true,
                actions: action_descriptors(),
            })
        })
    }

    fn intercept<'a>(
        &'a self,
        request: &'a InterceptionRequest,
    ) -> InterceptorFuture<
        'a,
        Result<InterceptionResponse, actrpc_orchestrator::error::InterceptorRuntimeError>,
    >
    where
        Self: 'a,
    {
        Box::pin(async move {
            self.intercept_inner(request).map_err(|error| {
                actrpc_orchestrator::error::InterceptorRuntimeError::Request {
                    message: error.to_string(),
                }
            })
        })
    }
}

impl PolicyInterceptor {
    fn intercept_inner(
        &self,
        request: &InterceptionRequest,
    ) -> Result<InterceptionResponse, PolicyError> {
        let decision = self.engine.evaluate(request);
        let review_results = collect_request_review_results(request)?;

        let missing_review_actions =
            missing_review_actions(&decision.matched_rules, &review_results)?;

        if !missing_review_actions.is_empty() {
            return Ok(InterceptionResponse {
                actions: missing_review_actions,
                continuation: InterceptorContinuation::Reinvoke,
            });
        }

        let finalized_effects = finalized_effects(&decision.matched_rules, &review_results)?;

        let mut actions = Vec::with_capacity(finalized_effects.len());

        for effect in &finalized_effects {
            actions.push(effect_to_action(effect)?);
        }

        Ok(InterceptionResponse {
            actions,
            continuation: InterceptorContinuation::Stop,
        })
    }
}

fn missing_review_actions(
    matched_rules: &[MatchedPolicyRule],
    review_results: &HashMap<String, RequestReviewResult>,
) -> Result<Vec<RequestedActionRecord>, PolicyError> {
    let mut actions = Vec::new();

    for rule in matched_rules {
        let Some(review) = &rule.apply.review else {
            continue;
        };

        if review_results.contains_key(&rule.name) {
            continue;
        }

        actions.push(request_review_action(RequestReviewParams {
            rule_name: rule.name.clone(),
            title: review.title.clone(),
            reason: review.reason.clone(),
            severity: review_severity_to_string(review.severity),
        })?);
    }

    Ok(actions)
}

fn finalized_effects(
    matched_rules: &[MatchedPolicyRule],
    review_results: &HashMap<String, RequestReviewResult>,
) -> Result<Vec<PolicyEffect>, PolicyError> {
    let mut effects = Vec::new();

    for rule in matched_rules {
        effects.extend(rule.apply.immediate.clone());

        let Some(review) = &rule.apply.review else {
            continue;
        };

        let review_result =
            review_results
                .get(&rule.name)
                .ok_or_else(|| PolicyError::InvalidReviewResult {
                    message: format!(
                        "policy rule {} requires review, but no review result was found",
                        rule.name
                    ),
                })?;

        match review_result.decision.as_str() {
            decision if decision == REVIEW_DECISION_APPROVED => {
                effects.extend(review.on_approve.clone());
            }

            decision if decision == REVIEW_DECISION_DENIED => {
                effects.extend(review.effective_on_deny());
            }

            other => {
                return Err(PolicyError::InvalidReviewResult {
                    message: format!("unknown review decision for rule {}: {other}", rule.name),
                });
            }
        }
    }

    Ok(effects)
}

fn collect_request_review_results(
    request: &InterceptionRequest,
) -> Result<HashMap<String, RequestReviewResult>, PolicyError> {
    let mut results = HashMap::new();

    for actions in request.iter_resolved_action_rounds() {
        for action in actions {
            if action.kind != RequestReview::action_kind() {
                continue;
            }

            let params = decode_review_params(action)?;
            let result = decode_review_result(action)?;

            results.insert(params.rule_name, result);
        }
    }

    Ok(results)
}

fn request_review_action(
    params: RequestReviewParams,
) -> Result<RequestedActionRecord, PolicyError> {
    RequestedAction::<RequestReview> { params }
        .try_into()
        .map_err(|source| PolicyError::ActionEncoding { source })
}

fn decode_review_params(action: &ResolvedActionRecord) -> Result<RequestReviewParams, PolicyError> {
    let Some(value) = &action.params else {
        return Err(PolicyError::InvalidReviewResult {
            message: "request_review action is missing params".to_owned(),
        });
    };

    serde_json::from_value(value.clone()).map_err(|source| PolicyError::InvalidReviewResult {
        message: format!("failed to decode request_review params: {source}"),
    })
}

fn decode_review_result(action: &ResolvedActionRecord) -> Result<RequestReviewResult, PolicyError> {
    let Ok(Some(value)) = &action.result else {
        return Err(PolicyError::InvalidReviewResult {
            message: "request_review action did not resolve successfully".to_owned(),
        });
    };

    serde_json::from_value(value.clone()).map_err(|source| PolicyError::InvalidReviewResult {
        message: format!("failed to decode request_review result: {source}"),
    })
}

fn review_severity_to_string(severity: PolicyReviewSeverity) -> String {
    match severity {
        PolicyReviewSeverity::Low => "low".to_owned(),
        PolicyReviewSeverity::Medium => "medium".to_owned(),
        PolicyReviewSeverity::High => "high".to_owned(),
    }
}

fn action_descriptors() -> HashMap<ActionKind, ActionDescriptor> {
    actrpc_core::action::action_descriptor_map!(RejectCall, ExcludeInterceptors, RequestReview)
}
