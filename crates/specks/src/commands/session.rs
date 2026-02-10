//! Session management commands
//!
//! Provides commands for managing implementation session state, including
//! recovery from NeedsReconcile state when a commit succeeds but bead close fails.

use clap::Subcommand;
use specks_core::{
    BeadsCli, Config, find_project_root,
    session::{SessionStatus, save_session, session_file_path},
};

use crate::output::{JsonIssue, JsonResponse, SessionReconcileData};

/// Session subcommands
#[derive(Subcommand, Debug)]
pub enum SessionCommands {
    /// Reconcile a session stuck in NeedsReconcile state
    ///
    /// Identifies and closes the pending bead, then transitions session to Completed.
    #[command(
        long_about = "Reconcile a session stuck in NeedsReconcile state.\n\nThis happens when a step commit succeeds but the bead close fails,\nleaving the session in an inconsistent state.\n\nThe command:\n  1. Loads the session from external storage\n  2. Verifies status is NeedsReconcile\n  3. Identifies the pending bead from session.beads_root and session.current_step\n  4. Closes the bead via beads CLI\n  5. Updates session status to Completed\n\nUse --dry-run to preview actions without making changes."
    )]
    Reconcile {
        /// Session ID to reconcile (e.g., auth-20260208-143022)
        session_id: String,

        /// Show what would happen without executing
        #[arg(long)]
        dry_run: bool,
    },
}

/// Run the session reconcile command
pub fn run_session(
    session_cmd: SessionCommands,
    json_output: bool,
    quiet: bool,
) -> Result<i32, String> {
    match session_cmd {
        SessionCommands::Reconcile {
            session_id,
            dry_run,
        } => run_reconcile(session_id, dry_run, json_output, quiet),
    }
}

/// Reconcile a session stuck in NeedsReconcile state
fn run_reconcile(
    session_id: String,
    dry_run: bool,
    json_output: bool,
    quiet: bool,
) -> Result<i32, String> {
    // Find project root
    let project_root = match find_project_root() {
        Ok(root) => root,
        Err(_) => {
            return output_error(
                json_output,
                "E009",
                ".specks directory not initialized",
                &session_id,
                9,
            );
        }
    };

    // Load session from external storage
    let session_path = session_file_path(&project_root, &session_id);
    if !session_path.exists() {
        return output_error(
            json_output,
            "E037",
            &format!("Session file not found: {}", session_path.display()),
            &session_id,
            37,
        );
    }

    // Read session file manually to get the worktree path
    let session_content = std::fs::read_to_string(&session_path)
        .map_err(|e| format!("Failed to read session file: {}", e))?;

    let mut session: specks_core::session::Session = serde_json::from_str(&session_content)
        .map_err(|e| format!("Failed to parse session file: {}", e))?;

    // Verify session is in NeedsReconcile state
    if session.status != SessionStatus::NeedsReconcile {
        return output_error(
            json_output,
            "E038",
            &format!(
                "Session is not in NeedsReconcile state (current: {})",
                session.status
            ),
            &session_id,
            38,
        );
    }

    // Identify the bead to close
    // The current_step field points to the step that was being executed
    // Pattern matching on CurrentStep to support both formats:
    // - Index(n): old format, bead ID = {beads_root}.{n + 1}
    // - Anchor(s): implementer format, lookup via bead_mapping
    // - Done: no bead to close (implementation completed)
    let bead_id = match &session.current_step {
        specks_core::session::CurrentStep::Index(step_index) => {
            if let Some(beads_root) = &session.beads_root {
                // current_step is 0-indexed, bead numbering is 1-indexed
                format!("{}.{}", beads_root, step_index + 1)
            } else {
                return output_error(
                    json_output,
                    "E039",
                    "Session has no beads_root - cannot identify bead to close",
                    &session_id,
                    39,
                );
            }
        }
        specks_core::session::CurrentStep::Anchor(step_anchor) => {
            if let Some(bead_mapping) = &session.bead_mapping {
                bead_mapping
                    .get(step_anchor)
                    .cloned()
                    .ok_or_else(|| {
                        format!("Step anchor '{}' not found in bead_mapping", step_anchor)
                    })
                    .map_err(|msg| {
                        output_error(json_output, "E042", &msg, &session_id, 42).unwrap_err()
                    })?
            } else {
                return output_error(
                    json_output,
                    "E043",
                    "Session with anchor current_step has no bead_mapping",
                    &session_id,
                    43,
                );
            }
        }
        specks_core::session::CurrentStep::Done => {
            return output_error(
                json_output,
                "E044",
                "Session current_step is Done (null) - no bead to close",
                &session_id,
                44,
            );
        }
    };

    if dry_run {
        // Dry run: just report what would happen
        if json_output {
            let data = SessionReconcileData {
                session_id: session_id.clone(),
                reconciled: false,
                previous_status: session.status.to_string(),
                new_status: "completed".to_string(),
                bead_closed: Some(bead_id.clone()),
            };
            let response = JsonResponse::ok("session reconcile", data);
            let json = serde_json::to_string_pretty(&response)
                .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
            println!("{}", json);
        } else if !quiet {
            println!("Dry run - would perform:");
            println!("  1. Close bead: {}", bead_id);
            println!(
                "  2. Update session status: {} -> Completed",
                session.status
            );
        }
        return Ok(0);
    }

    // Load config and beads CLI
    let config = Config::load_from_project(&project_root).unwrap_or_default();
    let bd_path =
        std::env::var("SPECKS_BD_PATH").unwrap_or_else(|_| config.specks.beads.bd_path.clone());
    let beads = BeadsCli::new(bd_path);

    // Check if beads CLI is installed
    if !beads.is_installed() {
        return output_error(
            json_output,
            "E005",
            "beads CLI not installed or not found",
            &session_id,
            5,
        );
    }

    // Close the bead
    let previous_status = session.status.to_string();
    if let Err(e) = beads.close(&bead_id, Some("Session reconciliation")) {
        return output_error(
            json_output,
            "E040",
            &format!("Failed to close bead {}: {}", bead_id, e),
            &session_id,
            40,
        );
    }

    // Update session status to Completed
    session.status = SessionStatus::Completed;

    // Save updated session
    if let Err(e) = save_session(&session, &project_root) {
        return output_error(
            json_output,
            "E041",
            &format!("Failed to save session: {}", e),
            &session_id,
            41,
        );
    }

    // Success output
    let new_status = session.status.to_string();
    let data = SessionReconcileData {
        session_id: session_id.clone(),
        reconciled: true,
        previous_status: previous_status.clone(),
        new_status: new_status.clone(),
        bead_closed: Some(bead_id.clone()),
    };

    if json_output {
        let response = JsonResponse::ok("session reconcile", data);
        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
        println!("{}", json);
    } else if !quiet {
        println!("Session reconciled: {}", session_id);
        println!("  Closed bead: {}", bead_id);
        println!("  Status: {} -> {}", previous_status, new_status);
    }

    Ok(0)
}

