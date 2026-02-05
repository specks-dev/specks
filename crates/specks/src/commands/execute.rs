//! Implementation of the `specks execute` command
//!
//! Per Spec S02, this command executes a speck's steps through agent-driven
//! implementation using the director agent's S10 execution protocol.

use std::fs;
use std::path::{Path, PathBuf};

use specks_core::{Severity, SpecksError, find_project_root, parse_speck, validate_speck};
use uuid::Uuid;

use crate::agent::{AgentRunner, director_config, verify_required_agents};
use crate::output::{ExecuteData, JsonIssue, JsonResponse};

/// Commit policy for execution
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommitPolicy {
    /// Prompt user before each commit
    Manual,
    /// Commit automatically after each step
    Auto,
}

impl CommitPolicy {
    /// Parse from string, defaulting to Manual
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "auto" => CommitPolicy::Auto,
            _ => CommitPolicy::Manual,
        }
    }
}

impl std::fmt::Display for CommitPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommitPolicy::Manual => write!(f, "manual"),
            CommitPolicy::Auto => write!(f, "auto"),
        }
    }
}

/// Checkpoint mode for execution
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CheckpointMode {
    /// Pause after every step for user confirmation
    Step,
    /// Pause only at milestone boundaries
    Milestone,
    /// No pauses; only stop on error or halt
    Continuous,
}

impl CheckpointMode {
    /// Parse from string, defaulting to Step
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "milestone" => CheckpointMode::Milestone,
            "continuous" => CheckpointMode::Continuous,
            _ => CheckpointMode::Step,
        }
    }
}

impl std::fmt::Display for CheckpointMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckpointMode::Step => write!(f, "step"),
            CheckpointMode::Milestone => write!(f, "milestone"),
            CheckpointMode::Continuous => write!(f, "continuous"),
        }
    }
}

/// Execution context containing all parameters
#[derive(Debug)]
pub struct ExecutionContext {
    /// Path to the speck file
    pub speck_path: PathBuf,
    /// UUID for this run
    pub run_id: String,
    /// Path to the run directory
    pub run_directory: PathBuf,
    /// Step anchor to start from
    pub start_step: Option<String>,
    /// Step anchor to stop after
    pub end_step: Option<String>,
    /// Commit policy
    pub commit_policy: CommitPolicy,
    /// Checkpoint mode
    pub checkpoint_mode: CheckpointMode,
    /// Timeout per step in seconds
    pub timeout: u64,
}

/// Execution outcome
#[derive(Debug)]
pub struct ExecutionOutcome {
    /// UUID of this execution run
    pub run_id: String,
    /// Path to the run directory
    pub run_directory: PathBuf,
    /// Steps that were completed
    pub steps_completed: Vec<String>,
    /// Steps remaining
    pub steps_remaining: Vec<String>,
    /// Number of commits created
    pub commits_created: usize,
    /// Outcome status
    pub outcome: ExecutionOutcomeStatus,
}

/// Status of execution outcome
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExecutionOutcomeStatus {
    /// All requested steps completed successfully
    Success,
    /// Execution failed due to error
    Failure,
    /// Execution was halted by monitor
    Halted,
    /// Some steps completed, but not all
    Partial,
    /// Dry run - no actual execution
    DryRun,
}

impl std::fmt::Display for ExecutionOutcomeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionOutcomeStatus::Success => write!(f, "success"),
            ExecutionOutcomeStatus::Failure => write!(f, "failure"),
            ExecutionOutcomeStatus::Halted => write!(f, "halted"),
            ExecutionOutcomeStatus::Partial => write!(f, "partial"),
            ExecutionOutcomeStatus::DryRun => write!(f, "dry_run"),
        }
    }
}

/// Options for the execute command
pub struct ExecuteOptions {
    pub speck: String,
    pub start_step: Option<String>,
    pub end_step: Option<String>,
    pub commit_policy: String,
    pub checkpoint_mode: String,
    pub dry_run: bool,
    pub timeout: u64,
    pub json_output: bool,
    pub quiet: bool,
    pub verbose_agents: bool,
}

