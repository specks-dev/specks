//! CLI argument parsing with clap derive

use clap::{Parser, Subcommand};

use crate::commands::{BeadsCommands, WorktreeCommands};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Specks - From ideas to implementation via multi-agent orchestration
#[derive(Parser)]
#[command(name = "specks")]
#[command(version = VERSION)]
#[command(about = "From ideas to implementation via multi-agent orchestration")]
#[command(
    long_about = "Specks transforms ideas into working software through orchestrated LLM agents.\n\nA multi-agent suite collaborates to create structured plans and execute them to completion.\n\nPlanning and execution are invoked via Claude Code skills (/specks:planner, /specks:implementer).\n\nThe CLI provides utilities to initialize, validate, list, track progress, and integrate with beads for execution tracking."
)]
pub struct Cli {
    /// Increase output verbosity
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress non-error output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Output in JSON format
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a specks project in current directory
    ///
    /// Creates .specks/ directory with skeleton template and config.
    /// Creates .specks/ directory with skeleton template and config.
    /// Run this once in your project root to start using specks.
    #[command(
        long_about = "Initialize a specks project in current directory.\n\nCreates:\n  .specks/specks-skeleton.md  Template for new specks\n  .specks/config.toml         Project configuration\n  .specks/specks-implementation-log.md  Implementation progress tracking\n\nWith --check, performs a lightweight verification of initialization status without side effects."
    )]
    Init {
        /// Overwrite existing .specks directory
        #[arg(long, conflicts_with = "check")]
        force: bool,

        /// Check if project is initialized (no side effects)
        #[arg(long, conflicts_with = "force")]
        check: bool,
    },

    /// Validate speck structure against format conventions
    ///
    /// Checks anchors, references, metadata, and step dependencies.
    #[command(
        long_about = "Validate speck structure against format conventions.\n\nChecks:\n  - Required metadata fields (Owner, Status, Last updated)\n  - Anchor format and uniqueness\n  - Reference validity ([D01], #step-0, etc.)\n  - Step dependency cycles\n  - Cross-reference consistency"
    )]
    Validate {
        /// Speck file to validate (validates all if not specified)
        file: Option<String>,

        /// Enable strict validation mode (warnings become errors)
        #[arg(long)]
        strict: bool,
    },

    /// List all specks with summary information
    ///
    /// Shows each speck's name, status, and completion percentage.
    #[command(
        long_about = "List all specks with summary information.\n\nDisplays:\n  - Speck name (from filename)\n  - Status (draft, active, done)\n  - Progress (completed/total items)\n\nSpecks are found in .specks/ matching the naming pattern."
    )]
    List {
        /// Filter by status (draft, active, done)
        #[arg(long)]
        status: Option<String>,
    },

    /// Show detailed completion status for a speck
    ///
    /// Displays step-by-step progress with task and checkpoint counts.
    #[command(
        long_about = "Show detailed completion status for a speck.\n\nDisplays:\n  - Overall progress percentage\n  - Per-step completion (tasks, tests, checkpoints)\n  - Substep progress if present\n\nUse -v/--verbose to see individual task and checkpoint items."
    )]
    Status {
        /// Speck file to show status for
        file: String,

        /// Show individual task and checkpoint details
        #[arg(short, long)]
        verbose: bool,
    },

    /// Beads integration commands
    ///
    /// Sync steps to beads, link beads, show status, pull completion.
    #[command(
        subcommand,
        long_about = "Beads integration for two-way sync between specks and work tracking.\n\nRequires:\n  - Beads CLI (bd) installed and in PATH\n  - Beads initialized (bd init creates .beads/)\n  - Network connectivity\n\nSubcommands:\n  sync   Create beads from speck steps, write IDs back\n  link   Manually link a step to an existing bead\n  status Show execution status (complete/ready/blocked)\n  pull   Update speck checkboxes from bead completion\n\nTypical workflow:\n  1. specks beads sync specks-1.md    # Create beads\n  2. bd close <bead-id>               # Complete work\n  3. specks beads pull specks-1.md    # Update checkboxes"
    )]
    Beads(BeadsCommands),

    /// Worktree commands for isolated implementation environments
    ///
    /// Create, list, and clean up git worktrees for speck implementations.
    #[command(
        subcommand,
        long_about = "Worktree commands for isolated implementation environments.\n\nProvides git worktree integration for speck implementations:\n  - Each speck gets its own branch and worktree\n  - Isolated working directory prevents conflicts\n  - Clean up merged worktrees after PR completion\n\nSubcommands:\n  create  Create worktree and branch for a speck (optionally sync beads)\n  list    Show all active worktrees\n  cleanup Remove worktrees for merged branches\n\nTypical workflow:\n  1. specks worktree create .specks/specks-auth.md --sync-beads\n  2. (implement in worktree, create PR, merge)\n  3. specks worktree cleanup --merged"
    )]
    Worktree(WorktreeCommands),

    /// Show version information
    ///
    /// Display package version and optionally build metadata.
    #[command(
        long_about = "Show version information.\n\nBy default, displays the package version. With --verbose, also shows:\n  - Git commit hash\n  - Build date\n  - Rust compiler version\n\nUse --json for machine-readable output."
    )]
    Version {
        /// Show extended build information (commit, date, rustc version)
        #[arg(short, long)]
        verbose: bool,
    },
}

