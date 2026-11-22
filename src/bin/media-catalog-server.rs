use std::process::ExitCode;

use clap::Parser;

use media_catalog::server::{serve, ServeCli};

#[tokio::main]
async fn main() -> ExitCode {
    let cli = ServeCli::parse();
    let config = match cli.into_config() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("configuration error: {error:#}");
            return ExitCode::FAILURE;
        }
    };

    match serve(config).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}
