//! Beads integration utilities
//!
//! Provides types and functions for interacting with the beads CLI
//! conforming to the Beads JSON Contract.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

use crate::error::SpecksError;

/// Issue object returned by `bd create --json`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub status: String,
    pub priority: i32,
    pub issue_type: String,
}

/// IssueDetails returned by `bd show <id> --json`
/// May be returned as array or single object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueDetails {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub status: String,
    pub priority: i32,
    pub issue_type: String,
    #[serde(default)]
    pub dependencies: Vec<DependencyRef>,
    #[serde(default)]
    pub dependents: Vec<DependencyRef>,
    #[serde(default)]
    pub design: Option<String>,
    #[serde(default)]
    pub acceptance_criteria: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

/// Dependency reference in IssueDetails
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyRef {
    pub id: String,
    #[serde(default)]
    pub dependency_type: String,
}

/// Dependency with metadata from `bd dep list --json`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueWithDependencyMetadata {
    pub id: String,
    #[serde(default)]
    pub dependency_type: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub issue_type: String,
}

/// Result of dep add/remove operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepResult {
    pub status: String,
    #[serde(default)]
    pub issue_id: String,
    #[serde(default)]
    pub depends_on_id: String,
    #[serde(rename = "type", default)]
    pub dep_type: String,
}

/// Status of a step relative to beads
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeadStatus {
    /// Bead is closed (complete)
    Complete,
    /// Bead is open and all dependencies are complete
    Ready,
    /// Bead is open and waiting on dependencies
    Blocked,
    /// No bead linked yet
    Pending,
}

impl std::fmt::Display for BeadStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BeadStatus::Complete => write!(f, "complete"),
            BeadStatus::Ready => write!(f, "ready"),
            BeadStatus::Blocked => write!(f, "blocked"),
            BeadStatus::Pending => write!(f, "pending"),
        }
    }
}

/// Beads CLI wrapper
#[derive(Debug, Clone)]
pub struct BeadsCli {
    /// Path to the bd binary
    pub bd_path: String,
}

impl Default for BeadsCli {
    fn default() -> Self {
        Self {
            bd_path: "bd".to_string(),
        }
    }
}

impl BeadsCli {
    /// Create a new BeadsCli with the specified path
    pub fn new(bd_path: String) -> Self {
        Self { bd_path }
    }

    /// Check if beads CLI is installed
    pub fn is_installed(&self) -> bool {
        Command::new(&self.bd_path)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Check if beads is initialized (`.beads/` directory exists)
    pub fn is_initialized(&self, project_root: &Path) -> bool {
        project_root.join(".beads").is_dir()
    }

    /// Create a new bead
    #[allow(clippy::too_many_arguments)] // Backward compatibility requires optional parameters
    pub fn create(
        &self,
        title: &str,
        description: Option<&str>,
        parent: Option<&str>,
        issue_type: Option<&str>,
        priority: Option<i32>,
        design: Option<&str>,
        acceptance: Option<&str>,
        notes: Option<&str>,
    ) -> Result<Issue, SpecksError> {
        let mut cmd = Command::new(&self.bd_path);
        cmd.arg("create").arg("--json").arg(title);

        if let Some(desc) = description {
            cmd.arg("--description").arg(desc);
        }
        if let Some(p) = parent {
            cmd.arg("--parent").arg(p);
        }
        if let Some(t) = issue_type {
            cmd.arg("--type").arg(t);
        }
        if let Some(pri) = priority {
            cmd.arg(format!("-p{}", pri));
        }
        if let Some(d) = design {
            cmd.arg("--design").arg(d);
        }
        if let Some(a) = acceptance {
            cmd.arg("--acceptance").arg(a);
        }
        if let Some(n) = notes {
            cmd.arg("--notes").arg(n);
        }

        let output = cmd.output().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                SpecksError::BeadsNotInstalled
            } else {
                SpecksError::BeadsCommand(format!("failed to run bd create: {}", e))
            }
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpecksError::BeadsCommand(format!(
                "bd create failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(&stdout).map_err(|e| {
            SpecksError::BeadsCommand(format!("failed to parse bd create output: {}", e))
        })
    }

