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
    pub fn create(
        &self,
        title: &str,
        description: Option<&str>,
        parent: Option<&str>,
        issue_type: Option<&str>,
        priority: Option<i32>,
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
}
