//! Session state management for worktree-based speck implementations

use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::Path;
use crate::error::SpecksError;

/// Session status for worktree-based implementation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    /// Session created but not yet started
    Pending,
    /// Implementation in progress
    InProgress,
    /// All steps completed successfully
    Completed,
    /// Implementation failed
    Failed,
    /// Commit succeeded but bead close failed - needs reconciliation
    NeedsReconcile,
}

impl fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionStatus::Pending => write!(f, "pending"),
            SessionStatus::InProgress => write!(f, "in_progress"),
            SessionStatus::Completed => write!(f, "completed"),
            SessionStatus::Failed => write!(f, "failed"),
            SessionStatus::NeedsReconcile => write!(f, "needs_reconcile"),
        }
    }
}

/// Session state for a worktree-based speck implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Schema version for forward compatibility
    pub schema_version: String,
    /// Relative path to speck file from repo root
    pub speck_path: String,
    /// Short name derived from speck for branch naming
    pub speck_slug: String,
    /// Full branch name created for this implementation
    pub branch_name: String,
    /// Branch to merge back to (usually main)
    pub base_branch: String,
    /// Absolute path to worktree directory
    pub worktree_path: String,
    /// ISO 8601 timestamp of session creation
    pub created_at: String,
    /// Current session status
    pub status: SessionStatus,
    /// Index of the next step to execute (0-based)
    pub current_step: usize,
    /// Total number of steps in speck
    pub total_steps: usize,
    /// Root bead ID if beads are synced
    pub beads_root: Option<String>,
}

/// Generate ISO 8601 timestamp in UTC
///
/// Returns a string in the format "YYYY-MM-DDTHH:MM:SS.MMMZ"
/// This function is used internally for timestamp generation and is also
/// exposed for use by session creation code.
pub fn now_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("SystemTime before UNIX_EPOCH");

    let secs = duration.as_secs();
    let nanos = duration.subsec_nanos();

    // Convert to date/time components
    const SECONDS_PER_DAY: u64 = 86400;
    const DAYS_TO_EPOCH: i64 = 719162; // Days from 0000-01-01 to 1970-01-01

    let days_since_epoch = (secs / SECONDS_PER_DAY) as i64;
    let seconds_today = secs % SECONDS_PER_DAY;

    let hours = seconds_today / 3600;
    let minutes = (seconds_today % 3600) / 60;
    let seconds = seconds_today % 60;
    let millis = nanos / 1_000_000;

    // Calculate year, month, day (simplified algorithm)
    let total_days = DAYS_TO_EPOCH + days_since_epoch;

    // Approximate year (will refine)
    let mut year = (total_days / 365) as i32;
    let mut remaining_days = total_days - year_to_days(year);

    // Adjust for leap years
    while remaining_days < 0 {
        year -= 1;
        remaining_days = total_days - year_to_days(year);
    }
    while remaining_days >= days_in_year(year) {
        remaining_days -= days_in_year(year);
        year += 1;
    }

    // Calculate month and day
    let is_leap = is_leap_year(year);
    let mut month = 1;
    let mut day = remaining_days + 1;

    let days_in_months = if is_leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    for (m, &days) in days_in_months.iter().enumerate() {
        if day <= days as i64 {
            month = m + 1;
            break;
        }
        day -= days as i64;
    }

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        year, month, day, hours, minutes, seconds, millis
    )
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn days_in_year(year: i32) -> i64 {
    if is_leap_year(year) { 366 } else { 365 }
}

fn year_to_days(year: i32) -> i64 {
    let y = year as i64;
    y * 365 + y / 4 - y / 100 + y / 400
}

/// Load session from worktree directory
///
/// Reads and deserializes `session.json` from `{worktree_path}/.specks/session.json`
pub fn load_session(worktree_path: &Path) -> Result<Session, SpecksError> {
    let session_path = worktree_path.join(".specks").join("session.json");

    if !session_path.exists() {
        return Err(SpecksError::FileNotFound(
            session_path.display().to_string()
        ));
    }

    let content = fs::read_to_string(&session_path)?;
    let session: Session = serde_json::from_str(&content)
        .map_err(|e| SpecksError::Parse {
            message: format!("Failed to parse session.json: {}", e),
            line: None,
        })?;

    Ok(session)
}