/// Run the execute command
///
/// # Returns
///
/// Exit code per Spec S02:
/// - 0: Success - all requested steps completed
/// - 1: General error
/// - 3: Step failed validation/review
/// - 4: Execution halted by monitor
/// - 6: Claude CLI not installed
/// - 9: Not initialized
pub fn run_execute(opts: ExecuteOptions) -> Result<i32, String> {
    let ExecuteOptions {
        speck,
        start_step,
        end_step,
        commit_policy,
        checkpoint_mode,
        dry_run,
        timeout,
        json_output,
        quiet,
        verbose_agents,
    } = opts;
    // Find project root
    let project_root = match find_project_root() {
        Ok(root) => root,
        Err(_) => {
            let message = ".specks directory not initialized".to_string();
            if json_output {
                output_error_json("execute", "E009", &message, &speck);
            } else {
                eprintln!("error: {}", message);
            }
            return Ok(9);
        }
    };

    // Preflight: verify required agents are available
    match verify_required_agents("execute", &project_root) {
        Ok(agents) => {
            if verbose_agents && !quiet {
                eprintln!("Resolved agents for 'specks execute':");
                for (agent_name, path, source) in &agents {
                    eprintln!("  {} ({}) -> {}", agent_name, source, path.display());
                }
            }
        }
        Err(SpecksError::RequiredAgentsMissing {
            command,
            missing,
            searched,
        }) => {
            let message = format!(
                "Missing required agents for 'specks {}': {}\nSearched: {}",
                command,
                missing.join(", "),
                searched.join(", ")
            );
            if json_output {
                output_error_json("execute", "E026", &message, &speck);
            } else {
                eprintln!("error: {}", message);
            }
            return Ok(8);
        }
        Err(e) => {
            let message = e.to_string();
            if json_output {
                output_error_json("execute", e.code(), &message, &speck);
            } else {
                eprintln!("error: {}", message);
            }
            return Ok(e.exit_code());
        }
    }

    // Resolve speck path
    let speck_path = resolve_speck_path(&speck, &project_root);
    if !speck_path.exists() {
        let message = format!("Speck file not found: {}", speck);
        if json_output {
            output_error_json("execute", "E002", &message, &speck);
        } else {
            eprintln!("error: {}", message);
        }
        return Ok(1);
    }

    // Parse and validate speck
    let content = match fs::read_to_string(&speck_path) {
        Ok(c) => c,
        Err(e) => {
            let message = format!("Failed to read speck file: {}", e);
            if json_output {
                output_error_json("execute", "E002", &message, &speck);
            } else {
                eprintln!("error: {}", message);
            }
            return Ok(1);
        }
    };

    let parsed_speck = match parse_speck(&content) {
        Ok(s) => s,
        Err(e) => {
            let message = format!("Failed to parse speck: {}", e);
            if json_output {
                output_error_json("execute", "E001", &message, &speck);
            } else {
                eprintln!("error: {}", message);
            }
            return Ok(1);
        }
    };

    // Validate speck
    let validation_result = validate_speck(&parsed_speck);
    let error_count = validation_result.error_count();

    if error_count > 0 {
        let message = format!("Speck has {} validation error(s)", error_count);
        if json_output {
            output_error_json("execute", "E001", &message, &speck);
        } else {
            eprintln!("error: {}", message);
            for issue in validation_result
                .issues
                .iter()
                .filter(|i| i.severity == Severity::Error)
            {
                eprintln!("  {}: {}", issue.code, issue.message);
            }
        }
        return Ok(3);
    }

    // Verify speck status is "active"
    let status = parsed_speck
        .metadata
        .status
        .as_deref()
        .unwrap_or("draft")
        .to_lowercase();
    if status != "active" {
        let message = format!("Speck status is '{}', must be 'active' to execute", status);
        if json_output {
            output_error_json("execute", "E003", &message, &speck);
        } else {
            eprintln!("error: {}", message);
        }
        return Ok(1);
    }

    // Verify beads root exists (optional - warn if missing)
    let beads_root = parsed_speck.metadata.beads_root_id.clone();
    if beads_root.is_none() && !dry_run && !quiet {
        eprintln!(
            "warning: No Beads Root in metadata. Run `specks beads sync {}` to set up work tracking.",
            speck
        );
    }

    // Parse policies
    let commit_policy = CommitPolicy::from_str(&commit_policy);
    let checkpoint_mode = CheckpointMode::from_str(&checkpoint_mode);

    // Generate run UUID and create run directory
    let run_id = Uuid::new_v4().to_string();
    let run_directory = project_root.join(".specks/runs").join(&run_id);

    if !dry_run {
        if let Err(e) = fs::create_dir_all(&run_directory) {
            let message = format!("Failed to create run directory: {}", e);
            if json_output {
                output_error_json("execute", "E002", &message, &speck);
            } else {
                eprintln!("error: {}", message);
            }
            return Ok(1);
        }

        // Write invocation.json
        let invocation = serde_json::json!({
            "uuid": run_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "speck": make_relative_path(&project_root, &speck_path),
            "mode": "execute",
            "commit_policy": commit_policy.to_string(),
            "checkpoint_mode": checkpoint_mode.to_string(),
            "start_step": start_step,
            "end_step": end_step
        });

        let invocation_path = run_directory.join("invocation.json");
        if let Err(e) = fs::write(
            &invocation_path,
            serde_json::to_string_pretty(&invocation).unwrap(),
        ) {
            let message = format!("Failed to write invocation.json: {}", e);
            if json_output {
                output_error_json("execute", "E002", &message, &speck);
            } else {
                eprintln!("error: {}", message);
            }
            return Ok(1);
        }
    }

    // Collect steps to execute
    let all_steps: Vec<String> = parsed_speck
        .steps
        .iter()
        .map(|s| format!("#{}", s.anchor))
        .collect();

    let steps_to_execute = filter_steps(&all_steps, &start_step, &end_step);

    // Create execution context
    let context = ExecutionContext {
        speck_path: speck_path.clone(),
        run_id: run_id.clone(),
        run_directory: run_directory.clone(),
        start_step,
        end_step,
        commit_policy,
        checkpoint_mode,
        timeout,
    };

    // Handle dry run
    if dry_run {
        return handle_dry_run(
            &context,
            &project_root,
            &steps_to_execute,
            &all_steps,
            json_output,
            quiet,
        );
    }

    // Check Claude CLI
    let runner = AgentRunner::new(project_root.clone());
    if runner.check_claude_cli().is_err() {
        let message =
            "Claude CLI not installed. Install Claude Code from https://claude.ai/download";
        if json_output {
            output_error_json("execute", "E019", message, &speck);
        } else {
            eprintln!("error: {}", message);
        }
        return Ok(6);
    }

    if !quiet {
        eprintln!(
            "Executing speck: {}",
            make_relative_path(&project_root, &speck_path)
        );
        eprintln!("Run ID: {}", run_id);
        eprintln!("Steps to execute: {}", steps_to_execute.join(", "));
    }

    // Invoke director agent
    let outcome = invoke_director(
        &context,
        &project_root,
        &runner,
        &steps_to_execute,
        &all_steps,
        quiet,
    );

    match outcome {
        Ok(outcome) => {
            // Write status.json
            write_status_json(&run_directory, &outcome);

            // Output result
            if json_output {
                output_success_json(&project_root, &speck_path, &outcome);
            } else if !quiet {
                println!("Execution complete: {}", outcome.outcome);
                println!("Steps completed: {}", outcome.steps_completed.join(", "));
                if !outcome.steps_remaining.is_empty() {
                    println!("Steps remaining: {}", outcome.steps_remaining.join(", "));
                }
                println!("Commits created: {}", outcome.commits_created);
                println!(
                    "Run directory: {}",
                    make_relative_path(&project_root, &outcome.run_directory)
                );
            }

            match outcome.outcome {
                ExecutionOutcomeStatus::Success | ExecutionOutcomeStatus::DryRun => Ok(0),
                ExecutionOutcomeStatus::Failure => Ok(1),
                ExecutionOutcomeStatus::Halted => Ok(4),
                ExecutionOutcomeStatus::Partial => Ok(3),
            }
        }
        Err(e) => match e {
            SpecksError::ClaudeCliNotInstalled => {
                if json_output {
                    output_error_json("execute", "E019", &e.to_string(), &speck);
                } else {
                    eprintln!("error: {}", e);
                }
                Ok(6)
            }
            SpecksError::MonitorHalted { reason } => {
                if json_output {
                    output_error_json("execute", "E022", &reason, &speck);
                } else {
                    eprintln!("error: Monitor halted execution: {}", reason);
                }
                Ok(4)
            }
            SpecksError::AgentTimeout { secs } => {
                let message = format!("Agent timeout after {} seconds", secs);
                if json_output {
                    output_error_json("execute", "E021", &message, &speck);
                } else {
                    eprintln!("error: {}", message);
                }
                Ok(1)
            }
            SpecksError::AgentInvocationFailed { reason } => {
                let message = format!("Agent invocation failed: {}", reason);
                if json_output {
                    output_error_json("execute", "E020", &message, &speck);
                } else {
                    eprintln!("error: {}", message);
                }
                Ok(1)
            }
            _ => {
                if json_output {
                    output_error_json("execute", e.code(), &e.to_string(), &speck);
                } else {
                    eprintln!("error: {}", e);
                }
                Ok(e.exit_code())
            }
        },
    }
}

