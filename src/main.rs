use std::process::ExitCode;

use clap::Parser;

use media_catalog::config::Cli;
use media_catalog::run;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(outcome) => {
            println!("{}", outcome.summary.render());
            if outcome.summary.has_errors() || outcome.blocked {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}
