//! CLI argument parsing with clap derive

use clap::{Parser, Subcommand};

use crate::commands::BeadsCommands;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Specks - From ideas to implementation via multi-agent orchestration
#[derive(Parser)]
#[command(name = "specks")]
#[command(version = VERSION)]
#[command(about = "From ideas to implementation via multi-agent orchestration")]
#[command(
    long_about = "Specks transforms ideas into working software through orchestrated LLM agents.\n\nA multi-agent suite (director, planner, critic, architect, implementer, monitor, reviewer, auditor, logger, committer) collaborates to create structured plans and execute them to completion.\n\nThe CLI provides utilities to validate, list, track progress, and integrate with beads for execution tracking."
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
    /// Creates .specks/ directory with skeleton template, config, and runs directory.
    /// Run this once in your project root to start using specks.
    #[command(
        long_about = "Initialize a specks project in current directory.\n\nCreates:\n  .specks/specks-skeleton.md  Template for new specks\n  .specks/config.toml         Project configuration\n  .specks/runs/               Agent run artifacts (gitignored)"
    )]
    Init {
        /// Overwrite existing .specks directory
        #[arg(long)]
        force: bool,
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

    /// Create or revise a speck through iterative agent collaboration
    ///
    /// Runs an interactive planning loop: interviewer gathers input, planner creates
    /// the speck, critic reviews, and you approve or request revisions.
    #[command(
        long_about = "Create or revise a speck through iterative agent collaboration.\n\nWorkflow:\n  1. Interviewer gathers requirements from you\n  2. Planner creates or revises the speck\n  3. Critic reviews for quality and compliance\n  4. Interviewer presents results and asks: ready or revise?\n  5. Loop continues until you approve\n\nModes:\n  - New: Provide an idea string to create a new speck\n  - Revision: Provide a path to an existing speck to revise it\n  - Interactive: Omit input to be prompted for an idea\n\nRequires Claude Code CLI (claude) to be installed."
    )]
    Plan {
        /// Either an idea string OR path to existing speck for revision
        ///
        /// If a file path ending in .md exists, enters revision mode.
        /// Otherwise, treats the input as an idea for a new speck.
        input: Option<String>,

        /// Name for the speck file (default: auto-generated from idea)
        #[arg(long)]
        name: Option<String>,

        /// Additional context files to include (can be repeated)
        #[arg(long = "context", short = 'c')]
        context_files: Vec<String>,

        /// Timeout per agent invocation in seconds (default: 300)
        #[arg(long, default_value = "300")]
        timeout: u64,
    },

    /// Setup specks integrations
    ///
    /// Configure external tool integrations like Claude Code.
    #[command(
        subcommand,
        long_about = "Setup specks integrations with external tools.\n\nSubcommands:\n  claude   Install Claude Code skills for /specks-plan and /specks-execute\n\nSkills are pre-packaged configurations that enable specks functionality\nwithin Claude Code sessions. They must be installed to each project\nwhere you want to use the slash commands."
    )]
    Setup(SetupCommands),

    /// Execute a speck's steps through agent-driven implementation
    ///
    /// Runs the director agent to implement steps in the speck. Each step goes
    /// through architect, implementer, reviewer, auditor, and committer agents.
    #[command(
        long_about = "Execute a speck's steps through agent-driven implementation.\n\nWorkflow per step:\n  1. Architect creates implementation strategy\n  2. Implementer writes code (with Monitor watching)\n  3. Reviewer checks plan adherence\n  4. Auditor verifies code quality\n  5. Logger updates implementation log\n  6. Committer prepares/creates commit\n\nOptions:\n  --start-step   Begin from specific step (default: first ready)\n  --end-step     Stop after specific step (default: all)\n  --commit-policy  manual = prompt before commit (default)\n                   auto = commit automatically\n  --checkpoint-mode  step = pause after each step (default)\n                     milestone = pause at milestones only\n                     continuous = no pauses\n  --dry-run      Show execution plan without running\n\nRequires Claude Code CLI (claude) to be installed.\nSpeck must have Status = active in Plan Metadata."
    )]
    Execute {
        /// Path to speck file to execute
        speck: String,

        /// Step anchor to start from (default: first ready step)
        #[arg(long)]
        start_step: Option<String>,

        /// Step anchor to stop after (default: all steps)
        #[arg(long)]
        end_step: Option<String>,

        /// Commit policy: manual or auto (default: manual)
        #[arg(long, default_value = "manual")]
        commit_policy: String,

        /// Checkpoint mode: step, milestone, or continuous (default: step)
        #[arg(long, default_value = "step")]
        checkpoint_mode: String,

        /// Show what would be executed without doing it
        #[arg(long)]
        dry_run: bool,

        /// Timeout per step in seconds (default: 600)
        #[arg(long, default_value = "600")]
        timeout: u64,
    },
}