    /// Show a bead by ID
    /// Returns IssueDetails, handling both array and object responses
    pub fn show(&self, id: &str) -> Result<IssueDetails, SpecksError> {
        let output = Command::new(&self.bd_path)
            .arg("show")
            .arg(id)
            .arg("--json")
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    SpecksError::BeadsNotInstalled
                } else {
                    SpecksError::BeadsCommand(format!("failed to run bd show: {}", e))
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpecksError::BeadsCommand(format!(
                "bd show failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Try parsing as array first, then as single object
        if let Ok(arr) = serde_json::from_str::<Vec<IssueDetails>>(&stdout) {
            if let Some(issue) = arr.into_iter().next() {
                return Ok(issue);
            }
            return Err(SpecksError::BeadsCommand(
                "bd show returned empty array".to_string(),
            ));
        }

        serde_json::from_str(&stdout).map_err(|e| {
            SpecksError::BeadsCommand(format!("failed to parse bd show output: {}", e))
        })
    }

    /// Check if a bead exists
    pub fn bead_exists(&self, id: &str) -> bool {
        self.show(id).is_ok()
    }

    /// Update the description field of a bead
    pub fn update_description(&self, id: &str, content: &str) -> Result<(), SpecksError> {
        let output = Command::new(&self.bd_path)
            .arg("update")
            .arg(id)
            .arg("--description")
            .arg(content)
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    SpecksError::BeadsNotInstalled
                } else {
                    SpecksError::BeadsCommand(format!("failed to run bd update: {}", e))
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpecksError::BeadsCommand(format!(
                "bd update --description failed: {}",
                stderr
            )));
        }

        Ok(())
    }

