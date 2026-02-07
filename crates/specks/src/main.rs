//! specks CLI - From ideas to implementation via multi-agent orchestration

mod cli;
mod commands;
mod output;
mod splash;

use std::process::ExitCode;

use cli::Commands;
use commands::BeadsCommands;

fn main() -> ExitCode {
    let cli = cli::parse();

    let result = match cli.command {
        Some(Commands::Init { force }) => commands::run_init(force, cli.json, cli.quiet),
        Some(Commands::Validate { file, strict }) => {
            commands::run_validate(file, strict, cli.json, cli.quiet)
        }
        Some(Commands::List { status }) => commands::run_list(status, cli.json, cli.quiet),
        Some(Commands::Status { file, verbose }) => {
            // Use verbose flag from subcommand, or global verbose
            let verbose = verbose || cli.verbose;
            commands::run_status(file, verbose, cli.json, cli.quiet)
        }
        Some(Commands::Beads(beads_cmd)) => match beads_cmd {
            BeadsCommands::Sync {
                file,
                dry_run,
                update_title,
                update_body,
                prune_deps,
                substeps,
            } => commands::run_sync(commands::beads::sync::SyncOptions {
                file,
                dry_run,
                update_title,
                update_body,
                prune_deps,
                substeps_mode: substeps,
                json_output: cli.json,
                quiet: cli.quiet,
            }),
            BeadsCommands::Link {
                file,
                step_anchor,
                bead_id,
            } => commands::run_link(file, step_anchor, bead_id, cli.json, cli.quiet),
            BeadsCommands::Status { file, pull } => {
                commands::run_beads_status(file, pull, cli.json, cli.quiet)
            }
            BeadsCommands::Pull { file, no_overwrite } => {
                commands::run_pull(file, no_overwrite, cli.json, cli.quiet)
            }
        },
        Some(Commands::Version { verbose }) => commands::run_version(verbose, cli.json, cli.quiet),
        None => {
            // No subcommand - show splash screen
            if !cli.quiet {
                splash::show_splash();
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
