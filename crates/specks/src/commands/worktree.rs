//! Worktree CLI commands
//!
//! Provides subcommands for creating, listing, and cleaning up worktrees
//! for isolated speck implementation environments.

use clap::Subcommand;
use serde::Serialize;
use specks_core::{
    session::{Session, delete_session, session_id_from_worktree},
    worktree::{
        CleanupMode, WorktreeConfig, cleanup_worktrees, create_worktree, list_worktrees,
        remove_worktree,
    },
};
use std::path::{Path, PathBuf};

/// Worktree subcommands
#[derive(Subcommand, Debug)]
pub enum WorktreeCommands {
    /// Create worktree for implementation
    ///
    /// Creates a git worktree and branch for implementing a speck in isolation.
    #[command(
        long_about = "Create worktree for speck implementation.\n\nCreates:\n  - Branch: specks/<slug>-<timestamp>\n  - Worktree: .specks-worktrees/<sanitized-branch-name>/\n  - Session: session.json tracking state\n\nBeads sync is always-on:\n  - Atomically syncs beads and commits annotations in worktree\n  - Full rollback if sync or commit fails\n\nWorktree creation is idempotent:\n  - Returns existing worktree if one exists for this speck\n  - Creates new worktree if none exists\n\nValidates that the speck has at least one execution step."
    )]
    Create {
        /// Speck file to implement
        speck: String,

        /// Base branch to create worktree from (default: main)
        #[arg(long, default_value = "main")]
        base: String,
    },

    /// List active worktrees with progress
    ///
    /// Shows all worktrees with their branch and progress.
    #[command(
        long_about = "List active worktrees.\n\nDisplays:\n  - Branch name\n  - Worktree path\n  - Progress (completed / total steps)\n\nUse --json for machine-readable output."
    )]
    List,

    /// Remove worktrees based on cleanup mode
    ///
    /// Cleans up worktrees based on PR state.
    #[command(
        long_about = "Remove worktrees based on cleanup mode.\n\nModes:\n  --merged: Remove worktrees with merged PRs\n  --orphaned: Remove worktrees with no PR\n  --stale: Remove specks/* branches without worktrees\n  --all: Remove all eligible worktrees (merged + orphaned + closed + stale branches)\n\nUse --dry-run to preview what would be removed.\n\nWorktrees with open PRs are always protected."
    )]
    Cleanup {
        /// Only remove merged worktrees
        #[arg(long)]
        merged: bool,

        /// Only remove orphaned worktrees (no PR)
        #[arg(long)]
        orphaned: bool,

        /// Only remove stale branches (specks/* branches without worktrees)
        #[arg(long)]
        stale: bool,

        /// Remove all eligible worktrees (merged + orphaned + closed + stale branches)
        #[arg(long)]
        all: bool,

        /// Show what would be removed without removing
        #[arg(long)]
        dry_run: bool,
    },

    /// Remove a specific worktree
    ///
    /// Removes a worktree identified by speck path, branch name, or worktree path.
    #[command(
        long_about = "Remove a specific worktree.\n\nIdentifies worktree by:\n  - Speck path (e.g., .specks/specks-14.md)\n  - Branch name (e.g., specks/14-20250209-172637)\n  - Worktree path (e.g., .specks-worktrees/specks__14-...)\n\nIf multiple worktrees match a speck path, an error is returned\nlisting all candidates. Use branch name or worktree path to disambiguate.\n\nUse --force to remove dirty worktrees with uncommitted changes."
    )]
    Remove {
        /// Target identifier (speck path, branch name, or worktree path)
        target: String,

        /// Force removal of dirty worktree
        #[arg(long)]
        force: bool,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bead_mapping: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_bead_id: Option<String>,
    #[serde(skip_serializing_if = "is_false")]
    pub reused: bool,
    // V2 enriched fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts_base: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_steps: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ready_steps: Option<Vec<String>>,
}

fn is_false(b: &bool) -> bool {
    !b
}

/// JSON output for list command
#[derive(Serialize)]
pub struct ListData {
    pub worktrees: Vec<Session>,
}

/// JSON output for cleanup command
#[derive(Serialize)]
pub struct CleanupData {
    pub merged_removed: Vec<String>,
    pub orphaned_removed: Vec<String>,
    pub stale_branches_removed: Vec<String>,
    pub skipped: Vec<(String, String)>,
    pub dry_run: bool,
}

/// JSON output for remove command
#[derive(Serialize)]
pub struct RemoveData {
    pub worktree_path: String,
    pub branch_name: String,
    pub speck_path: String,
}

/// Sync beads within the worktree and return bead mapping
fn sync_beads_in_worktree(
    worktree_path: &Path,
    speck_path: &str,
) -> Result<
    (std::collections::HashMap<String, String>, Option<String>),
    specks_core::error::SpecksError,
> {
    use crate::commands::beads::sync::SyncData;
    use crate::output::JsonResponse;
    use std::process::Command;

    // Run specks beads sync in the worktree
    let output = Command::new(std::env::current_exe().map_err(|e| {
        specks_core::error::SpecksError::BeadsSyncFailed {
            reason: format!("failed to get current exe: {}", e),
        }
    })?)
    .args(["beads", "sync", speck_path, "--json"])
    .current_dir(worktree_path)
    .output()
    .map_err(|e| specks_core::error::SpecksError::BeadsSyncFailed {
        reason: format!("failed to execute beads sync: {}", e),
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(specks_core::error::SpecksError::BeadsSyncFailed {
            reason: format!("beads sync failed: {}", stderr),
        });
    }

    // Parse JSON output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let response: JsonResponse<SyncData> = serde_json::from_str(&stdout).map_err(|e| {
        specks_core::error::SpecksError::BeadsSyncFailed {
            reason: format!("failed to parse sync output: {}", e),
        }
    })?;

    if response.status != "ok" {
        return Err(specks_core::error::SpecksError::BeadsSyncFailed {
            reason: "beads sync returned error status".to_string(),
        });
    }

    // Re-parse the speck to extract bead mapping
    let speck_content = std::fs::read_to_string(worktree_path.join(speck_path)).map_err(|e| {
        specks_core::error::SpecksError::BeadsSyncFailed {
            reason: format!("failed to read synced speck: {}", e),
        }
    })?;

    let speck = specks_core::parse_speck(&speck_content).map_err(|e| {
        specks_core::error::SpecksError::BeadsSyncFailed {
            reason: format!("failed to parse synced speck: {}", e),
        }
    })?;

    // Build bead mapping from step anchors to bead IDs
    let mut bead_mapping = std::collections::HashMap::new();
    for step in &speck.steps {
        if let Some(ref bead_id) = step.bead_id {
            bead_mapping.insert(step.anchor.clone(), bead_id.clone());
        }
    }

    Ok((bead_mapping, response.data.root_bead_id))
}