    /// Update the design field of a bead
    pub fn update_design(&self, id: &str, content: &str) -> Result<(), SpecksError> {
        let output = Command::new(&self.bd_path)
            .arg("update")
            .arg(id)
            .arg("--design")
            .arg(content)
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    SpecksError::BeadsNotInstalled
                } else {
                    SpecksError::BeadsCommand(format!("failed to run bd update: {}", e))
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpecksError::BeadsCommand(format!(
                "bd update --design failed: {}",
                stderr
            )));
        }

        Ok(())
    }

    /// Update the acceptance_criteria field of a bead
    pub fn update_acceptance(&self, id: &str, content: &str) -> Result<(), SpecksError> {
        let output = Command::new(&self.bd_path)
            .arg("update")
            .arg(id)
            .arg("--acceptance")
            .arg(content)
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    SpecksError::BeadsNotInstalled
                } else {
                    SpecksError::BeadsCommand(format!("failed to run bd update: {}", e))
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpecksError::BeadsCommand(format!(
                "bd update --acceptance failed: {}",
                stderr
            )));
        }

        Ok(())
    }

    /// Add a dependency edge
    pub fn dep_add(&self, from_id: &str, to_id: &str) -> Result<DepResult, SpecksError> {
        let output = Command::new(&self.bd_path)
            .arg("dep")
            .arg("add")
            .arg(from_id)
            .arg(to_id)
            .arg("--json")
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    SpecksError::BeadsNotInstalled
                } else {
                    SpecksError::BeadsCommand(format!("failed to run bd dep add: {}", e))
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpecksError::BeadsCommand(format!(
                "bd dep add failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(&stdout).map_err(|e| {
            SpecksError::BeadsCommand(format!("failed to parse bd dep add output: {}", e))
        })
    }

    /// Remove a dependency edge
    pub fn dep_remove(&self, from_id: &str, to_id: &str) -> Result<DepResult, SpecksError> {
        let output = Command::new(&self.bd_path)
            .arg("dep")
            .arg("remove")
            .arg(from_id)
            .arg(to_id)
            .arg("--json")
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    SpecksError::BeadsNotInstalled
                } else {
                    SpecksError::BeadsCommand(format!("failed to run bd dep remove: {}", e))
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpecksError::BeadsCommand(format!(
                "bd dep remove failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(&stdout).map_err(|e| {
            SpecksError::BeadsCommand(format!("failed to parse bd dep remove output: {}", e))
        })
    }

    /// List dependencies for a bead
    pub fn dep_list(&self, id: &str) -> Result<Vec<IssueWithDependencyMetadata>, SpecksError> {
        let output = Command::new(&self.bd_path)
            .arg("dep")
            .arg("list")
            .arg(id)
            .arg("--json")
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    SpecksError::BeadsNotInstalled
                } else {
                    SpecksError::BeadsCommand(format!("failed to run bd dep list: {}", e))
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpecksError::BeadsCommand(format!(
                "bd dep list failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(&stdout).map_err(|e| {
            SpecksError::BeadsCommand(format!("failed to parse bd dep list output: {}", e))
        })
    }

    /// Close a bead
    pub fn close(&self, id: &str, reason: Option<&str>) -> Result<(), SpecksError> {
        let mut cmd = Command::new(&self.bd_path);
        cmd.arg("close").arg(id);

        if let Some(r) = reason {
            cmd.arg("--reason").arg(r);
        }

        let output = cmd.output().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                SpecksError::BeadsNotInstalled
            } else {
                SpecksError::BeadsCommand(format!("failed to run bd close: {}", e))
            }
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpecksError::BeadsCommand(format!(
                "bd close failed: {}",
                stderr
            )));
        }

        Ok(())
    }

    /// Sync beads state
    pub fn sync(&self) -> Result<(), SpecksError> {
        let output = Command::new(&self.bd_path)
            .arg("sync")
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    SpecksError::BeadsNotInstalled
                } else {
                    SpecksError::BeadsCommand(format!("failed to run bd sync: {}", e))
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpecksError::BeadsCommand(format!(
                "bd sync failed: {}",
                stderr
            )));
        }

        Ok(())
    }

    /// Batch check existence of multiple bead IDs in a single subprocess call.
    /// Uses: `bd list --id=<ids> --json --limit 0 --all`
    /// Returns a set of IDs that exist.
    pub fn list_by_ids(
        &self,
        ids: &[String],
    ) -> Result<std::collections::HashSet<String>, SpecksError> {
        use std::collections::HashSet;

        if ids.is_empty() {
            return Ok(HashSet::new());
        }

        let ids_arg = ids.join(",");
        let output = Command::new(&self.bd_path)
            .args(["list", "--id", &ids_arg, "--json", "--limit", "0", "--all"])
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    SpecksError::BeadsNotInstalled
                } else {
                    SpecksError::BeadsCommand(format!("failed to run bd list: {}", e))
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpecksError::BeadsCommand(format!(
                "bd list failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let issues: Vec<Issue> = serde_json::from_str(&stdout).map_err(|e| {
            SpecksError::BeadsCommand(format!("failed to parse bd list output: {}", e))
        })?;

        Ok(issues.into_iter().map(|i| i.id).collect())
    }

    /// Create a bead with inline dependencies (reduces subprocess calls).
    /// Uses: `bd create --deps "dep1,dep2"`
    #[allow(clippy::too_many_arguments)] // Backward compatibility requires optional parameters
    pub fn create_with_deps(
        &self,
        title: &str,
        description: Option<&str>,
        parent: Option<&str>,
        deps: &[String],
        issue_type: Option<&str>,
        priority: Option<i32>,
        design: Option<&str>,
        acceptance: Option<&str>,
        notes: Option<&str>,
    ) -> Result<Issue, SpecksError> {
        let mut cmd = Command::new(&self.bd_path);
        cmd.arg("create").arg("--json").arg(title);

        if let Some(desc) = description {
            cmd.arg("--description").arg(desc);
        }
        if let Some(p) = parent {
            cmd.arg("--parent").arg(p);
        }
        if !deps.is_empty() {
            cmd.arg("--deps").arg(deps.join(","));
        }
        if let Some(t) = issue_type {
            cmd.arg("--type").arg(t);
        }
        if let Some(pri) = priority {
            cmd.arg(format!("-p{}", pri));
        }
        if let Some(d) = design {
            cmd.arg("--design").arg(d);
        }
        if let Some(a) = acceptance {
            cmd.arg("--acceptance").arg(a);
        }
        if let Some(n) = notes {
            cmd.arg("--notes").arg(n);
        }

        let output = cmd.output().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                SpecksError::BeadsNotInstalled
            } else {
                SpecksError::BeadsCommand(format!("failed to run bd create: {}", e))
            }
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpecksError::BeadsCommand(format!(
                "bd create failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(&stdout).map_err(|e| {
            SpecksError::BeadsCommand(format!("failed to parse bd create output: {}", e))
        })
    }

    /// Get all children of a parent bead in a single subprocess call.
    /// Uses: `bd children <id> --json`
    pub fn children(&self, parent_id: &str) -> Result<Vec<Issue>, SpecksError> {
        let output = Command::new(&self.bd_path)
            .args(["children", parent_id, "--json"])
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    SpecksError::BeadsNotInstalled
                } else {
                    SpecksError::BeadsCommand(format!("failed to run bd children: {}", e))
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpecksError::BeadsCommand(format!(
                "bd children failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(&stdout).map_err(|e| {
            SpecksError::BeadsCommand(format!("failed to parse bd children output: {}", e))
        })
    }

    /// Get all ready beads (open beads with all dependencies complete).
    /// Uses: `bd ready --json` (all ready beads) or `bd ready <parent_id> --json` (ready children of parent).
    pub fn ready(&self, parent_id: Option<&str>) -> Result<Vec<Issue>, SpecksError> {
        let mut cmd = Command::new(&self.bd_path);
        cmd.arg("ready");

        if let Some(parent) = parent_id {
            cmd.arg(parent);
        }

        cmd.arg("--json");

        let output = cmd.output().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                SpecksError::BeadsNotInstalled
            } else {
                SpecksError::BeadsCommand(format!("failed to run bd ready: {}", e))
            }
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpecksError::BeadsCommand(format!(
                "bd ready failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(&stdout).map_err(|e| {
            SpecksError::BeadsCommand(format!("failed to parse bd ready output: {}", e))
        })
    }
}

