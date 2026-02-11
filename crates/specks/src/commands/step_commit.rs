//! step-commit command implementation
//!
//! Atomically performs log rotation, prepend, git commit, bead close, and session update.

use crate::commands::log::{log_prepend_inner, log_rotate_inner};
use crate::output::{JsonResponse, StepCommitData};
use specks_core::session::{save_session_atomic, CurrentStep, Session, SessionStatus, StepSummary, now_iso8601};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Run the step-commit command
#[allow(clippy::too_many_arguments)]
pub fn run_step_commit(
    worktree: String,
    step: String,
    speck: String,
    message: String,
    files: Vec<String>,
    bead: String,
    summary: String,
    session: String,
    close_reason: Option<String>,
    json: bool,
    quiet: bool,
) -> Result<i32, String> {
    // Convert paths to Path references
    let worktree_path = Path::new(&worktree);
    let session_path = Path::new(&session);

    // Validate inputs
    if !worktree_path.exists() {
        return error_response(
            "Worktree directory does not exist",
            json,
            quiet,
        );
    }

    if !session_path.exists() {
        return error_response(
            "Session file does not exist",
            json,
            quiet,
        );
    }

    // Validate that all files exist in worktree
    for file in &files {
        let file_path = worktree_path.join(file);
        if !file_path.exists() {
            return error_response(
                &format!("File not found in worktree: {}", file),
                json,
                quiet,
            );
        }
    }

    // Load session
    let mut session_data =
        load_session_file(session_path).map_err(|e| format!("Failed to load session: {}", e))?;

    // Extract repo root from worktree path (parent of .specks-worktrees)
    let repo_root = worktree_path
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| "Cannot derive repo root from worktree path".to_string())?;

    // Step 1: Rotate log if needed
    let rotate_result = log_rotate_inner(worktree_path, false)
        .map_err(|e| format!("Log rotation failed: {}", e))?;

    // Step 2: Prepend log entry
    let _prepend_result = log_prepend_inner(worktree_path, &step, &speck, &summary, Some(&bead))
        .map_err(|e| format!("Log prepend failed: {}", e))?;

    // Step 3: Stage files
    let mut files_to_stage = files.clone();

    // Add implementation log
    files_to_stage.push(".specks/specks-implementation-log.md".to_string());

    // If rotation occurred, add archive directory
    if rotate_result.rotated {
        files_to_stage.push(".specks/archive".to_string());
    }

    // Stage all files
    for file in &files_to_stage {
        let output = Command::new("git")
            .arg("-C")
            .arg(worktree_path)
            .arg("add")
            .arg(file)
            .output()
            .map_err(|e| format!("Failed to run git add: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("git add failed for {}: {}", file, stderr));
        }
    }

    // Step 4: Commit
    let output = Command::new("git")
        .arg("-C")
        .arg(worktree_path)
        .arg("commit")
        .arg("-m")
        .arg(&message)
        .output()
        .map_err(|e| format!("Failed to run git commit: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return error_response(&format!("git commit failed: {}", stderr), json, quiet);
    }

    // Step 5: Get commit hash
    let output = Command::new("git")
        .arg("-C")
        .arg(worktree_path)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .map_err(|e| format!("Failed to run git rev-parse: {}", e))?;

    if !output.status.success() {
        return error_response("Failed to get commit hash", json, quiet);
    }

    let commit_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Step 6: Close bead
    let (bead_closed, warnings) = close_bead_in_worktree(worktree_path, &bead, close_reason.as_deref())?;

    // If bead close failed after commit, set needs_reconcile
    let needs_reconcile = !bead_closed;

    // Step 7: Update session
    if bead_closed {
        // Only update session if bead close succeeded
        match update_session(&mut session_data, &step, &commit_hash, &summary, repo_root) {
            Ok(_) => {}
            Err(e) => {
                // Session update failure is a warning, not an error (commit + bead close succeeded)
                if !quiet {
                    eprintln!("Warning: Session update failed: {}", e);
                }
            }
        }
    } else {
        // Bead close failed - set NeedsReconcile status
        session_data.status = SessionStatus::NeedsReconcile;
        match save_session_atomic(&session_data, repo_root) {
            Ok(_) => {}
            Err(e) => {
                if !quiet {
                    eprintln!("Warning: Failed to update session to NeedsReconcile: {}", e);
                }
            }
        }
    }

    // Build response
    let data = StepCommitData {
        committed: true,
        commit_hash: Some(commit_hash),
        bead_closed,
        bead_id: if bead_closed { Some(bead.clone()) } else { None },
        log_updated: true,
        log_rotated: rotate_result.rotated,
        archived_path: rotate_result.archived_path.clone(),
        files_staged: files_to_stage,
        needs_reconcile,
        warnings,
    };

    if json {
        let response = JsonResponse::ok("step-commit", data);
        println!("{}", serde_json::to_string_pretty(&response).unwrap());
    } else if !quiet {
        println!("Step committed successfully");
        println!("  Commit: {}", data.commit_hash.as_ref().unwrap());
        println!("  Bead: {} ({})", bead, if bead_closed { "closed" } else { "FAILED - needs reconcile" });
        if rotate_result.rotated {
            println!("  Log rotated: {}", rotate_result.archived_path.unwrap_or_default());
        }
    }

    // Exit 0 even if needs_reconcile (commit succeeded)
    Ok(0)
}

/// Helper to load session from file
fn load_session_file(path: &Path) -> Result<Session, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read session file: {}", e))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse session JSON: {}", e))
}

