//! Implementation of the `specks plan` command
//!
//! NOTE: This command is deprecated and will be removed in Phase 3.
//! Planning is now handled by the `/specks:plan` skill in Claude Code.
//!
//! This stub exists only to maintain build compatibility during the
//! Step 8.1-8.4 transition. It will be deleted in Step 8.4.

/// Run the plan command (deprecated stub)
///
/// Returns exit code 1 with an error message indicating the command is no longer available.
#[allow(clippy::too_many_arguments)]
pub fn run_plan(
    _input: Option<String>,
    _name: Option<String>,
    _context_files: Vec<String>,
    _timeout: u64,
    json_output: bool,
    _quiet: bool,
    _verbose_agents: bool,
) -> Result<i32, String> {
    let message = "The 'specks plan' command has been removed. Use '/specks:plan' in Claude Code instead.";

    if json_output {
        // Output structured JSON error
        let response = serde_json::json!({
            "schema_version": "1",
            "command": "plan",
            "status": "error",
            "data": {},
            "issues": [{
                "code": "E028",
                "severity": "error",
                "message": message
            }]
        });
        println!("{}", serde_json::to_string_pretty(&response).unwrap());
    } else {
        eprintln!("error: {}", message);
    }

    Ok(1)
}
