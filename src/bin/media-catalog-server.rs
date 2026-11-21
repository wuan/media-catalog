use std::process::ExitCode;

use clap::Parser;

use media_catalog::server::{serve, ServeCli};

#[tokio::main]
async fn main() -> ExitCode {
    let cli = ServeCli::parse();
    match serve(cli.into_config()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}
