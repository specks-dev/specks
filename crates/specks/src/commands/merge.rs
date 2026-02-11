//! Merge command implementation
//!
//! Merges a speck's implementation branch into main and cleans up the worktree.
//! Uses git-native worktree discovery (not session files) for reliability.
//!
//! Two modes:
//! - Remote: Has origin remote → merge PR via `gh pr merge --squash`
//! - Local: No remote → `git merge --squash` directly

use serde::{Deserialize, Serialize};
use specks_core::{derive_speck_slug, find_worktree_by_speck, remove_worktree};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// JSON output for merge command
#[derive(Serialize)]
pub struct MergeData {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub squash_commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree_cleaned: Option<bool>,
    #[serde(skip_serializing_if = "is_false")]
    pub dry_run: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dirty_files: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

fn is_false(b: &bool) -> bool {
    !b
}

impl MergeData {
    fn error(msg: String, dry_run: bool) -> Self {
        MergeData {
            status: "error".to_string(),
            merge_mode: None,
            branch_name: None,
            worktree_path: None,
            pr_url: None,
            pr_number: None,
            squash_commit: None,
            worktree_cleaned: None,
            dry_run,
            dirty_files: None,
            error: Some(msg),
            message: None,
        }
    }
}

/// Information about a GitHub pull request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrInfo {
    pub number: u32,
    pub url: String,
    pub state: String,
}

/// Run a command and return detailed error on failure
fn run_cmd(cmd: &mut Command, name: &str) -> Result<Output, String> {
    let output = cmd
        .output()
        .map_err(|e| format!("Failed to execute '{}': {}", name, e))?;

    if !output.status.success() {
        let code = output
            .status
            .code()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "signal".to_string());
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("'{}' failed (exit {}): {}", name, code, stderr));
    }

    Ok(output)
}

