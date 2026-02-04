//! specks CLI - Agent-centric technical specifications

mod cli;
mod commands;
mod output;

use std::process::ExitCode;

use cli::Commands;

fn main() -> ExitCode {
    let cli = cli::parse();

    let result = match cli.command {
        Some(Commands::Init { force }) => {
            commands::run_init(force, cli.json, cli.quiet)
        }
        Some(Commands::Validate { file, strict }) => {
            commands::run_validate(file, strict, cli.json, cli.quiet)
        }
        Some(Commands::List { status }) => {
            commands::run_list(status, cli.json, cli.quiet)
        }
        Some(Commands::Status { file, verbose }) => {
            // Use verbose flag from subcommand, or global verbose
            let verbose = verbose || cli.verbose;
            commands::run_status(file, verbose, cli.json, cli.quiet)
        }
        None => {
            // No subcommand - print version info
            if !cli.quiet {
                println!("specks v{}", env!("CARGO_PKG_VERSION"));
                println!("Use --help for usage information");
            }
            Ok(0)
        }
    };

    match result {
        Ok(code) => ExitCode::from(code as u8),
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        crate::cli::Cli::command().debug_assert();
    }
}