/// Validate bead ID format
/// Pattern: ^[a-z0-9][a-z0-9-]*-[a-z0-9]+(\.[0-9]+)*$
pub fn is_valid_bead_id(id: &str) -> bool {
    use regex::Regex;
    use std::sync::LazyLock;

    static BEAD_ID_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^[a-z0-9][a-z0-9-]*-[a-z0-9]+(\.[0-9]+)*$").unwrap());

    BEAD_ID_REGEX.is_match(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_bead_id() {
        assert!(is_valid_bead_id("bd-abc123"));
        assert!(is_valid_bead_id("bd-fake-1"));
        assert!(is_valid_bead_id("bd-fake-1.1"));
        assert!(is_valid_bead_id("bd-fake-1.2.3"));
        assert!(is_valid_bead_id("gt-abc1"));

        assert!(!is_valid_bead_id(""));
        assert!(!is_valid_bead_id("bd"));
        assert!(!is_valid_bead_id("bd-"));
        assert!(!is_valid_bead_id("-abc123"));
        assert!(!is_valid_bead_id("BD-ABC123")); // Must be lowercase
    }

    #[test]
    fn test_bead_status_display() {
        assert_eq!(format!("{}", BeadStatus::Complete), "complete");
        assert_eq!(format!("{}", BeadStatus::Ready), "ready");
        assert_eq!(format!("{}", BeadStatus::Blocked), "blocked");
        assert_eq!(format!("{}", BeadStatus::Pending), "pending");
    }

    #[test]
    fn test_issue_details_serde_without_rich_fields() {
        // Test backward compatibility: IssueDetails without new fields should deserialize correctly
        let json = r#"{
            "id": "bd-test1",
            "title": "Test Issue",
            "description": "Test description",
            "status": "open",
            "priority": 2,
            "issue_type": "task",
            "dependencies": [],
            "dependents": []
        }"#;

        let details: IssueDetails = serde_json::from_str(json).unwrap();
        assert_eq!(details.id, "bd-test1");
        assert_eq!(details.title, "Test Issue");
        assert_eq!(details.description, "Test description");
        assert!(details.design.is_none());
        assert!(details.acceptance_criteria.is_none());
        assert!(details.notes.is_none());
    }

    #[test]
    fn test_issue_details_serde_with_rich_fields() {
        // Test new fields serialize and deserialize correctly
        let json = "{
            \"id\": \"bd-test2\",
            \"title\": \"Rich Issue\",
            \"description\": \"Description\",
            \"status\": \"open\",
            \"priority\": 1,
            \"issue_type\": \"feature\",
            \"dependencies\": [],
            \"dependents\": [],
            \"design\": \"Design content\",
            \"acceptance_criteria\": \"Acceptance content\",
            \"notes\": \"Notes content\"
        }";

        let details: IssueDetails = serde_json::from_str(json).unwrap();
        assert_eq!(details.id, "bd-test2");
        assert_eq!(details.design, Some("Design content".to_string()));
        assert_eq!(
            details.acceptance_criteria,
            Some("Acceptance content".to_string())
        );
        assert_eq!(details.notes, Some("Notes content".to_string()));
    }

    #[test]
    fn test_issue_details_roundtrip() {
        // Test that IssueDetails with all fields round-trips correctly
        let original = IssueDetails {
            id: "bd-test3".to_string(),
            title: "Full Issue".to_string(),
            description: "Full description".to_string(),
            status: "open".to_string(),
            priority: 3,
            issue_type: "bug".to_string(),
            dependencies: vec![],
            dependents: vec![],
            design: Some("Design content".to_string()),
            acceptance_criteria: Some("Acceptance content".to_string()),
            notes: Some("Notes content".to_string()),
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: IssueDetails = serde_json::from_str(&json).unwrap();

        assert_eq!(original.id, deserialized.id);
        assert_eq!(original.title, deserialized.title);
        assert_eq!(original.design, deserialized.design);
        assert_eq!(
            original.acceptance_criteria,
            deserialized.acceptance_criteria
        );
        assert_eq!(original.notes, deserialized.notes);
    }
}
