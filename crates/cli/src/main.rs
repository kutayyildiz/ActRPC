mod args;
mod cli_params;
mod cli_review_provider;

use crate::{
    args::{CliArgs, CliCommand},
    cli_params::parse_params,
    cli_review_provider::CliReviewProvider,
};
use actrpc_orchestrator::{
    config::OrchestratorConfig,
    method::{MethodName, ProviderName},
    runtime::{CallExecutionFactory, DefaultOrchestrator},
};
use actrpc_transport::DefaultJsonRpcClientProvider;
use clap::Parser;
use std::{error::Error, io, sync::Arc};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let args = CliArgs::parse();

    let config = OrchestratorConfig::from_paths(&args.configs)?;

    let provider = DefaultJsonRpcClientProvider::new();

    let resources = config
        .build_resources(&provider, Arc::new(CliReviewProvider))
        .await?;

    let orchestrator =
        DefaultOrchestrator::new(Arc::new(CallExecutionFactory::new(Arc::new(resources))));

    match args.command {
        CliCommand::Call {
            provider,
            method,
            params,
        } => {
            let params = parse_params(params)
                .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;

            let result = orchestrator
                .call(
                    ProviderName::from(provider),
                    MethodName::from(method),
                    params,
                )
                .await?;

            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }

    Ok(())
}