/// Check if current directory is the main worktree on main/master branch
fn is_main_worktree(repo_root: &Path) -> Result<(), String> {
    let git_path = repo_root.join(".git");
    if !git_path.exists() {
        return Err("Not in a git repository (no .git directory found)".to_string());
    }
    if !git_path.is_dir() {
        return Err("Running from a git worktree, not the main repository.\n\
             The merge command must run from the main worktree.\n\
             Please cd to the repository root and try again."
            .to_string());
    }

    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
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

/// Check if repository has a remote named 'origin'
fn has_remote_origin(repo_root: &Path) -> bool {
    Command::new("git")
        .current_dir(repo_root)
        .args(["remote", "get-url", "origin"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Get PR info for a branch via gh CLI
fn get_pr_for_branch(branch: &str) -> Result<PrInfo, String> {
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

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).map_err(|e| format!("Failed to parse gh pr view output: {}", e))
}

/// Get list of uncommitted files in the working tree, with porcelain status prefix
fn get_dirty_files(repo_root: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["status", "--porcelain", "-u"])
        .output()
        .map_err(|e| format!("Failed to execute git status: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git status failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .filter(|line| line.len() >= 4)
        .map(|line| line[3..].to_string())
        .collect())
}

fn is_infrastructure_path(path: &str) -> bool {
    path.starts_with(".specks/") || path.starts_with(".beads/")
}

/// Prepare main for merge by committing infrastructure files and discarding everything else.
/// Returns the list of dirty files that were found (for reporting).
fn prepare_main_for_merge(repo_root: &Path, quiet: bool) -> Result<Vec<String>, String> {
    let dirty_files = get_dirty_files(repo_root)?;
    if dirty_files.is_empty() {
        return Ok(vec![]);
    }

    let infra: Vec<&str> = dirty_files
        .iter()
        .filter(|f| is_infrastructure_path(f))
        .map(|f| f.as_str())
        .collect();
    let non_infra: Vec<&str> = dirty_files
        .iter()
        .filter(|f| !is_infrastructure_path(f))
        .map(|f| f.as_str())
        .collect();

    // Discard non-infrastructure files first (they'll come from the branch)
    if !non_infra.is_empty() {
        if !quiet {
            eprintln!(
                "Discarding {} leaked file(s) from main (will come from branch):",
                non_infra.len()
            );
            for f in &non_infra {
                eprintln!("  {}", f);
            }
        }

        // Reset tracked files
        let mut checkout_args = vec!["checkout", "--"];
        checkout_args.extend(non_infra.iter());
        let checkout = Command::new("git")
            .current_dir(repo_root)
            .args(&checkout_args)
            .output();
        // Ignore errors — file might be untracked

        if let Ok(ref _o) = checkout {
            // good
        }

        // Clean untracked files
        let mut clean_args = vec!["clean", "-f", "--"];
        clean_args.extend(non_infra.iter());
        let _ = Command::new("git")
            .current_dir(repo_root)
            .args(&clean_args)
            .output();
    }

    // Commit infrastructure files
    if !infra.is_empty() {
        if !quiet {
            eprintln!("Committing {} infrastructure file(s) on main:", infra.len());
            for f in &infra {
                eprintln!("  {}", f);
            }
        }

        let mut add_args = vec!["add", "--"];
        add_args.extend(infra.iter());
        let _ = Command::new("git")
            .current_dir(repo_root)
            .args(&add_args)
            .output();

        let commit = Command::new("git")
            .current_dir(repo_root)
            .args(["commit", "-m", "chore: pre-merge sync"])
            .output();

        // If commit fails (nothing to commit), that's fine
        if let Ok(ref o) = commit {
            if !o.status.success() && !quiet {
                let stderr = String::from_utf8_lossy(&o.stderr);
                if !stderr.contains("nothing to commit") {
                    eprintln!("Warning: pre-merge commit failed: {}", stderr);
                }
            }
        }
    }

    Ok(dirty_files)
}

/// Squash merge a branch into the current branch
fn squash_merge_branch(repo_root: &Path, branch: &str, message: &str) -> Result<String, String> {
    // git merge --squash
    let merge_output = Command::new("git")
        .current_dir(repo_root)
        .args(["merge", "--squash", branch])
        .output()
        .map_err(|e| format!("Failed to execute git merge --squash: {}", e))?;

    if !merge_output.status.success() {
        let stderr = String::from_utf8_lossy(&merge_output.stderr);
        let stdout = String::from_utf8_lossy(&merge_output.stdout);
        let combined = format!("{}{}", stdout, stderr);

        // Check if the failure is due to merge conflicts we can auto-resolve
        if combined.contains("CONFLICT") || combined.contains("Automatic merge failed") {
            if let Ok(resolved) = try_auto_resolve_conflicts(repo_root) {
                if resolved {
                    // All conflicts were in infrastructure files and have been resolved
                    eprintln!("Auto-resolved infrastructure file conflicts (took branch version)");
                    // Fall through to the commit step below
                } else {
                    // Code file conflicts — abort
                    let _ = Command::new("git")
                        .current_dir(repo_root)
                        .args(["reset", "--merge"])
                        .output();
                    return Err(format!(
                        "Merge failed with code file conflicts (repository restored to clean state): {}",
                        stderr
                    ));
                }
            } else {
                let _ = Command::new("git")
                    .current_dir(repo_root)
                    .args(["reset", "--merge"])
                    .output();
                return Err(format!(
                    "Merge failed (repository restored to clean state): {}",
                    stderr
                ));
            }
        } else {
            // Non-conflict failure (e.g., unrelated histories)
            let _ = Command::new("git")
                .current_dir(repo_root)
                .args(["reset", "--merge"])
                .output();
            return Err(format!(
                "Merge failed (repository restored to clean state): {}",
                stderr
            ));
        }
    }

    // git commit
    let commit_output = Command::new("git")
        .current_dir(repo_root)
        .args(["commit", "-m", message])
        .output()
        .map_err(|e| format!("Failed to execute git commit: {}", e))?;

    if !commit_output.status.success() {
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        let stdout = String::from_utf8_lossy(&commit_output.stdout);
        if stderr.contains("nothing to commit")
            || stdout.contains("nothing to commit")
            || stderr.contains("no changes added to commit")
            || stdout.contains("no changes added to commit")
        {
            return Err("Nothing to commit: merge produced no changes".to_string());
        }
        let msg = if !stderr.is_empty() {
            stderr.to_string()
        } else {
            stdout.to_string()
        };
        return Err(format!("Failed to create squash commit: {}", msg));
    }

    // Get commit hash
    let mut hash_cmd = Command::new("git");
    hash_cmd.current_dir(repo_root).args(["rev-parse", "HEAD"]);
    let hash_output = run_cmd(&mut hash_cmd, "git rev-parse HEAD")?;

    Ok(String::from_utf8_lossy(&hash_output.stdout)
        .trim()
        .to_string())
}

/// Check if all merge conflicts are in infrastructure files (.specks/, .beads/)
/// and auto-resolve them by taking the branch version.
/// Returns Ok(true) if all conflicts were resolved, Ok(false) if code files conflict.
fn try_auto_resolve_conflicts(repo_root: &Path) -> Result<bool, String> {
    // Get list of conflicted files
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["diff", "--name-only", "--diff-filter=U"])
        .output()
        .map_err(|e| format!("Failed to list conflicted files: {}", e))?;

    let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
    let conflicted: Vec<&str> = stdout_str
        .trim()
        .lines()
        .filter(|l| !l.is_empty())
        .collect();

    if conflicted.is_empty() {
        return Ok(false);
    }

    // Check if ALL conflicted files are infrastructure files
    let is_infrastructure =
        |path: &str| -> bool { path.starts_with(".specks/") || path.starts_with(".beads/") };

    let code_conflicts: Vec<&&str> = conflicted
        .iter()
        .filter(|f| !is_infrastructure(f))
        .collect();
    if !code_conflicts.is_empty() {
        return Ok(false);
    }

    // Auto-resolve all infrastructure conflicts by taking the branch version
    for file in &conflicted {
        let checkout = Command::new("git")
            .current_dir(repo_root)
            .args(["checkout", "--theirs", "--", file])
            .output()
            .map_err(|e| format!("Failed to resolve conflict in {}: {}", file, e))?;

        if !checkout.status.success() {
            return Err(format!(
                "Failed to resolve conflict in {}: {}",
                file,
                String::from_utf8_lossy(&checkout.stderr)
            ));
        }

        let add = Command::new("git")
            .current_dir(repo_root)
            .args(["add", "--", file])
            .output()
            .map_err(|e| format!("Failed to stage resolved {}: {}", file, e))?;

        if !add.status.success() {
            return Err(format!(
                "Failed to stage resolved {}: {}",
                file,
                String::from_utf8_lossy(&add.stderr)
            ));
        }
    }

    Ok(true)
}

/// Normalize speck path input to a relative path like `.specks/specks-N.md`
fn normalize_speck_path(input: &str) -> PathBuf {
    let s = input.strip_prefix("./").unwrap_or(input);
    if s.starts_with(".specks/") {
        PathBuf::from(s)
    } else {
        PathBuf::from(format!(".specks/{}", s))
    }
}

/// Check if local main is in sync with origin/main
#[allow(dead_code)] // Will be used in step-1
fn check_main_sync(repo_root: &Path) -> Result<(), String> {
    // Step 1: Fetch origin/main to get latest state
    let fetch_output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["fetch", "origin", "main"])
        .output()
        .map_err(|e| format!("Failed to fetch origin/main: {}", e))?;

    if !fetch_output.status.success() {
        let stderr = String::from_utf8_lossy(&fetch_output.stderr);
        return Err(format!("Failed to fetch origin/main: {}", stderr));
    }

    // Step 2: Get local main hash
    let local_output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["rev-parse", "main"])
        .output()
        .map_err(|e| format!("Failed to get local main hash: {}", e))?;

    if !local_output.status.success() {
        let stderr = String::from_utf8_lossy(&local_output.stderr);
        return Err(format!("Failed to get local main hash: {}", stderr));
    }

    let local_hash = String::from_utf8_lossy(&local_output.stdout).trim().to_string();

    // Step 3: Get origin/main hash
    let remote_output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["rev-parse", "origin/main"])
        .output()
        .map_err(|e| format!("Failed to get origin/main hash: {}", e))?;

    if !remote_output.status.success() {
        let stderr = String::from_utf8_lossy(&remote_output.stderr);
        return Err(format!("Failed to get origin/main hash: {}", stderr));
    }

    let remote_hash = String::from_utf8_lossy(&remote_output.stdout).trim().to_string();

    // Step 4: Compare hashes
    if local_hash != remote_hash {
        return Err(format!(
            "Local main is out of sync with origin/main.\n\
             Local:  {}\n\
             Remote: {}\n\
             \n\
             Please push your local changes first:\n\
             git push origin main",
            local_hash, remote_hash
        ));
    }

    Ok(())
}

