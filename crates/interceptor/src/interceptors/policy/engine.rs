use crate::interceptors::policy::{
    config::{MatchExpr, PolicyApply, PolicyCondition, PolicyConfig, PolicyEffect, PolicyRule},
    error::PolicyError,
    fact::PolicyFacts,
    matcher::CompiledPolicyMatcher,
};
use actrpc_core::interception::InterceptionRequest;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct PolicyEngine {
    rules: Vec<CompiledPolicyRule>,
}

#[derive(Debug, Clone)]
struct CompiledPolicyRule {
    name: String,
    match_expr: CompiledMatchExpr,
    apply: PolicyApply,
}

#[derive(Debug, Clone)]
enum CompiledMatchExpr {
    All(Vec<CompiledMatchExpr>),
    Any(Vec<CompiledMatchExpr>),
    Condition(CompiledPolicyCondition),
}

#[derive(Debug, Clone)]
struct CompiledPolicyCondition {
    condition: PolicyCondition,
    matcher: CompiledPolicyMatcher,
}

#[derive(Debug, Clone, Default)]
pub struct PolicyDecision {
    pub matched_rules: Vec<MatchedPolicyRule>,
}

#[derive(Debug, Clone)]
pub struct MatchedPolicyRule {
    pub name: String,
    pub apply: PolicyApply,
}

impl PolicyEngine {
    pub fn compile(config: PolicyConfig) -> Result<Self, PolicyError> {
        validate_config(&config)?;

        let mut rules = Vec::with_capacity(config.rules.len());

        for rule in config.rules {
            rules.push(CompiledPolicyRule {
                name: rule.name.clone(),
                match_expr: compile_match_expr(&rule.name, rule.match_expr)?,
                apply: rule.apply,
            });
        }

        Ok(Self { rules })
    }

    pub fn evaluate(&self, request: &InterceptionRequest) -> PolicyDecision {
        let facts = PolicyFacts::new(request);
        let mut decision = PolicyDecision::default();

        for rule in &self.rules {
            if !rule.match_expr.matches(&facts) {
                continue;
            }

            decision.matched_rules.push(MatchedPolicyRule {
                name: rule.name.clone(),
                apply: rule.apply.clone(),
            });
        }

        decision
    }
}

impl CompiledMatchExpr {
    fn matches(&self, facts: &PolicyFacts<'_>) -> bool {
        match self {
            CompiledMatchExpr::All(expressions) => expressions
                .iter()
                .all(|expression| expression.matches(facts)),

            CompiledMatchExpr::Any(expressions) => expressions
                .iter()
                .any(|expression| expression.matches(facts)),

            CompiledMatchExpr::Condition(condition) => {
                let value = facts.get_string(&condition.condition.fact);
                condition.matcher.matches(value.as_deref())
            }
        }
    }
}

fn compile_match_expr(rule: &str, expr: MatchExpr) -> Result<CompiledMatchExpr, PolicyError> {
    match expr {
        MatchExpr::All { all } => Ok(CompiledMatchExpr::All(
            all.into_iter()
                .map(|expr| compile_match_expr(rule, expr))
                .collect::<Result<Vec<_>, _>>()?,
        )),

        MatchExpr::Any { any } => Ok(CompiledMatchExpr::Any(
            any.into_iter()
                .map(|expr| compile_match_expr(rule, expr))
                .collect::<Result<Vec<_>, _>>()?,
        )),

        MatchExpr::Condition { condition } => {
            let matcher = CompiledPolicyMatcher::compile(rule, &condition.matcher)?;

            Ok(CompiledMatchExpr::Condition(CompiledPolicyCondition {
                condition,
                matcher,
            }))
        }
    }
}

fn validate_config(config: &PolicyConfig) -> Result<(), PolicyError> {
    let mut names = HashSet::new();

    for rule in &config.rules {
        validate_rule(rule)?;

        if !names.insert(rule.name.clone()) {
            return Err(PolicyError::DuplicateRuleName {
                name: rule.name.clone(),
            });
        }
    }

    Ok(())
}

fn validate_rule(rule: &PolicyRule) -> Result<(), PolicyError> {
    if rule.name.trim().is_empty() {
        return Err(PolicyError::InvalidConfig {
            message: "policy rule name cannot be empty".to_owned(),
        });
    }

    validate_match_expr(&rule.name, &rule.match_expr)?;

    if rule.apply.immediate.is_empty() && rule.apply.review.is_none() {
        return Err(PolicyError::InvalidConfig {
            message: format!("policy rule {} has no effects", rule.name),
        });
    }

    for effect in &rule.apply.immediate {
        validate_effect(&rule.name, effect)?;
    }

    if let Some(review) = &rule.apply.review {
        for effect in &review.on_approve {
            validate_effect(&rule.name, effect)?;
        }

        for effect in &review.on_deny {
            validate_effect(&rule.name, effect)?;
        }
    }

    Ok(())
}

fn validate_match_expr(rule: &str, expr: &MatchExpr) -> Result<(), PolicyError> {
    match expr {
        MatchExpr::All { all } => {
            if all.is_empty() {
                return Err(PolicyError::InvalidConfig {
                    message: format!("policy rule {rule} contains empty all expression"),
                });
            }

            for expr in all {
                validate_match_expr(rule, expr)?;
            }
        }

        MatchExpr::Any { any } => {
            if any.is_empty() {
                return Err(PolicyError::InvalidConfig {
                    message: format!("policy rule {rule} contains empty any expression"),
                });
            }

            for expr in any {
                validate_match_expr(rule, expr)?;
            }
        }

        MatchExpr::Condition { condition } => {
            if condition.fact.as_str().trim().is_empty() {
                return Err(PolicyError::InvalidConfig {
                    message: format!("policy rule {rule} has empty fact path"),
                });
            }
        }
    }

    Ok(())
}

fn validate_effect(rule: &str, effect: &PolicyEffect) -> Result<(), PolicyError> {
    match effect {
        PolicyEffect::ExcludeInterceptors {
            exclude_interceptors,
        } => {
            if exclude_interceptors.names.is_empty() {
                return Err(PolicyError::InvalidConfig {
                    message: format!(
                        "policy rule {rule} has exclude_interceptors effect with empty names"
                    ),
                });
            }
        }

        PolicyEffect::RejectCall { .. } => {}
    }

    Ok(())
}
