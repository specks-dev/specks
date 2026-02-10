//! specks CLI - From ideas to implementation via multi-agent orchestration

mod cli;
mod commands;
mod output;
mod splash;

use std::process::ExitCode;

use cli::Commands;
use commands::{BeadsCommands, LogCommands, WorktreeCommands};

fn main() -> ExitCode {
    let cli = cli::parse();

    let result = match cli.command {
        Some(Commands::Init { force, check }) => {
            commands::run_init(force, check, cli.json, cli.quiet)
        }
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
            BeadsCommands::Close { bead_id, reason } => {
                commands::run_close(bead_id, reason, cli.json, cli.quiet)
            }
        },
        Some(Commands::Worktree(worktree_cmd)) => match worktree_cmd {
            WorktreeCommands::Create {
                speck,
                base,
                sync_beads,
                reuse_existing,
            } => commands::run_worktree_create(speck, base, sync_beads, reuse_existing, cli.json, cli.quiet),
            WorktreeCommands::List => commands::run_worktree_list(cli.json, cli.quiet),
            WorktreeCommands::Cleanup { merged, dry_run } => {
                commands::run_worktree_cleanup(merged, dry_run, cli.json, cli.quiet)
            }
        },
        Some(Commands::Merge {
            speck,
            dry_run,
            force,
        }) => commands::run_merge(speck, dry_run, force, cli.json, cli.quiet),
        Some(Commands::Log(log_cmd)) => match log_cmd {
            LogCommands::Rotate { force } => commands::run_log_rotate(force, cli.json, cli.quiet),
            LogCommands::Prepend {
                step,
                speck,
                summary,
                bead,
            } => commands::run_log_prepend(step, speck, summary, bead, cli.json, cli.quiet),
        },
        Some(Commands::Doctor) => commands::run_doctor(cli.json, cli.quiet),
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
