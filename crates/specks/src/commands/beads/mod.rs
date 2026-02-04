//! Beads integration commands (Specs S06-S09)
//!
//! Provides subcommands for syncing specks to beads, linking steps to beads,
//! showing beads execution status, and pulling bead completion back to checkboxes.

pub mod link;
pub mod pull;
pub mod status;
pub mod sync;

use clap::Subcommand;

pub use link::run_link;
pub use pull::run_pull;
pub use status::run_beads_status;
pub use sync::run_sync;

/// Beads subcommands
#[derive(Subcommand, Debug)]
pub enum BeadsCommands {
    /// Sync speck steps to beads (creates/updates beads, writes IDs back)
    ///
    /// Creates a root bead for the speck and child beads for each step.
    /// Bead IDs are written back to the speck file.
    #[command(long_about = "Sync speck steps to beads (Spec S06).\n\nCreates:\n  - Root bead (epic) for the speck\n  - Child beads for each step\n  - Dependency edges matching **Depends on:**\n\nWrites bead IDs back to the speck file:\n  - **Beads Root:** in Plan Metadata\n  - **Bead:** line in each step")]
    Sync {
        /// Speck file to sync
        file: String,

        /// Show what would be created/updated without making changes
        #[arg(long)]
        dry_run: bool,

        /// Update bead titles for already-linked steps
        #[arg(long)]
        update_title: bool,

        /// Update bead descriptions for already-linked steps
        #[arg(long)]
        update_body: bool,

        /// Remove beads deps not present in the speck
        #[arg(long)]
        prune_deps: bool,

        /// Substep handling mode: none (default) or children
        #[arg(long, default_value = "none")]
        substeps: String,
    },

    /// Link an existing bead to a step
    ///
    /// Manually links a pre-existing bead to a step in the speck.
    #[command(long_about = "Link an existing bead to a step (Spec S07).\n\nWrites **Bead:** line to the specified step.\nValidates that both the step anchor and bead ID exist.")]
    Link {
        /// Speck file to modify
        file: String,

        /// Step anchor to link (e.g., step-0, step-2-1)
        step_anchor: String,

        /// Bead ID to link
        bead_id: String,
    },

    /// Show beads execution status aligned with speck steps
    ///
    /// Displays completion status for each step based on linked beads.
    #[command(long_about = "Show beads execution status (Spec S08).\n\nFor each step:\n  - complete: bead is closed\n  - ready: bead is open, all dependencies complete\n  - blocked: waiting on dependencies\n  - pending: no bead linked")]
    Status {
        /// Speck file (shows all specks if not specified)
        file: Option<String>,

        /// Also update checkboxes from bead completion (same as pull)
        #[arg(long)]
        pull: bool,
    },

    /// Update speck checkboxes from bead completion status
    ///
    /// Marks checkboxes as complete when their associated bead is closed.
    #[command(long_about = "Pull bead completion to checkboxes (Spec S09).\n\nFor each step with a linked bead:\n  - If bead is closed, check all checkpoint items\n  - Optionally check all items (Tasks/Tests/Checkpoints)")]
    Pull {
        /// Speck file (pulls all specks if not specified)
        file: Option<String>,

        /// Don't overwrite manually checked items
        #[arg(long)]
        no_overwrite: bool,
    },
}
