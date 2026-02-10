//! Session state management for worktree-based speck implementations

use crate::error::SpecksError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::Path;

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
    /// True if this session was reused from an existing worktree
    #[serde(default)]
    pub reused: bool,
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

/// Load session from external or internal storage
///
/// First checks external storage at `{repo_root}/.specks-worktrees/.sessions/{session-id}.json`,
/// then falls back to internal storage at `{worktree_path}/.specks/session.json`.
///
/// If `repo_root` is None, only checks internal storage (backward compatibility).
pub fn load_session(worktree_path: &Path, repo_root: Option<&Path>) -> Result<Session, SpecksError> {
    // Try external storage first if repo_root is provided
    if let Some(root) = repo_root {
        if let Some(session_id) = session_id_from_worktree(worktree_path) {
            let external_path = session_file_path(root, &session_id);
            if external_path.exists() {
                let content = fs::read_to_string(&external_path)?;
                let session: Session = serde_json::from_str(&content).map_err(|e| SpecksError::Parse {
                    message: format!("Failed to parse session.json: {}", e),
                    line: None,
                })?;
                return Ok(session);
            }
        }
    }

    // Fall back to internal storage (backward compatibility)
    let session_path = worktree_path.join(".specks").join("session.json");

    if !session_path.exists() {
        return Err(SpecksError::FileNotFound(
            session_path.display().to_string(),
        ));
    }

    let content = fs::read_to_string(&session_path)?;
    let session: Session = serde_json::from_str(&content).map_err(|e| SpecksError::Parse {
        message: format!("Failed to parse session.json: {}", e),
        line: None,
    })?;

    Ok(session)
}

/// Save session to external storage
///
/// Serializes and writes session to `{repo_root}/.specks-worktrees/.sessions/{session-id}.json`.
/// Creates the `.sessions/` directory if it doesn't exist.
pub fn save_session(session: &Session, repo_root: &Path) -> Result<(), SpecksError> {
    let worktree_path = Path::new(&session.worktree_path);

    // Extract session ID from worktree path
    let session_id = session_id_from_worktree(worktree_path).ok_or_else(|| {
        SpecksError::Parse {
            message: format!(
                "Cannot extract session ID from worktree path: {}",
                worktree_path.display()
            ),
            line: None,
        }
    })?;

    // Create .specks-worktrees/.sessions directory if it doesn't exist
    let sessions_directory = sessions_dir(repo_root);
    fs::create_dir_all(&sessions_directory)?;

    // Write to external storage
    let session_path = session_file_path(repo_root, &session_id);
    let content = serde_json::to_string_pretty(session).map_err(|e| SpecksError::Parse {
        message: format!("Failed to serialize session: {}", e),
        line: None,
    })?;

    fs::write(&session_path, content)?;

    Ok(())
}

/// Extract session ID from worktree directory path
///
/// Given a worktree path like `.specks-worktrees/specks__auth-20260208-143022`,
/// extracts the session ID by stripping the `specks__` prefix from the directory basename.
///
/// Returns the session ID (e.g., `auth-20260208-143022`) or None if the path doesn't
/// match the expected format.
pub fn session_id_from_worktree(worktree_path: &Path) -> Option<String> {
    let basename = worktree_path.file_name()?.to_str()?;
    basename.strip_prefix("specks__").map(|s| s.to_string())
}

/// Get the sessions directory path
///
/// Returns the path to the external sessions directory: `<repo_root>/.specks-worktrees/.sessions/`
/// This directory stores session.json files externally from worktrees.
pub fn sessions_dir(repo_root: &Path) -> std::path::PathBuf {
    repo_root.join(".specks-worktrees").join(".sessions")
}

/// Get the artifacts directory path for a session
///
/// Returns the path to the artifacts directory for a given session ID:
/// `<repo_root>/.specks-worktrees/.artifacts/<session-id>/`
///
/// This directory stores session-specific artifacts like log files and strategy JSONs.
pub fn artifacts_dir(repo_root: &Path, session_id: &str) -> std::path::PathBuf {
    repo_root
        .join(".specks-worktrees")
        .join(".artifacts")
        .join(session_id)
}

/// Get the full path to a session file in external storage
///
/// Returns the path to session.json in the external sessions directory:
/// `<repo_root>/.specks-worktrees/.sessions/<session-id>.json`
pub fn session_file_path(repo_root: &Path, session_id: &str) -> std::path::PathBuf {
    sessions_dir(repo_root).join(format!("{}.json", session_id))
}

