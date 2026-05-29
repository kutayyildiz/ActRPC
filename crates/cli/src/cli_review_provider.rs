use actrpc_orchestrator::{
    action::{
        ActionHandlerFuture,
        actions::request_review::{
            REVIEW_DECISION_APPROVED, REVIEW_DECISION_DENIED, RequestReviewParams,
            RequestReviewResult,
        },
    },
    error::ActionExecutionError,
    review::ReviewProvider,
};
use std::io::{self, Write};

#[derive(Debug, Default)]
pub struct CliReviewProvider;

impl ReviewProvider for CliReviewProvider {
    fn request_review<'a>(
        &'a self,
        params: RequestReviewParams,
    ) -> ActionHandlerFuture<'a, Result<RequestReviewResult, ActionExecutionError>> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || request_review_blocking(params))
                .await
                .map_err(|source| ActionExecutionError::InvalidState {
                    message: format!("review prompt task failed: {source}"),
                })?
        })
    }
}

fn request_review_blocking(
    params: RequestReviewParams,
) -> Result<RequestReviewResult, ActionExecutionError> {
    eprintln!();
    eprintln!("Review required");
    eprintln!("Rule: {}", params.rule_name);
    eprintln!("Title: {}", params.title);
    eprintln!("Severity: {}", params.severity);
    eprintln!("Reason: {}", params.reason);
    eprintln!();

    loop {
        eprint!("Approve? [y/N]: ");

        io::stderr()
            .flush()
            .map_err(|source| ActionExecutionError::InvalidState {
                message: format!("failed to flush review prompt: {source}"),
            })?;

        let mut input = String::new();

        io::stdin()
            .read_line(&mut input)
            .map_err(|source| ActionExecutionError::InvalidState {
                message: format!("failed to read review response: {source}"),
            })?;

        match parse_review_decision_input(&input) {
            Some(decision) => return Ok(RequestReviewResult { decision }),
            None => eprintln!("Please answer y or n."),
        }
    }
}

fn parse_review_decision_input(input: &str) -> Option<String> {
    match input.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" => Some(REVIEW_DECISION_APPROVED.to_owned()),
        "" | "n" | "no" => Some(REVIEW_DECISION_DENIED.to_owned()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actrpc_orchestrator::action::actions::request_review::{
        REVIEW_DECISION_APPROVED, REVIEW_DECISION_DENIED,
    };

    #[test]
    fn parse_review_decision_accepts_yes_variants() {
        assert_eq!(
            parse_review_decision_input("y"),
            Some(REVIEW_DECISION_APPROVED.to_owned())
        );
        assert_eq!(
            parse_review_decision_input("yes"),
            Some(REVIEW_DECISION_APPROVED.to_owned())
        );
        assert_eq!(
            parse_review_decision_input(" YES "),
            Some(REVIEW_DECISION_APPROVED.to_owned())
        );
    }

    #[test]
    fn parse_review_decision_accepts_no_variants_and_empty_default() {
        assert_eq!(
            parse_review_decision_input("n"),
            Some(REVIEW_DECISION_DENIED.to_owned())
        );
        assert_eq!(
            parse_review_decision_input("no"),
            Some(REVIEW_DECISION_DENIED.to_owned())
        );
        assert_eq!(
            parse_review_decision_input(""),
            Some(REVIEW_DECISION_DENIED.to_owned())
        );
        assert_eq!(
            parse_review_decision_input("   "),
            Some(REVIEW_DECISION_DENIED.to_owned())
        );
    }

    #[test]
    fn parse_review_decision_rejects_unknown_input() {
        assert_eq!(parse_review_decision_input("maybe"), None);
        assert_eq!(parse_review_decision_input("approve"), None);
        assert_eq!(parse_review_decision_input("deny"), None);
    }
}