/// Commit bead annotations in the worktree
fn commit_bead_annotations(
    worktree_path: &Path,
    speck_path: &str,
    speck_name: &str,
) -> Result<(), specks_core::error::SpecksError> {
    use std::process::Command;

    // Stage the .specks/ directory (includes init files: config, log, skeleton)
    let status = Command::new("git")
        .args(["-C", &worktree_path.to_string_lossy(), "add", ".specks/"])
        .status()
        .map_err(|e| specks_core::error::SpecksError::BeadCommitFailed {
            reason: format!("failed to stage .specks/ directory: {}", e),
        })?;

    if !status.success() {
        return Err(specks_core::error::SpecksError::BeadCommitFailed {
            reason: "git add .specks/ failed".to_string(),
        });
    }

    // Stage the speck file (includes bead annotations)
    let status = Command::new("git")
        .args(["-C", &worktree_path.to_string_lossy(), "add", speck_path])
        .status()
        .map_err(|e| specks_core::error::SpecksError::BeadCommitFailed {
            reason: format!("failed to stage speck: {}", e),
        })?;

    if !status.success() {
        return Err(specks_core::error::SpecksError::BeadCommitFailed {
            reason: "git add speck failed".to_string(),
        });
    }

    // Commit the changes (both init files and bead annotations)
    let commit_msg = format!("chore: init worktree and sync beads for {}", speck_name);
    let status = Command::new("git")
        .args([
            "-C",
            &worktree_path.to_string_lossy(),
            "commit",
            "-m",
            &commit_msg,
        ])
        .status()
        .map_err(|e| specks_core::error::SpecksError::BeadCommitFailed {
            reason: format!("failed to commit: {}", e),
        })?;

    if !status.success() {
        return Err(specks_core::error::SpecksError::BeadCommitFailed {
            reason: "git commit failed".to_string(),
        });
    }

    Ok(())
}

/// Rollback worktree creation by removing worktree and branch
fn rollback_worktree_creation(
    worktree_path: &Path,
    branch_name: &str,
    repo_root: &Path,
) -> Result<(), specks_core::error::SpecksError> {
    use std::process::Command;

    // Remove worktree directory
    let _ = Command::new("git")
        .args([
            "-C",
            &repo_root.to_string_lossy(),
            "worktree",
            "remove",
            &worktree_path.to_string_lossy(),
            "--force",
        ])
        .status();

    // Delete branch
    let _ = Command::new("git")
        .args([
            "-C",
            &repo_root.to_string_lossy(),
            "branch",
            "-D",
            branch_name,
        ])
        .status();

    Ok(())
}

/// Run worktree create command
///
/// If `override_root` is provided, use it instead of `current_dir()`.
/// This avoids the `set_current_dir` anti-pattern in tests.
pub fn run_worktree_create(
    speck: String,
    base: String,
    json_output: bool,
    quiet: bool,
) -> Result<i32, String> {
    run_worktree_create_with_root(speck, base, json_output, quiet, None)
}