/// Run the merge command
pub fn run_merge(
    speck: String,
    dry_run: bool,
    _force: bool,
    json: bool,
    quiet: bool,
) -> Result<i32, String> {
    let repo_root =
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;

    // Step 0: Validate we're on main in the main worktree
    if let Err(e) = is_main_worktree(&repo_root) {
        let data = MergeData::error(e.clone(), dry_run);
        if json {
            println!("{}", serde_json::to_string_pretty(&data).unwrap());
        }
        return Err(e);
    }

    // Step 1: Find the worktree via git-native discovery
    let speck_path = normalize_speck_path(&speck);
    let discovered = match find_worktree_by_speck(&repo_root, &speck_path) {
        Ok(Some(wt)) => wt,
        Ok(None) => {
            let slug = derive_speck_slug(&speck_path);
            let e = format!(
                "No worktree found for speck: {} (looked for branch specks/{}-*)",
                speck_path.display(),
                slug
            );
            let data = MergeData::error(e.clone(), dry_run);
            if json {
                println!("{}", serde_json::to_string_pretty(&data).unwrap());
            }
            return Err(e);
        }
        Err(err) => {
            let e = format!("Failed to discover worktrees: {}", err);
            let data = MergeData::error(e.clone(), dry_run);
            if json {
                println!("{}", serde_json::to_string_pretty(&data).unwrap());
            }
            return Err(e);
        }
    };

    let wt_path = &discovered.path;
    let branch = &discovered.branch;

    // Step 1a: Detect mode
    let has_origin = has_remote_origin(&repo_root);

    // Step 1b: Get PR info (remote mode only)
    let pr_info = if has_origin {
        get_pr_for_branch(branch).ok() // No PR or gh not available — will fall back to local
    } else {
        None
    };

    // Effective mode: if remote but no open PR, fall back to local
    let effective_mode = if has_origin && pr_info.as_ref().is_some_and(|p| p.state == "OPEN") {
        "remote"
    } else {
        "local"
    };

    // Step 2: Check for dirty files (dry-run) or prepare main (actual merge)
    let dirty_files = get_dirty_files(&repo_root).unwrap_or_default();

    // Dry-run: report and exit
    if dry_run {
        let data = MergeData {
            status: "ok".to_string(),
            merge_mode: Some(effective_mode.to_string()),
            branch_name: Some(branch.clone()),
            worktree_path: Some(wt_path.display().to_string()),
            pr_url: pr_info.as_ref().map(|p| p.url.clone()),
            pr_number: pr_info.as_ref().map(|p| p.number),
            squash_commit: None,
            worktree_cleaned: None,
            dry_run: true,
            dirty_files: if dirty_files.is_empty() {
                None
            } else {
                Some(dirty_files.clone())
            },
            error: None,
            message: Some(match effective_mode {
                "remote" => format!(
                    "Would squash-merge PR #{} and clean up worktree",
                    pr_info.as_ref().map(|p| p.number).unwrap_or(0)
                ),
                _ => format!(
                    "Would squash-merge branch '{}' into main and clean up worktree",
                    branch
                ),
            }),
        };

        if json {
            println!("{}", serde_json::to_string_pretty(&data).unwrap());
        } else if !quiet {
            println!("Dry-run mode: showing planned operations\n");
            println!("Worktree: {}", wt_path.display());
            println!("Branch:   {}", branch);
            println!("Mode:     {}", effective_mode);
            if let Some(ref pr) = pr_info {
                println!("PR:       #{} - {}", pr.number, pr.url);
            }
            if !dirty_files.is_empty() {
                println!(
                    "\nUncommitted files in main ({}):\n  {}",
                    dirty_files.len(),
                    dirty_files.join("\n  ")
                );
            }
            println!("\nWould squash-merge and clean up worktree");
        }

        return Ok(0);
    }

    // Step 3: Prepare main — commit infrastructure, discard leaked files
    if !dirty_files.is_empty() {
        prepare_main_for_merge(&repo_root, quiet)?;
    }

    // Step 4: Merge
    let squash_commit = if effective_mode == "remote" {
        let pr = pr_info.as_ref().unwrap();
        if !quiet {
            println!("Merging PR #{} via squash...", pr.number);
        }

        let mut cmd = Command::new("gh");
        cmd.args(["pr", "merge", "--squash", branch]);
        if let Err(e) = run_cmd(&mut cmd, &format!("gh pr merge --squash {}", branch)) {
            let data = MergeData {
                status: "error".to_string(),
                merge_mode: Some("remote".to_string()),
                branch_name: Some(branch.clone()),
                worktree_path: Some(wt_path.display().to_string()),
                pr_url: Some(pr.url.clone()),
                pr_number: Some(pr.number),
                squash_commit: None,
                worktree_cleaned: None,
                dry_run: false,
                dirty_files: None,
                error: Some(format!("Failed to merge PR: {}", e)),
                message: None,
            };
            if json {
                println!("{}", serde_json::to_string_pretty(&data).unwrap());
            }
            return Err(format!("Failed to merge PR: {}", e));
        }

        // Pull to get the squashed commit
        let mut pull_cmd = Command::new("git");
        pull_cmd
            .current_dir(&repo_root)
            .args(["pull", "origin", "main"]);
        let _ = run_cmd(&mut pull_cmd, "git pull origin main");

        if !quiet {
            println!("PR #{} merged successfully", pr.number);
        }
        None
    } else {
        // Local mode
        if !quiet {
            println!("Squash merging branch '{}' into main...", branch);
        }

        let commit_msg = format!("Merge branch '{}'", branch);
        match squash_merge_branch(&repo_root, branch, &commit_msg) {
            Ok(hash) => {
                if !quiet {
                    println!("Squash merge successful: {}", hash);
                }
                Some(hash)
            }
            Err(e) => {
                let data = MergeData {
                    status: "error".to_string(),
                    merge_mode: Some("local".to_string()),
                    branch_name: Some(branch.clone()),
                    worktree_path: Some(wt_path.display().to_string()),
                    pr_url: None,
                    pr_number: None,
                    squash_commit: None,
                    worktree_cleaned: None,
                    dry_run: false,
                    dirty_files: None,
                    error: Some(format!("Squash merge failed: {}", e)),
                    message: None,
                };
                if json {
                    println!("{}", serde_json::to_string_pretty(&data).unwrap());
                }
                return Err(format!("Squash merge failed: {}", e));
            }
        }
    };

    // Step 5: Cleanup worktree and branch
    if !quiet {
        println!("Cleaning up worktree...");
    }

    // Try normal removal first (handles session cleanup), then force if needed
    let worktree_cleaned = if remove_worktree(wt_path, &repo_root).is_ok() {
        true
    } else {
        // Force removal — after a successful merge we don't need the worktree
        let force = Command::new("git")
            .current_dir(&repo_root)
            .args(["worktree", "remove", "--force"])
            .arg(wt_path.as_os_str())
            .output();
        match force {
            Ok(o) if o.status.success() => true,
            _ => {
                // Last resort: remove directory and prune
                let _ = std::fs::remove_dir_all(wt_path);
                true
            }
        }
    };

    // Always delete the branch and prune
    let _ = Command::new("git")
        .current_dir(&repo_root)
        .args(["branch", "-D", branch])
        .output();
    let _ = Command::new("git")
        .current_dir(&repo_root)
        .args(["worktree", "prune"])
        .output();

    if !quiet && worktree_cleaned {
        println!("Worktree cleaned up");
    }

    // Step 6: Success response
    let data = MergeData {
        status: "ok".to_string(),
        merge_mode: Some(effective_mode.to_string()),
        branch_name: Some(branch.clone()),
        worktree_path: Some(wt_path.display().to_string()),
        pr_url: pr_info.as_ref().map(|p| p.url.clone()),
        pr_number: pr_info.as_ref().map(|p| p.number),
        squash_commit: squash_commit.clone(),
        worktree_cleaned: Some(worktree_cleaned),
        dry_run: false,
        dirty_files: None,
        error: None,
        message: Some(match effective_mode {
            "remote" => format!(
                "Merged PR #{} and cleaned up",
                pr_info.as_ref().map(|p| p.number).unwrap_or(0)
            ),
            _ => format!("Squash merged '{}' and cleaned up", branch),
        }),
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&data).unwrap());
    } else if !quiet {
        println!("\nMerge complete!");
        if let Some(ref pr) = pr_info {
            println!("PR: {}", pr.url);
        }
        if let Some(ref hash) = squash_commit {
            println!("Commit: {}", hash);
        }
        if worktree_cleaned {
            println!("Worktree cleaned: {}", wt_path.display());
        }
    }

    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    fn init_git_repo(path: &Path) {
        Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["init", "-b", "main"])
            .output()
            .expect("git init");
        Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .expect("git config email");
        Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["config", "user.name", "Test User"])
            .output()
            .expect("git config name");
    }

    fn make_initial_commit(path: &Path) {
        fs::write(path.join("README.md"), "Test repo").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["add", "README.md"])
            .output()
            .expect("git add");
        Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .expect("git commit");
    }

    // -- MergeData serialization tests --

    #[test]
    fn test_merge_data_error_helper() {
        let data = MergeData::error("something broke".to_string(), false);
        let json = serde_json::to_string_pretty(&data).unwrap();
        assert!(json.contains("\"status\": \"error\""));
        assert!(json.contains("something broke"));
        assert!(!json.contains("\"dry_run\"")); // omitted when false
    }

    #[test]
    fn test_merge_data_dry_run_local() {
        let data = MergeData {
            status: "ok".to_string(),
            merge_mode: Some("local".to_string()),
            branch_name: Some("specks/1-20260210-120000".to_string()),
            worktree_path: Some(".specks-worktrees/specks__1-20260210-120000".to_string()),
            pr_url: None,
            pr_number: None,
            squash_commit: None,
            worktree_cleaned: None,
            dry_run: true,
            dirty_files: Some(vec![".beads/beads.jsonl".to_string()]),
            error: None,
            message: Some("Would squash-merge".to_string()),
        };

        let json = serde_json::to_string_pretty(&data).unwrap();
        assert!(json.contains("\"merge_mode\": \"local\""));
        assert!(json.contains("\"dry_run\": true"));
        assert!(json.contains("\"dirty_files\""));
        assert!(json.contains("beads.jsonl"));
        assert!(!json.contains("\"pr_url\""));
    }

    #[test]
    fn test_merge_data_success_remote() {
        let data = MergeData {
            status: "ok".to_string(),
            merge_mode: Some("remote".to_string()),
            branch_name: Some("specks/auth-20260210-120000".to_string()),
            worktree_path: None,
            pr_url: Some("https://github.com/owner/repo/pull/42".to_string()),
            pr_number: Some(42),
            squash_commit: None,
            worktree_cleaned: Some(true),
            dry_run: false,
            dirty_files: None,
            error: None,
            message: Some("Merged PR #42".to_string()),
        };

        let json = serde_json::to_string_pretty(&data).unwrap();
        assert!(json.contains("\"merge_mode\": \"remote\""));
        assert!(json.contains("\"pr_number\": 42"));
        assert!(json.contains("\"worktree_cleaned\": true"));
        assert!(!json.contains("\"dry_run\""));
    }

    #[test]
    fn test_merge_data_success_local() {
        let data = MergeData {
            status: "ok".to_string(),
            merge_mode: Some("local".to_string()),
            branch_name: Some("specks/1-20260210-120000".to_string()),
            worktree_path: None,
            pr_url: None,
            pr_number: None,
            squash_commit: Some("abc123def456".to_string()),
            worktree_cleaned: Some(true),
            dry_run: false,
            dirty_files: None,
            error: None,
            message: Some("Squash merged".to_string()),
        };

        let json = serde_json::to_string_pretty(&data).unwrap();
        assert!(json.contains("\"squash_commit\": \"abc123def456\""));
        assert!(!json.contains("\"pr_url\""));
    }

    // -- PrInfo deserialization tests --

    #[test]
    fn test_pr_info_deserialization() {
        let json = r#"{"number": 123, "url": "https://github.com/o/r/pull/123", "state": "OPEN"}"#;
        let pr: PrInfo = serde_json::from_str(json).unwrap();
        assert_eq!(pr.number, 123);
        assert_eq!(pr.state, "OPEN");
    }

    #[test]
    fn test_pr_info_deserialization_merged() {
        let json =
            r#"{"number": 456, "url": "https://github.com/o/r/pull/456", "state": "MERGED"}"#;
        let pr: PrInfo = serde_json::from_str(json).unwrap();
        assert_eq!(pr.state, "MERGED");
    }

    // -- normalize_speck_path tests --

    #[test]
    fn test_normalize_speck_path_already_qualified() {
        assert_eq!(
            normalize_speck_path(".specks/specks-1.md"),
            PathBuf::from(".specks/specks-1.md")
        );
    }

    #[test]
    fn test_normalize_speck_path_bare_filename() {
        assert_eq!(
            normalize_speck_path("specks-1.md"),
            PathBuf::from(".specks/specks-1.md")
        );
    }

    #[test]
    fn test_normalize_speck_path_dotslash() {
        assert_eq!(
            normalize_speck_path("./.specks/specks-1.md"),
            PathBuf::from(".specks/specks-1.md")
        );
    }

    // -- is_main_worktree tests --

    #[test]
    fn test_is_main_worktree_detects_main() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        init_git_repo(temp_path);
        make_initial_commit(temp_path);

        assert!(is_main_worktree(temp_path).is_ok());
    }

    #[test]
    fn test_is_main_worktree_rejects_worktree() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        init_git_repo(temp_path);
        make_initial_commit(temp_path);

        let wt_path = temp_path.join("test-worktree");
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args([
                "worktree",
                "add",
                wt_path.to_str().unwrap(),
                "-b",
                "test-branch",
            ])
            .output()
            .expect("git worktree add");

        let result = is_main_worktree(&wt_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("git worktree"));
    }

    #[test]
    fn test_is_main_worktree_rejects_wrong_branch() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        init_git_repo(temp_path);
        make_initial_commit(temp_path);

        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["checkout", "-b", "feature-branch"])
            .output()
            .expect("git checkout");

        let result = is_main_worktree(temp_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("feature-branch"));
    }

    #[test]
    fn test_is_main_worktree_no_git() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let result = is_main_worktree(temp_dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Not in a git repository"));
    }

    // -- has_remote_origin tests --

    #[test]
    fn test_has_remote_origin_with_remote() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        init_git_repo(temp_path);
        make_initial_commit(temp_path);

        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args([
                "remote",
                "add",
                "origin",
                "https://github.com/test/repo.git",
            ])
            .output()
            .expect("git remote add");

        assert!(has_remote_origin(temp_path));
    }

    #[test]
    fn test_has_remote_origin_without_remote() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        init_git_repo(temp_path);
        make_initial_commit(temp_path);

        assert!(!has_remote_origin(temp_path));
    }

    // -- squash_merge_branch tests --

    #[test]
    fn test_squash_merge_success() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        init_git_repo(temp_path);

        // Initial commit on main
        fs::write(temp_path.join("file1.txt"), "main").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["add", "file1.txt"])
            .output()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["commit", "-m", "Initial"])
            .output()
            .unwrap();

        // Feature branch with 2 commits
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["checkout", "-b", "feature"])
            .output()
            .unwrap();

        fs::write(temp_path.join("file2.txt"), "feature1").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["add", "file2.txt"])
            .output()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["commit", "-m", "feat1"])
            .output()
            .unwrap();

        fs::write(temp_path.join("file3.txt"), "feature2").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["add", "file3.txt"])
            .output()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["commit", "-m", "feat2"])
            .output()
            .unwrap();

        // Back to main, squash merge
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["checkout", "main"])
            .output()
            .unwrap();

        let result = squash_merge_branch(temp_path, "feature", "Squashed");
        assert!(result.is_ok());

        let hash = result.unwrap();
        assert_eq!(hash.len(), 40);

        // Verify files exist
        assert!(temp_path.join("file2.txt").exists());
        assert!(temp_path.join("file3.txt").exists());
    }

    #[test]
    fn test_squash_merge_conflict_restores_clean_state() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        init_git_repo(temp_path);

        fs::write(temp_path.join("f.txt"), "main").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["add", "f.txt"])
            .output()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["commit", "-m", "init"])
            .output()
            .unwrap();

        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["checkout", "-b", "feat"])
            .output()
            .unwrap();
        fs::write(temp_path.join("f.txt"), "feature version").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["add", "f.txt"])
            .output()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["commit", "-m", "feat"])
            .output()
            .unwrap();

        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["checkout", "main"])
            .output()
            .unwrap();
        fs::write(temp_path.join("f.txt"), "main updated").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["add", "f.txt"])
            .output()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["commit", "-m", "main update"])
            .output()
            .unwrap();

        let result = squash_merge_branch(temp_path, "feat", "Should fail");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Merge failed"));
        assert!(err.contains("clean state"));

        // Verify repo is clean
        let status = Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["status", "--porcelain"])
            .output()
            .unwrap();
        assert!(String::from_utf8_lossy(&status.stdout).is_empty());
    }

    #[test]
    fn test_squash_merge_empty() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        init_git_repo(temp_path);

        fs::write(temp_path.join("f.txt"), "x").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["add", "f.txt"])
            .output()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["commit", "-m", "init"])
            .output()
            .unwrap();

        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["checkout", "-b", "empty-branch"])
            .output()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(temp_path)
            .args(["checkout", "main"])
            .output()
            .unwrap();

        let result = squash_merge_branch(temp_path, "empty-branch", "No changes");
        assert!(result.is_err());
    }

    #[test]
    fn test_squash_merge_nonexistent_branch() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        init_git_repo(temp_path);
        make_initial_commit(temp_path);

        let result = squash_merge_branch(temp_path, "nonexistent", "fail");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Merge failed"));
    }

    // -- auto-resolve infrastructure conflicts tests --

    /// Helper to create a repo with a branch that conflicts on the given file paths.
    /// Returns (temp_dir, branch_name).
    fn setup_conflict_repo(
        paths: &[(&str, &str, &str)], // (path, main_content, branch_content)
    ) -> (tempfile::TempDir, String) {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let p = temp_dir.path();
        init_git_repo(p);

        // Initial commit with all files
        for (path, content, _) in paths {
            if let Some(parent) = std::path::Path::new(path).parent() {
                if !parent.as_os_str().is_empty() {
                    fs::create_dir_all(p.join(parent)).unwrap();
                }
            }
            fs::write(p.join(path), content).unwrap();
            Command::new("git")
                .arg("-C")
                .arg(p)
                .args(["add", path])
                .output()
                .unwrap();
        }
        Command::new("git")
            .arg("-C")
            .arg(p)
            .args(["commit", "-m", "initial"])
            .output()
            .unwrap();

        // Create branch and modify files there
        Command::new("git")
            .arg("-C")
            .arg(p)
            .args(["checkout", "-b", "feature"])
            .output()
            .unwrap();
        for (path, _, branch_content) in paths {
            fs::write(p.join(path), branch_content).unwrap();
            Command::new("git")
                .arg("-C")
                .arg(p)
                .args(["add", path])
                .output()
                .unwrap();
        }
        Command::new("git")
            .arg("-C")
            .arg(p)
            .args(["commit", "-m", "branch changes"])
            .output()
            .unwrap();

        // Back to main, make conflicting changes
        Command::new("git")
            .arg("-C")
            .arg(p)
            .args(["checkout", "main"])
            .output()
            .unwrap();
        for (path, _, _) in paths {
            let main_conflict = format!("main-conflict-{}", path);
            fs::write(p.join(path), main_conflict).unwrap();
            Command::new("git")
                .arg("-C")
                .arg(p)
                .args(["add", path])
                .output()
                .unwrap();
        }
        Command::new("git")
            .arg("-C")
            .arg(p)
            .args(["commit", "-m", "main conflict"])
            .output()
            .unwrap();

        (temp_dir, "feature".to_string())
    }

    #[test]
    fn test_squash_merge_auto_resolves_infrastructure_conflicts() {
        let (temp_dir, branch) = setup_conflict_repo(&[
            (
                ".specks/specks-implementation-log.md",
                "# Log\noriginal",
                "# Log\nstep-0\nstep-1\nstep-2",
            ),
            (".beads/beads.jsonl", "original", "updated-beads"),
        ]);
        let p = temp_dir.path();

        let result = squash_merge_branch(p, &branch, "Squash with auto-resolve");
        assert!(result.is_ok(), "Expected success, got: {:?}", result);

        // Verify the branch versions won (theirs)
        let log = fs::read_to_string(p.join(".specks/specks-implementation-log.md")).unwrap();
        assert!(
            log.contains("step-0"),
            "Log should have branch content: {}",
            log
        );
        let beads = fs::read_to_string(p.join(".beads/beads.jsonl")).unwrap();
        assert!(
            beads.contains("updated-beads"),
            "Beads should have branch content: {}",
            beads
        );
    }

    #[test]
    fn test_squash_merge_fails_on_code_conflicts() {
        let (temp_dir, branch) = setup_conflict_repo(&[
            ("src/main.py", "original", "branch version"),
            (
                ".specks/specks-implementation-log.md",
                "# Log",
                "# Log\nentry",
            ),
        ]);
        let p = temp_dir.path();

        let result = squash_merge_branch(p, &branch, "Should fail");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("code file conflicts"),
            "Expected code conflict error, got: {}",
            err
        );

        // Verify repo is clean after abort
        let status = Command::new("git")
            .arg("-C")
            .arg(p)
            .args(["status", "--porcelain"])
            .output()
            .unwrap();
        assert!(
            String::from_utf8_lossy(&status.stdout).is_empty(),
            "Repo should be clean after abort"
        );
    }

    // -- run_cmd tests --

    #[test]
    fn test_run_cmd_success() {
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        let result = run_cmd(&mut cmd, "echo hello");
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_cmd_failure_includes_context() {
        let mut cmd = Command::new("false");
        let result = run_cmd(&mut cmd, "false");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("'false' failed"));
    }

    #[test]
    fn test_run_cmd_missing_command() {
        let mut cmd = Command::new("this-does-not-exist-12345");
        let result = run_cmd(&mut cmd, "missing");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to execute"));
    }

    // -- get_dirty_files tests --

    #[test]
    fn test_get_dirty_files_clean_repo() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        init_git_repo(temp_path);
        make_initial_commit(temp_path);

        let files = get_dirty_files(temp_path).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_get_dirty_files_with_changes() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        init_git_repo(temp_path);
        make_initial_commit(temp_path);

        fs::write(temp_path.join("new_file.txt"), "new").unwrap();
        fs::create_dir_all(temp_path.join(".beads")).unwrap();
        fs::write(temp_path.join(".beads/beads.jsonl"), "{}").unwrap();

        let files = get_dirty_files(temp_path).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.contains(&".beads/beads.jsonl".to_string()));
        assert!(files.contains(&"new_file.txt".to_string()));
    }

    // -- check_main_sync tests --

    #[test]
    fn test_check_main_sync_in_sync() {
        use tempfile::TempDir;

        // Create bare origin repo
        let origin_dir = TempDir::new().unwrap();
        let origin_path = origin_dir.path();
        Command::new("git")
            .arg("-C")
            .arg(origin_path)
            .args(["init", "--bare"])
            .output()
            .expect("git init --bare");

        // Create clone
        let clone_dir = TempDir::new().unwrap();
        let clone_path = clone_dir.path();
        Command::new("git")
            .args(["clone", origin_path.to_str().unwrap(), clone_path.to_str().unwrap()])
            .output()
            .expect("git clone");

        // Configure clone
        Command::new("git")
            .arg("-C")
            .arg(clone_path)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .expect("git config email");
        Command::new("git")
            .arg("-C")
            .arg(clone_path)
            .args(["config", "user.name", "Test User"])
            .output()
            .expect("git config name");

        // Make initial commit and push to origin
        fs::write(clone_path.join("README.md"), "Test").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(clone_path)
            .args(["add", "README.md"])
            .output()
            .expect("git add");
        Command::new("git")
            .arg("-C")
            .arg(clone_path)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .expect("git commit");
        Command::new("git")
            .arg("-C")
            .arg(clone_path)
            .args(["push", "origin", "main"])
            .output()
            .expect("git push");

        // Check sync — should pass since we just pushed
        let result = check_main_sync(clone_path);
        assert!(result.is_ok(), "Expected sync check to pass: {:?}", result);
    }

    #[test]
    fn test_check_main_sync_diverged() {
        use tempfile::TempDir;

        // Create bare origin repo
        let origin_dir = TempDir::new().unwrap();
        let origin_path = origin_dir.path();
        Command::new("git")
            .arg("-C")
            .arg(origin_path)
            .args(["init", "--bare"])
            .output()
            .expect("git init --bare");

        // Create clone
        let clone_dir = TempDir::new().unwrap();
        let clone_path = clone_dir.path();
        Command::new("git")
            .args(["clone", origin_path.to_str().unwrap(), clone_path.to_str().unwrap()])
            .output()
            .expect("git clone");

        // Configure clone
        Command::new("git")
            .arg("-C")
            .arg(clone_path)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .expect("git config email");
        Command::new("git")
            .arg("-C")
            .arg(clone_path)
            .args(["config", "user.name", "Test User"])
            .output()
            .expect("git config name");

        // Initial commit and push
        fs::write(clone_path.join("README.md"), "Test").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(clone_path)
            .args(["add", "README.md"])
            .output()
            .expect("git add");
        Command::new("git")
            .arg("-C")
            .arg(clone_path)
            .args(["commit", "-m", "Initial"])
            .output()
            .expect("git commit");
        Command::new("git")
            .arg("-C")
            .arg(clone_path)
            .args(["push", "origin", "main"])
            .output()
            .expect("git push");

        // Make local commit but don't push
        fs::write(clone_path.join("local.txt"), "local change").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(clone_path)
            .args(["add", "local.txt"])
            .output()
            .expect("git add");
        Command::new("git")
            .arg("-C")
            .arg(clone_path)
            .args(["commit", "-m", "Local commit"])
            .output()
            .expect("git commit");

        // Check sync — should fail with actionable message
        let result = check_main_sync(clone_path);
        assert!(result.is_err(), "Expected sync check to fail");
        let err = result.unwrap_err();
        assert!(err.contains("out of sync"), "Error should mention sync: {}", err);
        assert!(err.contains("git push origin main"), "Error should suggest push: {}", err);
    }

    #[test]
    fn test_check_main_sync_no_origin() {
        use tempfile::TempDir;

        // Create standalone repo without origin remote
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        init_git_repo(temp_path);
        make_initial_commit(temp_path);

        // Check sync — should fail because no origin remote
        let result = check_main_sync(temp_path);
        assert!(result.is_err(), "Expected sync check to fail without origin");
        let err = result.unwrap_err();
        assert!(err.contains("Failed to"), "Error should indicate fetch failure: {}", err);
    }
}
