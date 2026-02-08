//! Worktree management for speck implementations
//!
//! Provides functions for creating, listing, and cleaning up git worktrees
//! for isolated speck implementation environments.

use crate::error::SpecksError;
use crate::parser::parse_speck;
use crate::session::{Session, SessionStatus, now_iso8601, save_session};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Configuration for worktree creation
#[derive(Debug, Clone)]
pub struct WorktreeConfig {
    /// Path to speck file (relative to repo root)
    pub speck_path: PathBuf,
    /// Base branch to create worktree from
    pub base_branch: String,
    /// Repository root directory
    pub repo_root: PathBuf,
}

/// Derive speck slug from speck path per Spec S05
///
/// Strips "specks-" prefix from filename (without extension).
/// Examples:
/// - .specks/specks-auth.md -> auth
/// - .specks/specks-worktree-integration.md -> worktree-integration
/// - .specks/specks-1.md -> 1
/// - .specks/my-feature.md -> my-feature
pub fn derive_speck_slug(speck_path: &Path) -> String {
    let filename = speck_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    filename
        .strip_prefix("specks-")
        .unwrap_or(filename)
        .to_string()
}

/// Sanitize branch name for filesystem-safe directory name per D08
///
/// Replaces problematic characters to create a valid directory name:
/// - '/' -> '__' (git path separators)
/// - '\\' -> '__' (Windows path separators)
/// - ':' -> '_' (Windows drive letters)
/// - ' ' -> '_' (shell escaping)
/// - Filters to alphanumeric, '-', and '_' only
///
/// Returns "specks-worktree" as defensive fallback if result is empty.
pub fn sanitize_branch_name(branch_name: &str) -> String {
    let sanitized: String = branch_name
        .replace(['/', '\\'], "__")
        .replace([':', ' '], "_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect();

    if sanitized.is_empty() {
        "specks-worktree".to_string()
    } else {
        sanitized
    }
}

/// Generate UTC timestamp in YYYYMMDD-HHMMSS format per Spec S05
fn generate_timestamp_utc() -> Result<String, SpecksError> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| {
        SpecksError::WorktreeCreationFailed {
            reason: format!("system time error: {}", e),
        }
    })?;

    let secs = duration.as_secs();

    // Convert to date/time components
    const SECONDS_PER_DAY: u64 = 86400;
    const DAYS_TO_EPOCH: i64 = 719162; // Days from 0000-01-01 to 1970-01-01

    let days_since_epoch = (secs / SECONDS_PER_DAY) as i64;
    let seconds_today = secs % SECONDS_PER_DAY;

    let hours = seconds_today / 3600;
    let minutes = (seconds_today % 3600) / 60;
    let seconds = seconds_today % 60;

    // Calculate year, month, day
    let total_days = DAYS_TO_EPOCH + days_since_epoch;

    let mut year = (total_days / 365) as i32;
    let mut remaining_days = total_days - year_to_days(year);

    while remaining_days < 0 {
        year -= 1;
        remaining_days = total_days - year_to_days(year);
    }
    while remaining_days >= days_in_year(year) {
        remaining_days -= days_in_year(year);
        year += 1;
    }

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

    Ok(format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}",
        year, month, day, hours, minutes, seconds
    ))
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

/// Generate branch name in format specks/<slug>-<timestamp>
pub fn generate_branch_name(slug: &str) -> Result<String, SpecksError> {
    let timestamp = generate_timestamp_utc()?;
    Ok(format!("specks/{}-{}", slug, timestamp))
}

/// Git CLI wrapper for worktree operations
struct GitCli<'a> {
    repo_root: &'a Path,
}

impl<'a> GitCli<'a> {
    fn new(repo_root: &'a Path) -> Self {
        Self { repo_root }
    }

