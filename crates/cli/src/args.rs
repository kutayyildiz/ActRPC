use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser, PartialEq)]
#[command(name = "actrpc")]
pub struct CliArgs {
    #[arg(short, long = "config", required = true)]
    pub configs: Vec<PathBuf>,

    #[command(subcommand)]
    pub command: CliCommand,
}

#[derive(Debug, Subcommand, PartialEq)]
pub enum CliCommand {
    Call {
        provider: String,
        method: String,

        #[arg(long)]
        params: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn parses_call_command_with_one_config_and_params() {
        let args = CliArgs::parse_from([
            "actrpc",
            "--config",
            "actrpc.yaml",
            "call",
            "filesystem",
            "write_file",
            "--params",
            r#"{"path":"test.txt"}"#,
        ]);

        assert_eq!(
            args,
            CliArgs {
                configs: vec![PathBuf::from("actrpc.yaml")],
                command: CliCommand::Call {
                    provider: "filesystem".to_owned(),
                    method: "write_file".to_owned(),
                    params: Some(r#"{"path":"test.txt"}"#.to_owned()),
                },
            }
        );
    }

    #[test]
    fn parses_call_command_with_multiple_configs() {
        let args = CliArgs::parse_from([
            "actrpc",
            "--config",
            "base.yaml",
            "--config",
            "local.yaml",
            "call",
            "math",
            "sum",
        ]);

        assert_eq!(
            args,
            CliArgs {
                configs: vec![PathBuf::from("base.yaml"), PathBuf::from("local.yaml")],
                command: CliCommand::Call {
                    provider: "math".to_owned(),
                    method: "sum".to_owned(),
                    params: None,
                },
            }
        );
    }

    #[test]
    fn rejects_missing_config() {
        let result = CliArgs::try_parse_from(["actrpc", "call", "math", "sum"]);

        assert!(result.is_err());
    }

    #[test]
    fn rejects_missing_subcommand() {
        let result = CliArgs::try_parse_from(["actrpc", "--config", "actrpc.yaml"]);

        assert!(result.is_err());
    }

    #[test]
    fn rejects_missing_provider() {
        let result = CliArgs::try_parse_from(["actrpc", "--config", "actrpc.yaml", "call"]);

        assert!(result.is_err());
    }

    #[test]
    fn rejects_missing_method() {
        let result = CliArgs::try_parse_from(["actrpc", "--config", "actrpc.yaml", "call", "math"]);

        assert!(result.is_err());
    }
}