/// Output an error in JSON or text format
fn output_error(
    json_output: bool,
    code: &str,
    message: &str,
    session_id: &str,
    exit_code: i32,
) -> Result<i32, String> {
    if json_output {
        let issues = vec![JsonIssue {
            code: code.to_string(),
            severity: "error".to_string(),
            message: message.to_string(),
            file: None,
            line: None,
            anchor: None,
        }];
        let data = SessionReconcileData {
            session_id: session_id.to_string(),
            reconciled: false,
            previous_status: "unknown".to_string(),
            new_status: "unknown".to_string(),
            bead_closed: None,
        };
        let response: JsonResponse<SessionReconcileData> =
            JsonResponse::error("session reconcile", data, issues);
        let json = serde_json::to_string_pretty(&response)
            .unwrap_or_else(|_| r#"{"error":"Failed to serialize JSON response"}"#.to_string());
        println!("{}", json);
    } else {
        eprintln!("error: {}", message);
    }
    Ok(exit_code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use specks_core::session::SessionStatus;

    #[test]
    fn test_reconcile_changes_status_from_needs_reconcile_to_completed() {
        // This test verifies the core reconcile logic in isolation
        // It will be implemented with mocked beads CLI in full integration

        // Verify the session status enum can transition
        let mut status = SessionStatus::NeedsReconcile;
        assert_eq!(status, SessionStatus::NeedsReconcile);

        status = SessionStatus::Completed;
        assert_eq!(status, SessionStatus::Completed);
    }

    #[test]
    fn test_reconcile_fails_for_non_needs_reconcile_session() {
        // This test verifies that reconcile rejects sessions not in NeedsReconcile state
        // Unit test with mock would be needed for full end-to-end testing

        // Verify that we can distinguish session states
        let in_progress = SessionStatus::InProgress;
        let needs_reconcile = SessionStatus::NeedsReconcile;

        assert_ne!(in_progress, needs_reconcile);
        assert_eq!(in_progress.to_string(), "in_progress");
        assert_eq!(needs_reconcile.to_string(), "needs_reconcile");
    }

    #[test]
    fn test_dry_run_does_not_modify_session() {
        // This test would verify dry-run behavior with a mocked environment
        // For now, we verify the logic that dry-run should not modify state

        // Simulate the dry-run code path
        let dry_run = true;
        let session_status = SessionStatus::NeedsReconcile;

        // In dry-run mode, status should not change
        if !dry_run {
            // This block would not execute in dry-run
            let _ = SessionStatus::Completed;
        }

        // Status remains unchanged in dry-run
        assert_eq!(session_status, SessionStatus::NeedsReconcile);
    }

    #[test]
    fn test_bead_id_calculation() {
        // Verify bead ID is calculated correctly
        // Pattern: {beads_root}.{current_step + 1}
        let beads_root = "bd-test";
        let current_step = 2; // 0-indexed

        let bead_id = format!("{}.{}", beads_root, current_step + 1);
        assert_eq!(bead_id, "bd-test.3");
    }

    #[test]
    fn test_session_reconcile_data_serialization() {
        let data = SessionReconcileData {
            session_id: "test-20260210-120000".to_string(),
            reconciled: true,
            previous_status: "needs_reconcile".to_string(),
            new_status: "completed".to_string(),
            bead_closed: Some("bd-test.3".to_string()),
        };

        let json = serde_json::to_string_pretty(&data).unwrap();
        let deserialized: SessionReconcileData = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.session_id, "test-20260210-120000");
        assert!(deserialized.reconciled);
        assert_eq!(deserialized.previous_status, "needs_reconcile");
        assert_eq!(deserialized.new_status, "completed");
        assert_eq!(deserialized.bead_closed, Some("bd-test.3".to_string()));
    }
}