    /// Check if git version is sufficient (2.15+)
    fn check_git_version(&self) -> Result<bool, SpecksError> {
        let output = Command::new("git").arg("--version").output().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                SpecksError::NotAGitRepository
            } else {
                SpecksError::WorktreeCreationFailed {
                    reason: format!("failed to run git: {}", e),
                }
            }
        })?;

        if !output.status.success() {
            return Ok(false);
        }

        let version_str = String::from_utf8_lossy(&output.stdout);
        // Parse version (e.g., "git version 2.39.0")
        if let Some(version_part) = version_str.split_whitespace().nth(2) {
            if let Some(major_minor) = version_part
                .split('.')
                .take(2)
                .collect::<Vec<_>>()
                .get(0..2)
            {
                if let (Ok(major), Ok(minor)) =
                    (major_minor[0].parse::<u32>(), major_minor[1].parse::<u32>())
                {
                    return Ok(major > 2 || (major == 2 && minor >= 15));
                }
            }
        }

        Ok(false)
    }

    /// Check if a branch exists
    fn branch_exists(&self, branch: &str) -> bool {
        Command::new("git")
            .arg("-C")
            .arg(self.repo_root)
            .args(["rev-parse", "--verify", branch])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Create a new branch from base
    fn create_branch(&self, base: &str, new_branch: &str) -> Result<(), SpecksError> {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.repo_root)
            .args(["branch", new_branch, base])
            .output()
            .map_err(|e| SpecksError::WorktreeCreationFailed {
                reason: format!("failed to create branch: {}", e),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpecksError::WorktreeCreationFailed {
                reason: format!("git branch failed: {}", stderr),
            });
        }

        Ok(())
    }

    /// Delete a branch
    fn delete_branch(&self, branch: &str) -> Result<(), SpecksError> {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.repo_root)
            .args(["branch", "-D", branch])
            .output()
            .map_err(|e| SpecksError::WorktreeCleanupFailed {
                reason: format!("failed to delete branch: {}", e),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpecksError::WorktreeCleanupFailed {
                reason: format!("git branch -D failed: {}", stderr),
            });
        }

        Ok(())
    }

    /// Add a worktree
    fn worktree_add(&self, path: &Path, branch: &str) -> Result<(), SpecksError> {
        let path_str = path
            .to_str()
            .ok_or_else(|| SpecksError::WorktreeCreationFailed {
                reason: format!("worktree path is not valid UTF-8: {}", path.display()),
            })?;

        let output = Command::new("git")
            .arg("-C")
            .arg(self.repo_root)
            .args(["worktree", "add", path_str, branch])
            .output()
            .map_err(|e| SpecksError::WorktreeCreationFailed {
                reason: format!("failed to add worktree: {}", e),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpecksError::WorktreeCreationFailed {
                reason: format!("git worktree add failed: {}", stderr),
            });
        }

        Ok(())
    }

    /// Remove a worktree
    fn worktree_remove(&self, path: &Path) -> Result<(), SpecksError> {
        let path_str = path
            .to_str()
            .ok_or_else(|| SpecksError::WorktreeCleanupFailed {
                reason: format!("worktree path is not valid UTF-8: {}", path.display()),
            })?;

        let output = Command::new("git")
            .arg("-C")
            .arg(self.repo_root)
            .args(["worktree", "remove", path_str])
            .output()
            .map_err(|e| SpecksError::WorktreeCleanupFailed {
                reason: format!("failed to remove worktree: {}", e),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpecksError::WorktreeCleanupFailed {
                reason: format!("git worktree remove failed: {}", stderr),
            });
        }

        Ok(())
    }

    /// Prune stale worktree metadata
    fn worktree_prune(&self) -> Result<(), SpecksError> {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.repo_root)
            .args(["worktree", "prune"])
            .output()
            .map_err(|e| SpecksError::WorktreeCleanupFailed {
                reason: format!("failed to prune worktrees: {}", e),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpecksError::WorktreeCleanupFailed {
                reason: format!("git worktree prune failed: {}", stderr),
            });
        }

        Ok(())
    }

    /// Check if branch is ancestor of base (for merge detection per D09)
    fn is_ancestor(&self, branch: &str, base: &str) -> bool {
        Command::new("git")
            .arg("-C")
            .arg(self.repo_root)
            .args(["merge-base", "--is-ancestor", branch, base])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

/// Create a worktree for speck implementation
///
/// Validates speck has at least one execution step, generates branch name,
/// creates branch from base, creates worktree, and initializes session.json.
///
/// Implements partial failure recovery:
/// - If branch creation succeeds but worktree creation fails: delete the branch
/// - If worktree creation succeeds but session.json write fails: remove worktree and delete branch
pub fn create_worktree(config: &WorktreeConfig) -> Result<Session, SpecksError> {
    let git = GitCli::new(&config.repo_root);

    // Check git version
    if !git.check_git_version()? {
        return Err(SpecksError::GitVersionInsufficient);
    }

    // Check if we're in a git repository
    if !config.repo_root.join(".git").exists() {
        return Err(SpecksError::NotAGitRepository);
    }

    // Check if base branch exists
    if !git.branch_exists(&config.base_branch) {
        return Err(SpecksError::BaseBranchNotFound {
            branch: config.base_branch.clone(),
        });
    }

    // Parse speck to validate it has execution steps
    let speck_full_path = config.repo_root.join(&config.speck_path);
    let speck_content = std::fs::read_to_string(&speck_full_path)?;
    let speck = parse_speck(&speck_content)?;

    if speck.steps.is_empty() {
        return Err(SpecksError::SpeckHasNoSteps);
    }

    // Generate branch name and worktree directory
    let slug = derive_speck_slug(&config.speck_path);
    let branch_name = generate_branch_name(&slug)?;
    let worktree_dir_name = sanitize_branch_name(&branch_name);
    let worktree_path = config
        .repo_root
        .join(".specks-worktrees")
        .join(&worktree_dir_name);

    // Check if worktree already exists
    if worktree_path.exists() {
        return Err(SpecksError::WorktreeAlreadyExists);
    }

    // Check if branch already exists
    if git.branch_exists(&branch_name) {
        return Err(SpecksError::WorktreeAlreadyExists);
    }

    // Create branch from base
    git.create_branch(&config.base_branch, &branch_name)?;

    // Create worktree (with partial failure recovery)
    if let Err(e) = git.worktree_add(&worktree_path, &branch_name) {
        // Clean up: delete the branch we just created
        let _ = git.delete_branch(&branch_name);
        return Err(e);
    }

    // Create session
    let session = Session {
        schema_version: "1".to_string(),
        speck_path: config.speck_path.display().to_string(),
        speck_slug: slug,
        branch_name: branch_name.clone(),
        base_branch: config.base_branch.clone(),
        worktree_path: worktree_path.display().to_string(),
        created_at: now_iso8601(),
        status: SessionStatus::Pending,
        current_step: 0,
        total_steps: speck.steps.len(),
        beads_root: None,
    };

    // Save session (with partial failure recovery)
    if let Err(e) = save_session(&session) {
        // Clean up: remove worktree and delete branch
        let _ = git.worktree_remove(&worktree_path);
        let _ = git.delete_branch(&branch_name);
        return Err(e);
    }

    Ok(session)
}

/// List all active worktrees
///
/// Prunes stale worktree metadata first, then scans .specks-worktrees/
/// for session.json files. Skips orphaned entries where directory doesn't exist.
pub fn list_worktrees(repo_root: &Path) -> Result<Vec<Session>, SpecksError> {
    let git = GitCli::new(repo_root);

    // Prune stale worktree metadata
    git.worktree_prune()?;

    let worktrees_dir = repo_root.join(".specks-worktrees");
    if !worktrees_dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();

    // Scan for session.json files
    if let Ok(entries) = std::fs::read_dir(&worktrees_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let session_file = path.join(".specks").join("session.json");
                if session_file.exists() {
                    // Try to load session, skip if it fails
                    if let Ok(session) = crate::session::load_session(&path) {
                        sessions.push(session);
                    }
                }
            }
        }
    }

    Ok(sessions)
}

