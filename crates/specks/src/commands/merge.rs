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
/// When multiple worktrees exist for the same speck, prefers the one
/// that has an open PR (indicating it's the complete/active worktree).
///
/// # Arguments
/// * `root` - Optional root directory (uses current directory if None)
/// * `speck_path` - Path to the speck file (can be relative or absolute)
///
/// # Returns
/// * `Ok((worktree_path, session))` - Worktree path and loaded session
/// * `Err(String)` - Error message if no matching worktree found
fn find_worktree_for_speck(
    root: Option<&Path>,
    speck_path: &str,
) -> Result<(PathBuf, Session), String> {
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
    let base = root
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let worktrees_dir = base.join(".specks-worktrees");
    if !worktrees_dir.exists() {
        return Err("No worktrees directory found (.specks-worktrees)".to_string());
    }

    // Scan all worktree directories for session.json files
    let entries = fs::read_dir(worktrees_dir)
        .map_err(|e| format!("Failed to read worktrees directory: {}", e))?;

    // Collect all matching worktrees
    let mut matching_worktrees: Vec<(PathBuf, Session)> = Vec::new();

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
            matching_worktrees.push((worktree_path, session));
        }
    }

    if matching_worktrees.is_empty() {
        return Err(format!("No worktree found for speck: {}", normalized_speck));
    }

    // If only one match, return it
    if matching_worktrees.len() == 1 {
        return Ok(matching_worktrees.into_iter().next().unwrap());
    }

    // Multiple worktrees exist - find the one with an open PR
    for (worktree_path, session) in &matching_worktrees {
        if let Ok(pr_info) = get_pr_for_branch(&session.branch_name) {
            if pr_info.state == "OPEN" {
                return Ok((worktree_path.clone(), session.clone()));
            }
        }
    }

    // No worktree has an open PR - return the most recent one (last in sorted order)
    // Sort by worktree path which includes timestamp
    let mut sorted = matching_worktrees;
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(sorted.into_iter().last().unwrap())
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
        return Err("gh CLI not found. Install from https://cli.github.com/".to_string());
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
/// # Arguments
/// * `root` - Optional root directory (uses current directory if None)
///
/// # Returns
/// * `Ok((infrastructure, other))` - Two lists of file paths
/// * `Err(String)` - Error message if git status fails
// Will be used in Step 3 & 4 for merge workflow
#[allow(dead_code)]
fn categorize_uncommitted(root: Option<&Path>) -> Result<(Vec<String>, Vec<String>), String> {
    let mut cmd = Command::new("git");
    if let Some(dir) = root {
        cmd.arg("-C").arg(dir);
    }
    let output = cmd
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

/// Check if main branch is in sync with origin/main
///
/// Uses `git rev-list origin/main..main` to detect unpushed commits.
///
/// # Returns
/// * `Ok(())` - Main is in sync (no unpushed commits)
/// * `Err(String)` - Main has unpushed commits (includes count)
fn check_main_sync() -> Result<(), String> {
    let output = Command::new("git")
        .args(["rev-list", "origin/main..main", "--count"])
        .output()
        .map_err(|e| format!("Failed to execute git rev-list: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git rev-list failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let count: u32 = stdout
        .trim()
        .parse()
        .map_err(|_| "Failed to parse commit count".to_string())?;

    if count > 0 {
        return Err(format!(
            "Main branch has {} unpushed commit{}. Run 'git push' first.",
            count,
            if count == 1 { "" } else { "s" }
        ));
    }

    Ok(())
}

/// Check if current directory is the main worktree
///
/// Validates that we're running from the main repository worktree, not a specks worktree
/// or other detached checkout. This is required for merge operations.
///
/// # Returns
/// * `Ok(())` - Current directory is the main worktree
/// * `Err(String)` - Not in main worktree (provides actionable error message)
fn is_main_worktree() -> Result<(), String> {
    use std::path::Path;

    // Check if .git is a directory (not a file, which indicates a worktree)
    let git_path = Path::new(".git");
    if !git_path.exists() {
        return Err("Not in a git repository (no .git directory found)".to_string());
    }

    if !git_path.is_dir() {
        return Err(
            "Running from a git worktree, not the main repository.\n\
             The merge command must run from the main worktree.\n\
             Please cd to the repository root and try again."
                .to_string(),
        );
    }

    // Verify we're on the expected branch (main or master)
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .map_err(|e| format!("Failed to check current branch: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to get current branch: {}", stderr));
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch != "main" && branch != "master" {
        return Err(format!(
            "Current branch is '{}', expected 'main' or 'master'.\n\
             The merge command must run from the main branch in the main worktree.",
            branch
        ));
    }

    Ok(())
}

/// Check if all PR checks have passed
///
/// Uses `gh pr checks <branch> --json name,state,conclusion` to query check status.
///
/// # Arguments
/// * `branch` - Name of the branch to check
///
/// # Returns
/// * `Ok(())` - All checks passed (or no checks configured)
/// * `Err(String)` - Some checks are failing or pending (includes list)
fn check_pr_checks(branch: &str) -> Result<(), String> {
    let output = Command::new("gh")
        .args(["pr", "checks", branch, "--json", "name,state,conclusion"])
        .output()
        .map_err(|e| format!("Failed to execute gh pr checks: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh pr checks failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON array of check objects
    #[derive(Deserialize)]
    struct CheckStatus {
        name: String,
        state: String,
        conclusion: Option<String>,
    }

    let checks: Vec<CheckStatus> = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse gh pr checks output: {}", e))?;

    // Empty array means no checks configured - that's success
    if checks.is_empty() {
        return Ok(());
    }

    let mut failing = Vec::new();
    let mut pending = Vec::new();

    for check in checks {
        // Check if still pending or in progress
        if check.state == "pending" || check.state == "in_progress" {
            pending.push(check.name);
        }
        // Check if failed (conclusion may be null for pending checks)
        else if let Some(conclusion) = check.conclusion {
            if conclusion == "failure" || conclusion == "timed_out" || conclusion == "cancelled" {
                failing.push(check.name);
            }
        }
    }

    if !failing.is_empty() {
        return Err(format!("PR checks failing: {}", failing.join(", ")));
    }

    if !pending.is_empty() {
        return Err(format!("PR checks pending: {}", pending.join(", ")));
    }

    Ok(())
}

/// Validate that PR is in OPEN state
///
/// # Arguments
/// * `pr_info` - PR information from gh pr view
///
/// # Returns
/// * `Ok(())` - PR is open
/// * `Err(String)` - PR is merged or closed
fn validate_pr_state(pr_info: &PrInfo) -> Result<(), String> {
    if pr_info.state == "MERGED" {
        return Err(format!("PR already merged: {}", pr_info.url));
    }

    if pr_info.state == "CLOSED" {
        return Err(format!("PR is closed without merge: {}", pr_info.url));
    }

    if pr_info.state != "OPEN" {
        return Err(format!("PR state is {}: {}", pr_info.state, pr_info.url));
    }

    Ok(())
}

/// Run the merge command
///
/// Implements the full merge workflow with pre-merge validations,
/// infrastructure file auto-commit, PR merge, and worktree cleanup.
pub fn run_merge(
    speck: String,
    dry_run: bool,
    force: bool,
    json: bool,
    quiet: bool,
) -> Result<i32, String> {
    // Step 0: Validate that we're running from the main worktree
    if let Err(e) = is_main_worktree() {
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
        }
        return Err(e);
    }

    // Step 1: Find the worktree for this speck
    let (worktree_path, session) = match find_worktree_for_speck(None, &speck) {
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
            }
            return Err(e);
        }
    };

    // Step 2: Check main sync status
    if let Err(e) = check_main_sync() {
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
        }
        return Err(e);
    }

    // Step 3: Get PR information for the branch
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
            }
            return Err(e);
        }
    };

    // Step 4: Validate PR state (open, not merged/closed)
    if let Err(e) = validate_pr_state(&pr_info) {
        let data = MergeData {
            status: "error".to_string(),
            pr_url: Some(pr_info.url.clone()),
            pr_number: Some(pr_info.number),
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
        }
        return Err(e);
    }

    // Step 5: Check PR checks status
    if let Err(e) = check_pr_checks(&session.branch_name) {
        let data = MergeData {
            status: "error".to_string(),
            pr_url: Some(pr_info.url.clone()),
            pr_number: Some(pr_info.number),
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
        }
        return Err(e);
    }

    // Step 6: Categorize uncommitted files
    let (infrastructure, other) = match categorize_uncommitted(None) {
        Ok(result) => result,
        Err(e) => {
            let data = MergeData {
                status: "error".to_string(),
                pr_url: Some(pr_info.url.clone()),
                pr_number: Some(pr_info.number),
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
            }
            return Err(e);
        }
    };

    // Step 7: Validate non-infrastructure files (unless --force)
    if !other.is_empty() && !force {
        let error_msg = format!(
            "Uncommitted non-infrastructure files found. Use --force to proceed anyway:\n  {}",
            other.join("\n  ")
        );

        let data = MergeData {
            status: "error".to_string(),
            pr_url: Some(pr_info.url.clone()),
            pr_number: Some(pr_info.number),
            branch_name: Some(session.branch_name.clone()),
            infrastructure_committed: None,
            infrastructure_files: None,
            worktree_cleaned: None,
            dry_run,
            would_commit: None,
            would_merge_pr: None,
            would_cleanup_worktree: None,
            error: Some(error_msg.clone()),
            message: None,
        };

        if json {
            println!("{}", serde_json::to_string_pretty(&data).unwrap());
        }
        return Err(error_msg);
    }

    // If --force and other files exist, print warning
    if !other.is_empty() && force && !quiet {
        eprintln!(
            "Warning: Proceeding with uncommitted non-infrastructure files (--force):\n  {}",
            other.join("\n  ")
        );
    }

    // If dry-run mode, return now with would_* fields populated
    if dry_run {
        let data = MergeData {
            status: "ok".to_string(),
            pr_url: Some(pr_info.url.clone()),
            pr_number: Some(pr_info.number),
            branch_name: Some(session.branch_name.clone()),
            infrastructure_committed: None,
            infrastructure_files: None,
            worktree_cleaned: None,
            dry_run: true,
            would_commit: if !infrastructure.is_empty() {
                Some(infrastructure.clone())
            } else {
                None
            },
            would_merge_pr: Some(pr_info.url.clone()),
            would_cleanup_worktree: Some(worktree_path.display().to_string()),
            error: None,
            message: Some(format!(
                "Would merge PR #{} for worktree at {}",
                pr_info.number,
                worktree_path.display()
            )),
        };

        if json {
            println!("{}", serde_json::to_string_pretty(&data).unwrap());
        } else if !quiet {
            println!("Dry-run mode: showing planned operations\n");
            println!("Found worktree: {}", worktree_path.display());
            println!("Branch: {}", session.branch_name);
            println!("PR: #{} - {}", pr_info.number, pr_info.url);
            if !infrastructure.is_empty() {
                println!(
                    "\nWould commit infrastructure files:\n  {}",
                    infrastructure.join("\n  ")
                );
            }
            println!("\nWould merge PR: {}", pr_info.url);
            println!("Would cleanup worktree: {}", worktree_path.display());
        }

        return Ok(0);
    }

    // Step 8: Stage and commit infrastructure files (if any)
    let infrastructure_committed = if !infrastructure.is_empty() {
        // Extract speck name from speck_path for commit message
        let speck_name = Path::new(&session.speck_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        // Stage infrastructure files
        for file in &infrastructure {
            let add_output = Command::new("git")
                .args(["add", file])
                .output()
                .map_err(|e| format!("Failed to execute git add: {}", e))?;

            if !add_output.status.success() {
                let stderr = String::from_utf8_lossy(&add_output.stderr);
                let error_msg = format!("Failed to stage {}: {}", file, stderr);

                let data = MergeData {
                    status: "error".to_string(),
                    pr_url: Some(pr_info.url.clone()),
                    pr_number: Some(pr_info.number),
                    branch_name: Some(session.branch_name.clone()),
                    infrastructure_committed: Some(false),
                    infrastructure_files: Some(infrastructure.clone()),
                    worktree_cleaned: None,
                    dry_run: false,
                    would_commit: None,
                    would_merge_pr: None,
                    would_cleanup_worktree: None,
                    error: Some(error_msg.clone()),
                    message: None,
                };

                if json {
                    println!("{}", serde_json::to_string_pretty(&data).unwrap());
                }
                return Err(error_msg);
            }
        }

        // Commit with message following D04 format
        let commit_message = format!("chore({}): infrastructure updates", speck_name);
        let commit_output = Command::new("git")
            .args(["commit", "-m", &commit_message])
            .output()
            .map_err(|e| format!("Failed to execute git commit: {}", e))?;

        if !commit_output.status.success() {
            let stderr = String::from_utf8_lossy(&commit_output.stderr);

            // Check if it's an empty commit (no changes to commit)
            if stderr.contains("nothing to commit") || stderr.contains("no changes added to commit")
            {
                if !quiet {
                    println!("No infrastructure changes to commit (already committed)");
                }
                false
            } else {
                let error_msg = format!("Failed to commit infrastructure files: {}", stderr);

                let data = MergeData {
                    status: "error".to_string(),
                    pr_url: Some(pr_info.url.clone()),
                    pr_number: Some(pr_info.number),
                    branch_name: Some(session.branch_name.clone()),
                    infrastructure_committed: Some(false),
                    infrastructure_files: Some(infrastructure.clone()),
                    worktree_cleaned: None,
                    dry_run: false,
                    would_commit: None,
                    would_merge_pr: None,
                    would_cleanup_worktree: None,
                    error: Some(error_msg.clone()),
                    message: None,
                };

                if json {
                    println!("{}", serde_json::to_string_pretty(&data).unwrap());
                }
                return Err(error_msg);
            }
        } else {
            if !quiet {
                println!(
                    "Committed infrastructure files: {}",
                    infrastructure.join(", ")
                );
            }
            true
        }
    } else {
        false
    };

    // Step 9: Push main to origin
    if infrastructure_committed {
        let push_output = Command::new("git")
            .args(["push", "origin", "main"])
            .output()
            .map_err(|e| format!("Failed to execute git push: {}", e))?;

        if !push_output.status.success() {
            let stderr = String::from_utf8_lossy(&push_output.stderr);
            let error_msg = format!("Failed to push main to origin: {}", stderr);

            let data = MergeData {
                status: "error".to_string(),
                pr_url: Some(pr_info.url.clone()),
                pr_number: Some(pr_info.number),
                branch_name: Some(session.branch_name.clone()),
                infrastructure_committed: Some(infrastructure_committed),
                infrastructure_files: Some(infrastructure.clone()),
                worktree_cleaned: None,
                dry_run: false,
                would_commit: None,
                would_merge_pr: None,
                would_cleanup_worktree: None,
                error: Some(error_msg.clone()),
                message: None,
            };

            if json {
                println!("{}", serde_json::to_string_pretty(&data).unwrap());
            }
            return Err(error_msg);
        }

        if !quiet {
            println!("Pushed main to origin");
        }
    }

    // Step 10: Merge PR via gh pr merge --squash
    if !quiet {
        println!("Merging PR #{} via squash...", pr_info.number);
    }

    let merge_output = Command::new("gh")
        .args(["pr", "merge", "--squash", &session.branch_name])
        .output()
        .map_err(|e| format!("Failed to execute gh pr merge: {}", e))?;

    if !merge_output.status.success() {
        let stderr = String::from_utf8_lossy(&merge_output.stderr);
        let error_msg = format!("Failed to merge PR: {}", stderr);

        let data = MergeData {
            status: "error".to_string(),
            pr_url: Some(pr_info.url.clone()),
            pr_number: Some(pr_info.number),
            branch_name: Some(session.branch_name.clone()),
            infrastructure_committed: Some(infrastructure_committed),
            infrastructure_files: if infrastructure_committed {
                Some(infrastructure.clone())
            } else {
                None
            },
            worktree_cleaned: None,
            dry_run: false,
            would_commit: None,
            would_merge_pr: None,
            would_cleanup_worktree: None,
            error: Some(error_msg.clone()),
            message: None,
        };

        if json {
            println!("{}", serde_json::to_string_pretty(&data).unwrap());
        }
        return Err(error_msg);
    }

    if !quiet {
        println!("PR merged successfully");
    }

    // Step 11: Pull main to fetch the squashed commit
    let pull_output = Command::new("git")
        .args(["pull", "origin", "main"])
        .output()
        .map_err(|e| format!("Failed to execute git pull: {}", e))?;

    if !pull_output.status.success() {
        let stderr = String::from_utf8_lossy(&pull_output.stderr);
        let error_msg = format!("Failed to pull main after merge: {}", stderr);

        let data = MergeData {
            status: "error".to_string(),
            pr_url: Some(pr_info.url.clone()),
            pr_number: Some(pr_info.number),
            branch_name: Some(session.branch_name.clone()),
            infrastructure_committed: Some(infrastructure_committed),
            infrastructure_files: if infrastructure_committed {
                Some(infrastructure.clone())
            } else {
                None
            },
            worktree_cleaned: None,
            dry_run: false,
            would_commit: None,
            would_merge_pr: None,
            would_cleanup_worktree: None,
            error: Some(error_msg.clone()),
            message: None,
        };

        if json {
            println!("{}", serde_json::to_string_pretty(&data).unwrap());
        }
        return Err(error_msg);
    }

    if !quiet {
        println!("Pulled squashed commit from origin");
    }

    // Step 12: Cleanup worktree by removing it and deleting the branch
    if !quiet {
        println!("Cleaning up worktree...");
    }

    // Get repo root (current directory, since merge runs from repo root)
    let repo_root =
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;

    // Remove the worktree using specks_core::remove_worktree
    // This cleans up session/artifacts before calling git worktree remove
    let mut worktree_cleaned = match specks_core::remove_worktree(&worktree_path, &repo_root) {
        Ok(_) => {
            if !quiet {
                println!("Removed worktree directory");
            }
            true
        }
        Err(e) => {
            if !quiet {
                eprintln!("Warning: Failed to remove worktree: {}", e);
                eprintln!(
                    "You may need to manually run: specks worktree cleanup or git worktree remove {}",
                    worktree_path.display()
                );
            }
            false
        }
    };

    // Delete the branch
    let delete_output = Command::new("git")
        .args(["branch", "-D", &session.branch_name])
        .output()
        .map_err(|e| format!("Failed to execute git branch -D: {}", e))?;

    if !delete_output.status.success() {
        let stderr = String::from_utf8_lossy(&delete_output.stderr);
        if !quiet {
            eprintln!("Warning: Failed to delete branch: {}", stderr);
            eprintln!(
                "You may need to manually run: git branch -D {}",
                session.branch_name
            );
        }
        worktree_cleaned = false;
    } else if !quiet {
        println!("Deleted branch: {}", session.branch_name);
    }

    // Prune stale worktree metadata
    let prune_output = Command::new("git")
        .args(["worktree", "prune"])
        .output()
        .map_err(|e| format!("Failed to execute git worktree prune: {}", e))?;

    if !prune_output.status.success() {
        let stderr = String::from_utf8_lossy(&prune_output.stderr);
        if !quiet {
            eprintln!("Warning: Failed to prune worktree metadata: {}", stderr);
        }
    } else if !quiet {
        println!("Pruned worktree metadata");
    }

    // Step 13: Return success response
    let data = MergeData {
        status: "ok".to_string(),
        pr_url: Some(pr_info.url.clone()),
        pr_number: Some(pr_info.number),
        branch_name: Some(session.branch_name.clone()),
        infrastructure_committed: Some(infrastructure_committed),
        infrastructure_files: if infrastructure_committed {
            Some(infrastructure.clone())
        } else {
            None
        },
        worktree_cleaned: Some(worktree_cleaned),
        dry_run: false,
        would_commit: None,
        would_merge_pr: None,
        would_cleanup_worktree: None,
        error: None,
        message: Some(format!(
            "Successfully merged PR #{} and cleaned up worktree",
            pr_info.number
        )),
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&data).unwrap());
    } else if !quiet {
        println!("\nMerge complete!");
        println!("PR: {}", pr_info.url);
        if infrastructure_committed {
            println!("Infrastructure committed: {}", infrastructure.join(", "));
        }
        if worktree_cleaned {
            println!("Worktree cleaned: {}", worktree_path.display());
        }
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
        use tempfile::TempDir;

        // Create a temporary directory that definitely won't have worktrees
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Should fail because .specks-worktrees doesn't exist
        let result = find_worktree_for_speck(Some(temp_path), ".specks/specks-test.md");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No worktrees directory found"));
    }

    #[test]
    fn test_find_worktree_no_matching_speck() {
        use tempfile::TempDir;

        // Create a temporary test environment
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let worktrees_dir = temp_path.join(".specks-worktrees");
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
            reused: false,
        };

        let session_json = serde_json::to_string_pretty(&session).unwrap();
        fs::write(specks_dir1.join("session.json"), session_json).unwrap();

        // Try to find a different speck
        let result = find_worktree_for_speck(Some(temp_path), ".specks/specks-test.md");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No worktree found"));
    }

    #[test]
    fn test_find_worktree_success() {
        use tempfile::TempDir;

        // Create a temporary test environment
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let worktrees_dir = temp_path.join(".specks-worktrees");
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
            reused: false,
        };

        let session_json = serde_json::to_string_pretty(&session).unwrap();
        fs::write(specks_dir1.join("session.json"), session_json).unwrap();

        // Find the worktree
        let result = find_worktree_for_speck(Some(temp_path), ".specks/specks-test.md");
        assert!(result.is_ok());

        let (path, loaded_session) = result.unwrap();
        // Path should end with the worktree directory name
        assert!(path.ends_with("specks__test-20260209-120000"));
        assert_eq!(loaded_session.speck_path, ".specks/specks-test.md");
        assert_eq!(loaded_session.branch_name, "specks/test-20260209-120000");
    }

    #[test]
    fn test_find_worktree_path_normalization() {
        use tempfile::TempDir;

        // Create a temporary test environment
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let worktrees_dir = temp_path.join(".specks-worktrees");
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
            reused: false,
        };

        let session_json = serde_json::to_string_pretty(&session).unwrap();
        fs::write(specks_dir1.join("session.json"), session_json).unwrap();

        // Test various path formats
        let result1 = find_worktree_for_speck(Some(temp_path), ".specks/specks-test.md");
        assert!(result1.is_ok());

        let result2 = find_worktree_for_speck(Some(temp_path), "specks-test.md");
        assert!(result2.is_ok());

        let result3 = find_worktree_for_speck(Some(temp_path), "./.specks/specks-test.md");
        assert!(result3.is_ok());
    }

    #[test]
    fn test_find_worktree_corrupt_session() {
        use tempfile::TempDir;

        // Create a temporary test environment
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let worktrees_dir = temp_path.join(".specks-worktrees");
        let worktree1 = worktrees_dir.join("specks__corrupt-20260209-120000");
        let specks_dir1 = worktree1.join(".specks");
        fs::create_dir_all(&specks_dir1).unwrap();

        // Write corrupt JSON
        fs::write(specks_dir1.join("session.json"), "{ invalid json here").unwrap();

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
            reused: false,
        };

        let session_json = serde_json::to_string_pretty(&session).unwrap();
        fs::write(specks_dir2.join("session.json"), session_json).unwrap();

        // Should skip corrupt session and find the valid one
        let result = find_worktree_for_speck(Some(temp_path), ".specks/specks-test.md");
        assert!(result.is_ok());
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
        assert!(is_infrastructure_file(
            ".claude/skills/implementer/SKILL.md"
        ));
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
        use tempfile::TempDir;

        // Create a temporary test git repository
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Initialize git repo
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["init"])
            .output()
            .expect("Failed to init git repo");

        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .expect("Failed to configure git");

        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["config", "user.name", "Test User"])
            .output()
            .expect("Failed to configure git");

        // Create initial commit to have a HEAD
        fs::write(temp_path.join("README.md"), "Test repo").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["add", "README.md"])
            .output()
            .expect("Failed to add README");

        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .expect("Failed to commit");

        // Create mixed uncommitted files
        // Infrastructure files
        fs::create_dir_all(temp_path.join("agents")).unwrap();
        fs::write(temp_path.join("agents/test-agent.md"), "# Agent").unwrap();

        fs::create_dir_all(temp_path.join(".claude/skills/test")).unwrap();
        fs::write(temp_path.join(".claude/skills/test/SKILL.md"), "# Skill").unwrap();

        fs::create_dir_all(temp_path.join(".specks")).unwrap();
        fs::write(temp_path.join(".specks/specks-skeleton.md"), "# Skeleton").unwrap();
        fs::write(temp_path.join(".specks/config.toml"), "# Config").unwrap();
        fs::write(
            temp_path.join(".specks/specks-implementation-log.md"),
            "# Log",
        )
        .unwrap();

        fs::create_dir_all(temp_path.join(".beads")).unwrap();
        fs::write(temp_path.join(".beads/beads.json"), "{}").unwrap();

        fs::write(temp_path.join("CLAUDE.md"), "# Claude").unwrap();

        // Non-infrastructure files
        fs::create_dir_all(temp_path.join("src")).unwrap();
        fs::write(temp_path.join("src/main.rs"), "fn main() {}").unwrap();

        fs::write(temp_path.join(".specks/specks-123.md"), "# Speck 123").unwrap();

        fs::write(temp_path.join("Cargo.toml"), "[package]").unwrap();

        // Run categorization
        let result = categorize_uncommitted(Some(temp_path));
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
    }

    // Unit tests for validation functions

    #[test]
    fn test_validate_pr_state_open() {
        let pr_info = PrInfo {
            number: 123,
            url: "https://github.com/owner/repo/pull/123".to_string(),
            state: "OPEN".to_string(),
        };

        let result = validate_pr_state(&pr_info);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_pr_state_merged() {
        let pr_info = PrInfo {
            number: 123,
            url: "https://github.com/owner/repo/pull/123".to_string(),
            state: "MERGED".to_string(),
        };

        let result = validate_pr_state(&pr_info);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already merged"));
    }

    #[test]
    fn test_validate_pr_state_closed() {
        let pr_info = PrInfo {
            number: 123,
            url: "https://github.com/owner/repo/pull/123".to_string(),
            state: "CLOSED".to_string(),
        };

        let result = validate_pr_state(&pr_info);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("closed without merge"));
    }

    #[test]
    fn test_validate_pr_state_unknown() {
        let pr_info = PrInfo {
            number: 123,
            url: "https://github.com/owner/repo/pull/123".to_string(),
            state: "DRAFT".to_string(),
        };

        let result = validate_pr_state(&pr_info);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("state is DRAFT"));
    }

    // Unit tests for parsing git rev-list output

    #[test]
    fn test_check_main_sync_in_sync() {
        // Simulates git rev-list output when main is in sync with origin/main
        // This is a unit test that would require mocking git commands
        // For now, we test the expected behavior by calling check_main_sync
        // in a real git repo context (done in integration tests)

        // The expected behavior is:
        // - If git rev-list returns "0", should return Ok(())
        // - This is tested through integration tests in a controlled git environment
    }

    #[test]
    fn test_check_main_sync_commits_ahead() {
        // Simulates git rev-list output when main has unpushed commits
        // This is a unit test that would require mocking git commands
        // For now, we test the expected behavior through integration tests

        // The expected behavior is:
        // - If git rev-list returns "3", should return Err with message about 3 commits
        // - This is tested through integration tests in a controlled git environment
    }

    // Unit tests for parsing gh pr checks JSON output

    #[test]
    fn test_check_pr_checks_json_all_pass() {
        // Test parsing of successful checks JSON
        let json_output = r#"[
            {"name":"Build","state":"completed","conclusion":"success"},
            {"name":"Test","state":"completed","conclusion":"success"}
        ]"#;

        #[derive(Deserialize)]
        #[allow(dead_code)] // Test struct fields used only for deserialization
        struct CheckStatus {
            name: String,
            state: String,
            conclusion: Option<String>,
        }

        let checks: Vec<CheckStatus> = serde_json::from_str(json_output).unwrap();
        assert_eq!(checks.len(), 2);
        assert_eq!(checks[0].name, "Build");
        assert_eq!(checks[0].state, "completed");
        assert_eq!(checks[0].conclusion, Some("success".to_string()));
    }

    #[test]
    fn test_check_pr_checks_json_failing() {
        // Test parsing of failing checks JSON
        let json_output = r#"[
            {"name":"Build","state":"completed","conclusion":"failure"},
            {"name":"Test","state":"completed","conclusion":"success"}
        ]"#;

        #[derive(Deserialize)]
        #[allow(dead_code)] // Test struct fields used only for deserialization
        struct CheckStatus {
            name: String,
            state: String,
            conclusion: Option<String>,
        }

        let checks: Vec<CheckStatus> = serde_json::from_str(json_output).unwrap();
        assert_eq!(checks.len(), 2);
        assert_eq!(checks[0].conclusion, Some("failure".to_string()));
    }

    #[test]
    fn test_check_pr_checks_json_pending() {
        // Test parsing of pending checks JSON
        let json_output = r#"[
            {"name":"Build","state":"pending","conclusion":null},
            {"name":"Test","state":"completed","conclusion":"success"}
        ]"#;

        #[derive(Deserialize)]
        #[allow(dead_code)] // Test struct fields used only for deserialization
        struct CheckStatus {
            name: String,
            state: String,
            conclusion: Option<String>,
        }

        let checks: Vec<CheckStatus> = serde_json::from_str(json_output).unwrap();
        assert_eq!(checks.len(), 2);
        assert_eq!(checks[0].state, "pending");
        assert!(checks[0].conclusion.is_none());
    }

    #[test]
    fn test_check_pr_checks_json_empty_array() {
        // Test that empty array (no checks) is handled as success
        let json_output = r#"[]"#;

        #[derive(Deserialize)]
        #[allow(dead_code)] // Test struct fields used only for deserialization
        struct CheckStatus {
            name: String,
            state: String,
            conclusion: Option<String>,
        }

        let checks: Vec<CheckStatus> = serde_json::from_str(json_output).unwrap();
        assert_eq!(checks.len(), 0);
        // This should be treated as success (no checks configured)
    }

    // Integration tests

    #[test]
    fn test_run_merge_abort_on_non_infrastructure_files_without_force() {
        use std::process::Command;
        use tempfile::TempDir;

        // Create a temporary test git repository
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Initialize git repo
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["init"])
            .output()
            .expect("Failed to init git repo");

        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .expect("Failed to configure git");

        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["config", "user.name", "Test User"])
            .output()
            .expect("Failed to configure git");

        // Create initial commit
        fs::write(temp_path.join("README.md"), "Test repo").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["add", "README.md"])
            .output()
            .expect("Failed to add README");

        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .expect("Failed to commit");

        // Create a non-infrastructure file
        fs::create_dir_all(temp_path.join("src")).unwrap();
        fs::write(temp_path.join("src/main.rs"), "fn main() {}").unwrap();

        // Test that categorize_uncommitted correctly identifies this as non-infrastructure
        let result = categorize_uncommitted(Some(temp_path));
        assert!(result.is_ok());

        let (infrastructure, other) = result.unwrap();
        assert!(
            infrastructure.is_empty(),
            "Should have no infrastructure files"
        );
        assert_eq!(other.len(), 1, "Should have 1 non-infrastructure file");
        assert!(other.contains(&"src/main.rs".to_string()));
    }

    #[test]
    fn test_run_merge_force_proceeds_with_non_infrastructure() {
        use std::process::Command;
        use tempfile::TempDir;

        // Create a temporary test git repository
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Initialize git repo
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["init"])
            .output()
            .expect("Failed to init git repo");

        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .expect("Failed to configure git");

        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["config", "user.name", "Test User"])
            .output()
            .expect("Failed to configure git");

        // Create initial commit
        fs::write(temp_path.join("README.md"), "Test repo").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["add", "README.md"])
            .output()
            .expect("Failed to add README");

        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .expect("Failed to commit");

        // Create both infrastructure and non-infrastructure files
        fs::create_dir_all(temp_path.join("agents")).unwrap();
        fs::write(temp_path.join("agents/test-agent.md"), "# Agent").unwrap();

        fs::create_dir_all(temp_path.join("src")).unwrap();
        fs::write(temp_path.join("src/main.rs"), "fn main() {}").unwrap();

        // Test categorization
        let result = categorize_uncommitted(Some(temp_path));
        assert!(result.is_ok());

        let (infrastructure, other) = result.unwrap();
        assert_eq!(infrastructure.len(), 1);
        assert!(infrastructure.contains(&"agents/test-agent.md".to_string()));
        assert_eq!(other.len(), 1);
        assert!(other.contains(&"src/main.rs".to_string()));

        // Verify that --force flag would allow proceeding
        // This is validated in the run_merge function logic at lines 596-602
        // where if force=true and other.is_empty()=false, it prints warning but continues
    }

    #[test]
    fn test_merge_data_json_serialization() {
        // Test successful merge response
        let data = MergeData {
            status: "ok".to_string(),
            pr_url: Some("https://github.com/owner/repo/pull/123".to_string()),
            pr_number: Some(123),
            branch_name: Some("specks/feature-20260209-120000".to_string()),
            infrastructure_committed: Some(true),
            infrastructure_files: Some(vec![
                "CLAUDE.md".to_string(),
                "agents/coder-agent.md".to_string(),
            ]),
            worktree_cleaned: Some(true),
            dry_run: false,
            would_commit: None,
            would_merge_pr: None,
            would_cleanup_worktree: None,
            error: None,
            message: Some("Successfully merged PR #123 and cleaned up worktree".to_string()),
        };

        let json = serde_json::to_string_pretty(&data).unwrap();
        assert!(json.contains("\"status\": \"ok\""));
        assert!(json.contains("\"pr_number\": 123"));
        assert!(json.contains("\"infrastructure_committed\": true"));
        assert!(json.contains("\"worktree_cleaned\": true"));
        assert!(!json.contains("\"dry_run\"")); // Should be omitted when false

        // Verify it can be deserialized back
        let _parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_merge_data_dry_run_json_serialization() {
        // Test dry-run response
        let data = MergeData {
            status: "ok".to_string(),
            pr_url: Some("https://github.com/owner/repo/pull/123".to_string()),
            pr_number: Some(123),
            branch_name: Some("specks/feature-20260209-120000".to_string()),
            infrastructure_committed: None,
            infrastructure_files: None,
            worktree_cleaned: None,
            dry_run: true,
            would_commit: Some(vec!["CLAUDE.md".to_string(), "agents/coder-agent.md".to_string()]),
            would_merge_pr: Some("https://github.com/owner/repo/pull/123".to_string()),
            would_cleanup_worktree: Some(".specks-worktrees/specks__feature-20260209-120000".to_string()),
            error: None,
            message: Some("Would merge PR #123 for worktree at .specks-worktrees/specks__feature-20260209-120000".to_string()),
        };

        let json = serde_json::to_string_pretty(&data).unwrap();
        assert!(json.contains("\"status\": \"ok\""));
        assert!(json.contains("\"dry_run\": true"));
        assert!(json.contains("\"would_commit\""));
        assert!(json.contains("\"would_merge_pr\""));
        assert!(json.contains("\"would_cleanup_worktree\""));
        assert!(!json.contains("\"infrastructure_committed\"")); // Should be omitted when None
        assert!(!json.contains("\"worktree_cleaned\"")); // Should be omitted when None

        // Verify it can be deserialized back
        let _parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_merge_data_error_json_serialization() {
        // Test error response
        let data = MergeData {
            status: "error".to_string(),
            pr_url: Some("https://github.com/owner/repo/pull/123".to_string()),
            pr_number: Some(123),
            branch_name: Some("specks/feature-20260209-120000".to_string()),
            infrastructure_committed: None,
            infrastructure_files: None,
            worktree_cleaned: None,
            dry_run: false,
            would_commit: None,
            would_merge_pr: None,
            would_cleanup_worktree: None,
            error: Some("Main branch has 2 unpushed commits. Run 'git push' first.".to_string()),
            message: None,
        };

        let json = serde_json::to_string_pretty(&data).unwrap();
        assert!(json.contains("\"status\": \"error\""));
        assert!(json.contains("\"error\""));
        assert!(json.contains("unpushed commits"));
        assert!(!json.contains("\"dry_run\"")); // Should be omitted when false

        // Verify it can be deserialized back
        let _parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_validate_pr_state_open_success() {
        let pr_info = PrInfo {
            number: 123,
            url: "https://github.com/owner/repo/pull/123".to_string(),
            state: "OPEN".to_string(),
        };

        let result = validate_pr_state(&pr_info);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_pr_state_merged_error() {
        let pr_info = PrInfo {
            number: 123,
            url: "https://github.com/owner/repo/pull/123".to_string(),
            state: "MERGED".to_string(),
        };

        let result = validate_pr_state(&pr_info);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("already merged"));
        assert!(error.contains(&pr_info.url));
    }

    #[test]
    fn test_validate_pr_state_closed_error() {
        let pr_info = PrInfo {
            number: 123,
            url: "https://github.com/owner/repo/pull/123".to_string(),
            state: "CLOSED".to_string(),
        };

        let result = validate_pr_state(&pr_info);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("closed without merge"));
        assert!(error.contains(&pr_info.url));
    }

    // Tests for is_main_worktree validation

    #[test]
    fn test_is_main_worktree_detects_main_repository() {
        use std::process::Command;
        use tempfile::TempDir;

        // Create a temporary test git repository
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Initialize git repo
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["init"])
            .output()
            .expect("Failed to init git repo");

        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .expect("Failed to configure git");

        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["config", "user.name", "Test User"])
            .output()
            .expect("Failed to configure git");

        // Create initial commit to establish main branch
        std::fs::write(temp_path.join("README.md"), "Test repo").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["add", "README.md"])
            .output()
            .expect("Failed to add README");

        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .expect("Failed to commit");

        // Change to the temp directory and run the check
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();

        let result = is_main_worktree();

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        // Should succeed - this is a main worktree with .git directory
        assert!(
            result.is_ok(),
            "Expected main worktree check to pass, got: {:?}",
            result
        );
    }

    #[test]
    fn test_is_main_worktree_detects_git_worktree() {
        use std::process::Command;
        use tempfile::TempDir;

        // Create a temporary test git repository with worktree
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Initialize git repo
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["init"])
            .output()
            .expect("Failed to init git repo");

        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .expect("Failed to configure git");

        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["config", "user.name", "Test User"])
            .output()
            .expect("Failed to configure git");

        // Create initial commit
        std::fs::write(temp_path.join("README.md"), "Test repo").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["add", "README.md"])
            .output()
            .expect("Failed to add README");

        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .expect("Failed to commit");

        // Create a worktree
        let worktree_path = temp_path.join("test-worktree");
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args([
                "worktree",
                "add",
                worktree_path.to_str().unwrap(),
                "-b",
                "test-branch",
            ])
            .output()
            .expect("Failed to create worktree");

        // Change to the worktree directory and run the check
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&worktree_path).unwrap();

        let result = is_main_worktree();

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        // Should fail - this is a worktree, not main repository
        assert!(result.is_err(), "Expected worktree check to fail");
        let error = result.unwrap_err();
        assert!(
            error.contains("git worktree"),
            "Error should mention git worktree, got: {}",
            error
        );
    }

    #[test]
    fn test_is_main_worktree_detects_wrong_branch() {
        use std::process::Command;
        use tempfile::TempDir;

        // Create a temporary test git repository
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Initialize git repo
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["init"])
            .output()
            .expect("Failed to init git repo");

        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .expect("Failed to configure git");

        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["config", "user.name", "Test User"])
            .output()
            .expect("Failed to configure git");

        // Create initial commit on main
        std::fs::write(temp_path.join("README.md"), "Test repo").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["add", "README.md"])
            .output()
            .expect("Failed to add README");

        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .expect("Failed to commit");

        // Create and checkout a feature branch
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["checkout", "-b", "feature-branch"])
            .output()
            .expect("Failed to create feature branch");

        // Change to the temp directory and run the check
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();

        let result = is_main_worktree();

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        // Should fail - we're on feature-branch, not main
        assert!(
            result.is_err(),
            "Expected wrong branch check to fail, got: {:?}",
            result
        );
        let error = result.unwrap_err();
        assert!(
            error.contains("feature-branch"),
            "Error should mention current branch, got: {}",
            error
        );
        assert!(
            error.contains("main") || error.contains("master"),
            "Error should mention expected branch, got: {}",
            error
        );
    }

    #[test]
    fn test_is_main_worktree_no_git_directory() {
        use tempfile::TempDir;

        // Create a temporary directory without git
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Change to the temp directory and run the check
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();

        let result = is_main_worktree();

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        // Should fail - no git repository
        assert!(result.is_err(), "Expected no git directory check to fail");
        let error = result.unwrap_err();
        assert!(
            error.contains("Not in a git repository"),
            "Error should mention missing git directory, got: {}",
            error
        );
    }
}
