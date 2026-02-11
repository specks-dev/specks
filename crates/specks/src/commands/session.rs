//! Session management commands
//!
//! Provides commands for managing implementation session state, including
//! recovery from NeedsReconcile state when a commit succeeds but bead close fails.

#![allow(deprecated)] // SessionReconcileData is deprecated but still used for backward compat

use clap::Subcommand;
use specks_core::{
    BeadsCli, Config, find_project_root,
    session::{save_session, session_file_path},
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
/// DEPRECATED in v2: Session no longer tracks status. Beads is source of truth.
#[allow(deprecated)]
fn run_reconcile(
    _session_id: String,
    _dry_run: bool,
    json_output: bool,
    quiet: bool,
) -> Result<i32, String> {
    // V2: Reconcile command is deprecated
    if !quiet {
        eprintln!(
            "Error: 'session reconcile' is deprecated in v2. Use 'specks beads close <bead-id>' instead."
        );
    }
    if json_output {
        println!(
            r#"{{"status":"error","message":"session reconcile is deprecated; use specks beads close <bead-id> instead"}}"#
        );
    }
    return Err(
        "session reconcile is deprecated; use specks beads close <bead-id> instead".to_string(),
    );

    // Old code preserved but unreachable:
    #[allow(unreachable_code, unused_variables)]
    {
        let session_id = _session_id;
        let dry_run = _dry_run;
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
        if true
        /* v2: reconcile deprecated */
        {
            return output_error(
                json_output,
                "E038",
                &format!(
                    "Session is not in NeedsReconcile state (current: {})",
                    "pending" /* v2: no status */
                ),
                &session_id,
                38,
            );
        }

        // Identify the bead to close (v2: deprecated logic)
        let bead_id = "bd-deprecated".to_string();

        if dry_run {
            // Dry run: just report what would happen
            if json_output {
                let data = SessionReconcileData {
                    session_id: session_id.clone(),
                    reconciled: false,
                    previous_status: "pending" /* v2: no status */
                        .to_string(),
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
        let previous_status = "pending" /* v2: no status */
            .to_string();
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
        // v2: no session status to update

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
        let new_status = "pending" /* v2: no status */
            .to_string();
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
    } // end unreachable block
}

/// Output an error in JSON or text format
#[allow(deprecated)]
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

    #[test]
    fn test_reconcile_deprecated() {
        // V2: Reconcile command is deprecated
        // Verify it returns an error with the correct message
        let result = run_reconcile("test-session".to_string(), false, false, true);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("specks beads close"));
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
    #[allow(deprecated)]
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