/// Clean up worktrees for merged branches
///
/// Checks each worktree branch for merged status per D09 (git-only).
/// If dry_run is true, returns what would be removed without actually removing.
pub fn cleanup_worktrees(repo_root: &Path, dry_run: bool) -> Result<Vec<String>, SpecksError> {
    let git = GitCli::new(repo_root);
    let sessions = list_worktrees(repo_root)?;
    let mut removed = Vec::new();

    for session in sessions {
        // Check if branch is merged using git merge-base
        if git.is_ancestor(&session.branch_name, &session.base_branch) {
            removed.push(session.branch_name.clone());

            if !dry_run {
                // Remove worktree
                let worktree_path = Path::new(&session.worktree_path);
                git.worktree_remove(worktree_path)?;

                // Delete branch
                git.delete_branch(&session.branch_name)?;
            }
        }
    }

    // Final prune to clean up any stale metadata
    if !dry_run {
        git.worktree_prune()?;
    }

    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_speck_slug() {
        assert_eq!(
            derive_speck_slug(Path::new(".specks/specks-auth.md")),
            "auth"
        );
        assert_eq!(
            derive_speck_slug(Path::new(".specks/specks-worktree-integration.md")),
            "worktree-integration"
        );
        assert_eq!(derive_speck_slug(Path::new(".specks/specks-1.md")), "1");
        assert_eq!(
            derive_speck_slug(Path::new(".specks/my-feature.md")),
            "my-feature"
        );
    }

    #[test]
    fn test_sanitize_branch_name() {
        assert_eq!(
            sanitize_branch_name("specks/auth-20260208-143022"),
            "specks__auth-20260208-143022"
        );
        assert_eq!(
            sanitize_branch_name("specks\\windows\\path"),
            "specks__windows__path"
        );
        assert_eq!(sanitize_branch_name("feature:v1.0"), "feature_v10");
        assert_eq!(sanitize_branch_name("my feature"), "my_feature");
        assert_eq!(sanitize_branch_name("!@#$%"), "specks-worktree"); // Fallback
    }

    #[test]
    fn test_generate_branch_name() {
        let branch = generate_branch_name("auth").expect("timestamp generation should succeed");
        assert!(branch.starts_with("specks/auth-"));
        assert!(branch.len() > "specks/auth-".len());

        // Check timestamp format (YYYYMMDD-HHMMSS)
        let parts: Vec<&str> = branch.split('-').collect();
        assert!(parts.len() >= 3); // specks/auth, YYYYMMDD, HHMMSS
    }

    #[test]
    fn test_generate_timestamp_utc() {
        let timestamp = generate_timestamp_utc().expect("timestamp generation should succeed");

        // Format: YYYYMMDD-HHMMSS
        assert_eq!(timestamp.len(), 15); // 8 + 1 + 6
        assert!(timestamp.contains('-'));

        // Split and validate
        let parts: Vec<&str> = timestamp.split('-').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].len(), 8); // YYYYMMDD
        assert_eq!(parts[1].len(), 6); // HHMMSS

        // Validate year is reasonable
        let year: i32 = parts[0][..4].parse().expect("Year should be valid");
        assert!(year >= 2020 && year <= 2100);
    }
}
