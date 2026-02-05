//! Implementation of the `specks plan` command
//!
//! Per Spec S01, this command creates or revises specks through an iterative
//! planning loop with interviewer, planner, and critic agents.

use std::path::PathBuf;

use specks_core::{find_project_root, SpecksError};

use crate::output::{JsonIssue, JsonResponse, PlanData, PlanValidation};
use crate::planning_loop::{detect_input_type, resolve_speck_path, LoopContext, PlanMode, PlanningLoop};

/// Run the plan command
///
/// # Arguments
///
/// * `input` - Either an idea string or path to existing speck for revision
/// * `name` - Optional name for the speck file
/// * `context_files` - Additional context files to include
/// * `timeout` - Timeout per agent invocation in seconds
/// * `json_output` - Whether to output in JSON format
/// * `quiet` - Whether to suppress progress messages
///
/// # Returns
///
/// Exit code: 0 = success, 1 = error, 3 = validation error, 5 = aborted, 6 = claude not installed
pub fn run_plan(
    input: Option<String>,
    name: Option<String>,
    context_files: Vec<String>,
    timeout: u64,
    json_output: bool,
    quiet: bool,
) -> Result<i32, String> {
    // Find project root
    let project_root = match find_project_root() {
        Ok(root) => root,
        Err(_) => {
            let message = ".specks directory not initialized".to_string();
            if json_output {
                output_error_json("plan", "E009", &message);
            } else {
                eprintln!("error: {}", message);
            }
            return Ok(9);
        }
    };

    // Get input from user if not provided
    let input = match input {
        Some(i) if !i.is_empty() => i,
        _ => {
            if json_output {
                output_error_json("plan", "E002", "No input provided. Use `specks plan \"your idea\"` or `specks plan path/to/speck.md`");
                return Ok(1);
            } else {
                // In non-JSON mode, we could prompt interactively, but for now require input
                eprintln!("error: No input provided. Use `specks plan \"your idea\"` or `specks plan path/to/speck.md`");
                return Ok(1);
            }
        }
    };

    // Load context files
    let context_contents: Vec<String> = context_files
        .iter()
        .filter_map(|path| {
            match std::fs::read_to_string(path) {
                Ok(content) => Some(format!("--- {} ---\n{}", path, content)),
                Err(e) => {
                    if !quiet {
                        eprintln!("warning: Failed to read context file {}: {}", path, e);
                    }
                    None
                }
            }
        })
        .collect();

    // Detect input type and create context
    let mode = detect_input_type(&input, &project_root);
    let context = match mode {
        PlanMode::New => LoopContext::new_idea(input.clone(), context_contents),
        PlanMode::Revision => {
            if let Some(path) = resolve_speck_path(&input, &project_root) {
                LoopContext::revision(path, context_contents)
            } else {
                // File specified but doesn't exist - treat as error
                let message = format!("Speck file not found: {}", input);
                if json_output {
                    output_error_json("plan", "E002", &message);
                } else {
                    eprintln!("error: {}", message);
                }
                return Ok(2);
            }
        }
    };

    if !quiet {
        match mode {
            PlanMode::New => eprintln!("Creating new speck from idea: {}", input),
            PlanMode::Revision => eprintln!("Revising existing speck: {}", input),
        }
    }

    // Create and run the planning loop
    let mut planning_loop = PlanningLoop::new(
        context,
        project_root.clone(),
        timeout,
        name,
        json_output,
        quiet,
    );

    match planning_loop.run() {
        Ok(outcome) => {
            // Success!
            if json_output {
                let data = PlanData {
                    speck_path: make_relative_path(&project_root, &outcome.speck_path),
                    speck_name: outcome.speck_name.clone(),
                    mode: outcome.mode.to_string(),
                    iterations: outcome.iterations,
                    validation: PlanValidation {
                        errors: outcome.validation_errors,
                        warnings: outcome.validation_warnings,
                    },
                    critic_approved: outcome.critic_approved,
                    user_approved: outcome.user_approved,
                };

                let response = if outcome.validation_errors > 0 {
                    let issues = vec![JsonIssue {
                        code: "E001".to_string(),
                        severity: "error".to_string(),
                        message: format!("Speck has {} validation errors", outcome.validation_errors),
                        file: Some(make_relative_path(&project_root, &outcome.speck_path)),
                        line: None,
                        anchor: None,
                    }];
                    JsonResponse::error("plan", data, issues)
                } else if outcome.validation_warnings > 0 {
                    let issues = vec![JsonIssue {
                        code: "E023".to_string(),
                        severity: "warning".to_string(),
                        message: format!("Speck has {} validation warnings", outcome.validation_warnings),
                        file: Some(make_relative_path(&project_root, &outcome.speck_path)),
                        line: None,
                        anchor: None,
                    }];
                    JsonResponse::ok_with_issues("plan", data, issues)
                } else {
                    JsonResponse::ok("plan", data)
                };

                println!("{}", serde_json::to_string_pretty(&response).unwrap());
            } else if !quiet {
                println!(
                    "Speck created: {}",
                    make_relative_path(&project_root, &outcome.speck_path)
                );
                println!("Iterations: {}", outcome.iterations);
                if outcome.validation_errors > 0 {
                    println!(
                        "Warning: Speck has {} validation error{}",
                        outcome.validation_errors,
                        if outcome.validation_errors == 1 { "" } else { "s" }
                    );
                } else if outcome.validation_warnings > 0 {
                    println!(
                        "Note: Speck has {} validation warning{}",
                        outcome.validation_warnings,
                        if outcome.validation_warnings == 1 { "" } else { "s" }
                    );
                }
            }

            // Return appropriate exit code
            if outcome.validation_errors > 0 {
                Ok(3) // Validation error
            } else {
                Ok(0) // Success
            }
        }
        Err(SpecksError::UserAborted) => {
            if json_output {
                output_error_json("plan", "E024", "User aborted planning loop");
            } else if !quiet {
                eprintln!("Planning loop aborted by user");
            }
            Ok(5) // User aborted
        }
        Err(SpecksError::ClaudeCliNotInstalled) => {
            if json_output {
                output_error_json(
                    "plan",
                    "E019",
                    "Claude CLI not installed. Install Claude Code from https://claude.ai/download",
                );
            } else {
                eprintln!(
                    "error: Claude CLI not installed. Install Claude Code from https://claude.ai/download"
                );
            }
            Ok(6) // Claude CLI not installed
        }
        Err(SpecksError::AgentTimeout { secs }) => {
            let message = format!("Agent timeout after {} seconds", secs);
            if json_output {
                output_error_json("plan", "E021", &message);
            } else {
                eprintln!("error: {}", message);
            }
            Ok(1)
        }
        Err(SpecksError::AgentInvocationFailed { reason }) => {
            let message = format!("Agent invocation failed: {}", reason);
            if json_output {
                output_error_json("plan", "E020", &message);
            } else {
                eprintln!("error: {}", message);
            }
            Ok(1)
        }
        Err(e) => {
            let message = e.to_string();
            if json_output {
                output_error_json("plan", e.code(), &message);
            } else {
                eprintln!("error: {}", message);
            }
            Ok(e.exit_code())
        }
    }
}