/// Setup subcommands
#[derive(Subcommand)]
pub enum SetupCommands {
    /// Install Claude Code skills for slash commands
    ///
    /// Installs /specks-plan and /specks-execute skills to the current project.
    /// These skills enable using specks from within Claude Code sessions.
    #[command(
        long_about = "Install Claude Code skills for specks slash commands.\n\nThis command copies skill files from the specks distribution to your\nproject's .claude/skills/ directory. Once installed, you can use:\n\n  /specks-plan \"idea\"           Create a new speck from an idea\n  /specks-plan path/to/speck.md Revise an existing speck\n  /specks-execute path/to/speck Execute a speck's steps\n\nOptions:\n  --check   Verify installation status without making changes\n  --force   Overwrite existing skills even if unchanged\n\nSkill source locations (in order of precedence):\n  1. SPECKS_SHARE_DIR environment variable\n  2. ../share/specks/ relative to the specks binary\n  3. /opt/homebrew/share/specks/ (macOS ARM)\n  4. /usr/local/share/specks/ (macOS x86_64)\n\nIf skills are missing or share directory not found, verify your\nspecks installation or set SPECKS_SHARE_DIR explicitly."
    )]
    Claude {
        /// Verify installation status without installing
        #[arg(long)]
        check: bool,

        /// Overwrite existing skills even if unchanged
        #[arg(long)]
        force: bool,
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
    fn test_plan_command_parses_idea() {
        // Test parsing plan command with an idea string
        let cli = Cli::try_parse_from(["specks", "plan", "add a new feature"]).unwrap();

        match cli.command {
            Some(Commands::Plan {
                input,
                name,
                context_files,
                timeout,
            }) => {
                assert_eq!(input, Some("add a new feature".to_string()));
                assert!(name.is_none());
                assert!(context_files.is_empty());
                assert_eq!(timeout, 300);
            }
            _ => panic!("Expected Plan command"),
        }
    }

    #[test]
    fn test_plan_command_with_name() {
        let cli =
            Cli::try_parse_from(["specks", "plan", "add feature", "--name", "my-feature"]).unwrap();

        match cli.command {
            Some(Commands::Plan { input, name, .. }) => {
                assert_eq!(input, Some("add feature".to_string()));
                assert_eq!(name, Some("my-feature".to_string()));
            }
            _ => panic!("Expected Plan command"),
        }
    }

    #[test]
    fn test_plan_command_with_context_files() {
        let cli = Cli::try_parse_from([
            "specks",
            "plan",
            "add feature",
            "-c",
            "context1.md",
            "--context",
            "context2.md",
        ])
        .unwrap();

        match cli.command {
            Some(Commands::Plan { context_files, .. }) => {
                assert_eq!(context_files.len(), 2);
                assert_eq!(context_files[0], "context1.md");
                assert_eq!(context_files[1], "context2.md");
            }
            _ => panic!("Expected Plan command"),
        }
    }

    #[test]
    fn test_plan_command_with_timeout() {
        let cli =
            Cli::try_parse_from(["specks", "plan", "add feature", "--timeout", "600"]).unwrap();

        match cli.command {
            Some(Commands::Plan { timeout, .. }) => {
                assert_eq!(timeout, 600);
            }
            _ => panic!("Expected Plan command"),
        }
    }

    #[test]
    fn test_plan_command_no_input() {
        // Plan command should work without input (will prompt or error at runtime)
        let cli = Cli::try_parse_from(["specks", "plan"]).unwrap();

        match cli.command {
            Some(Commands::Plan { input, .. }) => {
                assert!(input.is_none());
            }
            _ => panic!("Expected Plan command"),
        }
    }

    #[test]
    fn test_plan_command_with_global_flags() {
        let cli =
            Cli::try_parse_from(["specks", "--json", "--quiet", "plan", "add feature"]).unwrap();

        assert!(cli.json);
        assert!(cli.quiet);
        match cli.command {
            Some(Commands::Plan { input, .. }) => {
                assert_eq!(input, Some("add feature".to_string()));
            }
            _ => panic!("Expected Plan command"),
        }
    }

    #[test]
    fn test_setup_claude_command() {
        let cli = Cli::try_parse_from(["specks", "setup", "claude"]).unwrap();

        match cli.command {
            Some(Commands::Setup(SetupCommands::Claude { check, force })) => {
                assert!(!check);
                assert!(!force);
            }
            _ => panic!("Expected Setup Claude command"),
        }
    }

    #[test]
    fn test_setup_claude_with_check() {
        let cli = Cli::try_parse_from(["specks", "setup", "claude", "--check"]).unwrap();

        match cli.command {
            Some(Commands::Setup(SetupCommands::Claude { check, force })) => {
                assert!(check);
                assert!(!force);
            }
            _ => panic!("Expected Setup Claude command"),
        }
    }

    #[test]
    fn test_setup_claude_with_force() {
        let cli = Cli::try_parse_from(["specks", "setup", "claude", "--force"]).unwrap();

        match cli.command {
            Some(Commands::Setup(SetupCommands::Claude { check, force })) => {
                assert!(!check);
                assert!(force);
            }
            _ => panic!("Expected Setup Claude command"),
        }
    }

    #[test]
    fn test_setup_claude_with_global_flags() {
        let cli = Cli::try_parse_from(["specks", "--json", "setup", "claude", "--check"]).unwrap();

        assert!(cli.json);
        match cli.command {
            Some(Commands::Setup(SetupCommands::Claude { check, force })) => {
                assert!(check);
                assert!(!force);
            }
            _ => panic!("Expected Setup Claude command"),
        }
    }

    #[test]
    fn test_execute_command_basic() {
        let cli = Cli::try_parse_from(["specks", "execute", ".specks/specks-1.md"]).unwrap();

        match cli.command {
            Some(Commands::Execute {
                speck,
                start_step,
                end_step,
                commit_policy,
                checkpoint_mode,
                dry_run,
                timeout,
            }) => {
                assert_eq!(speck, ".specks/specks-1.md");
                assert!(start_step.is_none());
                assert!(end_step.is_none());
                assert_eq!(commit_policy, "manual");
                assert_eq!(checkpoint_mode, "step");
                assert!(!dry_run);
                assert_eq!(timeout, 600);
            }
            _ => panic!("Expected Execute command"),
        }
    }

    #[test]
    fn test_execute_command_with_step_range() {
        let cli = Cli::try_parse_from([
            "specks",
            "execute",
            ".specks/specks-1.md",
            "--start-step",
            "#step-1",
            "--end-step",
            "#step-3",
        ])
        .unwrap();

        match cli.command {
            Some(Commands::Execute {
                start_step,
                end_step,
                ..
            }) => {
                assert_eq!(start_step, Some("#step-1".to_string()));
                assert_eq!(end_step, Some("#step-3".to_string()));
            }
            _ => panic!("Expected Execute command"),
        }
    }

    #[test]
    fn test_execute_command_with_policies() {
        let cli = Cli::try_parse_from([
            "specks",
            "execute",
            ".specks/specks-1.md",
            "--commit-policy",
            "auto",
            "--checkpoint-mode",
            "milestone",
        ])
        .unwrap();

        match cli.command {
            Some(Commands::Execute {
                commit_policy,
                checkpoint_mode,
                ..
            }) => {
                assert_eq!(commit_policy, "auto");
                assert_eq!(checkpoint_mode, "milestone");
            }
            _ => panic!("Expected Execute command"),
        }
    }

    #[test]
    fn test_execute_command_dry_run() {
        let cli =
            Cli::try_parse_from(["specks", "execute", ".specks/specks-1.md", "--dry-run"]).unwrap();

        match cli.command {
            Some(Commands::Execute { dry_run, .. }) => {
                assert!(dry_run);
            }
            _ => panic!("Expected Execute command"),
        }
    }

    #[test]
    fn test_execute_command_with_timeout() {
        let cli = Cli::try_parse_from([
            "specks",
            "execute",
            ".specks/specks-1.md",
            "--timeout",
            "1200",
        ])
        .unwrap();

        match cli.command {
            Some(Commands::Execute { timeout, .. }) => {
                assert_eq!(timeout, 1200);
            }
            _ => panic!("Expected Execute command"),
        }
    }

    #[test]
    fn test_execute_command_with_global_flags() {
        let cli = Cli::try_parse_from([
            "specks",
            "--json",
            "--quiet",
            "execute",
            ".specks/specks-1.md",
        ])
        .unwrap();

        assert!(cli.json);
        assert!(cli.quiet);
        match cli.command {
            Some(Commands::Execute { speck, .. }) => {
                assert_eq!(speck, ".specks/specks-1.md");
            }
            _ => panic!("Expected Execute command"),
        }
    }
}