/// Resolve speck path relative to project root
fn resolve_speck_path(speck: &str, project_root: &std::path::Path) -> PathBuf {
    let path = PathBuf::from(speck);
    if path.is_absolute() {
        path
    } else if path.starts_with(".specks/") || path.starts_with(".specks\\") {
        project_root.join(path)
    } else {
        // Try as-is first, then in .specks/
        if project_root.join(&path).exists() {
            project_root.join(path)
        } else {
            project_root.join(".specks").join(path)
        }
    }
}

/// Filter steps based on start and end anchors
fn filter_steps(
    all_steps: &[String],
    start_step: &Option<String>,
    end_step: &Option<String>,
) -> Vec<String> {
    let mut in_range = start_step.is_none();
    let mut result = Vec::new();

    for step in all_steps {
        // Normalize the step anchor for comparison
        let normalized = if step.starts_with('#') {
            step.clone()
        } else {
            format!("#{}", step)
        };

        // Check if we should start
        if let Some(start) = start_step {
            let start_normalized = if start.starts_with('#') {
                start.clone()
            } else {
                format!("#{}", start)
            };
            if normalized == start_normalized {
                in_range = true;
            }
        }

        // Add step if in range
        if in_range {
            result.push(step.clone());
        }

        // Check if we should stop
        if let Some(end) = end_step {
            let end_normalized = if end.starts_with('#') {
                end.clone()
            } else {
                format!("#{}", end)
            };
            if normalized == end_normalized {
                break;
            }
        }
    }

    result
}