/// Output an error in JSON format
fn output_error_json(command: &str, code: &str, message: &str) {
    let issues = vec![JsonIssue {
        code: code.to_string(),
        severity: "error".to_string(),
        message: message.to_string(),
        file: None,
        line: None,
        anchor: None,
    }];

    let data = PlanData {
        speck_path: String::new(),
        speck_name: String::new(),
        mode: String::new(),
        iterations: 0,
        validation: PlanValidation {
            errors: 0,
            warnings: 0,
        },
        critic_approved: false,
        user_approved: false,
    };

    let response: JsonResponse<PlanData> = JsonResponse::error(command, data, issues);
    println!("{}", serde_json::to_string_pretty(&response).unwrap());
}

/// Make a path relative to the project root using forward slashes
fn make_relative_path(project_root: &std::path::Path, path: &PathBuf) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path.as_path())
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_relative_path() {
        let project_root = PathBuf::from("/project");
        let path = PathBuf::from("/project/.specks/specks-1.md");
        let relative = make_relative_path(&project_root, &path);
        assert_eq!(relative, ".specks/specks-1.md");
    }

    #[test]
    fn test_make_relative_path_already_relative() {
        let project_root = PathBuf::from("/project");
        let path = PathBuf::from(".specks/specks-1.md");
        let relative = make_relative_path(&project_root, &path);
        assert_eq!(relative, ".specks/specks-1.md");
    }
}