/// Delete session and artifacts for a given session ID
///
/// Removes both the session file at `.specks-worktrees/.sessions/<session-id>.json`
/// and the entire artifacts directory at `.specks-worktrees/.artifacts/<session-id>/`.
///
/// This function gracefully handles missing files and directories - if they don't exist,
/// the operation succeeds without error. This is intentional since the goal is to ensure
/// the session data is removed, whether it existed or not.
///
/// # Arguments
///
/// * `session_id` - The session ID (derived from worktree directory name)
/// * `repo_root` - The repository root path
///
/// # Returns
///
/// * `Ok(())` if deletion succeeds or files don't exist
/// * `Err(SpecksError)` if deletion fails due to permission or I/O errors
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use specks_core::session::delete_session;
///
/// let repo_root = Path::new("/path/to/repo");
/// delete_session("auth-20260208-143022", repo_root).unwrap();
/// ```
pub fn delete_session(session_id: &str, repo_root: &Path) -> Result<(), SpecksError> {
    // Delete session file
    let session_path = session_file_path(repo_root, session_id);
    if session_path.exists() {
        fs::remove_file(&session_path)?;
    }

    // Delete artifacts directory (recursively)
    let artifacts_path = artifacts_dir(repo_root, session_id);
    if artifacts_path.exists() {
        fs::remove_dir_all(&artifacts_path)?;
    }

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
            worktree_path: "/abs/path/to/.specks-worktrees/specks__auth-20260208-143022"
                .to_string(),
            created_at: "2026-02-08T14:30:22Z".to_string(),
            status: SessionStatus::InProgress,
            current_step: 2,
            total_steps: 5,
            beads_root: Some("bd-abc123".to_string()),
            reused: false,
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
        let result = load_session(&temp_dir, None);
        assert!(result.is_err());
        match result {
            Err(SpecksError::FileNotFound(path)) => {
                assert!(path.contains("session.json"));
            }
            _ => panic!("Expected FileNotFound error"),
        }
    }

    #[test]
    fn test_save_and_load_session_internal() {
        // Test old internal storage path for backward compatibility
        let temp_dir = std::env::temp_dir().join("specks-test-session-internal");
        let _ = fs::remove_dir_all(&temp_dir); // Clean up from previous tests

        let worktree_path = temp_dir.join(".specks-worktrees/specks__test-20260208-120000");
        let session = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            speck_slug: "test".to_string(),
            branch_name: "specks/test-20260208-120000".to_string(),
            base_branch: "main".to_string(),
            worktree_path: worktree_path.display().to_string(),
            created_at: "2026-02-08T12:00:00Z".to_string(),
            status: SessionStatus::Pending,
            current_step: 0,
            total_steps: 3,
            beads_root: None,
            reused: false,
        };

        // Create worktree directory and internal .specks dir
        let specks_dir = worktree_path.join(".specks");
        fs::create_dir_all(&specks_dir).unwrap();

        // Write to old internal location
        let session_path = specks_dir.join("session.json");
        let content = serde_json::to_string_pretty(&session).unwrap();
        fs::write(&session_path, content).unwrap();

        // Load session with no repo_root (should fall back to internal)
        let loaded = load_session(&worktree_path, None).unwrap();
        assert_eq!(loaded.schema_version, session.schema_version);
        assert_eq!(loaded.speck_path, session.speck_path);
        assert_eq!(loaded.status, SessionStatus::Pending);
        assert_eq!(loaded.current_step, 0);
        assert_eq!(loaded.total_steps, 3);

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_save_and_load_session_external() {
        // Test new external storage path
        let temp_dir = std::env::temp_dir().join("specks-test-session-external");
        let _ = fs::remove_dir_all(&temp_dir); // Clean up from previous tests

        let repo_root = temp_dir.clone();
        let worktree_path = temp_dir.join(".specks-worktrees/specks__test-20260208-120000");

        let session = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            speck_slug: "test".to_string(),
            branch_name: "specks/test-20260208-120000".to_string(),
            base_branch: "main".to_string(),
            worktree_path: worktree_path.display().to_string(),
            created_at: "2026-02-08T12:00:00Z".to_string(),
            status: SessionStatus::Pending,
            current_step: 0,
            total_steps: 3,
            beads_root: None,
            reused: false,
        };

        // Create worktree directory
        fs::create_dir_all(&worktree_path).unwrap();

        // Save session to external storage
        save_session(&session, &repo_root).unwrap();

        // Verify external file exists
        let external_path = session_file_path(&repo_root, "test-20260208-120000");
        assert!(external_path.exists());

        // Load session from external storage
        let loaded = load_session(&worktree_path, Some(&repo_root)).unwrap();
        assert_eq!(loaded.schema_version, session.schema_version);
        assert_eq!(loaded.speck_path, session.speck_path);
        assert_eq!(loaded.status, SessionStatus::Pending);
        assert_eq!(loaded.current_step, 0);
        assert_eq!(loaded.total_steps, 3);

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_load_session_fallback_to_internal() {
        // Test fallback: external doesn't exist, but internal does
        let temp_dir = std::env::temp_dir().join("specks-test-session-fallback");
        let _ = fs::remove_dir_all(&temp_dir); // Clean up from previous tests

        let repo_root = temp_dir.clone();
        let worktree_path = temp_dir.join(".specks-worktrees/specks__test-20260208-120000");

        let session = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            speck_slug: "test".to_string(),
            branch_name: "specks/test-20260208-120000".to_string(),
            base_branch: "main".to_string(),
            worktree_path: worktree_path.display().to_string(),
            created_at: "2026-02-08T12:00:00Z".to_string(),
            status: SessionStatus::Pending,
            current_step: 0,
            total_steps: 3,
            beads_root: None,
            reused: false,
        };

        // Create worktree directory and internal .specks dir
        let specks_dir = worktree_path.join(".specks");
        fs::create_dir_all(&specks_dir).unwrap();

        // Write to old internal location only
        let session_path = specks_dir.join("session.json");
        let content = serde_json::to_string_pretty(&session).unwrap();
        fs::write(&session_path, content).unwrap();

        // Load with repo_root provided, should fall back to internal
        let loaded = load_session(&worktree_path, Some(&repo_root)).unwrap();
        assert_eq!(loaded.schema_version, session.schema_version);
        assert_eq!(loaded.speck_path, session.speck_path);
        assert_eq!(loaded.status, SessionStatus::Pending);

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_save_session_creates_directory() {
        // Test that save_session creates .sessions directory if it doesn't exist
        let temp_dir = std::env::temp_dir().join("specks-test-session-create-dir");
        let _ = fs::remove_dir_all(&temp_dir); // Clean up from previous tests

        let repo_root = temp_dir.clone();
        let worktree_path = temp_dir.join(".specks-worktrees/specks__test-20260208-120000");

        let session = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            speck_slug: "test".to_string(),
            branch_name: "specks/test-20260208-120000".to_string(),
            base_branch: "main".to_string(),
            worktree_path: worktree_path.display().to_string(),
            created_at: "2026-02-08T12:00:00Z".to_string(),
            status: SessionStatus::Pending,
            current_step: 0,
            total_steps: 3,
            beads_root: None,
            reused: false,
        };

        // Create worktree directory
        fs::create_dir_all(&worktree_path).unwrap();

        // Verify .sessions directory doesn't exist yet
        let sessions_directory = sessions_dir(&repo_root);
        assert!(!sessions_directory.exists());

        // Save session (should create directory)
        save_session(&session, &repo_root).unwrap();

        // Verify .sessions directory was created
        assert!(sessions_directory.exists());
        assert!(sessions_directory.is_dir());

        // Verify session file exists
        let external_path = session_file_path(&repo_root, "test-20260208-120000");
        assert!(external_path.exists());

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_load_session_external_takes_precedence() {
        // Test that external storage takes precedence over internal when both exist
        let temp_dir = std::env::temp_dir().join("specks-test-session-precedence");
        let _ = fs::remove_dir_all(&temp_dir); // Clean up from previous tests

        let repo_root = temp_dir.clone();
        let worktree_path = temp_dir.join(".specks-worktrees/specks__test-20260208-120000");

        // Create a session for external storage
        let external_session = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            speck_slug: "test".to_string(),
            branch_name: "specks/test-20260208-120000".to_string(),
            base_branch: "main".to_string(),
            worktree_path: worktree_path.display().to_string(),
            created_at: "2026-02-08T12:00:00Z".to_string(),
            status: SessionStatus::InProgress,
            current_step: 2,
            total_steps: 3,
            beads_root: Some("bd-external".to_string()),
            reused: false,
        };

        // Create a different session for internal storage
        let internal_session = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            speck_slug: "test".to_string(),
            branch_name: "specks/test-20260208-120000".to_string(),
            base_branch: "main".to_string(),
            worktree_path: worktree_path.display().to_string(),
            created_at: "2026-02-08T12:00:00Z".to_string(),
            status: SessionStatus::Pending,
            current_step: 0,
            total_steps: 3,
            beads_root: Some("bd-internal".to_string()),
            reused: false,
        };

        // Create worktree directory and internal .specks dir
        let specks_dir = worktree_path.join(".specks");
        fs::create_dir_all(&specks_dir).unwrap();

        // Write to internal location
        let internal_path = specks_dir.join("session.json");
        let internal_content = serde_json::to_string_pretty(&internal_session).unwrap();
        fs::write(&internal_path, internal_content).unwrap();

        // Write to external location
        save_session(&external_session, &repo_root).unwrap();

        // Load session - should get external version
        let loaded = load_session(&worktree_path, Some(&repo_root)).unwrap();
        assert_eq!(loaded.status, SessionStatus::InProgress);
        assert_eq!(loaded.current_step, 2);
        assert_eq!(loaded.beads_root, Some("bd-external".to_string()));

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_now_iso8601_format() {
        let timestamp = now_iso8601();

        // Check basic format: YYYY-MM-DDTHH:MM:SS.MMMZ
        assert!(timestamp.len() >= 20, "Timestamp too short: {}", timestamp);
        assert!(
            timestamp.ends_with('Z'),
            "Timestamp should end with Z: {}",
            timestamp
        );
        assert!(
            timestamp.contains('T'),
            "Timestamp should contain T: {}",
            timestamp
        );

        // Verify it can be parsed (basic validation)
        let parts: Vec<&str> = timestamp.split('T').collect();
        assert_eq!(parts.len(), 2, "Should have date and time parts");

        let date_parts: Vec<&str> = parts[0].split('-').collect();
        assert_eq!(date_parts.len(), 3, "Date should have year, month, day");

        // Year should be reasonable (between 2020 and 2100)
        let year: i32 = date_parts[0].parse().expect("Year should be valid number");
        assert!(
            (2020..=2100).contains(&year),
            "Year should be reasonable: {}",
            year
        );
    }

    #[test]
    fn test_session_id_from_worktree_basic() {
        let path = Path::new(".specks-worktrees/specks__auth-20260208-143022");
        let session_id = session_id_from_worktree(path);
        assert_eq!(session_id, Some("auth-20260208-143022".to_string()));
    }

    #[test]
    fn test_session_id_from_worktree_numeric() {
        let path = Path::new(".specks-worktrees/specks__14-20250209-172747");
        let session_id = session_id_from_worktree(path);
        assert_eq!(session_id, Some("14-20250209-172747".to_string()));
    }

    #[test]
    fn test_session_id_from_worktree_with_hyphenated_slug() {
        let path = Path::new(".specks-worktrees/specks__my-feature-name-20260208-143022");
        let session_id = session_id_from_worktree(path);
        assert_eq!(
            session_id,
            Some("my-feature-name-20260208-143022".to_string())
        );
    }

    #[test]
    fn test_session_id_from_worktree_absolute_path() {
        let path = Path::new("/abs/path/to/.specks-worktrees/specks__auth-20260208-143022");
        let session_id = session_id_from_worktree(path);
        assert_eq!(session_id, Some("auth-20260208-143022".to_string()));
    }

    #[test]
    fn test_session_id_from_worktree_invalid_prefix() {
        let path = Path::new(".specks-worktrees/other__auth-20260208-143022");
        let session_id = session_id_from_worktree(path);
        assert_eq!(session_id, None);
    }

    #[test]
    fn test_session_id_from_worktree_no_prefix() {
        let path = Path::new(".specks-worktrees/auth-20260208-143022");
        let session_id = session_id_from_worktree(path);
        assert_eq!(session_id, None);
    }

    #[test]
    fn test_session_id_from_worktree_root_path() {
        let path = Path::new("/");
        let session_id = session_id_from_worktree(path);
        assert_eq!(session_id, None);
    }

    #[test]
    fn test_sessions_dir() {
        let repo_root = Path::new("/repo");
        let dir = sessions_dir(repo_root);
        assert_eq!(
            dir,
            std::path::PathBuf::from("/repo/.specks-worktrees/.sessions")
        );
    }

    #[test]
    fn test_artifacts_dir() {
        let repo_root = Path::new("/repo");
        let dir = artifacts_dir(repo_root, "auth-20260208-143022");
        assert_eq!(
            dir,
            std::path::PathBuf::from("/repo/.specks-worktrees/.artifacts/auth-20260208-143022")
        );
    }

    #[test]
    fn test_artifacts_dir_numeric() {
        let repo_root = Path::new("/repo");
        let dir = artifacts_dir(repo_root, "14-20250209-172747");
        assert_eq!(
            dir,
            std::path::PathBuf::from("/repo/.specks-worktrees/.artifacts/14-20250209-172747")
        );
    }

    #[test]
    fn test_session_file_path() {
        let repo_root = Path::new("/repo");
        let path = session_file_path(repo_root, "auth-20260208-143022");
        assert_eq!(
            path,
            std::path::PathBuf::from("/repo/.specks-worktrees/.sessions/auth-20260208-143022.json")
        );
    }

    #[test]
    fn test_session_file_path_numeric() {
        let repo_root = Path::new("/repo");
        let path = session_file_path(repo_root, "14-20250209-172747");
        assert_eq!(
            path,
            std::path::PathBuf::from("/repo/.specks-worktrees/.sessions/14-20250209-172747.json")
        );
    }

    #[test]
    fn test_path_helpers_integration() {
        // Simulate a typical worktree path
        let worktree_path = Path::new(".specks-worktrees/specks__auth-20260208-143022");
        let repo_root = Path::new("/repo");

        // Extract session ID
        let session_id = session_id_from_worktree(worktree_path).unwrap();
        assert_eq!(session_id, "auth-20260208-143022");

        // Derive session file path
        let session_file = session_file_path(repo_root, &session_id);
        assert_eq!(
            session_file,
            std::path::PathBuf::from("/repo/.specks-worktrees/.sessions/auth-20260208-143022.json")
        );

        // Derive artifacts directory
        let artifacts = artifacts_dir(repo_root, &session_id);
        assert_eq!(
            artifacts,
            std::path::PathBuf::from("/repo/.specks-worktrees/.artifacts/auth-20260208-143022")
        );
    }

    #[test]
    fn test_delete_session_success() {
        // Test successful deletion of session file and artifacts
        let temp_dir = std::env::temp_dir().join("specks-test-delete-session-success");
        let _ = fs::remove_dir_all(&temp_dir); // Clean up from previous tests

        let repo_root = temp_dir.clone();
        let session_id = "test-20260208-143022";
        let worktree_path = temp_dir.join(format!(".specks-worktrees/specks__{}", session_id));

        // Create session
        let session = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            speck_slug: "test".to_string(),
            branch_name: format!("specks/{}", session_id),
            base_branch: "main".to_string(),
            worktree_path: worktree_path.display().to_string(),
            created_at: "2026-02-08T14:30:22Z".to_string(),
            status: SessionStatus::InProgress,
            current_step: 0,
            total_steps: 3,
            beads_root: None,
            reused: false,
        };

        // Create worktree directory
        fs::create_dir_all(&worktree_path).unwrap();

        // Save session (creates session file)
        save_session(&session, &repo_root).unwrap();

        // Create artifacts directory with nested subdirectories
        let artifacts_path = artifacts_dir(&repo_root, session_id);
        let step_dir = artifacts_path.join("step-1");
        fs::create_dir_all(&step_dir).unwrap();
        fs::write(step_dir.join("architect-output.json"), "{}").unwrap();
        fs::write(step_dir.join("coder-output.json"), "{}").unwrap();

        // Verify files exist before deletion
        let session_path = session_file_path(&repo_root, session_id);
        assert!(session_path.exists());
        assert!(artifacts_path.exists());

        // Delete session
        delete_session(session_id, &repo_root).unwrap();

        // Verify files are deleted
        assert!(!session_path.exists());
        assert!(!artifacts_path.exists());

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_delete_session_missing_files() {
        // Test that delete_session succeeds even if files don't exist
        let temp_dir = std::env::temp_dir().join("specks-test-delete-session-missing");
        let _ = fs::remove_dir_all(&temp_dir); // Clean up from previous tests

        let repo_root = temp_dir.clone();
        let session_id = "nonexistent-20260208-143022";

        // Verify files don't exist
        let session_path = session_file_path(&repo_root, session_id);
        let artifacts_path = artifacts_dir(&repo_root, session_id);
        assert!(!session_path.exists());
        assert!(!artifacts_path.exists());

        // Delete session (should succeed even though nothing exists)
        let result = delete_session(session_id, &repo_root);
        assert!(result.is_ok());

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_delete_session_nested_artifacts() {
        // Test recursive deletion of deeply nested artifact directories
        let temp_dir = std::env::temp_dir().join("specks-test-delete-session-nested");
        let _ = fs::remove_dir_all(&temp_dir); // Clean up from previous tests

        let repo_root = temp_dir.clone();
        let session_id = "nested-20260208-143022";

        // Create deeply nested artifact structure
        let artifacts_path = artifacts_dir(&repo_root, session_id);
        let step1_dir = artifacts_path.join("step-1");
        let step2_dir = artifacts_path.join("step-2");
        let deep_dir = step1_dir.join("subdir").join("nested").join("deep");

        fs::create_dir_all(&deep_dir).unwrap();
        fs::create_dir_all(&step2_dir).unwrap();

        // Create files at various levels
        fs::write(artifacts_path.join("session-log.txt"), "log").unwrap();
        fs::write(step1_dir.join("architect-output.json"), "{}").unwrap();
        fs::write(deep_dir.join("data.json"), "{}").unwrap();
        fs::write(step2_dir.join("coder-output.json"), "{}").unwrap();

        // Verify structure exists
        assert!(artifacts_path.exists());
        assert!(deep_dir.exists());

        // Delete session (artifacts only, no session file in this test)
        delete_session(session_id, &repo_root).unwrap();

        // Verify entire directory tree is deleted
        assert!(!artifacts_path.exists());

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_delete_session_only_session_file() {
        // Test deletion when only session file exists (no artifacts)
        let temp_dir = std::env::temp_dir().join("specks-test-delete-session-file-only");
        let _ = fs::remove_dir_all(&temp_dir); // Clean up from previous tests

        let repo_root = temp_dir.clone();
        let session_id = "file-only-20260208-143022";
        let worktree_path = temp_dir.join(format!(".specks-worktrees/specks__{}", session_id));

        // Create session
        let session = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            speck_slug: "test".to_string(),
            branch_name: format!("specks/{}", session_id),
            base_branch: "main".to_string(),
            worktree_path: worktree_path.display().to_string(),
            created_at: "2026-02-08T14:30:22Z".to_string(),
            status: SessionStatus::Pending,
            current_step: 0,
            total_steps: 3,
            beads_root: None,
            reused: false,
        };

        // Create worktree directory
        fs::create_dir_all(&worktree_path).unwrap();

        // Save session (creates session file only)
        save_session(&session, &repo_root).unwrap();

        // Verify session file exists, artifacts don't
        let session_path = session_file_path(&repo_root, session_id);
        let artifacts_path = artifacts_dir(&repo_root, session_id);
        assert!(session_path.exists());
        assert!(!artifacts_path.exists());

        // Delete session
        delete_session(session_id, &repo_root).unwrap();

        // Verify session file is deleted
        assert!(!session_path.exists());

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_delete_session_only_artifacts() {
        // Test deletion when only artifacts exist (no session file)
        let temp_dir = std::env::temp_dir().join("specks-test-delete-session-artifacts-only");
        let _ = fs::remove_dir_all(&temp_dir); // Clean up from previous tests

        let repo_root = temp_dir.clone();
        let session_id = "artifacts-only-20260208-143022";

        // Create artifacts directory only
        let artifacts_path = artifacts_dir(&repo_root, session_id);
        let step_dir = artifacts_path.join("step-1");
        fs::create_dir_all(&step_dir).unwrap();
        fs::write(step_dir.join("architect-output.json"), "{}").unwrap();

        // Verify artifacts exist, session file doesn't
        let session_path = session_file_path(&repo_root, session_id);
        assert!(!session_path.exists());
        assert!(artifacts_path.exists());

        // Delete session
        delete_session(session_id, &repo_root).unwrap();

        // Verify artifacts are deleted
        assert!(!artifacts_path.exists());

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