/// Inner implementation that accepts an explicit repo root.
pub fn run_worktree_create_with_root(
    speck: String,
    base: String,
    json_output: bool,
    quiet: bool,
    override_root: Option<&Path>,
) -> Result<i32, String> {
    let repo_root = match override_root {
        Some(root) => root.to_path_buf(),
        None => std::env::current_dir().map_err(|e| e.to_string())?,
    };
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
        speck_path: speck_path.clone(),
        base_branch: base,
        repo_root: repo_root.clone(),
    };

    match create_worktree(&config) {
        Ok((worktree_path, branch_name, speck_slug)) => {
            let speck_name = speck_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");

            // Check if this is a reused worktree by looking for existing session
            let existing_session =
                specks_core::session::load_session(&worktree_path, Some(&repo_root)).ok();
            let reused = existing_session.is_some();

            // Run specks init in the worktree (idempotent, creates .specks/ infrastructure)
            let init_result = std::env::current_exe()
                .map_err(|e| specks_core::error::SpecksError::InitFailed {
                    reason: format!("failed to get current executable: {}", e),
                })
                .and_then(|exe| {
                    use std::process::Command;
                    Command::new(exe)
                        .arg("init")
                        .current_dir(&worktree_path)
                        .output()
                        .map_err(|e| specks_core::error::SpecksError::InitFailed {
                            reason: format!("failed to execute init: {}", e),
                        })
                })
                .and_then(|output| {
                    if output.status.success() {
                        Ok(())
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        Err(specks_core::error::SpecksError::InitFailed {
                            reason: format!("init failed: {}", stderr),
                        })
                    }
                });

            if let Err(e) = init_result {
                // Init failed - rollback
                let _ = rollback_worktree_creation(&worktree_path, &branch_name, &repo_root);

                if json_output {
                    let data = CreateData {
                        worktree_path: String::new(),
                        branch_name: String::new(),
                        base_branch: config.base_branch.clone(),
                        speck_path: speck.clone(),
                        total_steps: 0,
                        bead_mapping: None,
                        root_bead_id: None,
                        reused: false,
                        session_id: None,
                        session_file: None,
                        artifacts_base: None,
                        all_steps: None,
                        ready_steps: None,
                    };
                    eprintln!(
                        "{}",
                        serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?
                    );
                } else if !quiet {
                    eprintln!("error: {}", e);
                    eprintln!("Rolled back worktree creation");
                }
                return Ok(e.exit_code());
            }

            // Sync beads and commit (always-on)
            // Try to sync beads
            let (bead_mapping, root_bead_id) = match sync_beads_in_worktree(&worktree_path, &speck)
            {
                Ok((mapping, root_id)) => {
                    // Try to commit the changes
                    match commit_bead_annotations(&worktree_path, &speck, speck_name) {
                        Ok(()) => (Some(mapping), root_id),
                        Err(e) => {
                            // Commit failed - rollback
                            let _ = rollback_worktree_creation(
                                &worktree_path,
                                &branch_name,
                                &repo_root,
                            );

                            if json_output {
                                let data = CreateData {
                                    worktree_path: String::new(),
                                    branch_name: String::new(),
                                    base_branch: config.base_branch.clone(),
                                    speck_path: speck.clone(),
                                    total_steps: 0,
                                    bead_mapping: None,
                                    root_bead_id: None,
                                    reused: false,
                                    session_id: None,
                                    session_file: None,
                                    artifacts_base: None,
                                    all_steps: None,
                                    ready_steps: None,
                                };
                                eprintln!(
                                    "{}",
                                    serde_json::to_string_pretty(&data)
                                        .map_err(|e| e.to_string())?
                                );
                            } else if !quiet {
                                eprintln!("error: {}", e);
                                eprintln!("Rolled back worktree creation");
                            }
                            return Ok(e.exit_code());
                        }
                    }
                }
                Err(e) => {
                    // Sync failed - rollback
                    let _ = rollback_worktree_creation(&worktree_path, &branch_name, &repo_root);

                    if json_output {
                        let data = CreateData {
                            worktree_path: String::new(),
                            branch_name: String::new(),
                            base_branch: config.base_branch.clone(),
                            speck_path: speck.clone(),
                            total_steps: 0,
                            bead_mapping: None,
                            root_bead_id: None,
                            reused: false,
                            session_id: None,
                            session_file: None,
                            artifacts_base: None,
                            all_steps: None,
                            ready_steps: None,
                        };
                        eprintln!(
                            "{}",
                            serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?
                        );
                    } else if !quiet {
                        eprintln!("error: {}", e);
                        eprintln!("Rolled back worktree creation");
                    }
                    return Ok(e.exit_code());
                }
            };

            // Parse the synced speck to extract all_steps (already have bead_mapping from sync_beads_in_worktree)
            let synced_speck_path = worktree_path.join(&speck);
            let synced_speck_content = std::fs::read_to_string(&synced_speck_path)
                .map_err(|e| format!("failed to read synced speck: {}", e))?;
            let synced_speck = specks_core::parse_speck(&synced_speck_content)
                .map_err(|e| format!("failed to parse synced speck: {}", e))?;

            let all_steps: Vec<String> = synced_speck
                .steps
                .iter()
                .map(|s| s.anchor.clone())
                .collect();
            let total_steps = synced_speck.steps.len();

            // Query bd ready to get ready_steps (only if root_bead_id is available)
            let ready_steps: Option<Vec<String>> = if let Some(ref root_id) = root_bead_id {
                use specks_core::beads::BeadsCli;
                let bd = BeadsCli::default();
                match bd.ready(Some(root_id)) {
                    Ok(ready_beads) => {
                        // Map bead IDs to step anchors using bead_mapping
                        if let Some(ref mapping) = bead_mapping {
                            let ready_anchors: Vec<String> = ready_beads
                                .iter()
                                .filter_map(|bead| {
                                    // Find step anchor for this bead ID
                                    mapping
                                        .iter()
                                        .find(|(_, bid)| *bid == &bead.id)
                                        .map(|(anchor, _)| anchor.clone())
                                })
                                .collect();
                            Some(ready_anchors)
                        } else {
                            None
                        }
                    }
                    Err(_) => None, // bd not available or failed - continue without ready_steps
                }
            } else {
                None
            };

            // Derive session_id from branch name: "specks/auth-20260208-143022" -> "auth-20260208-143022"
            let session_id = branch_name
                .strip_prefix("specks/")
                .unwrap_or(&branch_name)
                .to_string();

            // Create artifact directories
            let artifacts_base = repo_root
                .join(".specks-worktrees/.artifacts")
                .join(&session_id);
            if let Err(e) = std::fs::create_dir_all(&artifacts_base) {
                eprintln!("warning: failed to create artifacts base directory: {}", e);
            }

            // Create per-step artifact directories
            for (idx, _step_anchor) in all_steps.iter().enumerate() {
                let step_dir = artifacts_base.join(format!("step-{}", idx));
                if let Err(e) = std::fs::create_dir_all(&step_dir) {
                    eprintln!(
                        "warning: failed to create step-{} artifact directory: {}",
                        idx, e
                    );
                }
            }

            // Create Session in CLI layer (schema v2)
            let session = Session {
                schema_version: "2".to_string(),
                session_id: session_id.clone(),
                speck_path: speck.clone(),
                speck_slug: speck_slug.clone(),
                branch_name: branch_name.clone(),
                base_branch: config.base_branch.clone(),
                worktree_path: worktree_path.display().to_string(),
                created_at: specks_core::session::now_iso8601(),
                last_updated_at: None,
                total_steps,
                root_bead_id: root_bead_id.clone(),
                reused,
                step_summaries: None,
                ..Default::default()
            };

            // Save session to external sessions dir
            if let Err(e) = specks_core::session::save_session(&session, &repo_root) {
                eprintln!("warning: failed to save session: {}", e);
            }

            // Compute session_file path for output
            let session_file = repo_root
                .join(".specks-worktrees/.sessions")
                .join(format!("{}.json", session_id));

            if json_output {
                let data = CreateData {
                    worktree_path: worktree_path.display().to_string(),
                    branch_name: branch_name.clone(),
                    base_branch: config.base_branch.clone(),
                    speck_path: speck.clone(),
                    total_steps,
                    bead_mapping,
                    root_bead_id,
                    reused,
                    session_id: Some(session_id),
                    session_file: Some(session_file.display().to_string()),
                    artifacts_base: Some(artifacts_base.display().to_string()),
                    all_steps: Some(all_steps),
                    ready_steps,
                };
                println!(
                    "{}",
                    serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?
                );
            } else if !quiet {
                if reused {
                    println!("Reused existing worktree for speck: {}", speck);
                } else {
                    println!("Created worktree for speck: {}", speck);
                }
                println!("  Branch: {}", branch_name);
                println!("  Worktree: {}", worktree_path.display());
                println!("  Steps: {}", total_steps);
                if bead_mapping.is_some() {
                    println!("  Beads synced and committed");
                }
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
    run_worktree_list_with_root(json_output, quiet, None)
}

/// Inner implementation that accepts an explicit repo root.
pub fn run_worktree_list_with_root(
    json_output: bool,
    quiet: bool,
    override_root: Option<&Path>,
) -> Result<i32, String> {
    let repo_root = match override_root {
        Some(root) => root.to_path_buf(),
        None => std::env::current_dir().map_err(|e| e.to_string())?,
    };

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

                        // V2: No status field - just show "active" label
                        let completed = session
                            .step_summaries
                            .as_ref()
                            .map(|s| s.len())
                            .unwrap_or(0);
                        println!(
                            "  Progress: {}/{} steps completed",
                            completed, session.total_steps
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
    merged: bool,
    orphaned: bool,
    stale: bool,
    all: bool,
    dry_run: bool,
    json_output: bool,
    quiet: bool,
) -> Result<i32, String> {
    run_worktree_cleanup_with_root(
        merged,
        orphaned,
        stale,
        all,
        dry_run,
        json_output,
        quiet,
        None,
    )
}

/// Inner implementation that accepts an explicit repo root.
#[allow(clippy::too_many_arguments)]
pub fn run_worktree_cleanup_with_root(
    merged: bool,
    orphaned: bool,
    stale: bool,
    all: bool,
    dry_run: bool,
    json_output: bool,
    quiet: bool,
    override_root: Option<&Path>,
) -> Result<i32, String> {
    let repo_root = match override_root {
        Some(root) => root.to_path_buf(),
        None => std::env::current_dir().map_err(|e| e.to_string())?,
    };

    // Determine cleanup mode
    let mode = if all {
        CleanupMode::All
    } else if stale {
        CleanupMode::Stale
    } else if orphaned {
        CleanupMode::Orphaned
    } else if merged {
        CleanupMode::Merged
    } else {
        // Default to Merged for backward compatibility
        CleanupMode::Merged
    };

    match cleanup_worktrees(&repo_root, mode, dry_run) {
        Ok(result) => {
            if json_output {
                let data = CleanupData {
                    merged_removed: result.merged_removed.clone(),
                    orphaned_removed: result.orphaned_removed.clone(),
                    stale_branches_removed: result.stale_branches_removed.clone(),
                    skipped: result.skipped.clone(),
                    dry_run,
                };
                println!(
                    "{}",
                    serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?
                );
            } else if !quiet {
                let total_removed = result.merged_removed.len()
                    + result.orphaned_removed.len()
                    + result.stale_branches_removed.len();

                if dry_run {
                    if total_removed == 0 {
                        println!("No worktrees or branches to remove");
                    } else {
                        println!("Would remove {} item(s):", total_removed);
                        if !result.merged_removed.is_empty() {
                            println!("\nMerged PRs:");
                            for branch in &result.merged_removed {
                                println!("  - {}", branch);
                            }
                        }
                        if !result.orphaned_removed.is_empty() {
                            println!("\nOrphaned (no PR):");
                            for branch in &result.orphaned_removed {
                                println!("  - {}", branch);
                            }
                        }
                        if !result.stale_branches_removed.is_empty() {
                            println!("\nStale branches (no worktree):");
                            for branch in &result.stale_branches_removed {
                                println!("  - {}", branch);
                            }
                        }
                    }
                } else if total_removed == 0 {
                    println!("No worktrees or branches removed");
                } else {
                    println!("Removed {} item(s):", total_removed);
                    if !result.merged_removed.is_empty() {
                        println!("\nMerged PRs:");
                        for branch in &result.merged_removed {
                            println!("  - {}", branch);
                        }
                    }
                    if !result.orphaned_removed.is_empty() {
                        println!("\nOrphaned (no PR):");
                        for branch in &result.orphaned_removed {
                            println!("  - {}", branch);
                        }
                    }
                    if !result.stale_branches_removed.is_empty() {
                        println!("\nStale branches (no worktree):");
                        for branch in &result.stale_branches_removed {
                            println!("  - {}", branch);
                        }
                    }
                }

                if !result.skipped.is_empty() {
                    println!("\nSkipped {} item(s):", result.skipped.len());
                    for (branch, reason) in &result.skipped {
                        println!("  - {}: {}", branch, reason);
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

/// Run worktree remove command
pub fn run_worktree_remove(
    target: String,
    force: bool,
    json_output: bool,
    quiet: bool,
) -> Result<i32, String> {
    run_worktree_remove_with_root(target, force, json_output, quiet, None)
}

/// Inner implementation that accepts an explicit repo root.
pub fn run_worktree_remove_with_root(
    target: String,
    force: bool,
    json_output: bool,
    quiet: bool,
    override_root: Option<&Path>,
) -> Result<i32, String> {
    use std::process::Command;

    let repo_root = match override_root {
        Some(root) => root.to_path_buf(),
        None => std::env::current_dir().map_err(|e| e.to_string())?,
    };

    // List all worktrees
    let worktrees = list_worktrees(&repo_root).map_err(|e| e.to_string())?;

    // Try to identify the worktree by:
    // 1. Speck path (can match multiple - error if so)
    // 2. Branch name (exact match)
    // 3. Worktree path (exact match)

    let mut matching_sessions: Vec<&Session> = Vec::new();

    // Check if target is a speck path
    let target_path = PathBuf::from(&target);
    let canonical_target = if target_path.is_absolute() {
        target_path.clone()
    } else {
        repo_root.join(&target_path)
    };

    // Try to match by speck path
    for session in &worktrees {
        let session_speck_path = PathBuf::from(&session.speck_path);
        let canonical_session_speck = if session_speck_path.is_absolute() {
            session_speck_path.clone()
        } else {
            repo_root.join(&session_speck_path)
        };

        // Compare canonical paths
        if canonical_session_speck == canonical_target {
            matching_sessions.push(session);
        }
    }

    // If multiple matches by speck path, error with candidate list (D10)
    if matching_sessions.len() > 1 {
        if json_output {
            eprintln!(r#"{{"error": "Multiple worktrees found for {}"}}"#, target);
        } else if !quiet {
            eprintln!("Error: Multiple worktrees found for {}\n", target);
            for session in &matching_sessions {
                let status_str = format!("{:?}", "active" /* v2 */);
                eprintln!(
                    "  {}  {}  {}",
                    session.branch_name, status_str, session.created_at
                );
            }
            eprintln!("\nUse branch name or worktree path to disambiguate:");
            if let Some(first) = matching_sessions.first() {
                eprintln!("  specks worktree remove {}", first.branch_name);
            }
        }
        return Ok(1);
    }

    // If exactly one match by speck path, use it
    let session = if matching_sessions.len() == 1 {
        matching_sessions[0]
    } else {
        // Try to match by branch name or worktree path
        worktrees
            .iter()
            .find(|s| s.branch_name == target || s.worktree_path == target)
            .ok_or_else(|| {
                if json_output {
                    format!(r#"{{"error": "No worktree found matching: {}"}}"#, target)
                } else {
                    format!("error: No worktree found matching: {}", target)
                }
            })?
    };

    // Check if worktree has uncommitted changes (unless --force)
    if !force {
        let status_output = Command::new("git")
            .args(["-C", &session.worktree_path, "status", "--porcelain"])
            .output()
            .map_err(|e| format!("failed to check git status: {}", e))?;

        if status_output.status.success() {
            let stdout = String::from_utf8_lossy(&status_output.stdout);
            if !stdout.trim().is_empty() {
                if json_output {
                    eprintln!(
                        r#"{{"error": "Worktree has uncommitted changes. Use --force to override."}}"#
                    );
                } else if !quiet {
                    eprintln!("error: Worktree has uncommitted changes");
                    eprintln!("Use --force to override:");
                    eprintln!("  specks worktree remove {} --force", target);
                }
                return Ok(1);
            }
        }
    }

    // Remove the worktree
    let worktree_path = PathBuf::from(&session.worktree_path);

    // If --force is passed, we need to manually force-remove the worktree
    if force {
        // Use git worktree remove --force directly
        let remove_output = Command::new("git")
            .args([
                "-C",
                &repo_root.to_string_lossy(),
                "worktree",
                "remove",
                "--force",
                &session.worktree_path,
            ])
            .output()
            .map_err(|e| format!("failed to remove worktree: {}", e))?;

        if !remove_output.status.success() {
            let stderr = String::from_utf8_lossy(&remove_output.stderr);
            if json_output {
                eprintln!(r#"{{"error": "Failed to remove worktree: {}"}}"#, stderr);
            } else if !quiet {
                eprintln!("error: Failed to remove worktree: {}", stderr);
            }
            return Ok(1);
        }

        // Clean up session files manually
        if let Some(session_id) = session_id_from_worktree(&worktree_path) {
            let _ = delete_session(&session_id, &repo_root);
        }
    } else {
        // Use the remove_worktree function which handles session cleanup
        if let Err(e) = remove_worktree(&worktree_path, &repo_root) {
            if json_output {
                eprintln!(r#"{{"error": "{}"}}"#, e);
            } else if !quiet {
                eprintln!("error: {}", e);
            }
            return Ok(1);
        }
    }

    // Delete the branch
    let delete_output = Command::new("git")
        .args([
            "-C",
            &repo_root.to_string_lossy(),
            "branch",
            "-D",
            &session.branch_name,
        ])
        .output()
        .map_err(|e| format!("failed to delete branch: {}", e))?;

    if !delete_output.status.success() {
        let stderr = String::from_utf8_lossy(&delete_output.stderr);
        // Warn but don't fail - worktree removal succeeded
        if !quiet && !json_output {
            eprintln!("warning: Failed to delete branch: {}", stderr);
        }
    }

    // Prune stale worktree metadata
    let _ = Command::new("git")
        .args(["-C", &repo_root.to_string_lossy(), "worktree", "prune"])
        .output();

    if json_output {
        let data = RemoveData {
            worktree_path: session.worktree_path.clone(),
            branch_name: session.branch_name.clone(),
            speck_path: session.speck_path.clone(),
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?
        );
    } else if !quiet {
        println!("Removed worktree:");
        println!("  Branch: {}", session.branch_name);
        println!("  Worktree: {}", session.worktree_path);
        println!("  Speck: {}", session.speck_path);
    }

    Ok(0)
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
            bead_mapping: None,
            root_bead_id: None,
            reused: false,
            session_id: None,
            session_file: None,
            artifacts_base: None,
            all_steps: None,
            ready_steps: None,
        };

        let json = serde_json::to_string(&data).expect("serialization should succeed");
        assert!(json.contains("worktree_path"));
        assert!(json.contains("branch_name"));
        // reused should be skipped when false
        assert!(!json.contains("reused"));
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
            merged_removed: vec!["specks/merged-123".to_string()],
            orphaned_removed: vec!["specks/orphan-456".to_string()],
            stale_branches_removed: vec![],
            skipped: vec![("specks/skip-789".to_string(), "InProgress".to_string())],
            dry_run: true,
        };

        let json = serde_json::to_string(&data).expect("serialization should succeed");
        assert!(json.contains("merged_removed"));
        assert!(json.contains("orphaned_removed"));
        assert!(json.contains("stale_branches_removed"));
        assert!(json.contains("skipped"));
        assert!(json.contains("dry_run"));
    }

    #[test]
    fn test_remove_data_serialization() {
        let data = RemoveData {
            worktree_path: ".specks-worktrees/specks__test-20260210-120000".to_string(),
            branch_name: "specks/test-20260210-120000".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
        };

        let json = serde_json::to_string(&data).expect("serialization should succeed");
        assert!(json.contains("worktree_path"));
        assert!(json.contains("branch_name"));
        assert!(json.contains("speck_path"));
    }

    #[test]
    fn test_worktree_create_help_documents_always_on_beads() {
        use crate::cli::Cli;
        use clap::CommandFactory;

        let app = Cli::command();
        let worktree_subcommand = app
            .find_subcommand("worktree")
            .expect("worktree subcommand should exist");

        // Find the create subcommand
        let create_subcommand = worktree_subcommand
            .get_subcommands()
            .find(|cmd| cmd.get_name() == "create")
            .expect("create subcommand should exist");

        // Get the long_about text
        let long_about = create_subcommand
            .get_long_about()
            .expect("create should have long_about");

        // Verify beads sync is documented as always-on
        assert!(
            long_about.to_string().contains("always-on"),
            "create help should document always-on beads sync"
        );
        assert!(
            long_about.to_string().contains("atomically")
                || long_about.to_string().contains("rollback"),
            "create help should explain atomic behavior"
        );
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use specks_core::worktree::CleanupMode;
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

        let (worktree_path, branch_name, speck_slug) = result.unwrap();
        assert_eq!(speck_slug, "test");
        assert!(branch_name.starts_with("specks/test-"));

        // Verify worktree directory exists
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
        let (_worktree_path, branch_name, _speck_slug) =
            create_worktree(&config).expect("create_worktree should succeed");

        // Create Session manually for list_worktrees to find
        let session = Session {
            schema_version: "2".to_string(),
            session_id: branch_name
                .strip_prefix("specks/")
                .unwrap_or(&branch_name)
                .to_string(),
            speck_path: speck_path.to_string(),
            speck_slug: "test".to_string(),
            branch_name: branch_name.clone(),
            base_branch: "main".to_string(),
            worktree_path: _worktree_path.display().to_string(),
            created_at: specks_core::session::now_iso8601(),
            last_updated_at: None,
            total_steps: 1,
            root_bead_id: None,
            reused: false,
            step_summaries: None,
            ..Default::default()
        };
        specks_core::session::save_session(&session, &repo_path)
            .expect("save_session should succeed");

        // List worktrees
        let worktrees = list_worktrees(&repo_path).expect("list_worktrees should succeed");

        assert_eq!(worktrees.len(), 1, "should have one worktree");
        assert_eq!(worktrees[0].branch_name, branch_name);
    }

    #[test]
    fn test_worktree_create_reuses_existing() {
        let (_temp, repo_path) = setup_test_repo();
        let speck_path = ".specks/specks-test.md";

        // Create first worktree
        let config1 = WorktreeConfig {
            speck_path: PathBuf::from(speck_path),
            base_branch: "main".to_string(),
            repo_root: repo_path.clone(),
        };

        let (worktree_path1, branch_name1, _slug1) =
            create_worktree(&config1).expect("first create_worktree should succeed");

        // Create Session for first worktree
        let session1 = Session {
            schema_version: "2".to_string(),
            session_id: branch_name1
                .strip_prefix("specks/")
                .unwrap_or(&branch_name1)
                .to_string(),
            speck_path: speck_path.to_string(),
            speck_slug: "test".to_string(),
            branch_name: branch_name1.clone(),
            base_branch: "main".to_string(),
            worktree_path: worktree_path1.display().to_string(),
            created_at: specks_core::session::now_iso8601(),
            last_updated_at: None,
            total_steps: 1,
            root_bead_id: None,
            reused: false,
            step_summaries: None,
            ..Default::default()
        };
        specks_core::session::save_session(&session1, &repo_path)
            .expect("save_session should succeed");

        // Try to create again - should reuse existing (always-on behavior)
        let config2 = WorktreeConfig {
            speck_path: PathBuf::from(speck_path),
            base_branch: "main".to_string(),
            repo_root: repo_path.clone(),
        };

        let (worktree_path2, branch_name2, _slug2) =
            create_worktree(&config2).expect("second create_worktree should succeed");

        // Verify we got back the same worktree
        assert_eq!(worktree_path1, worktree_path2);
        assert_eq!(branch_name1, branch_name2);
    }

    #[test]
    fn test_worktree_create_is_idempotent() {
        let (_temp, repo_path) = setup_test_repo();
        let speck_path = ".specks/specks-test.md";

        // Create first worktree
        let config1 = WorktreeConfig {
            speck_path: PathBuf::from(speck_path),
            base_branch: "main".to_string(),
            repo_root: repo_path.clone(),
        };

        let (worktree_path1, branch_name1, _slug1) =
            create_worktree(&config1).expect("first create_worktree should succeed");

        // Create Session for first worktree
        let session1 = Session {
            schema_version: "2".to_string(),
            session_id: branch_name1
                .strip_prefix("specks/")
                .unwrap_or(&branch_name1)
                .to_string(),
            speck_path: speck_path.to_string(),
            speck_slug: "test".to_string(),
            branch_name: branch_name1.clone(),
            base_branch: "main".to_string(),
            worktree_path: worktree_path1.display().to_string(),
            created_at: specks_core::session::now_iso8601(),
            last_updated_at: None,
            total_steps: 1,
            root_bead_id: None,
            reused: false,
            step_summaries: None,
            ..Default::default()
        };
        specks_core::session::save_session(&session1, &repo_path)
            .expect("save_session should succeed");

        // Try to create again - should return same worktree (idempotent)
        let config2 = WorktreeConfig {
            speck_path: PathBuf::from(speck_path),
            base_branch: "main".to_string(),
            repo_root: repo_path.clone(),
        };

        let (worktree_path2, branch_name2, _slug2) =
            create_worktree(&config2).expect("second create_worktree should succeed");

        // Verify we got back the same worktree
        assert_eq!(worktree_path1, worktree_path2);
        assert_eq!(branch_name1, branch_name2);
    }

    #[test]
    fn test_worktree_create_when_none_exists() {
        let (_temp, repo_path) = setup_test_repo();
        let speck_path = ".specks/specks-test.md";

        // Try to create when no worktree exists
        let config = WorktreeConfig {
            speck_path: PathBuf::from(speck_path),
            base_branch: "main".to_string(),
            repo_root: repo_path.clone(),
        };

        let (worktree_path, branch_name, speck_slug) =
            create_worktree(&config).expect("create_worktree should create new when none exists");

        // Verify a new worktree was created
        assert!(worktree_path.exists(), "worktree should exist");
        assert!(branch_name.starts_with("specks/test-"));
        assert_eq!(speck_slug, "test");
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
        let (worktree_path, branch_name, _slug) =
            create_worktree(&config).expect("create_worktree should succeed");

        // Create Session
        let session = Session {
            schema_version: "2".to_string(),
            session_id: branch_name
                .strip_prefix("specks/")
                .unwrap_or(&branch_name)
                .to_string(),
            speck_path: speck_path.to_string(),
            speck_slug: "test".to_string(),
            branch_name: branch_name.clone(),
            base_branch: "main".to_string(),
            worktree_path: worktree_path.display().to_string(),
            created_at: specks_core::session::now_iso8601(),
            last_updated_at: None,
            total_steps: 1,
            root_bead_id: None,
            reused: false,
            step_summaries: None,
            ..Default::default()
        };
        specks_core::session::save_session(&session, &repo_path)
            .expect("save_session should succeed");

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
        let checkout_output = Command::new("git")
            .args(["checkout", "main"])
            .current_dir(&repo_path)
            .output()
            .expect("failed to checkout main");
        assert!(
            checkout_output.status.success(),
            "checkout should succeed: {}",
            String::from_utf8_lossy(&checkout_output.stderr)
        );

        let merge_output = Command::new("git")
            .args(["merge", &session.branch_name])
            .current_dir(&repo_path)
            .output()
            .expect("failed to merge");
        assert!(
            merge_output.status.success(),
            "merge should succeed: {}",
            String::from_utf8_lossy(&merge_output.stderr)
        );

        // Dry run should detect merged worktree but not remove it
        // Use force=true since gh CLI is not available in test environment
        let result = cleanup_worktrees(&repo_path, CleanupMode::Merged, true)
            .expect("cleanup_worktrees dry run should succeed");

        assert_eq!(
            result.merged_removed.len(),
            1,
            "should detect one merged worktree"
        );
        assert_eq!(result.merged_removed[0], session.branch_name);
        assert!(
            worktree_path.exists(),
            "worktree directory should still exist after dry run"
        );

        // Verify worktree is still listed
        let worktrees = list_worktrees(&repo_path).expect("list_worktrees should succeed");
        assert_eq!(worktrees.len(), 1, "worktree should still be listed");
    }

    /// Helper to create worktree and session for tests
    fn create_worktree_with_session(config: &WorktreeConfig) -> Session {
        let (worktree_path, branch_name, speck_slug) =
            create_worktree(config).expect("create_worktree should succeed");
        let session = Session {
            schema_version: "2".to_string(),
            session_id: branch_name
                .strip_prefix("specks/")
                .unwrap_or(&branch_name)
                .to_string(),
            speck_path: config.speck_path.display().to_string(),
            speck_slug,
            branch_name,
            base_branch: config.base_branch.clone(),
            worktree_path: worktree_path.display().to_string(),
            created_at: specks_core::session::now_iso8601(),
            last_updated_at: None,
            total_steps: 1,
            root_bead_id: None,
            reused: false,
            step_summaries: None,
            ..Default::default()
        };
        specks_core::session::save_session(&session, &config.repo_root)
            .expect("save_session should succeed");
        session
    }

    #[test]
    fn test_remove_by_branch_name() {
        let (_temp, repo_path) = setup_test_repo();
        let speck_path = ".specks/specks-test.md";

        let config = WorktreeConfig {
            speck_path: PathBuf::from(speck_path),
            base_branch: "main".to_string(),
            repo_root: repo_path.clone(),
        };

        // Create a worktree
        let session = create_worktree_with_session(&config);
        let worktree_path = PathBuf::from(&session.worktree_path);

        // Verify worktree exists
        assert!(worktree_path.exists(), "worktree should exist");

        // Remove by branch name — pass explicit repo_root, no set_current_dir needed
        let result = run_worktree_remove_with_root(
            session.branch_name.clone(),
            false,
            false,
            true,
            Some(&repo_path),
        );

        assert!(result.is_ok(), "remove should succeed: {:?}", result.err());
        assert_eq!(result.unwrap(), 0, "exit code should be 0");

        // Verify worktree is removed
        assert!(
            !worktree_path.exists(),
            "worktree directory should be removed"
        );

        // Verify branch is removed
        let branch_exists = Command::new("git")
            .args([
                "-C",
                &repo_path.to_string_lossy(),
                "rev-parse",
                "--verify",
                &session.branch_name,
            ])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        assert!(!branch_exists, "branch should be deleted");
    }

    #[test]
    fn test_remove_by_speck_path() {
        let (_temp, repo_path) = setup_test_repo();
        let speck_path = ".specks/specks-test.md";

        let config = WorktreeConfig {
            speck_path: PathBuf::from(speck_path),
            base_branch: "main".to_string(),
            repo_root: repo_path.clone(),
        };

        // Create a worktree
        let session = create_worktree_with_session(&config);
        let worktree_path = PathBuf::from(&session.worktree_path);

        // Verify worktree exists
        assert!(worktree_path.exists(), "worktree should exist");

        // Remove by speck path — pass explicit repo_root
        let result = run_worktree_remove_with_root(
            speck_path.to_string(),
            false,
            false,
            true,
            Some(&repo_path),
        );

        assert!(result.is_ok(), "remove should succeed: {:?}", result.err());
        assert_eq!(result.unwrap(), 0, "exit code should be 0");

        // Verify worktree is removed
        assert!(
            !worktree_path.exists(),
            "worktree directory should be removed"
        );
    }

    #[test]
    fn test_remove_by_worktree_path() {
        let (_temp, repo_path) = setup_test_repo();
        let speck_path = ".specks/specks-test.md";

        let config = WorktreeConfig {
            speck_path: PathBuf::from(speck_path),
            base_branch: "main".to_string(),
            repo_root: repo_path.clone(),
        };

        // Create a worktree
        let session = create_worktree_with_session(&config);
        let worktree_path = PathBuf::from(&session.worktree_path);

        // Verify worktree exists
        assert!(worktree_path.exists(), "worktree should exist");

        // Remove by worktree path — pass explicit repo_root
        let result = run_worktree_remove_with_root(
            session.worktree_path.clone(),
            false,
            false,
            true,
            Some(&repo_path),
        );

        assert!(result.is_ok(), "remove should succeed: {:?}", result.err());
        assert_eq!(result.unwrap(), 0, "exit code should be 0");

        // Verify worktree is removed
        assert!(
            !worktree_path.exists(),
            "worktree directory should be removed"
        );
    }

    #[test]
    fn test_remove_refuses_dirty_without_force() {
        let (_temp, repo_path) = setup_test_repo();
        let speck_path = ".specks/specks-test.md";

        let config = WorktreeConfig {
            speck_path: PathBuf::from(speck_path),
            base_branch: "main".to_string(),
            repo_root: repo_path.clone(),
        };

        // Create a worktree
        let session = create_worktree_with_session(&config);
        let worktree_path = PathBuf::from(&session.worktree_path);

        // Make worktree dirty
        fs::write(worktree_path.join("dirty.txt"), "uncommitted")
            .expect("failed to create dirty file");

        // Try to remove without --force (should fail) — pass explicit repo_root
        let result = run_worktree_remove_with_root(
            session.branch_name.clone(),
            false,
            false,
            true,
            Some(&repo_path),
        );
        assert!(result.is_ok(), "command should return result");
        assert_eq!(result.unwrap(), 1, "exit code should be 1 (error)");

        // Verify worktree still exists
        assert!(worktree_path.exists(), "worktree should still exist");

        // Now try with --force (should succeed)
        let result_force = run_worktree_remove_with_root(
            session.branch_name.clone(),
            true,
            false,
            true,
            Some(&repo_path),
        );
        assert!(result_force.is_ok(), "remove with force should succeed");
        assert_eq!(result_force.unwrap(), 0, "exit code should be 0");

        // Verify worktree is removed
        assert!(
            !worktree_path.exists(),
            "worktree should be removed with --force"
        );
    }

    #[test]
    fn test_remove_ambiguous_fails_fast() {
        let (_temp, repo_path) = setup_test_repo();
        let speck_path = ".specks/specks-test.md";

        // For this test, we need to create two worktrees with different branches manually
        // because create_worktree enforces unique branch names
        let config1 = WorktreeConfig {
            speck_path: PathBuf::from(speck_path),
            base_branch: "main".to_string(),
            repo_root: repo_path.clone(),
        };
        let session1 = create_worktree_with_session(&config1);

        // Wait a moment to ensure different timestamps
        std::thread::sleep(std::time::Duration::from_millis(1100));

        // Create a second worktree for the same speck by creating a second git branch manually
        let branch_name_2 = "specks/test-second";
        Command::new("git")
            .args([
                "-C",
                &repo_path.to_string_lossy(),
                "branch",
                branch_name_2,
                "main",
            ])
            .output()
            .expect("failed to create second branch");

        let worktree_path_2 = repo_path.join(".specks-worktrees/specks__test-second");
        Command::new("git")
            .args([
                "-C",
                &repo_path.to_string_lossy(),
                "worktree",
                "add",
                &worktree_path_2.to_string_lossy(),
                branch_name_2,
            ])
            .output()
            .expect("failed to add second worktree");

        // Create a session file for the second worktree with the same speck_path
        let session2 = Session {
            schema_version: "2".to_string(),
            session_id: "test2-timestamp".to_string(),
            speck_path: speck_path.to_string(),
            speck_slug: "test".to_string(),
            branch_name: branch_name_2.to_string(),
            base_branch: "main".to_string(),
            worktree_path: worktree_path_2.to_string_lossy().to_string(),
            created_at: specks_core::session::now_iso8601(),
            last_updated_at: None,
            total_steps: 1,
            root_bead_id: None,
            reused: false,
            step_summaries: None,
            ..Default::default()
        };
        specks_core::session::save_session(&session2, &repo_path).expect("failed to save session2");

        // Try to remove by speck path (should fail with ambiguity error) — pass explicit repo_root
        let result = run_worktree_remove_with_root(
            speck_path.to_string(),
            false,
            false,
            true,
            Some(&repo_path),
        );
        assert!(result.is_ok(), "command should return result");
        assert_eq!(
            result.unwrap(),
            1,
            "exit code should be 1 (ambiguous match)"
        );

        // Verify both worktrees still exist
        let worktree_path1 = PathBuf::from(&session1.worktree_path);
        assert!(worktree_path1.exists(), "first worktree should still exist");
        assert!(
            worktree_path_2.exists(),
            "second worktree should still exist"
        );

        // Clean up - remove by specific branch names
        let _ = run_worktree_remove_with_root(
            session1.branch_name.clone(),
            false,
            false,
            true,
            Some(&repo_path),
        );
        let _ = run_worktree_remove_with_root(
            branch_name_2.to_string(),
            false,
            false,
            true,
            Some(&repo_path),
        );
    }
}
