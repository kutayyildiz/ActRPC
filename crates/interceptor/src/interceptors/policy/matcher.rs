use crate::interceptors::policy::{
    config::{PolicyMatcher, PolicyMatcherKind},
    error::PolicyError,
};
use globset::{Glob, GlobMatcher};
use regex::Regex;

#[derive(Debug, Clone)]
pub struct CompiledPolicyMatcher {
    negated: bool,
    inner: CompiledPolicyMatcherInner,
}

#[derive(Debug, Clone)]
enum CompiledPolicyMatcherInner {
    Exact(String),
    Glob(GlobMatcher),
    Regex(Regex),
}

impl CompiledPolicyMatcher {
    pub fn compile(rule: &str, matcher: &PolicyMatcher) -> Result<Self, PolicyError> {
        let inner = match matcher.kind {
            PolicyMatcherKind::Exact => CompiledPolicyMatcherInner::Exact(matcher.value.clone()),

            PolicyMatcherKind::Glob => {
                let glob =
                    Glob::new(&matcher.value).map_err(|source| PolicyError::InvalidGlob {
                        rule: rule.to_owned(),
                        source,
                    })?;

                CompiledPolicyMatcherInner::Glob(glob.compile_matcher())
            }

            PolicyMatcherKind::Regex => {
                let regex =
                    Regex::new(&matcher.value).map_err(|source| PolicyError::InvalidRegex {
                        rule: rule.to_owned(),
                        source,
                    })?;

                CompiledPolicyMatcherInner::Regex(regex)
            }
        };

        Ok(Self {
            negated: matcher.negated,
            inner,
        })
    }

    pub fn matches(&self, value: Option<&str>) -> bool {
        let matched = match value {
            Some(value) => match &self.inner {
                CompiledPolicyMatcherInner::Exact(expected) => value == expected,
                CompiledPolicyMatcherInner::Glob(glob) => glob.is_match(value),
                CompiledPolicyMatcherInner::Regex(regex) => regex.is_match(value),
            },
            None => false,
        };

        if self.negated { !matched } else { matched }
    }
}
