//! Merge command implementation
//!
//! Automates the post-implementation merge workflow:
//! - Commits infrastructure changes in main
//! - Verifies PR checks pass
//! - Merges PR via squash
//! - Cleans up worktree

use serde::{Deserialize, Serialize};
use specks_core::session::Session;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// JSON output for merge command
#[derive(Serialize)]
pub struct MergeData {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub infrastructure_committed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub infrastructure_files: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree_cleaned: Option<bool>,
    #[serde(skip_serializing_if = "is_false")]
    pub dry_run: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub would_commit: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub would_merge_pr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub would_cleanup_worktree: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

fn is_false(b: &bool) -> bool {
    !b
}

/// Information about a GitHub pull request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrInfo {
    pub number: u32,
    pub url: String,
    pub state: String,
}

/// Find the worktree directory and session for a given speck path
///
/// Searches all session.json files in .specks-worktrees/ directories
/// and returns the session that matches the given speck path.
///
/// # Arguments
/// * `speck_path` - Path to the speck file (can be relative or absolute)
///
/// # Returns
/// * `Ok((worktree_path, session))` - Worktree path and loaded session
/// * `Err(String)` - Error message if no matching worktree found
fn find_worktree_for_speck(speck_path: &str) -> Result<(PathBuf, Session), String> {
    // Normalize the speck path to handle both relative and absolute paths
    let normalized_speck = if let Some(stripped) = speck_path.strip_prefix("./") {
        stripped.to_string()
    } else if let Some(stripped) = speck_path.strip_prefix(".specks/") {
        format!(".specks/{}", stripped)
    } else if speck_path.starts_with(".specks/") {
        speck_path.to_string()
    } else {
        format!(".specks/{}", speck_path)
    };

    // Find the worktrees directory
    let worktrees_dir = Path::new(".specks-worktrees");
    if !worktrees_dir.exists() {
        return Err("No worktrees directory found (.specks-worktrees)".to_string());
    }

    // Scan all worktree directories for session.json files
    let entries = fs::read_dir(worktrees_dir)
        .map_err(|e| format!("Failed to read worktrees directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let worktree_path = entry.path();

        if !worktree_path.is_dir() {
            continue;
        }

        let session_path = worktree_path.join(".specks").join("session.json");
        if !session_path.exists() {
            continue;
        }

        // Try to load and parse the session
        let session_content = match fs::read_to_string(&session_path) {
            Ok(content) => content,
            Err(_) => continue, // Skip corrupt files
        };

        let session: Session = match serde_json::from_str(&session_content) {
            Ok(s) => s,
            Err(_) => continue, // Skip invalid JSON
        };

        // Check if this session matches our speck
        if session.speck_path == normalized_speck {
            return Ok((worktree_path, session));
        }
    }

    Err(format!(
        "No worktree found for speck: {}",
        normalized_speck
    ))
}

/// Get pull request information for a branch using gh CLI
///
/// Executes `gh pr view <branch> --json number,url,state` and parses the response.
///
/// # Arguments
/// * `branch` - Name of the branch to query
///
/// # Returns
/// * `Ok(PrInfo)` - PR information if found
/// * `Err(String)` - Error message if PR not found or gh CLI error
fn get_pr_for_branch(branch: &str) -> Result<PrInfo, String> {
    // Check if gh CLI is available
    let gh_check = Command::new("gh").arg("--version").output();

    if gh_check.is_err() {
        return Err(
            "gh CLI not found. Install from https://cli.github.com/".to_string()
        );
    }

    // Query PR information
    let output = Command::new("gh")
        .args(["pr", "view", branch, "--json", "number,url,state"])
        .output()
        .map_err(|e| format!("Failed to execute gh pr view: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("no pull requests found") {
            return Err(format!("No PR found for branch: {}", branch));
        }
        return Err(format!("gh pr view failed: {}", stderr));
    }

    // Parse the JSON response
    let stdout = String::from_utf8_lossy(&output.stdout);
    let pr_info: PrInfo = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse gh pr view output: {}", e))?;

    Ok(pr_info)
}

/// Infrastructure file patterns from Table T01
/// These files are auto-committed in main before merging PRs
// Will be used in Step 3 & 4 for merge workflow
#[allow(dead_code)]
const INFRASTRUCTURE_PATTERNS: &[&str] = &[
    "agents/",
    ".claude/skills/",
    ".specks/specks-skeleton.md",
    ".specks/config.toml",
    ".specks/specks-implementation-log.md",
    ".beads/",
    "CLAUDE.md",
];

/// Check if a file path matches infrastructure patterns
///
/// Returns true if the file is considered infrastructure (auto-committable in main).
/// Returns false for speck content files like `.specks/specks-123.md` (except skeleton and implementation-log).
///
/// # Arguments
/// * `path` - File path to check (relative to repo root)
// Will be used in Step 3 & 4 for merge workflow
#[allow(dead_code)]
fn is_infrastructure_file(path: &str) -> bool {
    // Special handling for .specks/ directory
    if path.starts_with(".specks/") {
        // Only skeleton and implementation-log are infrastructure
        return path == ".specks/specks-skeleton.md"
            || path == ".specks/config.toml"
            || path == ".specks/specks-implementation-log.md";
    }

    // Check against infrastructure patterns
    for pattern in INFRASTRUCTURE_PATTERNS {
        if pattern.ends_with('/') {
            // Directory pattern - check prefix
            if path.starts_with(pattern) {
                return true;
            }
        } else {
            // Exact match pattern
            if path == *pattern {
                return true;
            }
        }
    }

    false
}

/// Categorize uncommitted files from git status
///
/// Runs `git status --porcelain -u` and separates files into infrastructure vs other.
/// Infrastructure files can be auto-committed in main before merging.
/// Uses `-u` (--untracked-files=all) to show individual files instead of directories.
///
/// # Returns
/// * `Ok((infrastructure, other))` - Two lists of file paths
/// * `Err(String)` - Error message if git status fails
// Will be used in Step 3 & 4 for merge workflow
#[allow(dead_code)]
fn categorize_uncommitted() -> Result<(Vec<String>, Vec<String>), String> {
    let output = Command::new("git")
        .args(["status", "--porcelain", "-u"])
        .output()
        .map_err(|e| format!("Failed to execute git status: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git status failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut infrastructure = Vec::new();
    let mut other = Vec::new();

    for line in stdout.lines() {
        if line.len() < 4 {
            continue; // Skip malformed lines
        }

        // git status --porcelain format: "XY filename"
        // XY is a two-letter status code, then a space, then the filename
        let file_path = &line[3..];

        if is_infrastructure_file(file_path) {
            infrastructure.push(file_path.to_string());
        } else {
            other.push(file_path.to_string());
        }
    }

    Ok((infrastructure, other))
}

/// Run the merge command
///
/// Implements the full merge workflow with pre-merge validations,
/// infrastructure file auto-commit, PR merge, and worktree cleanup.
pub fn run_merge(
    speck: String,
    dry_run: bool,
    _force: bool,
    json: bool,
    quiet: bool,
) -> Result<i32, String> {
    // Step 1: Find the worktree for this speck
    let (worktree_path, session) = match find_worktree_for_speck(&speck) {
        Ok(result) => result,
        Err(e) => {
            let data = MergeData {
                status: "error".to_string(),
                pr_url: None,
                pr_number: None,
                branch_name: None,
                infrastructure_committed: None,
                infrastructure_files: None,
                worktree_cleaned: None,
                dry_run,
                would_commit: None,
                would_merge_pr: None,
                would_cleanup_worktree: None,
                error: Some(e.clone()),
                message: None,
            };

            if json {
                println!("{}", serde_json::to_string_pretty(&data).unwrap());
            } else if !quiet {
                eprintln!("Error: {}", e);
            }
            return Err(e);
        }
    };

    // Step 2: Get PR information for the branch
    let pr_info = match get_pr_for_branch(&session.branch_name) {
        Ok(info) => info,
        Err(e) => {
            let data = MergeData {
                status: "error".to_string(),
                pr_url: None,
                pr_number: None,
                branch_name: Some(session.branch_name.clone()),
                infrastructure_committed: None,
                infrastructure_files: None,
                worktree_cleaned: None,
                dry_run,
                would_commit: None,
                would_merge_pr: None,
                would_cleanup_worktree: None,
                error: Some(e.clone()),
                message: None,
            };

            if json {
                println!("{}", serde_json::to_string_pretty(&data).unwrap());
            } else if !quiet {
                eprintln!("Error: {}", e);
            }
            return Err(e);
        }
    };

    // Populate response with lookup results
    let data = MergeData {
        status: "ok".to_string(),
        pr_url: Some(pr_info.url.clone()),
        pr_number: Some(pr_info.number),
        branch_name: Some(session.branch_name.clone()),
        infrastructure_committed: None,
        infrastructure_files: None,
        worktree_cleaned: None,
        dry_run,
        would_commit: None,
        would_merge_pr: if dry_run {
            Some(pr_info.url.clone())
        } else {
            None
        },
        would_cleanup_worktree: if dry_run {
            Some(worktree_path.display().to_string())
        } else {
            None
        },
        error: None,
        message: Some(format!(
            "Found PR #{} ({}) for worktree at {}",
            pr_info.number,
            pr_info.state,
            worktree_path.display()
        )),
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&data).unwrap());
    } else if !quiet {
        println!("Found worktree: {}", worktree_path.display());
        println!("Branch: {}", session.branch_name);
        println!("PR: #{} - {}", pr_info.number, pr_info.url);
        println!("State: {}", pr_info.state);
    }

    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use specks_core::session::{Session, SessionStatus};
    use std::fs;

    #[test]
    fn test_pr_info_deserialization() {
        // Test parsing gh pr view JSON output
        let json = r#"{
            "number": 123,
            "url": "https://github.com/owner/repo/pull/123",
            "state": "OPEN"
        }"#;

        let pr_info: PrInfo = serde_json::from_str(json).unwrap();
        assert_eq!(pr_info.number, 123);
        assert_eq!(pr_info.url, "https://github.com/owner/repo/pull/123");
        assert_eq!(pr_info.state, "OPEN");
    }

    #[test]
    fn test_pr_info_deserialization_merged() {
        let json = r#"{
            "number": 456,
            "url": "https://github.com/owner/repo/pull/456",
            "state": "MERGED"
        }"#;

        let pr_info: PrInfo = serde_json::from_str(json).unwrap();
        assert_eq!(pr_info.number, 456);
        assert_eq!(pr_info.state, "MERGED");
    }

    #[test]
    fn test_pr_info_deserialization_closed() {
        let json = r#"{
            "number": 789,
            "url": "https://github.com/owner/repo/pull/789",
            "state": "CLOSED"
        }"#;

        let pr_info: PrInfo = serde_json::from_str(json).unwrap();
        assert_eq!(pr_info.number, 789);
        assert_eq!(pr_info.state, "CLOSED");
    }

    #[test]
    fn test_find_worktree_missing_directory() {
        // Create a temporary directory that definitely won't have worktrees
        let temp_dir = std::env::temp_dir().join(format!(
            "specks-test-no-worktrees-{}",
            std::process::id()
        ));
        let original_dir = std::env::current_dir().unwrap();

        // Create and change to temp directory
        fs::create_dir_all(&temp_dir).unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Should fail because .specks-worktrees doesn't exist
        let result = find_worktree_for_speck(".specks/specks-test.md");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("No worktrees directory found"));

        // Cleanup
        std::env::set_current_dir(original_dir).unwrap();
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_find_worktree_no_matching_speck() {
        // Create a temporary test environment
        let temp_dir = std::env::temp_dir().join(format!(
            "specks-test-no-match-{}",
            std::process::id()
        ));
        let original_dir = std::env::current_dir().unwrap();

        // Create directory structure
        fs::create_dir_all(&temp_dir).unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let worktrees_dir = temp_dir.join(".specks-worktrees");
        let worktree1 = worktrees_dir.join("specks__test1-20260209-120000");
        let specks_dir1 = worktree1.join(".specks");
        fs::create_dir_all(&specks_dir1).unwrap();

        // Create a session for a different speck
        let session = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-other.md".to_string(),
            speck_slug: "other".to_string(),
            branch_name: "specks/other-20260209-120000".to_string(),
            base_branch: "main".to_string(),
            worktree_path: worktree1.display().to_string(),
            created_at: "2026-02-09T12:00:00Z".to_string(),
            status: SessionStatus::InProgress,
            current_step: 0,
            total_steps: 3,
            beads_root: None,
        };

        let session_json = serde_json::to_string_pretty(&session).unwrap();
        fs::write(specks_dir1.join("session.json"), session_json).unwrap();

        // Try to find a different speck
        let result = find_worktree_for_speck(".specks/specks-test.md");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No worktree found"));

        // Cleanup
        std::env::set_current_dir(original_dir).unwrap();
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_find_worktree_success() {
        // Create a temporary test environment
        let temp_dir = std::env::temp_dir().join(format!(
            "specks-test-success-{}",
            std::process::id()
        ));
        let original_dir = std::env::current_dir().unwrap();

        // Create directory structure
        fs::create_dir_all(&temp_dir).unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let worktrees_dir = temp_dir.join(".specks-worktrees");
        let worktree1 = worktrees_dir.join("specks__test-20260209-120000");
        let specks_dir1 = worktree1.join(".specks");
        fs::create_dir_all(&specks_dir1).unwrap();

        // Create a matching session
        let session = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            speck_slug: "test".to_string(),
            branch_name: "specks/test-20260209-120000".to_string(),
            base_branch: "main".to_string(),
            worktree_path: worktree1.display().to_string(),
            created_at: "2026-02-09T12:00:00Z".to_string(),
            status: SessionStatus::InProgress,
            current_step: 0,
            total_steps: 3,
            beads_root: None,
        };

        let session_json = serde_json::to_string_pretty(&session).unwrap();
        fs::write(specks_dir1.join("session.json"), session_json).unwrap();

        // Find the worktree
        let result = find_worktree_for_speck(".specks/specks-test.md");
        assert!(result.is_ok());

        let (path, loaded_session) = result.unwrap();
        // Path should end with the worktree directory name
        assert!(path.ends_with("specks__test-20260209-120000"));
        assert_eq!(loaded_session.speck_path, ".specks/specks-test.md");
        assert_eq!(loaded_session.branch_name, "specks/test-20260209-120000");

        // Cleanup
        std::env::set_current_dir(original_dir).unwrap();
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_find_worktree_path_normalization() {
        // Create a temporary test environment
        let temp_dir = std::env::temp_dir().join(format!(
            "specks-test-norm-{}",
            std::process::id()
        ));
        let original_dir = std::env::current_dir().unwrap();

        // Create directory structure
        fs::create_dir_all(&temp_dir).unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let worktrees_dir = temp_dir.join(".specks-worktrees");
        let worktree1 = worktrees_dir.join("specks__test-20260209-120000");
        let specks_dir1 = worktree1.join(".specks");
        fs::create_dir_all(&specks_dir1).unwrap();

        // Create a session
        let session = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            speck_slug: "test".to_string(),
            branch_name: "specks/test-20260209-120000".to_string(),
            base_branch: "main".to_string(),
            worktree_path: worktree1.display().to_string(),
            created_at: "2026-02-09T12:00:00Z".to_string(),
            status: SessionStatus::InProgress,
            current_step: 0,
            total_steps: 3,
            beads_root: None,
        };

        let session_json = serde_json::to_string_pretty(&session).unwrap();
        fs::write(specks_dir1.join("session.json"), session_json).unwrap();

        // Test various path formats
        let result1 = find_worktree_for_speck(".specks/specks-test.md");
        assert!(result1.is_ok());

        let result2 = find_worktree_for_speck("specks-test.md");
        assert!(result2.is_ok());

        let result3 = find_worktree_for_speck("./.specks/specks-test.md");
        assert!(result3.is_ok());

        // Cleanup
        std::env::set_current_dir(original_dir).unwrap();
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_find_worktree_corrupt_session() {
        // Create a temporary test environment
        let temp_dir = std::env::temp_dir().join(format!(
            "specks-test-corrupt-{}",
            std::process::id()
        ));
        let original_dir = std::env::current_dir().unwrap();

        // Create directory structure
        fs::create_dir_all(&temp_dir).unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let worktrees_dir = temp_dir.join(".specks-worktrees");
        let worktree1 = worktrees_dir.join("specks__corrupt-20260209-120000");
        let specks_dir1 = worktree1.join(".specks");
        fs::create_dir_all(&specks_dir1).unwrap();

        // Write corrupt JSON
        fs::write(
            specks_dir1.join("session.json"),
            "{ invalid json here",
        )
        .unwrap();

        // Create a valid worktree
        let worktree2 = worktrees_dir.join("specks__test-20260209-120000");
        let specks_dir2 = worktree2.join(".specks");
        fs::create_dir_all(&specks_dir2).unwrap();

        let session = Session {
            schema_version: "1".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            speck_slug: "test".to_string(),
            branch_name: "specks/test-20260209-120000".to_string(),
            base_branch: "main".to_string(),
            worktree_path: worktree2.display().to_string(),
            created_at: "2026-02-09T12:00:00Z".to_string(),
            status: SessionStatus::InProgress,
            current_step: 0,
            total_steps: 3,
            beads_root: None,
        };

        let session_json = serde_json::to_string_pretty(&session).unwrap();
        fs::write(specks_dir2.join("session.json"), session_json).unwrap();

        // Should skip corrupt session and find the valid one
        let result = find_worktree_for_speck(".specks/specks-test.md");
        assert!(result.is_ok());

        // Cleanup
        std::env::set_current_dir(original_dir).unwrap();
        let _ = fs::remove_dir_all(&temp_dir);
    }

    // Tests for is_infrastructure_file() covering all patterns in Table T01
    #[test]
    fn test_is_infrastructure_file_agents() {
        assert!(is_infrastructure_file("agents/coder-agent.md"));
        assert!(is_infrastructure_file("agents/architect-agent.md"));
        assert!(is_infrastructure_file("agents/subdir/some-agent.md"));
    }

    #[test]
    fn test_is_infrastructure_file_skills() {
        assert!(is_infrastructure_file(".claude/skills/planner/SKILL.md"));
        assert!(is_infrastructure_file(".claude/skills/implementer/SKILL.md"));
        assert!(is_infrastructure_file(".claude/skills/foo/bar/baz.txt"));
    }

    #[test]
    fn test_is_infrastructure_file_specks_skeleton() {
        assert!(is_infrastructure_file(".specks/specks-skeleton.md"));
    }

    #[test]
    fn test_is_infrastructure_file_config_toml() {
        assert!(is_infrastructure_file(".specks/config.toml"));
    }

    #[test]
    fn test_is_infrastructure_file_implementation_log() {
        assert!(is_infrastructure_file(
            ".specks/specks-implementation-log.md"
        ));
    }

    #[test]
    fn test_is_infrastructure_file_beads() {
        assert!(is_infrastructure_file(".beads/beads.json"));
        assert!(is_infrastructure_file(".beads/metadata/bd-123.json"));
    }

    #[test]
    fn test_is_infrastructure_file_claude_md() {
        assert!(is_infrastructure_file("CLAUDE.md"));
    }

    #[test]
    fn test_is_infrastructure_file_speck_content_not_infrastructure() {
        // Speck content files should NOT be infrastructure
        assert!(!is_infrastructure_file(".specks/specks-1.md"));
        assert!(!is_infrastructure_file(".specks/specks-123.md"));
        assert!(!is_infrastructure_file(".specks/specks-auth.md"));
        assert!(!is_infrastructure_file(".specks/specks-feature-name.md"));
    }

    #[test]
    fn test_is_infrastructure_file_other_files_not_infrastructure() {
        // Regular source files should NOT be infrastructure
        assert!(!is_infrastructure_file("src/main.rs"));
        assert!(!is_infrastructure_file("crates/specks/src/lib.rs"));
        assert!(!is_infrastructure_file("README.md"));
        assert!(!is_infrastructure_file("Cargo.toml"));
        assert!(!is_infrastructure_file("tests/integration_test.rs"));
    }

    #[test]
    fn test_is_infrastructure_file_edge_cases() {
        // Files that might look like infrastructure but aren't
        assert!(!is_infrastructure_file("agents-copy/file.md"));
        assert!(!is_infrastructure_file("my-agents/file.md"));
        assert!(!is_infrastructure_file(".specks-backup/specks-skeleton.md"));
        assert!(!is_infrastructure_file("docs/CLAUDE.md"));
    }

    #[test]
    fn test_categorize_uncommitted_integration() {
        use std::process::Command;

        // Create a temporary test git repository
        let temp_dir = std::env::temp_dir().join(format!(
            "specks-test-categorize-{}",
            std::process::id()
        ));
        let original_dir = std::env::current_dir().unwrap();

        // Cleanup if exists from previous run
        let _ = fs::remove_dir_all(&temp_dir);

        // Create directory structure
        fs::create_dir_all(&temp_dir).unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Initialize git repo
        Command::new("git")
            .args(["init"])
            .output()
            .expect("Failed to init git repo");

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .output()
            .expect("Failed to configure git");

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .output()
            .expect("Failed to configure git");

        // Create initial commit to have a HEAD
        fs::write("README.md", "Test repo").unwrap();
        Command::new("git")
            .args(["add", "README.md"])
            .output()
            .expect("Failed to add README");

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .output()
            .expect("Failed to commit");

        // Create mixed uncommitted files
        // Infrastructure files
        fs::create_dir_all("agents").unwrap();
        fs::write("agents/test-agent.md", "# Agent").unwrap();

        fs::create_dir_all(".claude/skills/test").unwrap();
        fs::write(".claude/skills/test/SKILL.md", "# Skill").unwrap();

        fs::create_dir_all(".specks").unwrap();
        fs::write(".specks/specks-skeleton.md", "# Skeleton").unwrap();
        fs::write(".specks/config.toml", "# Config").unwrap();
        fs::write(".specks/specks-implementation-log.md", "# Log").unwrap();

        fs::create_dir_all(".beads").unwrap();
        fs::write(".beads/beads.json", "{}").unwrap();

        fs::write("CLAUDE.md", "# Claude").unwrap();

        // Non-infrastructure files
        fs::create_dir_all("src").unwrap();
        fs::write("src/main.rs", "fn main() {}").unwrap();

        fs::write(".specks/specks-123.md", "# Speck 123").unwrap();

        fs::write("Cargo.toml", "[package]").unwrap();

        // Run categorization
        let result = categorize_uncommitted();
        assert!(result.is_ok());

        let (infrastructure, other) = result.unwrap();

        // Verify infrastructure files
        assert!(infrastructure.contains(&"agents/test-agent.md".to_string()));
        assert!(infrastructure.contains(&".claude/skills/test/SKILL.md".to_string()));
        assert!(infrastructure.contains(&".specks/specks-skeleton.md".to_string()));
        assert!(infrastructure.contains(&".specks/config.toml".to_string()));
        assert!(infrastructure.contains(&".specks/specks-implementation-log.md".to_string()));
        assert!(infrastructure.contains(&".beads/beads.json".to_string()));
        assert!(infrastructure.contains(&"CLAUDE.md".to_string()));

        // Verify non-infrastructure files
        assert!(other.contains(&"src/main.rs".to_string()));
        assert!(other.contains(&".specks/specks-123.md".to_string()));
        assert!(other.contains(&"Cargo.toml".to_string()));

        // Verify counts
        assert_eq!(infrastructure.len(), 7);
        assert_eq!(other.len(), 3);

        // Cleanup
        std::env::set_current_dir(original_dir).unwrap();
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
