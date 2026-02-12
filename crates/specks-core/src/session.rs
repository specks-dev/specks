//! Session state management for worktree-based speck implementations

use crate::error::SpecksError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Session state for a worktree-based speck implementation
///
/// Schema v2: Beads is the source of truth for step state.
/// Session tracks only worktree metadata, not step progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Schema version (v2 as of step-3)
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    /// Session ID (e.g., "15-20250210-024623")
    pub session_id: String,
    /// Relative path to speck file from repo root
    pub speck_path: String,
    /// Short name derived from speck for branch naming
    #[serde(default)]
    pub speck_slug: String,
    /// Full branch name created for this implementation
    pub branch_name: String,
    /// Branch to merge back to (usually main)
    pub base_branch: String,
    /// Absolute path to worktree directory
    pub worktree_path: String,
    /// ISO 8601 timestamp of session creation
    pub created_at: String,
    /// ISO 8601 timestamp of last session update
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_updated_at: Option<String>,
    /// Total number of steps in speck
    #[serde(default)]
    pub total_steps: usize,
    /// Root bead ID (alias "beads_root" for v1 backward compat)
    #[serde(alias = "beads_root")]
    pub root_bead_id: Option<String>,
    /// True if this session was reused from an existing worktree
    #[serde(default)]
    pub reused: bool,
    /// Summaries of completed steps with commit hashes
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_summaries: Option<Vec<StepSummary>>,

    // V1 backward compatibility: deserialize but don't serialize
    #[serde(default, skip_serializing)]
    pub _status: Option<String>,
    #[serde(default, skip_serializing)]
    pub _current_step: Option<serde_json::Value>,
    #[serde(default, skip_serializing)]
    pub _steps_completed: Option<Vec<String>>,
    #[serde(default, skip_serializing)]
    pub _steps_remaining: Option<Vec<String>>,
    #[serde(default, skip_serializing)]
    pub _bead_mapping: Option<std::collections::HashMap<String, String>>,
}

fn default_schema_version() -> String {
    "2".to_string()
}

impl Default for Session {
    fn default() -> Self {
        Self {
            schema_version: "2".to_string(),
            session_id: String::new(),
            speck_path: String::new(),
            speck_slug: String::new(),
            branch_name: String::new(),
            base_branch: String::new(),
            worktree_path: String::new(),
            created_at: String::new(),
            last_updated_at: None,
            total_steps: 0,
            root_bead_id: None,
            reused: false,
            step_summaries: None,
            _status: None,
            _current_step: None,
            _steps_completed: None,
            _steps_remaining: None,
            _bead_mapping: None,
        }
    }
}