/// Save session to worktree directory
///
/// Serializes and writes session to `{session.worktree_path}/.specks/session.json`
pub fn save_session(session: &Session) -> Result<(), SpecksError> {
    let worktree_path = Path::new(&session.worktree_path);
    let specks_dir = worktree_path.join(".specks");

    // Create .specks directory if it doesn't exist
    fs::create_dir_all(&specks_dir)?;

    let session_path = specks_dir.join("session.json");
    let content = serde_json::to_string_pretty(session)
        .map_err(|e| SpecksError::Parse {
            message: format!("Failed to serialize session: {}", e),
            line: None,
        })?;

    fs::write(&session_path, content)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_status_display() {
        assert_eq!(SessionStatus::Pending.to_string(), "pending");
        assert_eq!(SessionStatus::InProgress.to_string(), "in_progress");
        assert_eq!(SessionStatus::Completed.to_string(), "completed");
        assert_eq!(SessionStatus::Failed.to_string(), "failed");
        assert_eq!(SessionStatus::NeedsReconcile.to_string(), "needs_reconcile");
    }

    #[test]
    fn test_session_status_serialization() {
        let status = SessionStatus::InProgress;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#""in_progress""#);

        let deserialized: SessionStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, SessionStatus::InProgress);
    }

    #[test]
    fn test_session_serialization_roundtrip() {
        let session = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-5.md".to_string(),
            speck_slug: "auth".to_string(),
            branch_name: "specks/auth-20260208-143022".to_string(),
            base_branch: "main".to_string(),
            worktree_path: "/abs/path/to/.specks-worktrees/specks__auth-20260208-143022".to_string(),
            created_at: "2026-02-08T14:30:22Z".to_string(),
            status: SessionStatus::InProgress,
            current_step: 2,
            total_steps: 5,
            beads_root: Some("bd-abc123".to_string()),
        };

        let json = serde_json::to_string_pretty(&session).unwrap();
        let deserialized: Session = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.schema_version, session.schema_version);
        assert_eq!(deserialized.speck_path, session.speck_path);
        assert_eq!(deserialized.speck_slug, session.speck_slug);
        assert_eq!(deserialized.branch_name, session.branch_name);
        assert_eq!(deserialized.base_branch, session.base_branch);
        assert_eq!(deserialized.worktree_path, session.worktree_path);
        assert_eq!(deserialized.status, SessionStatus::InProgress);
        assert_eq!(deserialized.current_step, 2);
        assert_eq!(deserialized.total_steps, 5);
        assert_eq!(deserialized.beads_root, Some("bd-abc123".to_string()));
    }

    #[test]
    fn test_status_transitions() {
        // Test that we can transition between states
        let status = SessionStatus::Pending;
        assert_eq!(status, SessionStatus::Pending);

        let status = SessionStatus::InProgress;
        assert_eq!(status, SessionStatus::InProgress);

        let status = SessionStatus::Completed;
        assert_eq!(status, SessionStatus::Completed);

        let status = SessionStatus::Failed;
        assert_eq!(status, SessionStatus::Failed);

        let status = SessionStatus::NeedsReconcile;
        assert_eq!(status, SessionStatus::NeedsReconcile);
    }

    #[test]
    fn test_load_session_missing_file() {
        let temp_dir = std::env::temp_dir().join("specks-test-missing");
        let result = load_session(&temp_dir);
        assert!(result.is_err());
        match result {
            Err(SpecksError::FileNotFound(path)) => {
                assert!(path.contains("session.json"));
            }
            _ => panic!("Expected FileNotFound error"),
        }
    }

    #[test]
    fn test_save_and_load_session() {
        let temp_dir = std::env::temp_dir().join("specks-test-session");
        let _ = fs::remove_dir_all(&temp_dir); // Clean up from previous tests

        let session = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            speck_slug: "test".to_string(),
            branch_name: "specks/test-20260208-120000".to_string(),
            base_branch: "main".to_string(),
            worktree_path: temp_dir.display().to_string(),
            created_at: "2026-02-08T12:00:00Z".to_string(),
            status: SessionStatus::Pending,
            current_step: 0,
            total_steps: 3,
            beads_root: None,
        };

        // Save session
        save_session(&session).unwrap();

        // Load session
        let loaded = load_session(&temp_dir).unwrap();
        assert_eq!(loaded.schema_version, session.schema_version);
        assert_eq!(loaded.speck_path, session.speck_path);
        assert_eq!(loaded.status, SessionStatus::Pending);
        assert_eq!(loaded.current_step, 0);
        assert_eq!(loaded.total_steps, 3);

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_now_iso8601_format() {
        let timestamp = now_iso8601();

        // Check basic format: YYYY-MM-DDTHH:MM:SS.MMMZ
        assert!(timestamp.len() >= 20, "Timestamp too short: {}", timestamp);
        assert!(timestamp.ends_with('Z'), "Timestamp should end with Z: {}", timestamp);
        assert!(timestamp.contains('T'), "Timestamp should contain T: {}", timestamp);

        // Verify it can be parsed (basic validation)
        let parts: Vec<&str> = timestamp.split('T').collect();
        assert_eq!(parts.len(), 2, "Should have date and time parts");

        let date_parts: Vec<&str> = parts[0].split('-').collect();
        assert_eq!(date_parts.len(), 3, "Date should have year, month, day");

        // Year should be reasonable (between 2020 and 2100)
        let year: i32 = date_parts[0].parse().expect("Year should be valid number");
        assert!(year >= 2020 && year <= 2100, "Year should be reasonable: {}", year);
    }
}
