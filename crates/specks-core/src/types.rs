//! Core data types for specks

use serde::{Deserialize, Serialize};

/// A parsed speck document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Speck {
    /// Plan metadata section
    pub metadata: SpeckMetadata,
    /// Execution steps
    pub steps: Vec<Step>,
}

/// Plan metadata section from a speck
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeckMetadata {
    /// Owner of the speck
    pub owner: Option<String>,
    /// Status: draft, active, or done
    pub status: Option<String>,
    /// Target branch
    pub target_branch: Option<String>,
    /// Tracking issue or PR
    pub tracking: Option<String>,
    /// Last updated date
    pub last_updated: Option<String>,
    /// Beads root ID (optional, set after beads sync)
    pub beads_root_id: Option<String>,
}

/// An execution step within a speck
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    /// Step number (e.g., "0", "1", "2.1")
    pub number: String,
    /// Step title
    pub title: String,
    /// Step anchor (e.g., "step-0", "step-2-1")
    pub anchor: String,
    /// Dependencies (step anchors this step depends on)
    pub depends_on: Vec<String>,
    /// Associated bead ID (if synced)
    pub bead_id: Option<String>,
    /// Substeps (for nested steps like 2.1, 2.2)
    pub substeps: Vec<Substep>,
    /// Checkpoint items
    pub checkpoints: Vec<Checkpoint>,
}

/// A nested substep within a step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Substep {
    /// Substep number (e.g., "2.1")
    pub number: String,
    /// Substep title
    pub title: String,
    /// Substep anchor
    pub anchor: String,
    /// Associated bead ID (optional, only with --substeps children)
    pub bead_id: Option<String>,
    /// Checkpoint items
    pub checkpoints: Vec<Checkpoint>,
}

/// A checkbox item (task, test, or checkpoint)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Whether the checkbox is checked
    pub checked: bool,
    /// The text content of the checkbox item
    pub text: String,
    /// Type of checkpoint item
    pub kind: CheckpointKind,
}

/// Kind of checkpoint item
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CheckpointKind {
    /// Task item
    Task,
    /// Test item
    Test,
    /// Checkpoint/verification item
    Checkpoint,
}
