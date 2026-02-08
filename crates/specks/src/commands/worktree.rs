//! Worktree CLI commands
//!
//! Provides subcommands for creating, listing, and cleaning up worktrees
//! for isolated speck implementation environments.

use clap::Subcommand;
use serde::Serialize;
use specks_core::{
    session::Session,
    worktree::{WorktreeConfig, cleanup_worktrees, create_worktree, list_worktrees},
};
use std::path::PathBuf;

/// Worktree subcommands
#[derive(Subcommand, Debug)]
pub enum WorktreeCommands {
    /// Create worktree for implementation
    ///
    /// Creates a git worktree and branch for implementing a speck in isolation.
    #[command(
        long_about = "Create worktree for speck implementation.\n\nCreates:\n  - Branch: specks/<slug>-<timestamp>\n  - Worktree: .specks-worktrees/<sanitized-branch-name>/\n  - Session: session.json tracking state\n\nValidates that the speck has at least one execution step."
    )]
    Create {
        /// Speck file to implement
        speck: String,

        /// Base branch to create worktree from (default: main)
        #[arg(long, default_value = "main")]
        base: String,
    },

    /// List active worktrees with status
    ///
    /// Shows all worktrees with their branch, status, and progress.
    #[command(
        long_about = "List active worktrees.\n\nDisplays:\n  - Branch name\n  - Worktree path\n  - Status (pending/in_progress/completed/failed/needs_reconcile)\n  - Current step / total steps\n\nUse --json for machine-readable output."
    )]
    List,

    /// Remove worktrees for merged PRs
    ///
    /// Cleans up worktrees whose branches have been merged.
    #[command(
        long_about = "Remove worktrees for merged branches.\n\nUses git merge-base to detect merged branches.\nRemoves both the worktree directory and the branch.\n\nRequires --merged flag for safety.\nUse --dry-run to preview what would be removed.\n\nNote: Squash/rebase merges may not be detected. See speck D09 for details."
    )]
    Cleanup {
        /// Only remove merged worktrees (required)
        #[arg(long, required = true)]
        merged: bool,

        /// Show what would be removed without removing
        #[arg(long)]
        dry_run: bool,
    },
}

/// JSON output for create command
#[derive(Serialize)]
pub struct CreateData {
    pub worktree_path: String,
    pub branch_name: String,
    pub base_branch: String,
    pub speck_path: String,
    pub total_steps: usize,
}

/// JSON output for list command
#[derive(Serialize)]
pub struct ListData {
    pub worktrees: Vec<Session>,
}

/// JSON output for cleanup command
#[derive(Serialize)]
pub struct CleanupData {
    pub removed: Vec<String>,
    pub dry_run: bool,
}

