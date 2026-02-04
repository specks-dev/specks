//! Core data types for specks

use serde::{Deserialize, Serialize};

/// A parsed speck document
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Speck {
    /// The file path this speck was parsed from (if known)
    pub path: Option<String>,
    /// Phase title from the document header
    pub phase_title: Option<String>,
    /// Phase anchor (e.g., "phase-1")
    pub phase_anchor: Option<String>,
    /// Purpose statement
    pub purpose: Option<String>,
    /// Plan metadata section
    pub metadata: SpeckMetadata,
    /// All anchors found in the document (for cross-reference validation)
    pub anchors: Vec<Anchor>,
    /// Design decisions
    pub decisions: Vec<Decision>,
    /// Open questions
    pub questions: Vec<Question>,
    /// Execution steps
    pub steps: Vec<Step>,
    /// Raw content (for line number lookups)
    pub raw_content: String,
}

/// Plan metadata section from a speck
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

impl SpeckMetadata {
    /// Check if the status value is valid (draft, active, done)
    pub fn is_valid_status(&self) -> bool {
        match &self.status {
            Some(s) => {
                let lower = s.to_lowercase();
                lower == "draft" || lower == "active" || lower == "done"
            }
            None => false,
        }
    }
}

/// An anchor found in the document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anchor {
    /// The anchor name (without the #)
    pub name: String,
    /// Line number where the anchor was found
    pub line: usize,
}

/// A design decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    /// Decision ID (e.g., "D01")
    pub id: String,
    /// Decision title
    pub title: String,
    /// Status (DECIDED, OPEN)
    pub status: Option<String>,
    /// Anchor name
    pub anchor: Option<String>,
    /// Line number
    pub line: usize,
}

/// An open question
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    /// Question ID (e.g., "Q01")
    pub id: String,
    /// Question title
    pub title: String,
    /// Resolution status (OPEN, DECIDED, DEFERRED)
    pub resolution: Option<String>,
    /// Anchor name
    pub anchor: Option<String>,
    /// Line number
    pub line: usize,
}

/// An execution step within a speck
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Step {
    /// Step number (e.g., "0", "1", "2")
    pub number: String,
    /// Step title
    pub title: String,
    /// Step anchor (e.g., "step-0", "step-2-1")
    pub anchor: String,
    /// Line number where the step starts
    pub line: usize,
    /// Dependencies (step anchors this step depends on)
    pub depends_on: Vec<String>,
    /// Associated bead ID (if synced)
    pub bead_id: Option<String>,
    /// Beads hints (type, priority, labels, estimate_minutes)
    pub beads_hints: Option<BeadsHints>,
    /// Commit message
    pub commit_message: Option<String>,
    /// References line content
    pub references: Option<String>,
    /// Task items
    pub tasks: Vec<Checkpoint>,
    /// Test items
    pub tests: Vec<Checkpoint>,
    /// Checkpoint/verification items
    pub checkpoints: Vec<Checkpoint>,
    /// Substeps (for nested steps like 2.1, 2.2)
    pub substeps: Vec<Substep>,
}

impl Step {
    /// Count total checkbox items (tasks + tests + checkpoints)
    pub fn total_items(&self) -> usize {
        self.tasks.len() + self.tests.len() + self.checkpoints.len()
    }

    /// Count completed checkbox items
    pub fn completed_items(&self) -> usize {
        self.tasks.iter().filter(|c| c.checked).count()
            + self.tests.iter().filter(|c| c.checked).count()
            + self.checkpoints.iter().filter(|c| c.checked).count()
    }
}

/// Beads hints for a step (optional metadata for bead creation)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BeadsHints {
    /// Issue type (task, feature, bug, epic, chore)
    pub issue_type: Option<String>,
    /// Priority (1-4)
    pub priority: Option<u8>,
    /// Labels (comma-separated)
    pub labels: Vec<String>,
    /// Time estimate in minutes
    pub estimate_minutes: Option<u32>,
}

/// A nested substep within a step
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Substep {
    /// Substep number (e.g., "2.1")
    pub number: String,
    /// Substep title
    pub title: String,
    /// Substep anchor
    pub anchor: String,
    /// Line number where the substep starts
    pub line: usize,
    /// Dependencies (step anchors this substep depends on)
    pub depends_on: Vec<String>,
    /// Associated bead ID (optional, only with --substeps children)
    pub bead_id: Option<String>,
    /// Beads hints
    pub beads_hints: Option<BeadsHints>,
    /// Commit message
    pub commit_message: Option<String>,
    /// References line content
    pub references: Option<String>,
    /// Task items
    pub tasks: Vec<Checkpoint>,
    /// Test items
    pub tests: Vec<Checkpoint>,
    /// Checkpoint/verification items
    pub checkpoints: Vec<Checkpoint>,
}

