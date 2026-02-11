//! step-commit command implementation
//!
//! Atomically performs log rotation, prepend, git commit, bead close, and session update.

use crate::output::{JsonResponse, StepCommitData};

/// Run the step-commit command
#[allow(clippy::too_many_arguments)]
pub fn run_step_commit(
    _worktree: String,
    _step: String,
    _speck: String,
    _message: String,
    _files: Vec<String>,
    _bead: String,
    _summary: String,
    _session: String,
    _close_reason: Option<String>,
    json: bool,
    _quiet: bool,
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
    }

    Err("step-commit not yet implemented".to_string())
}