/// Handle dry run - show execution plan without running
fn handle_dry_run(
    context: &ExecutionContext,
    project_root: &std::path::Path,
    steps_to_execute: &[String],
    all_steps: &[String],
    json_output: bool,
    quiet: bool,
) -> Result<i32, String> {
    let steps_remaining: Vec<String> = all_steps
        .iter()
        .filter(|s| !steps_to_execute.contains(s))
        .cloned()
        .collect();

    let outcome = ExecutionOutcome {
        run_id: context.run_id.clone(),
        run_directory: context.run_directory.clone(),
        steps_completed: vec![], // No steps completed in dry run
        steps_remaining: steps_to_execute.to_vec(), // All steps are "remaining" (to be executed)
        commits_created: 0,
        outcome: ExecutionOutcomeStatus::DryRun,
    };

    if json_output {
        let data = ExecuteData {
            speck_path: make_relative_path(project_root, &context.speck_path),
            run_id: outcome.run_id.clone(),
            run_directory: String::new(), // Not created in dry run
            steps_completed: vec![],
            steps_remaining: steps_to_execute.to_vec(),
            commits_created: 0,
            outcome: "dry_run".to_string(),
        };
        let response: JsonResponse<ExecuteData> = JsonResponse::ok("execute", data);
        println!("{}", serde_json::to_string_pretty(&response).unwrap());
    } else if !quiet {
        println!("Dry run - execution plan:");
        println!(
            "  Speck: {}",
            make_relative_path(project_root, &context.speck_path)
        );
        println!("  Commit policy: {}", context.commit_policy);
        println!("  Checkpoint mode: {}", context.checkpoint_mode);
        println!("  Timeout per step: {} seconds", context.timeout);
        println!();
        println!("Steps to execute:");
        for (i, step) in steps_to_execute.iter().enumerate() {
            println!("  {}. {}", i + 1, step);
        }
        if !steps_remaining.is_empty() {
            println!();
            println!("Steps outside range (will be skipped):");
            for step in &steps_remaining {
                println!("  - {}", step);
            }
        }
    }

    Ok(0)
}

