use actrpc_core::json_rpc::JsonRpcError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PolicyConfig {
    #[serde(default)]
    pub rules: Vec<PolicyRule>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyRule {
    pub name: String,
    pub match_expr: MatchExpr,

    #[serde(default)]
    pub apply: PolicyApply,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PolicyApply {
    #[serde(default)]
    pub immediate: Vec<PolicyEffect>,

    #[serde(default)]
    pub review: Option<PolicyReview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyReview {
    pub title: String,
    pub reason: String,
    pub severity: PolicyReviewSeverity,

    #[serde(default)]
    pub on_approve: Vec<PolicyEffect>,

    #[serde(default)]
    pub on_deny: Vec<PolicyEffect>,
}

impl PolicyReview {
    pub fn effective_on_deny(&self) -> Vec<PolicyEffect> {
        if self.on_deny.is_empty() {
            vec![PolicyEffect::reject_call(JsonRpcError {
                code: -32050,
                message: "operation denied by user review".to_owned(),
                data: None,
            })]
        } else {
            self.on_deny.clone()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyReviewSeverity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, untagged)]
pub enum MatchExpr {
    All { all: Vec<MatchExpr> },
    Any { any: Vec<MatchExpr> },
    Condition { condition: PolicyCondition },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyCondition {
    pub fact: PolicyFactPath,
    pub matcher: PolicyMatcher,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PolicyFactPath(String);

impl PolicyFactPath {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl From<String> for PolicyFactPath {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for PolicyFactPath {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyMatcher {
    pub kind: PolicyMatcherKind,
    pub value: String,

    #[serde(default)]
    pub negated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyMatcherKind {
    Exact,
    Glob,
    Regex,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, untagged)]
pub enum PolicyEffect {
    ExcludeInterceptors {
        exclude_interceptors: ExcludeInterceptorsEffect,
    },
    RejectCall {
        reject_call: RejectCallEffect,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExcludeInterceptorsEffect {
    pub names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RejectCallEffect {
    pub error: JsonRpcError,
}

impl PolicyEffect {
    pub fn exclude_interceptors(names: Vec<String>) -> Self {
        Self::ExcludeInterceptors {
            exclude_interceptors: ExcludeInterceptorsEffect { names },
        }
    }

    pub fn reject_call(error: JsonRpcError) -> Self {
        Self::RejectCall {
            reject_call: RejectCallEffect { error },
        }
    }
}

impl MatchExpr {
    pub fn all(expressions: impl Into<Vec<MatchExpr>>) -> Self {
        Self::All {
            all: expressions.into(),
        }
    }

    pub fn any(expressions: impl Into<Vec<MatchExpr>>) -> Self {
        Self::Any {
            any: expressions.into(),
        }
    }

    pub fn condition(fact: impl Into<PolicyFactPath>, matcher: PolicyMatcher) -> Self {
        Self::Condition {
            condition: PolicyCondition {
                fact: fact.into(),
                matcher,
            },
        }
    }
}

impl PolicyMatcher {
    pub fn exact(value: impl Into<String>) -> Self {
        Self {
            kind: PolicyMatcherKind::Exact,
            value: value.into(),
            negated: false,
        }
    }

    pub fn exact_not(value: impl Into<String>) -> Self {
        Self {
            kind: PolicyMatcherKind::Exact,
            value: value.into(),
            negated: true,
        }
    }

    pub fn glob(value: impl Into<String>) -> Self {
        Self {
            kind: PolicyMatcherKind::Glob,
            value: value.into(),
            negated: false,
        }
    }

    pub fn glob_not(value: impl Into<String>) -> Self {
        Self {
            kind: PolicyMatcherKind::Glob,
            value: value.into(),
            negated: true,
        }
    }

    pub fn regex(value: impl Into<String>) -> Self {
        Self {
            kind: PolicyMatcherKind::Regex,
            value: value.into(),
            negated: false,
        }
    }

    pub fn regex_not(value: impl Into<String>) -> Self {
        Self {
            kind: PolicyMatcherKind::Regex,
            value: value.into(),
            negated: true,
        }
    }
}