/// Summary of a completed step (implementer format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepSummary {
    /// Step anchor (e.g., "#step-0")
    pub step: String,
    /// Git commit hash for this step
    pub commit_hash: String,
    /// Human-readable summary of changes
    pub summary: String,
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
pub fn load_session(
    worktree_path: &Path,
    repo_root: Option<&Path>,
) -> Result<Session, SpecksError> {
    // Try external storage first if repo_root is provided
    if let Some(root) = repo_root {
        if let Some(session_id) = session_id_from_worktree(worktree_path) {
            let external_path = session_file_path(root, &session_id);
            if external_path.exists() {
                let content = fs::read_to_string(&external_path)?;
                let session: Session =
                    serde_json::from_str(&content).map_err(|e| SpecksError::Parse {
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

/// Save session to external storage atomically
///
/// Serializes and writes session to `{repo_root}/.specks-worktrees/.sessions/{session-id}.json`
/// using an atomic write pattern: write to temporary file, fsync, then rename.
///
/// This ensures that session files are never left in a partially-written state on interruption.
///
/// # Arguments
///
/// * `session` - The session to save
/// * `repo_root` - The repository root path
///
/// # Returns
///
/// * `Ok(())` if save succeeds
/// * `Err(SpecksError)` if save fails (temp file is cleaned up on error)
pub fn save_session_atomic(session: &Session, repo_root: &Path) -> Result<(), SpecksError> {
    let worktree_path = Path::new(&session.worktree_path);

    // Extract session ID from worktree path
    let session_id = session_id_from_worktree(worktree_path).ok_or_else(|| SpecksError::Parse {
        message: format!(
            "Cannot extract session ID from worktree path: {}",
            worktree_path.display()
        ),
        line: None,
    })?;

    // Create .specks-worktrees/.sessions directory if it doesn't exist
    let sessions_directory = sessions_dir(repo_root);
    fs::create_dir_all(&sessions_directory)?;

    // Get final and temporary paths
    let session_path = session_file_path(repo_root, &session_id);
    let temp_path = session_path.with_extension("tmp");

    // Serialize session to JSON
    let content = serde_json::to_string_pretty(session).map_err(|e| SpecksError::Parse {
        message: format!("Failed to serialize session: {}", e),
        line: None,
    })?;

    // Write to temporary file, fsync, then rename atomically
    // Clean up temp file on any error
    let result = (|| -> Result<(), SpecksError> {
        // Write to temp file
        fs::write(&temp_path, &content)?;

        // Fsync the file to ensure data is written to disk
        let file = fs::File::open(&temp_path)?;
        file.sync_all().map_err(SpecksError::Io)?;
        drop(file);

        // Atomically rename temp file to final location
        fs::rename(&temp_path, &session_path)?;

        Ok(())
    })();

    // Clean up temp file on error
    if result.is_err() && temp_path.exists() {
        let _ = fs::remove_file(&temp_path);
    }

    result
}

/// Save session to external storage
///
/// Serializes and writes session to `{repo_root}/.specks-worktrees/.sessions/{session-id}.json`.
/// Creates the `.sessions/` directory if it doesn't exist.
///
/// This function delegates to `save_session_atomic()` for atomic write behavior.
pub fn save_session(session: &Session, repo_root: &Path) -> Result<(), SpecksError> {
    save_session_atomic(session, repo_root)
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

/// Get the artifacts directory path inside a worktree
///
/// Returns the path to the artifacts directory inside the worktree:
/// `<worktree_path>/.specks/artifacts/`
///
/// This directory stores step-specific artifacts like log files and strategy JSONs.
/// Artifacts are now stored inside the worktree so they're automatically cleaned up
/// when the worktree is removed.
pub fn artifacts_dir(worktree_path: &Path) -> std::path::PathBuf {
    worktree_path.join(".specks/artifacts")
}

/// Get the full path to a session file in external storage
///
/// Returns the path to session.json in the external sessions directory:
/// `<repo_root>/.specks-worktrees/.sessions/<session-id>.json`
pub fn session_file_path(repo_root: &Path, session_id: &str) -> std::path::PathBuf {
    sessions_dir(repo_root).join(format!("{}.json", session_id))
}

/// Delete session file for a given session ID
///
/// Removes the session file at `.specks-worktrees/.sessions/<session-id>.json`.
///
/// Note: This function no longer removes artifacts since they're now stored inside the worktree
/// at `<worktree>/.specks/artifacts/` and are automatically cleaned up when the worktree is removed.
///
/// This function gracefully handles missing files - if the session file doesn't exist,
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
/// * `Ok(())` if deletion succeeds or file doesn't exist
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

    // Note: Artifacts are no longer deleted here since they live inside the worktree
    // at {worktree}/.specks/artifacts/ and are removed when the worktree is removed.

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_v2_schema() {
        // V2: Session no longer tracks status or current_step
        let session = Session::default();
        assert_eq!(session.schema_version, "2");
        assert!(session._status.is_none());
        assert!(session._current_step.is_none());
    }

    #[test]
    fn test_session_serialization_roundtrip() {
        let session = Session {
            schema_version: "2".to_string(),
            session_id: "test-20260208-120000".to_string(),
            speck_path: ".specks/specks-5.md".to_string(),
            speck_slug: "auth".to_string(),
            branch_name: "specks/auth-20260208-143022".to_string(),
            base_branch: "main".to_string(),
            worktree_path: "/abs/path/to/.specks-worktrees/specks__auth-20260208-143022"
                .to_string(),
            created_at: "2026-02-08T14:30:22Z".to_string(),
            last_updated_at: None,
            total_steps: 5,
            root_bead_id: Some("bd-abc123".to_string()),
            reused: false,
            step_summaries: None,
            ..Default::default()
        };

        let json = serde_json::to_string_pretty(&session).unwrap();
        let deserialized: Session = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.schema_version, session.schema_version);
        assert_eq!(deserialized.session_id, session.session_id);
        assert_eq!(deserialized.speck_path, session.speck_path);
        assert_eq!(deserialized.speck_slug, session.speck_slug);
        assert_eq!(deserialized.branch_name, session.branch_name);
        assert_eq!(deserialized.base_branch, session.base_branch);
        assert_eq!(deserialized.worktree_path, session.worktree_path);
        assert_eq!(deserialized.total_steps, 5);
        assert_eq!(deserialized.root_bead_id, Some("bd-abc123".to_string()));
    }
}