/// Run worktree create command
pub fn run_worktree_create(
    speck: String,
    base: String,
    json_output: bool,
    quiet: bool,
) -> Result<i32, String> {
    let repo_root = std::env::current_dir().map_err(|e| e.to_string())?;
    let speck_path = PathBuf::from(&speck);

    // Check if speck file exists
    if !repo_root.join(&speck_path).exists() {
        if json_output {
            eprintln!(
                r#"{{"error": "Speck file not found: {}"}}"#,
                speck_path.display()
            );
        } else if !quiet {
            eprintln!("error: Speck file not found: {}", speck_path.display());
        }
        return Ok(7); // Exit code 7: Speck file not found
    }

    let config = WorktreeConfig {
        speck_path,
        base_branch: base,
        repo_root: repo_root.clone(),
    };

    match create_worktree(&config) {
        Ok(session) => {
            if json_output {
                let data = CreateData {
                    worktree_path: session.worktree_path.clone(),
                    branch_name: session.branch_name.clone(),
                    base_branch: session.base_branch.clone(),
                    speck_path: session.speck_path.clone(),
                    total_steps: session.total_steps,
                };
                println!(
                    "{}",
                    serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?
                );
            } else if !quiet {
                println!("Created worktree for speck: {}", session.speck_path);
                println!("  Branch: {}", session.branch_name);
                println!("  Worktree: {}", session.worktree_path);
                println!("  Steps: {}", session.total_steps);
            }
            Ok(0)
        }
        Err(e) => {
            // Map error to appropriate exit code
            let exit_code = match &e {
                specks_core::error::SpecksError::NotAGitRepository => 5,
                specks_core::error::SpecksError::GitVersionInsufficient => 4,
                specks_core::error::SpecksError::BaseBranchNotFound { .. } => 6,
                specks_core::error::SpecksError::SpeckHasNoSteps => 8,
                specks_core::error::SpecksError::WorktreeAlreadyExists => 3,
                _ => 1,
            };

            if json_output {
                eprintln!(r#"{{"error": "{}"}}"#, e);
            } else if !quiet {
                eprintln!("error: {}", e);
            }
            Ok(exit_code)
        }
    }
}

/// Run worktree list command
pub fn run_worktree_list(json_output: bool, quiet: bool) -> Result<i32, String> {
    let repo_root = std::env::current_dir().map_err(|e| e.to_string())?;

    match list_worktrees(&repo_root) {
        Ok(sessions) => {
            if json_output {
                let data = ListData {
                    worktrees: sessions,
                };
                println!(
                    "{}",
                    serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?
                );
            } else if !quiet {
                if sessions.is_empty() {
                    println!("No active worktrees");
                } else {
                    println!("Active worktrees:\n");
                    for session in sessions {
                        println!("  Branch: {}", session.branch_name);
                        println!("  Path:   {}", session.worktree_path);
                        println!("  Speck:  {}", session.speck_path);
                        println!(
                            "  Status: {:?} (step {}/{})",
                            session.status, session.current_step, session.total_steps
                        );
                        println!();
                    }
                }
            }
            Ok(0)
        }
        Err(e) => {
            if json_output {
                eprintln!(r#"{{"error": "{}"}}"#, e);
            } else if !quiet {
                eprintln!("error: {}", e);
            }
            Ok(1)
        }
    }
}

/// Run worktree cleanup command
pub fn run_worktree_cleanup(
    _merged: bool, // Required by clap, but not used (always true due to clap required)
    dry_run: bool,
    json_output: bool,
    quiet: bool,
) -> Result<i32, String> {
    let repo_root = std::env::current_dir().map_err(|e| e.to_string())?;

    match cleanup_worktrees(&repo_root, dry_run) {
        Ok(removed) => {
            if json_output {
                let data = CleanupData {
                    removed: removed.clone(),
                    dry_run,
                };
                println!(
                    "{}",
                    serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?
                );
            } else if !quiet {
                if dry_run {
                    if removed.is_empty() {
                        println!("No merged worktrees to remove");
                    } else {
                        println!("Would remove {} merged worktree(s):", removed.len());
                        for branch in removed {
                            println!("  - {}", branch);
                        }
                    }
                } else if removed.is_empty() {
                    println!("No merged worktrees removed");
                } else {
                    println!("Removed {} merged worktree(s):", removed.len());
                    for branch in removed {
                        println!("  - {}", branch);
                    }
                }
            }
            Ok(0)
        }
        Err(e) => {
            if json_output {
                eprintln!(r#"{{"error": "{}"}}"#, e);
            } else if !quiet {
                eprintln!("error: {}", e);
            }
            Ok(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_data_serialization() {
        let data = CreateData {
            worktree_path: "/path/to/worktree".to_string(),
            branch_name: "specks/test-20260208-120000".to_string(),
            base_branch: "main".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            total_steps: 5,
        };

        let json = serde_json::to_string(&data).expect("serialization should succeed");
        assert!(json.contains("worktree_path"));
        assert!(json.contains("branch_name"));
    }

    #[test]
    fn test_list_data_serialization() {
        let data = ListData { worktrees: vec![] };

        let json = serde_json::to_string(&data).expect("serialization should succeed");
        assert!(json.contains("worktrees"));
    }

    #[test]
    fn test_cleanup_data_serialization() {
        let data = CleanupData {
            removed: vec!["specks/test-123".to_string()],
            dry_run: true,
        };

        let json = serde_json::to_string(&data).expect("serialization should succeed");
        assert!(json.contains("removed"));
        assert!(json.contains("dry_run"));
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    /// Create a test git repository with a minimal speck
    fn setup_test_repo() -> (TempDir, PathBuf) {
        let temp = tempfile::tempdir().expect("failed to create temp dir");
        let repo_path = temp.path().to_path_buf();

        // Initialize git repo with explicit main branch
        Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(&repo_path)
            .output()
            .expect("failed to init git repo");

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&repo_path)
            .output()
            .expect("failed to set git user.email");

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&repo_path)
            .output()
            .expect("failed to set git user.name");

        // Create .specks directory and a minimal speck
        let specks_dir = repo_path.join(".specks");
        fs::create_dir(&specks_dir).expect("failed to create .specks dir");

        let speck_path = specks_dir.join("specks-test.md");
        let speck_content = r#"## Phase 1.0: Test {#phase-1}

**Purpose:** Test speck.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | active |
| Last updated | 2026-02-08 |

---

### 1.0.0 Execution Steps {#execution-steps}

#### Step 0: Test Step {#step-0}

**Tasks:**
- [ ] Test task
"#;
        fs::write(&speck_path, speck_content).expect("failed to write speck");

        // Initial commit
        Command::new("git")
            .args(["add", "."])
            .current_dir(&repo_path)
            .output()
            .expect("failed to git add");

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&repo_path)
            .output()
            .expect("failed to git commit");

        (temp, repo_path)
    }

    #[test]
    fn test_create_worktree_succeeds() {
        let (_temp, repo_path) = setup_test_repo();
        let speck_path = ".specks/specks-test.md";

        let config = WorktreeConfig {
            speck_path: PathBuf::from(speck_path),
            base_branch: "main".to_string(),
            repo_root: repo_path.clone(),
        };

        let result = create_worktree(&config);
        assert!(
            result.is_ok(),
            "create_worktree should succeed: {:?}",
            result.err()
        );

        let session = result.unwrap();
        assert_eq!(session.total_steps, 1);
        assert_eq!(session.base_branch, "main");
        assert!(session.branch_name.starts_with("specks/test-"));

        // Verify worktree directory exists
        let worktree_path = PathBuf::from(&session.worktree_path);
        assert!(worktree_path.exists(), "worktree directory should exist");
    }

    #[test]
    fn test_list_worktrees() {
        let (_temp, repo_path) = setup_test_repo();
        let speck_path = ".specks/specks-test.md";

        let config = WorktreeConfig {
            speck_path: PathBuf::from(speck_path),
            base_branch: "main".to_string(),
            repo_root: repo_path.clone(),
        };

        // Create a worktree
        let session = create_worktree(&config).expect("create_worktree should succeed");

        // List worktrees
        let worktrees = list_worktrees(&repo_path).expect("list_worktrees should succeed");

        assert_eq!(worktrees.len(), 1, "should have one worktree");
        assert_eq!(worktrees[0].branch_name, session.branch_name);
    }

    #[test]
    fn test_cleanup_dry_run() {
        let (_temp, repo_path) = setup_test_repo();
        let speck_path = ".specks/specks-test.md";

        let config = WorktreeConfig {
            speck_path: PathBuf::from(speck_path),
            base_branch: "main".to_string(),
            repo_root: repo_path.clone(),
        };

        // Create a worktree
        let session = create_worktree(&config).expect("create_worktree should succeed");
        let worktree_path = PathBuf::from(&session.worktree_path);

        // Switch to worktree and make a commit
        fs::write(worktree_path.join("test.txt"), "test").expect("failed to write test file");

        Command::new("git")
            .args(["add", "test.txt"])
            .current_dir(&worktree_path)
            .output()
            .expect("failed to git add");

        Command::new("git")
            .args(["commit", "-m", "Test commit"])
            .current_dir(&worktree_path)
            .output()
            .expect("failed to git commit");

        // Merge the branch into main
        Command::new("git")
            .args(["checkout", "main"])
            .current_dir(&repo_path)
            .output()
            .expect("failed to checkout main");

        Command::new("git")
            .args(["merge", &session.branch_name])
            .current_dir(&repo_path)
            .output()
            .expect("failed to merge");

        // Dry run should detect merged worktree but not remove it
        let removed =
            cleanup_worktrees(&repo_path, true).expect("cleanup_worktrees dry run should succeed");

        assert_eq!(removed.len(), 1, "should detect one merged worktree");
        assert_eq!(removed[0], session.branch_name);
        assert!(
            worktree_path.exists(),
            "worktree directory should still exist after dry run"
        );

        // Verify worktree is still listed
        let worktrees = list_worktrees(&repo_path).expect("list_worktrees should succeed");
        assert_eq!(worktrees.len(), 1, "worktree should still be listed");
    }
}
