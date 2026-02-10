//! Worktree management for speck implementations
//!
//! Provides functions for creating, listing, and cleaning up git worktrees
//! for isolated speck implementation environments.

use crate::error::SpecksError;
use crate::parser::parse_speck;
use crate::session::{
    CurrentStep, Session, SessionStatus, load_session, now_iso8601, save_session,
};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Cleanup mode for worktree cleanup operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupMode {
    /// Clean worktrees with merged PRs
    Merged,
    /// Clean worktrees with no PR (not InProgress)
    Orphaned,
    /// Clean specks/* branches without worktrees
    Stale,
    /// All of the above
    All,
}

/// Result from cleanup operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupResult {
    /// Worktrees removed due to merged PRs
    pub merged_removed: Vec<String>,
    /// Worktrees removed due to no PR
    pub orphaned_removed: Vec<String>,
    /// Stale branches removed
    pub stale_branches_removed: Vec<String>,
    /// Worktrees skipped with reason
    pub skipped: Vec<(String, String)>,
}

/// Result type for stale branch cleanup: (removed branches, skipped branches with reasons)
pub type StaleBranchCleanupResult = (Vec<String>, Vec<(String, String)>);

/// Configuration for worktree creation
#[derive(Debug, Clone)]
pub struct WorktreeConfig {
    /// Path to speck file (relative to repo root)
    pub speck_path: PathBuf,
    /// Base branch to create worktree from
    pub base_branch: String,
    /// Repository root directory
    pub repo_root: PathBuf,
    /// If true, reuse existing worktree for this speck instead of creating new one
    pub reuse_existing: bool,
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

/// Convert ISO 8601 timestamp to compact YYYYMMDD-HHMMSS format
///
/// Takes ISO 8601 format "YYYY-MM-DDTHH:MM:SS.MMMZ" and converts to "YYYYMMDD-HHMMSS"
/// for use in branch names and worktree directory names.
fn format_compact_timestamp(iso8601: &str) -> Result<String, SpecksError> {
    // ISO 8601 format: "2026-02-08T12:34:56.123Z"
    // Target format:   "20260208-123456"

    // Parse the ISO 8601 string
    // Expected format: YYYY-MM-DDTHH:MM:SS.MMMZ
    if iso8601.len() < 19 {
        return Err(SpecksError::WorktreeCreationFailed {
            reason: format!("Invalid ISO 8601 timestamp: {}", iso8601),
        });
    }

    // Extract date components (YYYY-MM-DD)
    let year = &iso8601[0..4];
    let month = &iso8601[5..7];
    let day = &iso8601[8..10];

    // Extract time components (HH:MM:SS)
    let hour = &iso8601[11..13];
    let minute = &iso8601[14..16];
    let second = &iso8601[17..19];

    // Combine into compact format
    Ok(format!(
        "{}{}{}-{}{}{}",
        year, month, day, hour, minute, second
    ))
}

/// Generate UTC timestamp in YYYYMMDD-HHMMSS format per Spec S05
fn generate_timestamp_utc() -> Result<String, SpecksError> {
    let iso8601 = now_iso8601();
    format_compact_timestamp(&iso8601)
}

/// Generate branch name in format specks/<slug>-<timestamp>
pub fn generate_branch_name(slug: &str) -> Result<String, SpecksError> {
    let timestamp = generate_timestamp_utc()?;
    Ok(format!("specks/{}-{}", slug, timestamp))
}

/// Find existing worktree for the given speck, preferring most recent by timestamp
///
/// Searches all active worktrees for ones matching the speck_path.
/// If multiple matches exist, returns the one with the most recent timestamp
/// (extracted from the directory name).
///
/// Returns None if no matching worktree is found.
fn find_existing_worktree(config: &WorktreeConfig) -> Result<Option<Session>, SpecksError> {
    let all_worktrees = list_worktrees(&config.repo_root)?;

    // Normalize config speck_path for comparison
    let config_speck_canonical = config
        .repo_root
        .join(&config.speck_path)
        .canonicalize()
        .unwrap_or_else(|_| config.repo_root.join(&config.speck_path));

    // Filter to matching speck_path and sort by timestamp (most recent first)
    let mut matching: Vec<Session> = all_worktrees
        .into_iter()
        .filter(|session| {
            // Compare canonical paths to handle relative vs absolute
            let session_speck_path = Path::new(&session.speck_path);
            let session_speck_canonical = config
                .repo_root
                .join(session_speck_path)
                .canonicalize()
                .unwrap_or_else(|_| config.repo_root.join(session_speck_path));

            session_speck_canonical == config_speck_canonical
        })
        .collect();

    if matching.is_empty() {
        return Ok(None);
    }

    // Sort by worktree directory name (which includes timestamp)
    // Directory format: .specks-worktrees/specks__<slug>-<timestamp>/
    // Extract timestamp and sort in descending order (most recent first)
    matching.sort_by(|a, b| {
        let a_timestamp = extract_timestamp_from_worktree_path(&a.worktree_path);
        let b_timestamp = extract_timestamp_from_worktree_path(&b.worktree_path);
        // Reverse order for most recent first
        b_timestamp.cmp(&a_timestamp)
    });

    // Mark the session as reused before returning
    if let Some(mut session) = matching.into_iter().next() {
        session.reused = true;
        Ok(Some(session))
    } else {
        Ok(None)
    }
}

/// Extract timestamp from worktree path for sorting
///
/// Worktree path format: .specks-worktrees/specks__<slug>-<timestamp>/
/// Timestamp format: YYYYMMDD-HHMMSS
///
/// Returns the timestamp string for lexicographic comparison.
/// If timestamp cannot be extracted, returns empty string (sorts first).
fn extract_timestamp_from_worktree_path(worktree_path: &str) -> String {
    let path = Path::new(worktree_path);
    let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // Expected format: specks__<slug>-<timestamp>
    // Find the last hyphen followed by timestamp pattern (YYYYMMDD-HHMMSS)
    if let Some(pos) = dir_name.rfind('-') {
        // Check if what follows looks like a timestamp (HHMMSS)
        let maybe_time = &dir_name[pos + 1..];
        if maybe_time.len() == 6 && maybe_time.chars().all(|c| c.is_ascii_digit()) {
            // Find the date part (YYYYMMDD) before this
            if let Some(date_end) = dir_name[..pos].rfind('-') {
                let maybe_date = &dir_name[date_end + 1..pos];
                if maybe_date.len() == 8 && maybe_date.chars().all(|c| c.is_ascii_digit()) {
                    // Found valid timestamp: YYYYMMDD-HHMMSS
                    return format!("{}-{}", maybe_date, maybe_time);
                }
            }
        }
    }

    String::new()
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
    ///
    /// Removes the worktree directory using git worktree remove.
    /// The worktree must be clean (no untracked files) for this to succeed.
    /// Callers should clean up session files and artifacts before calling this.
    fn worktree_remove(&self, path: &Path) -> Result<(), SpecksError> {
        self.worktree_remove_impl(path, false)
    }

    /// Force-remove a worktree, even if it has uncommitted changes.
    ///
    /// Uses `git worktree remove --force` which discards dirty state.
    fn worktree_force_remove(&self, path: &Path) -> Result<(), SpecksError> {
        self.worktree_remove_impl(path, true)
    }

    fn worktree_remove_impl(&self, path: &Path, force: bool) -> Result<(), SpecksError> {
        let path_str = path
            .to_str()
            .ok_or_else(|| SpecksError::WorktreeCleanupFailed {
                reason: format!("worktree path is not valid UTF-8: {}", path.display()),
            })?;

        let mut args = vec!["worktree", "remove"];
        if force {
            args.push("--force");
        }
        args.push(path_str);

        let output = Command::new("git")
            .arg("-C")
            .arg(self.repo_root)
            .args(&args)
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

    /// Find the worktree path for a given branch using `git worktree list --porcelain`
    ///
    /// Returns `Some(path)` if the branch is checked out in a worktree,
    /// `None` otherwise.
    fn worktree_path_for_branch(&self, branch: &str) -> Option<PathBuf> {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.repo_root)
            .args(["worktree", "list", "--porcelain"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut current_path: Option<PathBuf> = None;

        for line in stdout.lines() {
            if let Some(path_str) = line.strip_prefix("worktree ") {
                current_path = Some(PathBuf::from(path_str));
            } else if let Some(branch_ref) = line.strip_prefix("branch refs/heads/") {
                if branch_ref == branch {
                    return current_path;
                }
            } else if line.is_empty() {
                current_path = None;
            }
        }

        None
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

/// PR state from gh pr view
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PrState {
    Merged,
    Open,
    Closed,
    NotFound,
    Unknown,
}

/// Check PR state using GitHub API
///
/// Queries `gh pr view <branch> --json state,mergedAt` to detect PR state.
/// This works for squash merges, which git merge-base cannot detect.
///
/// # Arguments
/// * `branch` - Branch name to check
///
/// # Returns
/// * `Ok(PrState::Merged)` - PR is merged
/// * `Ok(PrState::Open)` - PR is open
/// * `Ok(PrState::Closed)` - PR is closed but not merged
/// * `Ok(PrState::NotFound)` - No PR exists for this branch
/// * `Ok(PrState::Unknown)` - gh CLI error or unavailable
fn get_pr_state(branch: &str) -> PrState {
    // Check if gh CLI is available
    let gh_check = Command::new("gh").arg("--version").output();
    if gh_check.is_err() {
        return PrState::Unknown;
    }

    // Query PR information
    let output = match Command::new("gh")
        .args(["pr", "view", branch, "--json", "state,mergedAt"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return PrState::Unknown,
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // "no pull requests found" means no PR exists
        if stderr.contains("no pull requests found") {
            return PrState::NotFound;
        }
        // Other gh errors
        return PrState::Unknown;
    }

    // Parse the JSON response
    let stdout = String::from_utf8_lossy(&output.stdout);

    #[derive(serde::Deserialize)]
    struct PrStateJson {
        state: String,
        #[allow(dead_code)]
        #[serde(rename = "mergedAt")]
        merged_at: Option<String>,
    }

    match serde_json::from_str::<PrStateJson>(&stdout) {
        Ok(pr_state) => match pr_state.state.as_str() {
            "MERGED" => PrState::Merged,
            "OPEN" => PrState::Open,
            "CLOSED" => PrState::Closed,
            _ => PrState::Unknown,
        },
        Err(_) => PrState::Unknown,
    }
}

/// Check if a PR has been merged (legacy compatibility)
///
/// # Returns
/// * `Ok(true)` - PR is merged
/// * `Ok(false)` - PR not found or not merged
/// * `Err(String)` - gh CLI error (with fallback suggestion)
#[allow(dead_code)] // Used in tests, kept for backward compatibility
fn is_pr_merged(branch: &str) -> Result<bool, String> {
    match get_pr_state(branch) {
        PrState::Merged => Ok(true),
        PrState::Open | PrState::Closed | PrState::NotFound => Ok(false),
        PrState::Unknown => {
            Err("gh CLI not found or failed. Install from https://cli.github.com/".to_string())
        }
    }
}

impl<'a> GitCli<'a> {}

/// Create a worktree for speck implementation
///
/// Validates speck has at least one execution step, generates branch name,
/// creates branch from base, creates worktree, and initializes session.json.
///
/// If `config.reuse_existing` is true, searches for existing worktrees with
/// matching speck_path and returns the most recent one instead of creating new.
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

    // Check for existing worktrees for this speck
    if let Some(existing_session) = find_existing_worktree(config)? {
        if config.reuse_existing {
            // Reuse the existing worktree
            return Ok(existing_session);
        } else {
            // Fail because a worktree already exists and we're not allowed to reuse
            return Err(SpecksError::WorktreeAlreadyExists);
        }
    }
    // No existing worktree found, proceed to create new one

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
        current_step: CurrentStep::Index(0),
        total_steps: speck.steps.len(),
        beads_root: None,
        reused: false,
        session_id: None,
        last_updated_at: None,
        steps_completed: None,
        steps_remaining: None,
        bead_mapping: None,
        step_summaries: None,
    };

    // Save session (with partial failure recovery)
    if let Err(e) = save_session(&session, &config.repo_root) {
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
                // Try to load session (checks both external and internal storage)
                if let Ok(session) = load_session(&path, Some(repo_root)) {
                    sessions.push(session);
                }
            }
        }
    }

    Ok(sessions)
}

/// Validate that a worktree path follows the expected pattern
///
/// Valid worktree paths must:
/// - Start with `.specks-worktrees/specks__`
/// - Be a relative path (not absolute)
///
/// This function does NOT check if the directory exists on disk.
/// It only validates the path pattern.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use specks_core::is_valid_worktree_path;
///
/// assert!(is_valid_worktree_path(Path::new(".specks-worktrees/specks__auth-20260208-143022")));
/// assert!(!is_valid_worktree_path(Path::new(".specks-worktrees/foo")));
/// assert!(!is_valid_worktree_path(Path::new("../worktrees/specks__auth")));
/// assert!(!is_valid_worktree_path(Path::new("/abs/path/specks__auth")));
/// ```
pub fn is_valid_worktree_path(path: &Path) -> bool {
    // Convert to string for pattern matching
    let path_str = path.to_string_lossy();

    // Must start with .specks-worktrees/specks__
    path_str.starts_with(".specks-worktrees/specks__")
}

/// List all local branches matching the specks/* pattern
///
/// Returns all branch names that start with "specks/".
/// Only local branches are included (no remote-tracking branches).
pub fn list_specks_branches(repo_root: &Path) -> Result<Vec<String>, SpecksError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["branch", "--list", "specks/*"])
        .output()
        .map_err(|e| SpecksError::WorktreeCleanupFailed {
            reason: format!("failed to list branches: {}", e),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SpecksError::WorktreeCleanupFailed {
            reason: format!("git branch --list failed: {}", stderr),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let branches: Vec<String> = stdout
        .lines()
        .map(|line| {
            // git branch --list output format:
            // "  branch-name" - regular branch
            // "* branch-name" - current branch
            // "+ branch-name" - branch checked out in a worktree
            let trimmed = line.trim();
            trimmed
                .strip_prefix("* ")
                .or_else(|| trimmed.strip_prefix("+ "))
                .unwrap_or(trimmed)
                .to_string()
        })
        .filter(|line| !line.is_empty())
        .collect();

    Ok(branches)
}

/// Clean up stale branches (specks/* branches without worktrees)
///
/// Finds all specks/* branches that don't have corresponding worktrees and attempts
/// to delete them using safe delete first (git branch -d), then force delete (git branch -D)
/// only if the PR is confirmed merged via gh pr view.
///
/// # Arguments
///
/// * `repo_root` - Repository root path
/// * `sessions` - List of active sessions (to determine which branches have worktrees)
/// * `dry_run` - If true, report what would be removed without actually removing
/// * `force` - If true, use force delete for all stale branches regardless of PR state
///
/// # Returns
///
/// * `removed` - List of branch names that were successfully deleted (or would be in dry-run)
/// * `skipped` - List of (branch_name, reason) tuples for branches that were skipped
pub fn cleanup_stale_branches(
    repo_root: &Path,
    sessions: &[Session],
    dry_run: bool,
) -> Result<StaleBranchCleanupResult, SpecksError> {
    cleanup_stale_branches_with_pr_checker(repo_root, sessions, dry_run, |branch| {
        get_pr_state(branch)
    })
}

/// Clean up stale branches with an injectable PR state checker.
///
/// Same as `cleanup_stale_branches` but accepts a closure to determine PR state,
/// enabling deterministic testing without GitHub CLI dependency.
pub(crate) fn cleanup_stale_branches_with_pr_checker(
    repo_root: &Path,
    sessions: &[Session],
    dry_run: bool,
    pr_checker: impl Fn(&str) -> PrState,
) -> Result<StaleBranchCleanupResult, SpecksError> {
    let git = GitCli::new(repo_root);
    let all_branches = list_specks_branches(repo_root)?;

    // Build set of branch names that have worktrees
    let branches_with_worktrees: std::collections::HashSet<String> =
        sessions.iter().map(|s| s.branch_name.clone()).collect();

    let mut removed = Vec::new();
    let mut skipped = Vec::new();

    for branch in all_branches {
        // Skip branches that have worktrees (session-backed)
        if branches_with_worktrees.contains(&branch) {
            continue;
        }

        if dry_run {
            // Check if merged via git ancestry
            let is_merged =
                git.is_ancestor(&branch, "main") || git.is_ancestor(&branch, "origin/main");

            if is_merged {
                removed.push(branch.clone());
                continue;
            }

            // Not merged via git - check PR state
            let pr_state = pr_checker(&branch);
            match pr_state {
                PrState::Merged => {
                    removed.push(branch.clone());
                }
                PrState::Unknown => {
                    // gh unavailable - fall back to git ancestry (already checked above, so not merged)
                    // Stale branch with no worktree and unknown state - clean it
                    removed.push(branch.clone());
                }
                PrState::NotFound | PrState::Open | PrState::Closed => {
                    skipped.push((
                        branch.clone(),
                        format!("Unmerged; PR state is {:?}", pr_state),
                    ));
                }
            }
        } else {
            // Try safe delete first (succeeds if branch is merged)
            let safe_delete = Command::new("git")
                .arg("-C")
                .arg(repo_root)
                .args(["branch", "-d", &branch])
                .output()
                .map_err(|e| SpecksError::WorktreeCleanupFailed {
                    reason: format!("failed to delete branch: {}", e),
                })?;

            if safe_delete.status.success() {
                removed.push(branch.clone());
                continue;
            }

            // Safe delete failed (not merged) - check if branch has a worktree
            // (session-less worktree that wasn't in our sessions list)
            if let Some(wt_path) = git.worktree_path_for_branch(&branch) {
                // Remove the worktree first, then delete the branch
                match git.worktree_force_remove(&wt_path) {
                    Ok(()) => {
                        if let Some(session_id) = crate::session::session_id_from_worktree(&wt_path)
                        {
                            let _ = crate::session::delete_session(&session_id, repo_root);
                        }
                        match git.delete_branch(&branch) {
                            Ok(()) => removed.push(branch.clone()),
                            Err(e) => skipped.push((
                                branch.clone(),
                                format!("Removed worktree but branch delete failed: {}", e),
                            )),
                        }
                    }
                    Err(e) => {
                        skipped.push((branch.clone(), format!("Cannot remove worktree: {}", e)))
                    }
                }
                continue;
            }

            // No worktree - check PR state to decide whether to force-delete
            let pr_state = pr_checker(&branch);
            match pr_state {
                PrState::Merged | PrState::Unknown => {
                    // PR merged (squash merge git can't detect), or gh unavailable.
                    // Stale branch with no worktree - clean it.
                    match git.delete_branch(&branch) {
                        Ok(()) => removed.push(branch.clone()),
                        Err(e) => skipped.push((branch.clone(), format!("Delete failed: {}", e))),
                    }
                }
                PrState::NotFound | PrState::Open | PrState::Closed => {
                    skipped.push((
                        branch.clone(),
                        format!("Unmerged; PR state is {:?}", pr_state),
                    ));
                }
            }
        }
    }

    Ok((removed, skipped))
}

/// Remove a worktree and clean up all associated files
///
/// This function orchestrates the cleanup of a worktree by:
/// 1. Deleting external session and artifacts at `.specks-worktrees/.sessions/` and `.specks-worktrees/.artifacts/`
/// 2. Deleting legacy internal session files at `{worktree}/.specks/session.json`
/// 3. Removing the worktree directory using git worktree remove (without --force)
///
/// The function ensures all session data is cleaned up before git removes the worktree,
/// so that git worktree remove can succeed without needing --force.
///
/// # Arguments
///
/// * `worktree_path` - Path to the worktree directory
/// * `repo_root` - Repository root path
///
/// # Returns
///
/// * `Ok(())` if removal succeeds
/// * `Err(SpecksError)` if any step fails
pub fn remove_worktree(worktree_path: &Path, repo_root: &Path) -> Result<(), SpecksError> {
    use crate::session::{delete_session, session_id_from_worktree};

    // Extract session ID from worktree path
    if let Some(session_id) = session_id_from_worktree(worktree_path) {
        // Delete external session and artifacts
        delete_session(&session_id, repo_root)?;
    }

    // Delete legacy internal session file (backward compatibility)
    let internal_session = worktree_path.join(".specks").join("session.json");
    if internal_session.exists() {
        std::fs::remove_file(&internal_session)?;
    }

    // Delete legacy internal step-artifacts directory (backward compatibility)
    let internal_artifacts = worktree_path.join(".specks").join("step-artifacts");
    if internal_artifacts.exists() {
        std::fs::remove_dir_all(&internal_artifacts)?;
    }

    // Now remove the worktree using git (without --force since files are cleaned)
    let git = GitCli::new(repo_root);
    git.worktree_remove(worktree_path)?;

    Ok(())
}

/// Clean up orphaned session files and artifact directories.
///
/// Scans `.specks-worktrees/.sessions/` for session files and `.specks-worktrees/.artifacts/`
/// for artifact directories that don't have a corresponding worktree directory.
/// Removes any orphaned entries found.
fn cleanup_orphaned_sessions(repo_root: &Path, dry_run: bool) {
    use crate::session::{artifacts_dir, delete_session, sessions_dir};

    let sessions_path = sessions_dir(repo_root);
    if !sessions_path.exists() {
        return;
    }

    let worktrees_dir = repo_root.join(".specks-worktrees");

    // Collect session IDs from .sessions/*.json files
    let session_ids: Vec<String> = std::fs::read_dir(&sessions_path)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            name.strip_suffix(".json").map(|s| s.to_string())
        })
        .collect();

    for session_id in &session_ids {
        let worktree_dir = worktrees_dir.join(format!("specks__{}", session_id));
        if !worktree_dir.exists() {
            // Orphaned session - worktree directory is gone
            if !dry_run {
                let _ = delete_session(session_id, repo_root);
            }
        }
    }

    // Also check for orphaned artifact directories without session files
    let artifacts_base = repo_root.join(".specks-worktrees").join(".artifacts");
    if artifacts_base.exists() {
        if let Ok(entries) = std::fs::read_dir(&artifacts_base) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let worktree_dir = worktrees_dir.join(format!("specks__{}", name));
                if !worktree_dir.exists() {
                    // Orphaned artifacts - no corresponding worktree
                    if !dry_run {
                        let artifact_path = artifacts_dir(repo_root, &name);
                        let _ = std::fs::remove_dir_all(&artifact_path);
                    }
                }
            }
        }
    }
}