/// Invoke the director agent to execute steps
fn invoke_director(
    context: &ExecutionContext,
    project_root: &std::path::Path,
    runner: &AgentRunner,
    steps_to_execute: &[String],
    all_steps: &[String],
    quiet: bool,
) -> Result<ExecutionOutcome, SpecksError> {
    // Build the prompt for the director agent
    let prompt = build_director_prompt(context, project_root, steps_to_execute);

    // Get director config
    let mut config = director_config(project_root);
    config = config.with_timeout(context.timeout);

    if !quiet {
        eprintln!("Invoking director agent...");
    }

    // Invoke the director
    let result = runner.invoke_agent(&config, &prompt)?;

    // Check for halt signal
    let halt_file = context.run_directory.join(".halt");
    if halt_file.exists() {
        // Read halt reason
        let halt_content = fs::read_to_string(&halt_file).unwrap_or_default();
        let reason = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&halt_content) {
            json.get("reason")
                .and_then(|r| r.as_str())
                .unwrap_or("unknown")
                .to_string()
        } else {
            "execution halted".to_string()
        };
        return Err(SpecksError::MonitorHalted { reason });
    }

    // Parse the director's output to determine outcome
    // For now, we'll assume success if the agent completed without error
    // In a full implementation, we'd parse status.json from the run directory
    let status_file = context.run_directory.join("status.json");
    let outcome = if status_file.exists() {
        parse_status_json(&status_file, context, all_steps)
    } else {
        // Agent completed but didn't write status - assume success
        ExecutionOutcome {
            run_id: context.run_id.clone(),
            run_directory: context.run_directory.clone(),
            steps_completed: steps_to_execute.to_vec(),
            steps_remaining: all_steps
                .iter()
                .filter(|s| !steps_to_execute.contains(s))
                .cloned()
                .collect(),
            commits_created: 0,
            outcome: if result.success {
                ExecutionOutcomeStatus::Success
            } else {
                ExecutionOutcomeStatus::Failure
            },
        }
    };

    Ok(outcome)
}

/// Build the prompt for the director agent
fn build_director_prompt(
    context: &ExecutionContext,
    project_root: &std::path::Path,
    steps_to_execute: &[String],
) -> String {
    let speck_relative = make_relative_path(project_root, &context.speck_path);
    let run_dir_relative = make_relative_path(project_root, &context.run_directory);

    let steps_str = steps_to_execute.join(", ");

    format!(
        r#"Execute the speck at {speck_path} in execution mode.

Parameters:
- mode: execute
- run_directory: {run_dir}
- start_step: {start_step}
- end_step: {end_step}
- commit_policy: {commit_policy}
- checkpoint_mode: {checkpoint_mode}
- steps_to_execute: {steps}

Follow the S10 execution protocol:
1. Validate preconditions (speck exists, status is active)
2. For each step in order:
   a. Invoke architect for implementation strategy
   b. Spawn implementer (with monitor watching)
   c. After implementation, invoke reviewer and auditor
   d. If approved, invoke logger and committer
   e. Close the step's bead if beads are configured

Write status.json to the run directory with the outcome.
"#,
        speck_path = speck_relative,
        run_dir = run_dir_relative,
        start_step = context.start_step.as_deref().unwrap_or("first"),
        end_step = context.end_step.as_deref().unwrap_or("all"),
        commit_policy = context.commit_policy,
        checkpoint_mode = context.checkpoint_mode,
        steps = steps_str,
    )
}