/// Helper to close bead in worktree context
fn close_bead_in_worktree(
    worktree_path: &Path,
    bead_id: &str,
    reason: Option<&str>,
) -> Result<(bool, Vec<String>), String> {
    use specks_core::{BeadsCli, Config};

    // Load config from worktree
    let config = Config::load_from_project(worktree_path).unwrap_or_default();
    let bd_path = std::env::var("SPECKS_BD_PATH")
        .unwrap_or_else(|_| config.specks.beads.bd_path.clone());

    let beads = BeadsCli::new(bd_path);

    // Check if beads CLI is installed
    if !beads.is_installed() {
        return Ok((false, vec!["beads CLI not installed or not found".to_string()]));
    }

    // Close bead - must run from worktree directory so bd finds .beads/
    // Use Command::current_dir instead of calling BeadsCli::close directly
    let mut cmd = Command::new(&beads.bd_path);
    cmd.arg("close").arg(bead_id).current_dir(worktree_path);

    if let Some(r) = reason {
        cmd.arg("--reason").arg(r);
    }

    let output = cmd.output().map_err(|e| format!("Failed to run bd close: {}", e))?;

    if output.status.success() {
        Ok((true, vec![]))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Ok((false, vec![format!("Bead close failed: {}", stderr)]))
    }
}

/// Helper to update session after successful commit
fn update_session(
    session: &mut Session,
    step: &str,
    commit_hash: &str,
    summary: &str,
    repo_root: &Path,
) -> Result<(), String> {
    // Move step from remaining to completed
    if let Some(ref mut remaining) = session.steps_remaining {
        remaining.retain(|s| s != step);
    }
    if let Some(ref mut completed) = session.steps_completed {
        completed.push(step.to_string());
    }

    // Append to step_summaries
    if session.step_summaries.is_none() {
        session.step_summaries = Some(vec![]);
    }
    if let Some(ref mut summaries) = session.step_summaries {
        summaries.push(StepSummary {
            step: step.to_string(),
            commit_hash: commit_hash.to_string(),
            summary: summary.to_string(),
        });
    }

    // Advance current_step
    if let Some(ref remaining) = session.steps_remaining {
        if remaining.is_empty() {
            session.current_step = CurrentStep::Done;
        } else {
            session.current_step = CurrentStep::Anchor(remaining[0].clone());
        }
    } else {
        session.current_step = CurrentStep::Done;
    }

    // Update timestamp
    session.last_updated_at = Some(now_iso8601());

    // Save atomically
    save_session_atomic(session, repo_root)
        .map_err(|e| format!("Failed to save session: {}", e))
}

/// Helper to construct error response
fn error_response(
    message: &str,
    json: bool,
    quiet: bool,
) -> Result<i32, String> {
    let data = StepCommitData {
        committed: false,
        commit_hash: None,
        bead_closed: false,
        bead_id: None,
        log_updated: false,
        log_rotated: false,
        archived_path: None,
        files_staged: vec![],
        needs_reconcile: false,
        warnings: vec![],
    };

    if json {
        let response = JsonResponse::error("step-commit", data, vec![]);
        println!("{}", serde_json::to_string_pretty(&response).unwrap());
    } else if !quiet {
        eprintln!("Error: {}", message);
    }

    Err(message.to_string())
}