impl Substep {
    /// Count total checkbox items
    pub fn total_items(&self) -> usize {
        self.tasks.len() + self.tests.len() + self.checkpoints.len()
    }

    /// Count completed checkbox items
    pub fn completed_items(&self) -> usize {
        self.tasks.iter().filter(|c| c.checked).count()
            + self.tests.iter().filter(|c| c.checked).count()
            + self.checkpoints.iter().filter(|c| c.checked).count()
    }
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
    /// Line number where this item appears
    pub line: usize,
}

/// Kind of checkpoint item
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum CheckpointKind {
    /// Task item
    #[default]
    Task,
    /// Test item
    Test,
    /// Checkpoint/verification item
    Checkpoint,
}

/// Status of a speck based on metadata and completion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpeckStatus {
    /// Metadata Status = "draft"
    Draft,
    /// Metadata Status = "active", completion < 100%
    Active,
    /// Metadata Status = "done" OR completion = 100%
    Done,
}

impl std::fmt::Display for SpeckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpeckStatus::Draft => write!(f, "draft"),
            SpeckStatus::Active => write!(f, "active"),
            SpeckStatus::Done => write!(f, "done"),
        }
    }
}

impl Speck {
    /// Get the computed status based on metadata and completion
    pub fn computed_status(&self) -> SpeckStatus {
        let declared = self.metadata.status.as_deref().map(|s| s.to_lowercase());

        match declared.as_deref() {
            Some("draft") => SpeckStatus::Draft,
            Some("done") => SpeckStatus::Done,
            Some("active") => {
                if self.completion_percentage() >= 100.0 {
                    SpeckStatus::Done
                } else {
                    SpeckStatus::Active
                }
            }
            _ => SpeckStatus::Draft, // Default to draft if unknown
        }
    }

    /// Calculate completion percentage based on checkboxes in execution steps
    pub fn completion_percentage(&self) -> f64 {
        let (done, total) = self.completion_counts();
        if total == 0 {
            0.0
        } else {
            (done as f64 / total as f64) * 100.0
        }
    }

    /// Get (completed, total) counts for checkboxes in execution steps
    pub fn completion_counts(&self) -> (usize, usize) {
        let mut done = 0;
        let mut total = 0;

        for step in &self.steps {
            done += step.completed_items();
            total += step.total_items();

            for substep in &step.substeps {
                done += substep.completed_items();
                total += substep.total_items();
            }
        }

        (done, total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_status() {
        let mut meta = SpeckMetadata::default();
        assert!(!meta.is_valid_status());

        meta.status = Some("draft".to_string());
        assert!(meta.is_valid_status());

        meta.status = Some("ACTIVE".to_string());
        assert!(meta.is_valid_status());

        meta.status = Some("Done".to_string());
        assert!(meta.is_valid_status());

        meta.status = Some("invalid".to_string());
        assert!(!meta.is_valid_status());
    }

    #[test]
    fn test_step_counts() {
        let step = Step {
            tasks: vec![
                Checkpoint {
                    checked: true,
                    text: "Task 1".to_string(),
                    kind: CheckpointKind::Task,
                    line: 1,
                },
                Checkpoint {
                    checked: false,
                    text: "Task 2".to_string(),
                    kind: CheckpointKind::Task,
                    line: 2,
                },
            ],
            tests: vec![Checkpoint {
                checked: true,
                text: "Test 1".to_string(),
                kind: CheckpointKind::Test,
                line: 3,
            }],
            checkpoints: vec![Checkpoint {
                checked: false,
                text: "Check 1".to_string(),
                kind: CheckpointKind::Checkpoint,
                line: 4,
            }],
            ..Default::default()
        };

        assert_eq!(step.total_items(), 4);
        assert_eq!(step.completed_items(), 2);
    }

    #[test]
    fn test_speck_completion() {
        let mut speck = Speck::default();
        speck.metadata.status = Some("active".to_string());
        speck.steps.push(Step {
            tasks: vec![
                Checkpoint {
                    checked: true,
                    text: "Task 1".to_string(),
                    kind: CheckpointKind::Task,
                    line: 1,
                },
                Checkpoint {
                    checked: true,
                    text: "Task 2".to_string(),
                    kind: CheckpointKind::Task,
                    line: 2,
                },
            ],
            ..Default::default()
        });

        assert_eq!(speck.completion_counts(), (2, 2));
        assert_eq!(speck.completion_percentage(), 100.0);
        assert_eq!(speck.computed_status(), SpeckStatus::Done);
    }
}