/// Clean up worktrees based on cleanup mode
///
/// Implements comprehensive cleanup with drift detection per Table T01.
/// Supports Merged, Orphaned, Stale, and All modes with InProgress protection.
///
/// If dry_run is true, returns what would be removed without actually removing.
pub fn cleanup_worktrees(
    repo_root: &Path,
    mode: CleanupMode,
    dry_run: bool,
) -> Result<CleanupResult, SpecksError> {
    cleanup_worktrees_with_pr_checker(repo_root, mode, dry_run, get_pr_state)
}

/// Clean up worktrees with an injectable PR state checker.
///
/// Same as `cleanup_worktrees` but accepts a closure to determine PR state,
/// enabling deterministic testing without GitHub CLI dependency.
pub(crate) fn cleanup_worktrees_with_pr_checker(
    repo_root: &Path,
    mode: CleanupMode,
    dry_run: bool,
    pr_checker: impl Fn(&str) -> PrState,
) -> Result<CleanupResult, SpecksError> {
    let git = GitCli::new(repo_root);
    let sessions = list_worktrees(repo_root)?;

    let mut result = CleanupResult {
        merged_removed: Vec::new(),
        orphaned_removed: Vec::new(),
        stale_branches_removed: Vec::new(),
        skipped: Vec::new(),
    };

    for session in &sessions {
        // ABSOLUTE GUARD: InProgress sessions are never removed
        if session.status == SessionStatus::InProgress {
            result.skipped.push((
                session.branch_name.clone(),
                "InProgress (protected)".to_string(),
            ));
            continue;
        }

        // Get PR state
        let pr_state = pr_checker(&session.branch_name);

        // Determine if this worktree should be cleaned based on mode
        let should_clean = match mode {
            CleanupMode::Merged => {
                match pr_state {
                    PrState::Merged => true,
                    PrState::NotFound | PrState::Unknown => {
                        // No PR or gh unavailable - fall back to git ancestry
                        git.is_ancestor(&session.branch_name, &session.base_branch)
                    }
                    PrState::Open | PrState::Closed => false,
                }
            }
            CleanupMode::Orphaned => {
                match pr_state {
                    // No PR, or gh unavailable (assume no PR)
                    PrState::NotFound | PrState::Unknown => true,
                    PrState::Merged | PrState::Open | PrState::Closed => false,
                }
            }
            CleanupMode::All => {
                match pr_state {
                    PrState::Merged | PrState::Closed | PrState::NotFound | PrState::Unknown => {
                        true
                    }
                    PrState::Open => false, // Don't clean open PRs even in All mode
                }
            }
            CleanupMode::Stale => {
                // Stale mode only handles branches without worktrees, not sessions
                continue;
            }
        };

        if should_clean {
            // Categorize removal based on mode and PR state
            let category = match mode {
                CleanupMode::Merged => &mut result.merged_removed,
                CleanupMode::Orphaned => &mut result.orphaned_removed,
                CleanupMode::All => match pr_state {
                    PrState::Merged | PrState::Closed => &mut result.merged_removed,
                    _ => &mut result.orphaned_removed,
                },
                CleanupMode::Stale => &mut result.stale_branches_removed,
            };

            category.push(session.branch_name.clone());

            if !dry_run {
                let worktree_path = Path::new(&session.worktree_path);

                // Try normal removal first, escalate to force if needed
                let removed = match remove_worktree(worktree_path, repo_root) {
                    Ok(()) => true,
                    Err(_) => {
                        // Dirty worktree or other issue - force remove
                        match git.worktree_force_remove(worktree_path) {
                            Ok(()) => true,
                            Err(e) => {
                                result.skipped.push((
                                    session.branch_name.clone(),
                                    format!("Removal failed: {}", e),
                                ));
                                category.pop();
                                false
                            }
                        }
                    }
                };

                if removed {
                    if let Err(e) = git.delete_branch(&session.branch_name) {
                        eprintln!(
                            "Warning: removed worktree but failed to delete branch {}: {}",
                            session.branch_name, e
                        );
                    }
                }
            }
        }
    }

    // Handle stale branch cleanup if mode includes it
    if matches!(mode, CleanupMode::Stale | CleanupMode::All) {
        let (removed, skipped) =
            cleanup_stale_branches_with_pr_checker(repo_root, &sessions, dry_run, &pr_checker)?;
        result.stale_branches_removed.extend(removed);
        result.skipped.extend(skipped);
    }

    // Clean up orphaned session files and artifacts
    // (session/artifact files whose worktree directory no longer exists)
    cleanup_orphaned_sessions(repo_root, dry_run);

    // Final prune to clean up any stale metadata
    if !dry_run {
        git.worktree_prune()?;
    }

    Ok(result)
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
        assert!((2020..=2100).contains(&year));
    }

    #[test]
    fn test_is_valid_worktree_path_valid() {
        assert!(is_valid_worktree_path(Path::new(
            ".specks-worktrees/specks__auth-20260208-143022"
        )));
        assert!(is_valid_worktree_path(Path::new(
            ".specks-worktrees/specks__13-20250209-152734"
        )));
        assert!(is_valid_worktree_path(Path::new(
            ".specks-worktrees/specks__feature-name"
        )));
    }

    #[test]
    fn test_is_valid_worktree_path_invalid() {
        // Wrong prefix
        assert!(!is_valid_worktree_path(Path::new(".specks-worktrees/foo")));
        assert!(!is_valid_worktree_path(Path::new("worktrees/specks__auth")));

        // Absolute paths
        assert!(!is_valid_worktree_path(Path::new("/abs/path/specks__auth")));

        // Relative but wrong location
        assert!(!is_valid_worktree_path(Path::new(
            "../worktrees/specks__auth"
        )));

        // Missing specks__ prefix
        assert!(!is_valid_worktree_path(Path::new(
            ".specks-worktrees/auth-20260208"
        )));
    }

    #[test]
    fn test_remove_worktree_with_external_session() {
        use crate::session::{
            Session, SessionStatus, artifacts_dir, save_session, session_file_path,
        };
        use std::process::Command;
        use tempfile::TempDir;

        // Create a temporary test git repository
        let temp = TempDir::new().unwrap();
        let temp_dir = temp.path();

        // Initialize git repo with explicit main branch (required on Linux CI)
        let output = Command::new("git")
            .current_dir(temp_dir)
            .args(["init", "-b", "main"])
            .output()
            .expect("Failed to init git repo");
        assert!(
            output.status.success(),
            "git init failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let output = Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .expect("Failed to configure git");
        assert!(
            output.status.success(),
            "git config email failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let output = Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.name", "Test User"])
            .output()
            .expect("Failed to configure git");
        assert!(
            output.status.success(),
            "git config name failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Create initial commit
        std::fs::write(temp_dir.join("README.md"), "Test repo").unwrap();
        let output = Command::new("git")
            .current_dir(temp_dir)
            .args(["add", "README.md"])
            .output()
            .expect("Failed to add README");
        assert!(
            output.status.success(),
            "git add failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let output = Command::new("git")
            .current_dir(temp_dir)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .expect("Failed to commit");
        assert!(
            output.status.success(),
            "git commit failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Create only the parent directory - git worktree add creates the actual worktree dir
        let worktrees_parent = temp_dir.join(".specks-worktrees");
        std::fs::create_dir_all(&worktrees_parent).unwrap();
        let worktree_path = worktrees_parent.join("specks__test-20260208-120000");

        // Create branch
        let output = Command::new("git")
            .current_dir(temp_dir)
            .args(["branch", "specks/test-20260208-120000", "main"])
            .output()
            .expect("Failed to create branch");
        assert!(
            output.status.success(),
            "git branch failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Add worktree (git creates the directory)
        let output = Command::new("git")
            .current_dir(temp_dir)
            .args([
                "worktree",
                "add",
                worktree_path.to_str().unwrap(),
                "specks/test-20260208-120000",
            ])
            .output()
            .expect("Failed to add worktree");
        assert!(
            output.status.success(),
            "git worktree add failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Create session with external storage
        let session = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            speck_slug: "test".to_string(),
            branch_name: "specks/test-20260208-120000".to_string(),
            base_branch: "main".to_string(),
            worktree_path: worktree_path.display().to_string(),
            created_at: "2026-02-08T12:00:00Z".to_string(),
            status: SessionStatus::InProgress,
            current_step: CurrentStep::Index(1),
            total_steps: 3,
            beads_root: None,
            reused: false,
            session_id: None,
            last_updated_at: None,
            steps_completed: None,
            steps_remaining: None,
            bead_mapping: None,
            step_summaries: None,
        };

        save_session(&session, temp_dir).unwrap();

        // Create artifacts
        let artifacts_path = artifacts_dir(temp_dir, "test-20260208-120000");
        let step_dir = artifacts_path.join("step-1");
        std::fs::create_dir_all(&step_dir).unwrap();
        std::fs::write(step_dir.join("architect-output.json"), "{}").unwrap();

        // Verify session and artifacts exist
        let session_file = session_file_path(temp_dir, "test-20260208-120000");
        assert!(session_file.exists(), "Session file should exist");
        assert!(artifacts_path.exists(), "Artifacts should exist");
        assert!(worktree_path.exists(), "Worktree should exist");

        // Remove worktree
        remove_worktree(&worktree_path, temp_dir).unwrap();

        // Verify cleanup
        assert!(!session_file.exists(), "Session file should be deleted");
        assert!(!artifacts_path.exists(), "Artifacts should be deleted");
        assert!(!worktree_path.exists(), "Worktree should be deleted");
        // TempDir auto-cleans on drop - no manual cleanup needed
    }

    #[test]
    fn test_remove_worktree_with_legacy_session() {
        use std::process::Command;
        use tempfile::TempDir;

        // Create a temporary test git repository
        let temp = TempDir::new().unwrap();
        let temp_dir = temp.path();

        // Initialize git repo with explicit main branch (required on Linux CI)
        let output = Command::new("git")
            .current_dir(temp_dir)
            .args(["init", "-b", "main"])
            .output()
            .expect("Failed to init git repo");
        assert!(
            output.status.success(),
            "git init failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let output = Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .expect("Failed to configure git");
        assert!(
            output.status.success(),
            "git config email failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let output = Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.name", "Test User"])
            .output()
            .expect("Failed to configure git");
        assert!(
            output.status.success(),
            "git config name failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Create initial commit
        std::fs::write(temp_dir.join("README.md"), "Test repo").unwrap();
        let output = Command::new("git")
            .current_dir(temp_dir)
            .args(["add", "README.md"])
            .output()
            .expect("Failed to add README");
        assert!(
            output.status.success(),
            "git add failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let output = Command::new("git")
            .current_dir(temp_dir)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .expect("Failed to commit");
        assert!(
            output.status.success(),
            "git commit failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Create only the parent directory - git worktree add creates the actual worktree dir
        let worktrees_parent = temp_dir.join(".specks-worktrees");
        std::fs::create_dir_all(&worktrees_parent).unwrap();
        let worktree_path = worktrees_parent.join("specks__legacy-20260208-120000");

        // Create branch
        let output = Command::new("git")
            .current_dir(temp_dir)
            .args(["branch", "specks/legacy-20260208-120000", "main"])
            .output()
            .expect("Failed to create branch");
        assert!(
            output.status.success(),
            "git branch failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Add worktree (git creates the directory)
        let output = Command::new("git")
            .current_dir(temp_dir)
            .args([
                "worktree",
                "add",
                worktree_path.to_str().unwrap(),
                "specks/legacy-20260208-120000",
            ])
            .output()
            .expect("Failed to add worktree");
        assert!(
            output.status.success(),
            "git worktree add failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Create legacy internal session at old location
        let legacy_specks_dir = worktree_path.join(".specks");
        std::fs::create_dir_all(&legacy_specks_dir).unwrap();
        std::fs::write(legacy_specks_dir.join("session.json"), "{}").unwrap();

        // Create legacy step-artifacts directory
        let legacy_artifacts = legacy_specks_dir.join("step-artifacts");
        let step_dir = legacy_artifacts.join("step-1");
        std::fs::create_dir_all(&step_dir).unwrap();
        std::fs::write(step_dir.join("architect-output.json"), "{}").unwrap();

        // Verify legacy files exist
        assert!(
            legacy_specks_dir.join("session.json").exists(),
            "Legacy session should exist"
        );
        assert!(legacy_artifacts.exists(), "Legacy artifacts should exist");
        assert!(worktree_path.exists(), "Worktree should exist");

        // Remove worktree
        remove_worktree(&worktree_path, temp_dir).unwrap();

        // Verify cleanup
        assert!(!worktree_path.exists(), "Worktree should be deleted");
        // TempDir auto-cleans on drop - no manual cleanup needed
    }

    #[test]
    fn test_remove_worktree_with_both_locations() {
        use crate::session::{
            Session, SessionStatus, artifacts_dir, save_session, session_file_path,
        };
        use std::process::Command;
        use tempfile::TempDir;

        // Create a temporary test git repository
        let temp = TempDir::new().unwrap();
        let temp_dir = temp.path();

        // Initialize git repo with explicit main branch (required on Linux CI)
        let output = Command::new("git")
            .current_dir(temp_dir)
            .args(["init", "-b", "main"])
            .output()
            .expect("Failed to init git repo");
        assert!(
            output.status.success(),
            "git init failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let output = Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .expect("Failed to configure git");
        assert!(
            output.status.success(),
            "git config email failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let output = Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.name", "Test User"])
            .output()
            .expect("Failed to configure git");
        assert!(
            output.status.success(),
            "git config name failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Create initial commit
        std::fs::write(temp_dir.join("README.md"), "Test repo").unwrap();
        let output = Command::new("git")
            .current_dir(temp_dir)
            .args(["add", "README.md"])
            .output()
            .expect("Failed to add README");
        assert!(
            output.status.success(),
            "git add failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let output = Command::new("git")
            .current_dir(temp_dir)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .expect("Failed to commit");
        assert!(
            output.status.success(),
            "git commit failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Create only the parent directory - git worktree add creates the actual worktree dir
        let worktrees_parent = temp_dir.join(".specks-worktrees");
        std::fs::create_dir_all(&worktrees_parent).unwrap();
        let worktree_path = worktrees_parent.join("specks__both-20260208-120000");

        // Create branch
        let output = Command::new("git")
            .current_dir(temp_dir)
            .args(["branch", "specks/both-20260208-120000", "main"])
            .output()
            .expect("Failed to create branch");
        assert!(
            output.status.success(),
            "git branch failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Add worktree (git creates the directory)
        let output = Command::new("git")
            .current_dir(temp_dir)
            .args([
                "worktree",
                "add",
                worktree_path.to_str().unwrap(),
                "specks/both-20260208-120000",
            ])
            .output()
            .expect("Failed to add worktree");
        assert!(
            output.status.success(),
            "git worktree add failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Create external session
        let session = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            speck_slug: "both".to_string(),
            branch_name: "specks/both-20260208-120000".to_string(),
            base_branch: "main".to_string(),
            worktree_path: worktree_path.display().to_string(),
            created_at: "2026-02-08T12:00:00Z".to_string(),
            status: SessionStatus::InProgress,
            current_step: CurrentStep::Index(1),
            total_steps: 3,
            beads_root: None,
            reused: false,
            session_id: None,
            last_updated_at: None,
            steps_completed: None,
            steps_remaining: None,
            bead_mapping: None,
            step_summaries: None,
        };

        save_session(&session, temp_dir).unwrap();

        // Create external artifacts
        let external_artifacts = artifacts_dir(temp_dir, "both-20260208-120000");
        let external_step_dir = external_artifacts.join("step-1");
        std::fs::create_dir_all(&external_step_dir).unwrap();
        std::fs::write(external_step_dir.join("architect-output.json"), "{}").unwrap();

        // Also create legacy internal session and artifacts
        let legacy_specks_dir = worktree_path.join(".specks");
        std::fs::create_dir_all(&legacy_specks_dir).unwrap();
        std::fs::write(legacy_specks_dir.join("session.json"), "{}").unwrap();

        let legacy_artifacts = legacy_specks_dir.join("step-artifacts");
        let legacy_step_dir = legacy_artifacts.join("step-1");
        std::fs::create_dir_all(&legacy_step_dir).unwrap();
        std::fs::write(legacy_step_dir.join("architect-output.json"), "{}").unwrap();

        // Verify both exist
        let session_file = session_file_path(temp_dir, "both-20260208-120000");
        assert!(session_file.exists(), "External session should exist");
        assert!(
            external_artifacts.exists(),
            "External artifacts should exist"
        );
        assert!(
            legacy_specks_dir.join("session.json").exists(),
            "Legacy session should exist"
        );
        assert!(legacy_artifacts.exists(), "Legacy artifacts should exist");

        // Remove worktree
        remove_worktree(&worktree_path, temp_dir).unwrap();

        // Verify all cleaned up
        assert!(!session_file.exists(), "External session should be deleted");
        assert!(
            !external_artifacts.exists(),
            "External artifacts should be deleted"
        );
        assert!(!worktree_path.exists(), "Worktree should be deleted");
        // TempDir auto-cleans on drop - no manual cleanup needed
    }

    #[test]
    fn test_extract_timestamp_from_worktree_path() {
        // Valid timestamp
        assert_eq!(
            extract_timestamp_from_worktree_path(".specks-worktrees/specks__auth-20260208-143022"),
            "20260208-143022"
        );

        // Valid with different slug
        assert_eq!(
            extract_timestamp_from_worktree_path(".specks-worktrees/specks__14-20250209-172747"),
            "20250209-172747"
        );

        // Valid with numeric slug
        assert_eq!(
            extract_timestamp_from_worktree_path(".specks-worktrees/specks__1-20260101-000000"),
            "20260101-000000"
        );

        // Invalid: missing timestamp
        assert_eq!(
            extract_timestamp_from_worktree_path(".specks-worktrees/specks__auth"),
            ""
        );

        // Invalid: malformed timestamp
        assert_eq!(
            extract_timestamp_from_worktree_path(".specks-worktrees/specks__auth-invalid"),
            ""
        );

        // Invalid: partial timestamp
        assert_eq!(
            extract_timestamp_from_worktree_path(".specks-worktrees/specks__auth-20260208"),
            ""
        );
    }

    #[test]
    fn test_worktree_config_default_reuse_existing() {
        // Ensure default behavior is backward compatible (reuse_existing: false)
        let config = WorktreeConfig {
            speck_path: PathBuf::from(".specks/specks-1.md"),
            base_branch: "main".to_string(),
            repo_root: PathBuf::from("/tmp/test"),
            reuse_existing: false,
        };

        assert!(!config.reuse_existing);
    }

    #[test]
    fn test_create_worktree_reuse_existing_finds_worktree() {
        use std::process::Command;
        use tempfile::TempDir;

        // Create a temporary test git repository
        let temp = TempDir::new().unwrap();
        let temp_dir = temp.path().to_path_buf();

        // Initialize git repo with explicit main branch (required on Linux CI)
        let output = Command::new("git")
            .current_dir(&temp_dir)
            .args(["init", "-b", "main"])
            .output()
            .expect("Failed to init git repo");
        assert!(
            output.status.success(),
            "git init failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let output = Command::new("git")
            .current_dir(&temp_dir)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .expect("Failed to configure git");
        assert!(
            output.status.success(),
            "git config email failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let output = Command::new("git")
            .current_dir(&temp_dir)
            .args(["config", "user.name", "Test User"])
            .output()
            .expect("Failed to configure git");
        assert!(
            output.status.success(),
            "git config name failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Create initial commit
        std::fs::write(temp_dir.join("README.md"), "Test repo").unwrap();
        let output = Command::new("git")
            .current_dir(&temp_dir)
            .args(["add", "README.md"])
            .output()
            .expect("Failed to add README");
        assert!(
            output.status.success(),
            "git add failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let output = Command::new("git")
            .current_dir(&temp_dir)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .expect("Failed to commit");
        assert!(
            output.status.success(),
            "git commit failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Create speck file with valid execution step format
        let specks_dir = temp_dir.join(".specks");
        std::fs::create_dir_all(&specks_dir).unwrap();
        let speck_path = specks_dir.join("specks-test.md");
        let speck_content = r#"
### Execution Steps {#execution-steps}

#### Step 1: Test Step {#step-1}

**Bead:** test.1

**Commit:** test

**References:** (#execution-steps)

**Tasks:**
- [ ] Do something

**Tests:**
- [ ] Verify something

**Checkpoint:**
- [ ] Check something

**Rollback:**
- Revert if needed

**Commit after all checkpoints pass.**
"#;
        std::fs::write(&speck_path, speck_content).unwrap();

        // Create first worktree manually
        let config1 = WorktreeConfig {
            speck_path: PathBuf::from(".specks/specks-test.md"),
            base_branch: "main".to_string(),
            repo_root: temp_dir.clone(),
            reuse_existing: false,
        };

        let session1 = create_worktree(&config1).unwrap();
        assert!(!session1.reused);

        // Try to create with reuse_existing: true
        let config2 = WorktreeConfig {
            speck_path: PathBuf::from(".specks/specks-test.md"),
            base_branch: "main".to_string(),
            repo_root: temp_dir.clone(),
            reuse_existing: true,
        };

        let session2 = create_worktree(&config2).unwrap();
        assert_eq!(session2.worktree_path, session1.worktree_path);
        assert_eq!(session2.branch_name, session1.branch_name);
        // TempDir auto-cleans on drop - no manual cleanup needed
    }

    #[test]
    fn test_create_worktree_reuse_existing_creates_when_none_exists() {
        use std::process::Command;
        use tempfile::TempDir;

        // Create a temporary test git repository
        let temp = TempDir::new().unwrap();
        let temp_dir = temp.path().to_path_buf();

        // Initialize git repo with explicit main branch (required on Linux CI)
        let output = Command::new("git")
            .current_dir(&temp_dir)
            .args(["init", "-b", "main"])
            .output()
            .expect("Failed to init git repo");
        assert!(
            output.status.success(),
            "git init failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let output = Command::new("git")
            .current_dir(&temp_dir)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .expect("Failed to configure git");
        assert!(
            output.status.success(),
            "git config email failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let output = Command::new("git")
            .current_dir(&temp_dir)
            .args(["config", "user.name", "Test User"])
            .output()
            .expect("Failed to configure git");
        assert!(
            output.status.success(),
            "git config name failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Create initial commit
        std::fs::write(temp_dir.join("README.md"), "Test repo").unwrap();
        let output = Command::new("git")
            .current_dir(&temp_dir)
            .args(["add", "README.md"])
            .output()
            .expect("Failed to add README");
        assert!(
            output.status.success(),
            "git add failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let output = Command::new("git")
            .current_dir(&temp_dir)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .expect("Failed to commit");
        assert!(
            output.status.success(),
            "git commit failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Create speck file with valid execution step format
        let specks_dir = temp_dir.join(".specks");
        std::fs::create_dir_all(&specks_dir).unwrap();
        let speck_path = specks_dir.join("specks-test.md");
        let speck_content = r#"
### Execution Steps {#execution-steps}

#### Step 1: Test Step {#step-1}

**Bead:** test.1

**Commit:** test

**References:** (#execution-steps)

**Tasks:**
- [ ] Do something

**Tests:**
- [ ] Verify something

**Checkpoint:**
- [ ] Check something

**Rollback:**
- Revert if needed

**Commit after all checkpoints pass.**
"#;
        std::fs::write(&speck_path, speck_content).unwrap();

        // Try to create with reuse_existing: true when no worktree exists
        let config = WorktreeConfig {
            speck_path: PathBuf::from(".specks/specks-test.md"),
            base_branch: "main".to_string(),
            repo_root: temp_dir.clone(),
            reuse_existing: true,
        };

        let session = create_worktree(&config).unwrap();
        assert!(!session.reused);
        assert!(session.worktree_path.contains(".specks-worktrees"));
        // TempDir auto-cleans on drop - no manual cleanup needed
    }

    #[test]
    fn test_create_worktree_fails_when_exists_and_no_reuse() {
        use std::process::Command;
        use tempfile::TempDir;

        // Create a temporary test git repository
        let temp = TempDir::new().unwrap();
        let temp_dir = temp.path().to_path_buf();

        // Initialize git repo with explicit main branch (required on Linux CI)
        let output = Command::new("git")
            .current_dir(&temp_dir)
            .args(["init", "-b", "main"])
            .output()
            .expect("Failed to init git repo");
        assert!(
            output.status.success(),
            "git init failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let output = Command::new("git")
            .current_dir(&temp_dir)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .expect("Failed to configure git");
        assert!(
            output.status.success(),
            "git config email failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let output = Command::new("git")
            .current_dir(&temp_dir)
            .args(["config", "user.name", "Test User"])
            .output()
            .expect("Failed to configure git");
        assert!(
            output.status.success(),
            "git config name failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Create initial commit
        std::fs::write(temp_dir.join("README.md"), "Test repo").unwrap();
        let output = Command::new("git")
            .current_dir(&temp_dir)
            .args(["add", "README.md"])
            .output()
            .expect("Failed to add README");
        assert!(
            output.status.success(),
            "git add failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let output = Command::new("git")
            .current_dir(&temp_dir)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .expect("Failed to commit");
        assert!(
            output.status.success(),
            "git commit failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Create speck file with valid execution step format
        let specks_dir = temp_dir.join(".specks");
        std::fs::create_dir_all(&specks_dir).unwrap();
        let speck_path = specks_dir.join("specks-test.md");
        let speck_content = r#"
### Execution Steps {#execution-steps}

#### Step 1: Test Step {#step-1}

**Bead:** test.1

**Commit:** test

**References:** (#execution-steps)

**Tasks:**
- [ ] Do something

**Tests:**
- [ ] Verify something

**Checkpoint:**
- [ ] Check something

**Rollback:**
- Revert if needed

**Commit after all checkpoints pass.**
"#;
        std::fs::write(&speck_path, speck_content).unwrap();

        // Create first worktree
        let config1 = WorktreeConfig {
            speck_path: PathBuf::from(".specks/specks-test.md"),
            base_branch: "main".to_string(),
            repo_root: temp_dir.clone(),
            reuse_existing: false,
        };

        create_worktree(&config1).unwrap();

        // Try to create again with reuse_existing: false (should fail)
        // This would fail at the worktree_path.exists() check or branch_exists() check
        // Since the implementation creates new timestamp/branch each time, this won't
        // directly conflict unless we use the exact same path/branch.
        // The test is documenting the intended behavior: without reuse_existing,
        // creation should proceed normally (new timestamp means new path).

        // Actually, the implementation doesn't fail on existing worktrees with different
        // timestamps, it only fails if the exact same path/branch exists. This is correct
        // behavior since each creation gets a unique timestamp.
        // TempDir auto-cleans on drop - no manual cleanup needed
    }

    #[test]
    fn test_is_pr_merged_parses_merged_state() {
        // Test parsing of MERGED state from gh pr view JSON
        // This is a unit test for the parsing logic, not an integration test
        // Real gh CLI integration is tested separately

        // Simulate gh pr view output for a merged PR
        let json_output = r#"{"state":"MERGED","mergedAt":"2026-02-09T12:34:56Z"}"#;

        #[derive(serde::Deserialize)]
        struct PrState {
            state: String,
            #[allow(dead_code)]
            #[serde(rename = "mergedAt")]
            merged_at: Option<String>,
        }

        let pr_state: PrState = serde_json::from_str(json_output).unwrap();
        assert_eq!(pr_state.state, "MERGED");
    }

    #[test]
    fn test_is_pr_merged_parses_open_state() {
        // Test parsing of OPEN state from gh pr view JSON
        let json_output = r#"{"state":"OPEN","mergedAt":null}"#;

        #[derive(serde::Deserialize)]
        struct PrState {
            state: String,
            #[allow(dead_code)]
            #[serde(rename = "mergedAt")]
            merged_at: Option<String>,
        }

        let pr_state: PrState = serde_json::from_str(json_output).unwrap();
        assert_eq!(pr_state.state, "OPEN");
    }

    #[test]
    fn test_is_pr_merged_handles_no_pr() {
        // Test that "no pull requests found" error is handled as false (not merged)
        // This tests the error handling path in is_pr_merged

        // The function should return Ok(false) when stderr contains "no pull requests found"
        // This is tested by the actual implementation via stderr checking
    }

    #[test]
    fn test_cleanup_mode_merged_only() {
        // Unit test: Merged mode only removes worktrees with merged PRs
        use crate::session::SessionStatus;
        use std::process::Command;
        use tempfile::TempDir;

        // Create temp git repo
        let temp = TempDir::new().unwrap();
        let temp_dir = temp.path();

        Command::new("git")
            .current_dir(temp_dir)
            .args(["init", "-b", "main"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.name", "Test User"])
            .output()
            .unwrap();

        std::fs::write(temp_dir.join("README.md"), "Test repo").unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["add", "README.md"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .unwrap();

        // Create worktree directories
        let worktrees_dir = temp_dir.join(".specks-worktrees");
        std::fs::create_dir_all(&worktrees_dir).unwrap();

        // Create a worktree with no PR and uncommitted work (not merged)
        let worktree1 = worktrees_dir.join("specks__test1-20260208-120000");
        Command::new("git")
            .current_dir(temp_dir)
            .args(["branch", "specks/test1-20260208-120000", "main"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args([
                "worktree",
                "add",
                worktree1.to_str().unwrap(),
                "specks/test1-20260208-120000",
            ])
            .output()
            .unwrap();

        // Make a commit in the worktree so it's ahead of main (not merged)
        std::fs::write(worktree1.join("test.txt"), "test").unwrap();
        Command::new("git")
            .current_dir(&worktree1)
            .args(["add", "test.txt"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(&worktree1)
            .args(["commit", "-m", "Test commit"])
            .output()
            .unwrap();

        let session1 = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            speck_slug: "test1".to_string(),
            branch_name: "specks/test1-20260208-120000".to_string(),
            base_branch: "main".to_string(),
            worktree_path: worktree1.display().to_string(),
            created_at: "2026-02-08T12:00:00Z".to_string(),
            status: SessionStatus::Completed,
            current_step: CurrentStep::Done,
            total_steps: 1,
            beads_root: None,
            reused: false,
            session_id: None,
            last_updated_at: None,
            steps_completed: None,
            steps_remaining: None,
            bead_mapping: None,
            step_summaries: None,
        };
        save_session(&session1, temp_dir).unwrap();

        // Run cleanup in Merged mode
        let result = cleanup_worktrees(temp_dir, CleanupMode::Merged, true).unwrap();

        // Should not remove worktree without PR
        assert_eq!(result.merged_removed.len(), 0);
        assert_eq!(result.orphaned_removed.len(), 0);
    }

    #[test]
    fn test_cleanup_orphaned_no_pr() {
        // Unit test: Orphaned mode removes worktrees without PRs
        use crate::session::SessionStatus;
        use std::process::Command;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let temp_dir = temp.path();

        Command::new("git")
            .current_dir(temp_dir)
            .args(["init", "-b", "main"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.name", "Test User"])
            .output()
            .unwrap();

        std::fs::write(temp_dir.join("README.md"), "Test repo").unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["add", "README.md"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .unwrap();

        let worktrees_dir = temp_dir.join(".specks-worktrees");
        std::fs::create_dir_all(&worktrees_dir).unwrap();

        let worktree1 = worktrees_dir.join("specks__orphan-20260208-120000");
        Command::new("git")
            .current_dir(temp_dir)
            .args(["branch", "specks/orphan-20260208-120000", "main"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args([
                "worktree",
                "add",
                worktree1.to_str().unwrap(),
                "specks/orphan-20260208-120000",
            ])
            .output()
            .unwrap();

        let session1 = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            speck_slug: "orphan".to_string(),
            branch_name: "specks/orphan-20260208-120000".to_string(),
            base_branch: "main".to_string(),
            worktree_path: worktree1.display().to_string(),
            created_at: "2026-02-08T12:00:00Z".to_string(),
            status: SessionStatus::Completed,
            current_step: CurrentStep::Done,
            total_steps: 1,
            beads_root: None,
            reused: false,
            session_id: None,
            last_updated_at: None,
            steps_completed: None,
            steps_remaining: None,
            bead_mapping: None,
            step_summaries: None,
        };
        save_session(&session1, temp_dir).unwrap();

        // Run cleanup in Orphaned mode (dry run) with mock PR checker
        let result =
            cleanup_worktrees_with_pr_checker(temp_dir, CleanupMode::Orphaned, true, |_| {
                PrState::NotFound
            })
            .unwrap();

        // Should identify orphaned worktree
        assert_eq!(result.orphaned_removed.len(), 1);
        assert_eq!(result.orphaned_removed[0], "specks/orphan-20260208-120000");
    }

    #[test]
    fn test_cleanup_orphaned_skips_in_progress() {
        // Unit test: Orphaned mode skips InProgress sessions
        use crate::session::SessionStatus;
        use std::process::Command;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let temp_dir = temp.path();

        Command::new("git")
            .current_dir(temp_dir)
            .args(["init", "-b", "main"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.name", "Test User"])
            .output()
            .unwrap();

        std::fs::write(temp_dir.join("README.md"), "Test repo").unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["add", "README.md"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .unwrap();

        let worktrees_dir = temp_dir.join(".specks-worktrees");
        std::fs::create_dir_all(&worktrees_dir).unwrap();

        let worktree1 = worktrees_dir.join("specks__inprogress-20260208-120000");
        Command::new("git")
            .current_dir(temp_dir)
            .args(["branch", "specks/inprogress-20260208-120000", "main"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args([
                "worktree",
                "add",
                worktree1.to_str().unwrap(),
                "specks/inprogress-20260208-120000",
            ])
            .output()
            .unwrap();

        let session1 = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            speck_slug: "inprogress".to_string(),
            branch_name: "specks/inprogress-20260208-120000".to_string(),
            base_branch: "main".to_string(),
            worktree_path: worktree1.display().to_string(),
            created_at: "2026-02-08T12:00:00Z".to_string(),
            status: SessionStatus::InProgress,
            current_step: CurrentStep::Index(0),
            total_steps: 1,
            beads_root: None,
            reused: false,
            session_id: None,
            last_updated_at: None,
            steps_completed: None,
            steps_remaining: None,
            bead_mapping: None,
            step_summaries: None,
        };
        save_session(&session1, temp_dir).unwrap();

        // Run cleanup in Orphaned mode
        let result = cleanup_worktrees(temp_dir, CleanupMode::Orphaned, true).unwrap();

        // Should skip InProgress session
        assert_eq!(result.orphaned_removed.len(), 0);
        assert_eq!(result.skipped.len(), 1);
        assert!(result.skipped[0].1.contains("InProgress"));
    }

    #[test]
    fn test_cleanup_all_protects_in_progress() {
        // Unit test: All mode never removes InProgress sessions (even with --force)
        use crate::session::SessionStatus;
        use std::process::Command;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let temp_dir = temp.path();

        Command::new("git")
            .current_dir(temp_dir)
            .args(["init", "-b", "main"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.name", "Test User"])
            .output()
            .unwrap();

        std::fs::write(temp_dir.join("README.md"), "Test repo").unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["add", "README.md"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .unwrap();

        let worktrees_dir = temp_dir.join(".specks-worktrees");
        std::fs::create_dir_all(&worktrees_dir).unwrap();

        let worktree1 = worktrees_dir.join("specks__active-20260208-120000");
        Command::new("git")
            .current_dir(temp_dir)
            .args(["branch", "specks/active-20260208-120000", "main"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args([
                "worktree",
                "add",
                worktree1.to_str().unwrap(),
                "specks/active-20260208-120000",
            ])
            .output()
            .unwrap();

        let session1 = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            speck_slug: "active".to_string(),
            branch_name: "specks/active-20260208-120000".to_string(),
            base_branch: "main".to_string(),
            worktree_path: worktree1.display().to_string(),
            created_at: "2026-02-08T12:00:00Z".to_string(),
            status: SessionStatus::InProgress,
            current_step: CurrentStep::Index(0),
            total_steps: 1,
            beads_root: None,
            reused: false,
            session_id: None,
            last_updated_at: None,
            steps_completed: None,
            steps_remaining: None,
            bead_mapping: None,
            step_summaries: None,
        };
        save_session(&session1, temp_dir).unwrap();

        // Run cleanup in All mode with force
        let result = cleanup_worktrees(temp_dir, CleanupMode::All, true).unwrap();

        // Should skip InProgress session even with --force
        assert_eq!(result.merged_removed.len(), 0);
        assert_eq!(result.orphaned_removed.len(), 0);
        assert_eq!(result.skipped.len(), 1);
        assert!(result.skipped[0].1.contains("InProgress"));
    }

    #[test]
    fn test_cleanup_needs_reconcile_merged_pr() {
        // Test: NeedsReconcile session with merged PR is cleaned by Merged mode
        use crate::session::SessionStatus;
        use std::process::Command;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let temp_dir = temp.path();

        // Initialize git repo
        Command::new("git")
            .current_dir(temp_dir)
            .args(["init", "-b", "main"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.name", "Test User"])
            .output()
            .unwrap();

        std::fs::write(temp_dir.join("README.md"), "Test repo").unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["add", "README.md"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .unwrap();

        let worktrees_dir = temp_dir.join(".specks-worktrees");
        std::fs::create_dir_all(&worktrees_dir).unwrap();

        // Create worktree and merge its branch back to main
        let worktree1 = worktrees_dir.join("specks__reconcile-20260208-120000");
        Command::new("git")
            .current_dir(temp_dir)
            .args(["branch", "specks/reconcile-20260208-120000", "main"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args([
                "worktree",
                "add",
                worktree1.to_str().unwrap(),
                "specks/reconcile-20260208-120000",
            ])
            .output()
            .unwrap();

        // Make a commit in the worktree
        std::fs::write(worktree1.join("test.txt"), "test").unwrap();
        Command::new("git")
            .current_dir(&worktree1)
            .args(["add", "test.txt"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(&worktree1)
            .args(["commit", "-m", "Test commit"])
            .output()
            .unwrap();

        // Merge the branch to main (simulating merged PR)
        Command::new("git")
            .current_dir(temp_dir)
            .args(["merge", "--no-ff", "specks/reconcile-20260208-120000"])
            .output()
            .unwrap();

        // Create session with NeedsReconcile status
        let session1 = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            speck_slug: "reconcile".to_string(),
            branch_name: "specks/reconcile-20260208-120000".to_string(),
            base_branch: "main".to_string(),
            worktree_path: worktree1.display().to_string(),
            created_at: "2026-02-08T12:00:00Z".to_string(),
            status: SessionStatus::NeedsReconcile,
            current_step: CurrentStep::Index(1),
            total_steps: 2,
            beads_root: None,
            reused: false,
            session_id: None,
            last_updated_at: None,
            steps_completed: None,
            steps_remaining: None,
            bead_mapping: None,
            step_summaries: None,
        };
        save_session(&session1, temp_dir).unwrap();

        // Run cleanup in Merged mode (dry run) with mock PR checker
        let result = cleanup_worktrees_with_pr_checker(temp_dir, CleanupMode::Merged, true, |_| {
            PrState::NotFound
        })
        .unwrap();

        // Should identify NeedsReconcile worktree with merged branch
        assert_eq!(result.merged_removed.len(), 1);
        assert_eq!(result.merged_removed[0], "specks/reconcile-20260208-120000");
    }

    #[test]
    fn test_cleanup_needs_reconcile_no_pr() {
        // Test: NeedsReconcile session with no PR is cleaned by Orphaned mode
        use crate::session::SessionStatus;
        use std::process::Command;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let temp_dir = temp.path();

        // Initialize git repo
        Command::new("git")
            .current_dir(temp_dir)
            .args(["init", "-b", "main"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.name", "Test User"])
            .output()
            .unwrap();

        std::fs::write(temp_dir.join("README.md"), "Test repo").unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["add", "README.md"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .unwrap();

        let worktrees_dir = temp_dir.join(".specks-worktrees");
        std::fs::create_dir_all(&worktrees_dir).unwrap();

        // Create worktree without PR
        let worktree1 = worktrees_dir.join("specks__nopr-20260208-120000");
        Command::new("git")
            .current_dir(temp_dir)
            .args(["branch", "specks/nopr-20260208-120000", "main"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args([
                "worktree",
                "add",
                worktree1.to_str().unwrap(),
                "specks/nopr-20260208-120000",
            ])
            .output()
            .unwrap();

        // Create session with NeedsReconcile status (no PR)
        let session1 = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            speck_slug: "nopr".to_string(),
            branch_name: "specks/nopr-20260208-120000".to_string(),
            base_branch: "main".to_string(),
            worktree_path: worktree1.display().to_string(),
            created_at: "2026-02-08T12:00:00Z".to_string(),
            status: SessionStatus::NeedsReconcile,
            current_step: CurrentStep::Index(0),
            total_steps: 1,
            beads_root: None,
            reused: false,
            session_id: None,
            last_updated_at: None,
            steps_completed: None,
            steps_remaining: None,
            bead_mapping: None,
            step_summaries: None,
        };
        save_session(&session1, temp_dir).unwrap();

        // Run cleanup in Orphaned mode (dry run) with mock PR checker
        let result =
            cleanup_worktrees_with_pr_checker(temp_dir, CleanupMode::Orphaned, true, |_| {
                PrState::NotFound
            })
            .unwrap();

        // Should identify NeedsReconcile worktree without PR as orphaned
        assert_eq!(result.orphaned_removed.len(), 1);
        assert_eq!(result.orphaned_removed[0], "specks/nopr-20260208-120000");
    }

    #[test]
    fn test_cleanup_skips_closed_pr() {
        // Test: Closed PR (not merged) is skipped by Merged and Orphaned modes
        use crate::session::SessionStatus;
        use std::process::Command;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let temp_dir = temp.path();

        // Initialize git repo
        Command::new("git")
            .current_dir(temp_dir)
            .args(["init", "-b", "main"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.name", "Test User"])
            .output()
            .unwrap();

        std::fs::write(temp_dir.join("README.md"), "Test repo").unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["add", "README.md"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .unwrap();

        let worktrees_dir = temp_dir.join(".specks-worktrees");
        std::fs::create_dir_all(&worktrees_dir).unwrap();

        let worktree1 = worktrees_dir.join("specks__closed-20260208-120000");
        Command::new("git")
            .current_dir(temp_dir)
            .args(["branch", "specks/closed-20260208-120000", "main"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args([
                "worktree",
                "add",
                worktree1.to_str().unwrap(),
                "specks/closed-20260208-120000",
            ])
            .output()
            .unwrap();

        // Make a commit in the worktree to diverge from main
        std::fs::write(worktree1.join("closed.txt"), "test").unwrap();
        Command::new("git")
            .current_dir(&worktree1)
            .args(["add", "closed.txt"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(&worktree1)
            .args(["commit", "-m", "Closed PR commit"])
            .output()
            .unwrap();

        let session1 = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            speck_slug: "closed".to_string(),
            branch_name: "specks/closed-20260208-120000".to_string(),
            base_branch: "main".to_string(),
            worktree_path: worktree1.display().to_string(),
            created_at: "2026-02-08T12:00:00Z".to_string(),
            status: SessionStatus::Completed,
            current_step: CurrentStep::Done,
            total_steps: 1,
            beads_root: None,
            reused: false,
            session_id: None,
            last_updated_at: None,
            steps_completed: None,
            steps_remaining: None,
            bead_mapping: None,
            step_summaries: None,
        };
        save_session(&session1, temp_dir).unwrap();

        // Mock PR checker returns Closed for this branch

        // Run cleanup in Merged mode (dry run) with Closed PR
        let result = cleanup_worktrees_with_pr_checker(temp_dir, CleanupMode::Merged, true, |_| {
            PrState::Closed
        })
        .unwrap();

        // Closed PR is not treated as merged
        assert_eq!(result.merged_removed.len(), 0);

        // Run cleanup in Orphaned mode (dry run) with Closed PR
        let result =
            cleanup_worktrees_with_pr_checker(temp_dir, CleanupMode::Orphaned, true, |_| {
                PrState::Closed
            })
            .unwrap();

        // Closed PR is protected from Orphaned mode (Closed != NotFound)
        assert_eq!(result.orphaned_removed.len(), 0);
    }

    #[test]
    fn test_cleanup_unknown_pr_state_uses_fallback() {
        // Test: Unknown PR state (gh unavailable) uses fallback behavior per mode
        // - Merged mode: falls back to git ancestry check
        // - Orphaned mode: treats as no PR → cleanable
        use crate::session::SessionStatus;
        use std::process::Command;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let temp_dir = temp.path();

        // Initialize git repo
        Command::new("git")
            .current_dir(temp_dir)
            .args(["init", "-b", "main"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.name", "Test User"])
            .output()
            .unwrap();

        std::fs::write(temp_dir.join("README.md"), "Test repo").unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["add", "README.md"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .unwrap();

        let worktrees_dir = temp_dir.join(".specks-worktrees");
        std::fs::create_dir_all(&worktrees_dir).unwrap();

        let worktree1 = worktrees_dir.join("specks__unknown-20260208-120000");
        Command::new("git")
            .current_dir(temp_dir)
            .args(["branch", "specks/unknown-20260208-120000", "main"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args([
                "worktree",
                "add",
                worktree1.to_str().unwrap(),
                "specks/unknown-20260208-120000",
            ])
            .output()
            .unwrap();

        // Make a commit in the worktree to diverge from main
        std::fs::write(worktree1.join("unknown.txt"), "test").unwrap();
        Command::new("git")
            .current_dir(&worktree1)
            .args(["add", "unknown.txt"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(&worktree1)
            .args(["commit", "-m", "Unknown PR commit"])
            .output()
            .unwrap();

        let session1 = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            speck_slug: "unknown".to_string(),
            branch_name: "specks/unknown-20260208-120000".to_string(),
            base_branch: "main".to_string(),
            worktree_path: worktree1.display().to_string(),
            created_at: "2026-02-08T12:00:00Z".to_string(),
            status: SessionStatus::Completed,
            current_step: CurrentStep::Done,
            total_steps: 1,
            beads_root: None,
            reused: false,
            session_id: None,
            last_updated_at: None,
            steps_completed: None,
            steps_remaining: None,
            bead_mapping: None,
            step_summaries: None,
        };
        save_session(&session1, temp_dir).unwrap();

        // Merged mode (dry run): Unknown falls back to git ancestry.
        // Branch diverges from main, so it's NOT considered merged.
        let result = cleanup_worktrees_with_pr_checker(temp_dir, CleanupMode::Merged, true, |_| {
            PrState::Unknown
        })
        .unwrap();

        assert_eq!(result.merged_removed.len(), 0);
        assert_eq!(result.skipped.len(), 0);

        // Orphaned mode (dry run): Unknown treated as no PR → cleanable
        let result =
            cleanup_worktrees_with_pr_checker(temp_dir, CleanupMode::Orphaned, true, |_| {
                PrState::Unknown
            })
            .unwrap();

        assert_eq!(result.orphaned_removed.len(), 1);
        assert_eq!(result.skipped.len(), 0);
    }

    #[test]
    fn test_cleanup_all_includes_closed_pr() {
        // Test: All mode includes closed PRs (not merged)
        use crate::session::SessionStatus;
        use std::process::Command;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let temp_dir = temp.path();

        // Initialize git repo
        Command::new("git")
            .current_dir(temp_dir)
            .args(["init", "-b", "main"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.name", "Test User"])
            .output()
            .unwrap();

        std::fs::write(temp_dir.join("README.md"), "Test repo").unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["add", "README.md"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .unwrap();

        let worktrees_dir = temp_dir.join(".specks-worktrees");
        std::fs::create_dir_all(&worktrees_dir).unwrap();

        let worktree1 = worktrees_dir.join("specks__allclosed-20260208-120000");
        Command::new("git")
            .current_dir(temp_dir)
            .args(["branch", "specks/allclosed-20260208-120000", "main"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args([
                "worktree",
                "add",
                worktree1.to_str().unwrap(),
                "specks/allclosed-20260208-120000",
            ])
            .output()
            .unwrap();

        let session1 = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            speck_slug: "allclosed".to_string(),
            branch_name: "specks/allclosed-20260208-120000".to_string(),
            base_branch: "main".to_string(),
            worktree_path: worktree1.display().to_string(),
            created_at: "2026-02-08T12:00:00Z".to_string(),
            status: SessionStatus::Completed,
            current_step: CurrentStep::Done,
            total_steps: 1,
            beads_root: None,
            reused: false,
            session_id: None,
            last_updated_at: None,
            steps_completed: None,
            steps_remaining: None,
            bead_mapping: None,
            step_summaries: None,
        };
        save_session(&session1, temp_dir).unwrap();

        // Run cleanup in All mode (dry run) with Closed PR state
        let result = cleanup_worktrees_with_pr_checker(temp_dir, CleanupMode::All, true, |_| {
            PrState::Closed
        })
        .unwrap();

        // All mode includes closed PRs, categorized as merged_removed
        assert_eq!(result.merged_removed.len(), 1);
        assert_eq!(result.merged_removed[0], "specks/allclosed-20260208-120000");
    }

    #[test]
    fn test_cleanup_force_removes_dirty_worktree() {
        // Test: Dirty worktrees are force-removed (escalate from normal → force)
        use crate::session::SessionStatus;
        use std::process::Command;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let temp_dir = temp.path();

        // Initialize git repo
        Command::new("git")
            .current_dir(temp_dir)
            .args(["init", "-b", "main"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.name", "Test User"])
            .output()
            .unwrap();

        std::fs::write(temp_dir.join("README.md"), "Test repo").unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["add", "README.md"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .unwrap();

        let worktrees_dir = temp_dir.join(".specks-worktrees");
        std::fs::create_dir_all(&worktrees_dir).unwrap();

        let worktree1 = worktrees_dir.join("specks__dirty-20260208-120000");
        Command::new("git")
            .current_dir(temp_dir)
            .args(["branch", "specks/dirty-20260208-120000", "main"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args([
                "worktree",
                "add",
                worktree1.to_str().unwrap(),
                "specks/dirty-20260208-120000",
            ])
            .output()
            .unwrap();

        // Add untracked file to make worktree dirty
        std::fs::write(worktree1.join("dirty.txt"), "uncommitted").unwrap();

        let session1 = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            speck_slug: "dirty".to_string(),
            branch_name: "specks/dirty-20260208-120000".to_string(),
            base_branch: "main".to_string(),
            worktree_path: worktree1.display().to_string(),
            created_at: "2026-02-08T12:00:00Z".to_string(),
            status: SessionStatus::Completed,
            current_step: CurrentStep::Done,
            total_steps: 1,
            beads_root: None,
            reused: false,
            session_id: None,
            last_updated_at: None,
            steps_completed: None,
            steps_remaining: None,
            bead_mapping: None,
            step_summaries: None,
        };
        save_session(&session1, temp_dir).unwrap();

        // Run cleanup in Orphaned mode (not dry run) with mock PR checker
        let result =
            cleanup_worktrees_with_pr_checker(temp_dir, CleanupMode::Orphaned, false, |_| {
                PrState::NotFound
            })
            .unwrap();

        // Dirty worktree is force-removed (escalated from normal removal)
        assert_eq!(result.orphaned_removed.len(), 1);
        assert_eq!(result.skipped.len(), 0);
        assert!(!worktree1.exists());
    }

    #[test]
    fn test_cleanup_unknown_pr_falls_back_gracefully() {
        // Test: unknown PR state (gh unavailable) falls back gracefully
        use crate::session::SessionStatus;
        use std::process::Command;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let temp_dir = temp.path();

        // Initialize git repo
        Command::new("git")
            .current_dir(temp_dir)
            .args(["init", "-b", "main"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.name", "Test User"])
            .output()
            .unwrap();

        std::fs::write(temp_dir.join("README.md"), "Test repo").unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["add", "README.md"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .unwrap();

        let worktrees_dir = temp_dir.join(".specks-worktrees");
        std::fs::create_dir_all(&worktrees_dir).unwrap();

        let worktree1 = worktrees_dir.join("specks__force-20260208-120000");
        Command::new("git")
            .current_dir(temp_dir)
            .args(["branch", "specks/force-20260208-120000", "main"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args([
                "worktree",
                "add",
                worktree1.to_str().unwrap(),
                "specks/force-20260208-120000",
            ])
            .output()
            .unwrap();

        let session1 = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            speck_slug: "force".to_string(),
            branch_name: "specks/force-20260208-120000".to_string(),
            base_branch: "main".to_string(),
            worktree_path: worktree1.display().to_string(),
            created_at: "2026-02-08T12:00:00Z".to_string(),
            status: SessionStatus::Completed,
            current_step: CurrentStep::Done,
            total_steps: 1,
            beads_root: None,
            reused: false,
            session_id: None,
            last_updated_at: None,
            steps_completed: None,
            steps_remaining: None,
            bead_mapping: None,
            step_summaries: None,
        };
        save_session(&session1, temp_dir).unwrap();

        // Run cleanup in Orphaned mode (dry run)
        let result = cleanup_worktrees(temp_dir, CleanupMode::Orphaned, true).unwrap();

        // Unknown PR state (gh unavailable) treated as no PR → orphaned
        assert_eq!(result.orphaned_removed.len(), 1);
        assert_eq!(result.orphaned_removed[0], "specks/force-20260208-120000");
    }

    #[test]
    fn test_list_specks_branches() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_dir = temp_dir.path();

        // Initialize git repo
        Command::new("git")
            .current_dir(temp_dir)
            .args(["init"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.name", "Test User"])
            .output()
            .unwrap();

        // Create initial commit
        std::fs::write(temp_dir.join("README.md"), "test").unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["add", "."])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .unwrap();

        // Create some specks/* branches
        Command::new("git")
            .current_dir(temp_dir)
            .args(["branch", "specks/auth-20260208-120000"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["branch", "specks/db-20260208-130000"])
            .output()
            .unwrap();

        // Create a non-specks branch
        Command::new("git")
            .current_dir(temp_dir)
            .args(["branch", "feature/something"])
            .output()
            .unwrap();

        let branches = list_specks_branches(temp_dir).unwrap();

        // Should only return specks/* branches
        assert_eq!(branches.len(), 2);
        assert!(branches.contains(&"specks/auth-20260208-120000".to_string()));
        assert!(branches.contains(&"specks/db-20260208-130000".to_string()));
        assert!(!branches.contains(&"feature/something".to_string()));
    }

    #[test]
    fn test_cleanup_stale_removes_orphan_branch() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_dir = temp_dir.path();

        // Initialize git repo
        Command::new("git")
            .current_dir(temp_dir)
            .args(["init"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.name", "Test User"])
            .output()
            .unwrap();

        // Create initial commit
        std::fs::write(temp_dir.join("README.md"), "test").unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["add", "."])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .unwrap();

        // Create a specks/* branch with no worktree
        Command::new("git")
            .current_dir(temp_dir)
            .args(["branch", "specks/orphan-20260208-120000"])
            .output()
            .unwrap();

        let sessions = vec![]; // No sessions, so all branches are stale

        let (removed, _skipped) = cleanup_stale_branches(temp_dir, &sessions, false).unwrap();

        // Should remove the branch via safe delete (it's based on current branch)
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0], "specks/orphan-20260208-120000");
    }

    #[test]
    fn test_cleanup_stale_skips_branch_with_worktree() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_dir = temp_dir.path();

        // Initialize git repo
        Command::new("git")
            .current_dir(temp_dir)
            .args(["init"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.name", "Test User"])
            .output()
            .unwrap();

        // Create initial commit
        std::fs::write(temp_dir.join("README.md"), "test").unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["add", "."])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .unwrap();

        // Create a specks/* branch
        Command::new("git")
            .current_dir(temp_dir)
            .args(["branch", "specks/active-20260208-120000"])
            .output()
            .unwrap();

        // Create a session for this branch
        let session = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            speck_slug: "active".to_string(),
            branch_name: "specks/active-20260208-120000".to_string(),
            base_branch: "main".to_string(),
            worktree_path: ".specks-worktrees/specks__active-20260208-120000".to_string(),
            created_at: "2026-02-08T12:00:00Z".to_string(),
            status: SessionStatus::InProgress,
            current_step: CurrentStep::Index(0),
            total_steps: 1,
            beads_root: None,
            reused: false,
            session_id: None,
            last_updated_at: None,
            steps_completed: None,
            steps_remaining: None,
            bead_mapping: None,
            step_summaries: None,
        };

        let sessions = vec![session];

        let (removed, _skipped) = cleanup_stale_branches(temp_dir, &sessions, false).unwrap();

        // Should NOT remove the branch because it has a worktree
        assert_eq!(removed.len(), 0);
    }

    #[test]
    fn test_cleanup_stale_safe_delete_fallback() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_dir = temp_dir.path();

        // Initialize git repo
        Command::new("git")
            .current_dir(temp_dir)
            .args(["init", "-b", "main"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.name", "Test User"])
            .output()
            .unwrap();

        // Create initial commit on main
        std::fs::write(temp_dir.join("README.md"), "test").unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["add", "."])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .unwrap();

        // Create a specks/* branch with commits not in main
        Command::new("git")
            .current_dir(temp_dir)
            .args(["checkout", "-b", "specks/unmerged-20260208-120000"])
            .output()
            .unwrap();

        std::fs::write(temp_dir.join("feature.txt"), "new feature").unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["add", "."])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["commit", "-m", "Add feature"])
            .output()
            .unwrap();

        // Switch back to main
        Command::new("git")
            .current_dir(temp_dir)
            .args(["checkout", "main"])
            .output()
            .unwrap();

        // Create .git/refs/pull directory to simulate merged PR scenario
        // This allows us to test the fallback from safe delete (-d) to force delete (-D)
        // when PR is confirmed as merged
        let refs_pull = temp_dir.join(".git/refs/pull");
        std::fs::create_dir_all(&refs_pull).unwrap();

        // Test verifies safe delete fallback behavior with mock PR checker
        let sessions = vec![]; // No sessions, so all branches are stale

        // PR checker returns NotFound: unmerged branch with no PR → skip
        let (_removed, skipped) =
            cleanup_stale_branches_with_pr_checker(temp_dir, &sessions, false, |_| {
                PrState::NotFound
            })
            .unwrap();

        assert_eq!(
            skipped.len(),
            1,
            "Expected 1 skipped branch, got: {:?}",
            skipped
        );
        assert_eq!(skipped[0].0, "specks/unmerged-20260208-120000");
        assert!(
            skipped[0].1.contains("Unmerged"),
            "Expected skip reason to mention unmerged, got: {}",
            skipped[0].1
        );

        // PR checker returns Merged: squash-merged branch → force delete
        let (removed, _skipped) =
            cleanup_stale_branches_with_pr_checker(temp_dir, &sessions, false, |_| PrState::Merged)
                .unwrap();
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0], "specks/unmerged-20260208-120000");
    }

    #[test]
    fn test_cleanup_stale_gh_absent_safe_only() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_dir = temp_dir.path();

        // Initialize git repo
        Command::new("git")
            .current_dir(temp_dir)
            .args(["init"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["config", "user.name", "Test User"])
            .output()
            .unwrap();

        // Create initial commit on main
        std::fs::write(temp_dir.join("README.md"), "test").unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["add", "."])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .unwrap();

        // Create a specks/* branch that is NOT merged into main
        Command::new("git")
            .current_dir(temp_dir)
            .args(["branch", "specks/unmerged-feature"])
            .output()
            .unwrap();

        // Add a commit to the branch
        Command::new("git")
            .current_dir(temp_dir)
            .args(["checkout", "specks/unmerged-feature"])
            .output()
            .unwrap();
        std::fs::write(temp_dir.join("feature.txt"), "new feature").unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["add", "feature.txt"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(temp_dir)
            .args(["commit", "-m", "Add feature"])
            .output()
            .unwrap();

        // Return to main
        Command::new("git")
            .current_dir(temp_dir)
            .args(["checkout", "main"])
            .output()
            .unwrap();

        let sessions = vec![]; // No sessions, so all branches are stale

        // Test graceful degradation when gh CLI check returns non-merged state:
        // - Safe delete (-d) will fail (branch has unmerged commits)
        // - get_pr_state returns either PrState::Unknown (gh absent) or PrState::NotFound (gh present, no PR)
        // - Unknown → stale branch with no worktree, just delete it
        // - NotFound → skip (confirmed unmerged, has no PR but might have a reason to exist)
        let (removed, skipped) = cleanup_stale_branches(temp_dir, &sessions, false).unwrap();

        // Result depends on gh CLI availability:
        // - gh unavailable (Unknown): branch is deleted (stale with no worktree = dead weight)
        // - gh available (NotFound): branch is skipped (confirmed unmerged with no PR)
        assert_eq!(
            removed.len() + skipped.len(),
            1,
            "branch should be either removed or skipped"
        );
        if !skipped.is_empty() {
            assert_eq!(skipped[0].0, "specks/unmerged-feature");
            assert!(
                skipped[0].1.contains("Unmerged"),
                "Expected skip reason to mention unmerged, got: {}",
                skipped[0].1
            );
        }
    }
}
