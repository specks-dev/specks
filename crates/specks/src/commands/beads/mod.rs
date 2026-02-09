//! Beads integration commands
//!
//! Provides subcommands for syncing specks to beads, linking steps to beads,
//! showing beads execution status, pulling bead completion back to checkboxes,
//! and closing beads to mark work complete.
//!
//! Requires: beads CLI (`bd`) installed, `.beads/` initialized, network connectivity.

pub mod close;
pub mod link;
pub mod pull;
pub mod status;
pub mod sync;

use clap::Subcommand;

pub use close::run_close;
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
    #[command(
        long_about = "Sync speck steps to beads.\n\nCreates:\n  - Root bead (epic) for the speck\n  - Child beads for each execution step\n  - Dependency edges matching **Depends on:** lines\n\nWrites bead IDs back to the speck file:\n  - **Beads Root:** `bd-xxx` in Plan Metadata\n  - **Bead:** `bd-xxx.N` in each step\n\nRe-running sync is idempotent—existing beads are reused."
    )]
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
    #[command(
        long_about = "Link an existing bead to a step.\n\nWrites **Bead:** `<bead-id>` line to the specified step.\nValidates that both the step anchor exists in the speck\nand the bead ID exists in beads.\n\nUseful when you have pre-existing beads you want to\nassociate with speck steps without full sync."
    )]
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
    #[command(
        long_about = "Show execution status for each step based on linked beads.\n\nStatus values:\n  - complete: bead is closed (work done)\n  - ready: bead is open, all dependencies complete\n  - blocked: waiting on dependencies to complete\n  - pending: no bead linked yet\n\nUse with --pull to also update speck checkboxes."
    )]
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
    #[command(
        long_about = "Pull bead completion status to speck checkboxes.\n\nFor each step with a linked bead:\n  - If bead is closed, marks checkpoint items as complete\n  - By default only updates **Checkpoint:** items\n  - Configure pull_checkbox_mode in config.toml for all items\n\nUse --no-overwrite to preserve manually checked items."
    )]
    Pull {
        /// Speck file (pulls all specks if not specified)
        file: Option<String>,

        /// Don't overwrite manually checked items
        #[arg(long)]
        no_overwrite: bool,
    },

    /// Close a bead to mark work complete
    ///
    /// Closes the specified bead, optionally with a reason.
    #[command(
        long_about = "Close a bead to mark work complete.\n\nThis is typically called by the committer skill after a successful commit\nto finalize step completion.\n\nThe bead ID must exist and be open. Once closed, the bead status\nwill be reflected in `specks beads status` as complete.\n\nAuto-rotation: After closing a bead, the implementation log is checked\nfor size thresholds (500 lines or 100KB). If exceeded, the log is\nautomatically rotated to .specks/archive/ and a fresh log is created."
    )]
    Close {
        /// Bead ID to close
        bead_id: String,

        /// Reason for closing (e.g., "Step completed per speck")
        #[arg(long)]
        reason: Option<String>,
    },
}