/// Get the command args for use in the application
pub fn parse() -> Cli {
    Cli::parse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }

    #[test]
    fn build_env_vars_accessible() {
        // Verify that build.rs exports are accessible via env!()
        // These will fail at compile time if build.rs doesn't set them
        let commit = env!("SPECKS_COMMIT");
        let build_date = env!("SPECKS_BUILD_DATE");
        let rustc_version = env!("SPECKS_RUSTC_VERSION");

        // Basic sanity checks - values should be non-empty
        assert!(!commit.is_empty(), "SPECKS_COMMIT should not be empty");
        assert!(
            !build_date.is_empty(),
            "SPECKS_BUILD_DATE should not be empty"
        );
        assert!(
            !rustc_version.is_empty(),
            "SPECKS_RUSTC_VERSION should not be empty"
        );

        // Build date should match YYYY-MM-DD format or be "unknown"
        if build_date != "unknown" {
            assert!(
                build_date.len() == 10 && build_date.chars().nth(4) == Some('-'),
                "SPECKS_BUILD_DATE should be YYYY-MM-DD format, got: {}",
                build_date
            );
        }
    }

    #[test]
    fn test_init_command() {
        let cli = Cli::try_parse_from(["specks", "init"]).unwrap();

        match cli.command {
            Some(Commands::Init { force, check }) => {
                assert!(!force);
                assert!(!check);
            }
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_init_command_with_force() {
        let cli = Cli::try_parse_from(["specks", "init", "--force"]).unwrap();

        match cli.command {
            Some(Commands::Init { force, check }) => {
                assert!(force);
                assert!(!check);
            }
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_init_command_with_check() {
        let cli = Cli::try_parse_from(["specks", "init", "--check"]).unwrap();

        match cli.command {
            Some(Commands::Init { force, check }) => {
                assert!(!force);
                assert!(check);
            }
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_init_check_and_force_mutually_exclusive() {
        let result = Cli::try_parse_from(["specks", "init", "--check", "--force"]);
        assert!(
            result.is_err(),
            "--check and --force should be mutually exclusive"
        );
    }

    #[test]
    fn test_validate_command() {
        let cli = Cli::try_parse_from(["specks", "validate"]).unwrap();

        match cli.command {
            Some(Commands::Validate { file, strict }) => {
                assert!(file.is_none());
                assert!(!strict);
            }
            _ => panic!("Expected Validate command"),
        }
    }

    #[test]
    fn test_validate_command_with_file() {
        let cli = Cli::try_parse_from(["specks", "validate", "specks-1.md"]).unwrap();

        match cli.command {
            Some(Commands::Validate { file, strict }) => {
                assert_eq!(file, Some("specks-1.md".to_string()));
                assert!(!strict);
            }
            _ => panic!("Expected Validate command"),
        }
    }

    #[test]
    fn test_list_command() {
        let cli = Cli::try_parse_from(["specks", "list"]).unwrap();

        match cli.command {
            Some(Commands::List { status }) => {
                assert!(status.is_none());
            }
            _ => panic!("Expected List command"),
        }
    }

    #[test]
    fn test_status_command() {
        let cli = Cli::try_parse_from(["specks", "status", "specks-1.md"]).unwrap();

        match cli.command {
            Some(Commands::Status { file, verbose }) => {
                assert_eq!(file, "specks-1.md");
                assert!(!verbose);
            }
            _ => panic!("Expected Status command"),
        }
    }

    #[test]
    fn test_version_command() {
        let cli = Cli::try_parse_from(["specks", "version"]).unwrap();

        match cli.command {
            Some(Commands::Version { verbose }) => {
                assert!(!verbose);
            }
            _ => panic!("Expected Version command"),
        }
    }

    #[test]
    fn test_global_flags() {
        let cli = Cli::try_parse_from(["specks", "--json", "--quiet", "list"]).unwrap();

        assert!(cli.json);
        assert!(cli.quiet);
    }

    #[test]
    fn test_init_help_includes_check_flag() {
        use clap::CommandFactory;
        let app = Cli::command();
        let init_subcommand = app
            .find_subcommand("init")
            .expect("init subcommand should exist");

        // Get the long_about text
        let long_about = init_subcommand
            .get_long_about()
            .expect("init should have long_about");

        // Verify --check flag is documented
        assert!(
            long_about.to_string().contains("--check"),
            "init help should document --check flag"
        );
        assert!(
            long_about.to_string().contains("without side effects"),
            "init help should explain --check has no side effects"
        );
    }
}