/// Parse status.json from run directory
fn parse_status_json(
    status_file: &std::path::Path,
    context: &ExecutionContext,
    all_steps: &[String],
) -> ExecutionOutcome {
    let content = fs::read_to_string(status_file).unwrap_or_default();

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
        let outcome_str = json
            .get("outcome")
            .and_then(|o| o.as_str())
            .unwrap_or("success");

        let outcome = match outcome_str {
            "failure" => ExecutionOutcomeStatus::Failure,
            "halted" => ExecutionOutcomeStatus::Halted,
            "partial" => ExecutionOutcomeStatus::Partial,
            _ => ExecutionOutcomeStatus::Success,
        };

        let steps_completed: Vec<String> = json
            .get("steps_completed")
            .and_then(|s| s.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let steps_remaining: Vec<String> = json
            .get("steps_remaining")
            .and_then(|s| s.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_else(|| {
                all_steps
                    .iter()
                    .filter(|s| !steps_completed.contains(s))
                    .cloned()
                    .collect()
            });

        ExecutionOutcome {
            run_id: context.run_id.clone(),
            run_directory: context.run_directory.clone(),
            steps_completed,
            steps_remaining,
            commits_created: 0, // Would parse from status.json in full impl
            outcome,
        }
    } else {
        // Couldn't parse - assume partial completion
        ExecutionOutcome {
            run_id: context.run_id.clone(),
            run_directory: context.run_directory.clone(),
            steps_completed: vec![],
            steps_remaining: all_steps.to_vec(),
            commits_created: 0,
            outcome: ExecutionOutcomeStatus::Partial,
        }
    }
}

/// Write status.json to run directory
fn write_status_json(run_directory: &std::path::Path, outcome: &ExecutionOutcome) {
    let status = serde_json::json!({
        "uuid": outcome.run_id,
        "outcome": outcome.outcome.to_string(),
        "steps_completed": outcome.steps_completed,
        "steps_remaining": outcome.steps_remaining,
        "commits_created": outcome.commits_created,
        "timestamp_end": chrono::Utc::now().to_rfc3339()
    });

    let status_path = run_directory.join("status.json");
    let _ = fs::write(&status_path, serde_json::to_string_pretty(&status).unwrap());
}

/// Output an error in JSON format
fn output_error_json(command: &str, code: &str, message: &str, speck_path: &str) {
    let issues = vec![JsonIssue {
        code: code.to_string(),
        severity: "error".to_string(),
        message: message.to_string(),
        file: Some(speck_path.to_string()),
        line: None,
        anchor: None,
    }];

    let data = ExecuteData {
        speck_path: speck_path.to_string(),
        run_id: String::new(),
        run_directory: String::new(),
        steps_completed: vec![],
        steps_remaining: vec![],
        commits_created: 0,
        outcome: "failure".to_string(),
    };

    let response: JsonResponse<ExecuteData> = JsonResponse::error(command, data, issues);
    println!("{}", serde_json::to_string_pretty(&response).unwrap());
}

/// Output success in JSON format
fn output_success_json(
    project_root: &std::path::Path,
    speck_path: &Path,
    outcome: &ExecutionOutcome,
) {
    let data = ExecuteData {
        speck_path: make_relative_path(project_root, speck_path),
        run_id: outcome.run_id.clone(),
        run_directory: make_relative_path(project_root, &outcome.run_directory),
        steps_completed: outcome.steps_completed.clone(),
        steps_remaining: outcome.steps_remaining.clone(),
        commits_created: outcome.commits_created,
        outcome: outcome.outcome.to_string(),
    };

    let response: JsonResponse<ExecuteData> = if outcome.outcome == ExecutionOutcomeStatus::Success
        || outcome.outcome == ExecutionOutcomeStatus::DryRun
    {
        JsonResponse::ok("execute", data)
    } else {
        let issues = vec![JsonIssue {
            code: match outcome.outcome {
                ExecutionOutcomeStatus::Halted => "E022".to_string(),
                ExecutionOutcomeStatus::Failure => "E020".to_string(),
                _ => "E001".to_string(),
            },
            severity: "error".to_string(),
            message: format!("Execution {}", outcome.outcome),
            file: None,
            line: None,
            anchor: None,
        }];
        JsonResponse::error("execute", data, issues)
    };

    println!("{}", serde_json::to_string_pretty(&response).unwrap());
}

/// Make a path relative to the project root using forward slashes
fn make_relative_path(project_root: &std::path::Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_policy_from_str() {
        assert_eq!(CommitPolicy::from_str("manual"), CommitPolicy::Manual);
        assert_eq!(CommitPolicy::from_str("auto"), CommitPolicy::Auto);
        assert_eq!(CommitPolicy::from_str("AUTO"), CommitPolicy::Auto);
        assert_eq!(CommitPolicy::from_str("unknown"), CommitPolicy::Manual);
    }

    #[test]
    fn test_checkpoint_mode_from_str() {
        assert_eq!(CheckpointMode::from_str("step"), CheckpointMode::Step);
        assert_eq!(
            CheckpointMode::from_str("milestone"),
            CheckpointMode::Milestone
        );
        assert_eq!(
            CheckpointMode::from_str("continuous"),
            CheckpointMode::Continuous
        );
        assert_eq!(
            CheckpointMode::from_str("MILESTONE"),
            CheckpointMode::Milestone
        );
        assert_eq!(CheckpointMode::from_str("unknown"), CheckpointMode::Step);
    }

    #[test]
    fn test_filter_steps_no_range() {
        let steps = vec![
            "#step-0".to_string(),
            "#step-1".to_string(),
            "#step-2".to_string(),
        ];
        let result = filter_steps(&steps, &None, &None);
        assert_eq!(result, steps);
    }

    #[test]
    fn test_filter_steps_with_start() {
        let steps = vec![
            "#step-0".to_string(),
            "#step-1".to_string(),
            "#step-2".to_string(),
        ];
        let result = filter_steps(&steps, &Some("#step-1".to_string()), &None);
        assert_eq!(result, vec!["#step-1".to_string(), "#step-2".to_string()]);
    }

    #[test]
    fn test_filter_steps_with_end() {
        let steps = vec![
            "#step-0".to_string(),
            "#step-1".to_string(),
            "#step-2".to_string(),
        ];
        let result = filter_steps(&steps, &None, &Some("#step-1".to_string()));
        assert_eq!(result, vec!["#step-0".to_string(), "#step-1".to_string()]);
    }

    #[test]
    fn test_filter_steps_with_range() {
        let steps = vec![
            "#step-0".to_string(),
            "#step-1".to_string(),
            "#step-2".to_string(),
            "#step-3".to_string(),
        ];
        let result = filter_steps(
            &steps,
            &Some("#step-1".to_string()),
            &Some("#step-2".to_string()),
        );
        assert_eq!(result, vec!["#step-1".to_string(), "#step-2".to_string()]);
    }

    #[test]
    fn test_filter_steps_normalizes_anchors() {
        let steps = vec![
            "#step-0".to_string(),
            "#step-1".to_string(),
            "#step-2".to_string(),
        ];
        // Without leading # in argument
        let result = filter_steps(&steps, &Some("step-1".to_string()), &None);
        assert_eq!(result, vec!["#step-1".to_string(), "#step-2".to_string()]);
    }

    #[test]
    fn test_execution_outcome_status_display() {
        assert_eq!(ExecutionOutcomeStatus::Success.to_string(), "success");
        assert_eq!(ExecutionOutcomeStatus::Failure.to_string(), "failure");
        assert_eq!(ExecutionOutcomeStatus::Halted.to_string(), "halted");
        assert_eq!(ExecutionOutcomeStatus::Partial.to_string(), "partial");
        assert_eq!(ExecutionOutcomeStatus::DryRun.to_string(), "dry_run");
    }

    #[test]
    fn test_resolve_speck_path_absolute() {
        let project_root = PathBuf::from("/project");
        let result = resolve_speck_path("/absolute/path/speck.md", &project_root);
        assert_eq!(result, PathBuf::from("/absolute/path/speck.md"));
    }

    #[test]
    fn test_resolve_speck_path_relative_with_specks() {
        let project_root = PathBuf::from("/project");
        let result = resolve_speck_path(".specks/specks-1.md", &project_root);
        assert_eq!(result, PathBuf::from("/project/.specks/specks-1.md"));
    }

    #[test]
    fn test_make_relative_path() {
        let project_root = PathBuf::from("/project");
        let path = PathBuf::from("/project/.specks/runs/uuid123");
        let relative = make_relative_path(&project_root, &path);
        assert_eq!(relative, ".specks/runs/uuid123");
    }
}
