//! Implementation of the `specks status` command (Spec S04)

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use specks_core::{
    BeadsCli, IssueDetails, Speck, find_project_root, parse_close_reason, parse_speck,
    speck_name_from_path,
};

use crate::output::{
    BeadStepStatus, JsonIssue, JsonResponse, Progress, StatusData, StepInfo, StepStatus,
    SubstepStatus,
};

/// Run the status command
pub fn run_status(
    file: String,
    verbose: bool,
    full: bool,
    json_output: bool,
    _quiet: bool,
) -> Result<i32, String> {
    // Find project root
    let project_root = match find_project_root() {
        Ok(root) => root,
        Err(_) => {
            let message = ".specks directory not initialized".to_string();
            if json_output {
                let issues = vec![JsonIssue {
                    code: "E009".to_string(),
                    severity: "error".to_string(),
                    message: message.clone(),
                    file: None,
                    line: None,
                    anchor: None,
                }];
                let response: JsonResponse<StatusData> = JsonResponse::error(
                    "status",
                    StatusData {
                        name: String::new(),
                        status: String::new(),
                        progress: Progress { done: 0, total: 0 },
                        steps: vec![],
                        all_steps: None,
                        completed_steps: None,
                        remaining_steps: None,
                        next_step: None,
                        bead_mapping: None,
                        dependencies: None,
                        mode: None,
                        speck: None,
                        phase_title: None,
                        total_step_count: None,
                        completed_step_count: None,
                        ready_step_count: None,
                        blocked_step_count: None,
                        bead_steps: None,
                    },
                    issues,
                );
                println!("{}", serde_json::to_string_pretty(&response).unwrap());
            } else {
                eprintln!("error: {}", message);
            }
            return Ok(9);
        }
    };

    // Resolve file path
    let path = resolve_file_path(&project_root, &file);
    if !path.exists() {
        let message = format!("file not found: {}", file);
        if json_output {
            let issues = vec![JsonIssue {
                code: "E002".to_string(),
                severity: "error".to_string(),
                message: message.clone(),
                file: Some(file),
                line: None,
                anchor: None,
            }];
            let response: JsonResponse<StatusData> = JsonResponse::error(
                "status",
                StatusData {
                    name: String::new(),
                    status: String::new(),
                    progress: Progress { done: 0, total: 0 },
                    steps: vec![],
                    all_steps: None,
                    completed_steps: None,
                    remaining_steps: None,
                    next_step: None,
                    bead_mapping: None,
                    dependencies: None,
                    mode: None,
                    speck: None,
                    phase_title: None,
                    total_step_count: None,
                    completed_step_count: None,
                    ready_step_count: None,
                    blocked_step_count: None,
                    bead_steps: None,
                },
                issues,
            );
            println!("{}", serde_json::to_string_pretty(&response).unwrap());
        } else {
            eprintln!("error: {}", message);
        }
        return Ok(2);
    }

    // Read and parse the speck
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            let message = format!("failed to read file: {}", e);
            if json_output {
                let issues = vec![JsonIssue {
                    code: "E002".to_string(),
                    severity: "error".to_string(),
                    message: message.clone(),
                    file: Some(file),
                    line: None,
                    anchor: None,
                }];
                let response: JsonResponse<StatusData> = JsonResponse::error(
                    "status",
                    StatusData {
                        name: String::new(),
                        status: String::new(),
                        progress: Progress { done: 0, total: 0 },
                        steps: vec![],
                        all_steps: None,
                        completed_steps: None,
                        remaining_steps: None,
                        next_step: None,
                        bead_mapping: None,
                        dependencies: None,
                        mode: None,
                        speck: None,
                        phase_title: None,
                        total_step_count: None,
                        completed_step_count: None,
                        ready_step_count: None,
                        blocked_step_count: None,
                        bead_steps: None,
                    },
                    issues,
                );
                println!("{}", serde_json::to_string_pretty(&response).unwrap());
            } else {
                eprintln!("error: {}", message);
            }
            return Ok(2);
        }
    };

    let speck = match parse_speck(&content) {
        Ok(s) => s,
        Err(e) => {
            let message = format!("failed to parse speck: {}", e);
            if json_output {
                let issues = vec![JsonIssue {
                    code: "E001".to_string(),
                    severity: "error".to_string(),
                    message: message.clone(),
                    file: Some(file),
                    line: None,
                    anchor: None,
                }];
                let response: JsonResponse<StatusData> = JsonResponse::error(
                    "status",
                    StatusData {
                        name: String::new(),
                        status: String::new(),
                        progress: Progress { done: 0, total: 0 },
                        steps: vec![],
                        all_steps: None,
                        completed_steps: None,
                        remaining_steps: None,
                        next_step: None,
                        bead_mapping: None,
                        dependencies: None,
                        mode: None,
                        speck: None,
                        phase_title: None,
                        total_step_count: None,
                        completed_step_count: None,
                        ready_step_count: None,
                        blocked_step_count: None,
                        bead_steps: None,
                    },
                    issues,
                );
                println!("{}", serde_json::to_string_pretty(&response).unwrap());
            } else {
                eprintln!("error: {}", message);
            }
            return Ok(1);
        }
    };

    let name = speck_name_from_path(&path).unwrap_or_else(|| file.clone());

    // Check if beads integration is available
    if let Some(ref root_id) = speck.metadata.beads_root_id {
        // Try beads path
        let beads_cli = BeadsCli::default();

        if !beads_cli.is_installed(None) {
            eprintln!("warning: beads CLI not found, falling back to checkbox mode");
            // Fall back to checkbox mode
            let status_data = build_checkbox_status_data(&speck, &name);
            if json_output {
                let response = JsonResponse::ok("status", status_data);
                println!("{}", serde_json::to_string_pretty(&response).unwrap());
            } else {
                output_text(&status_data, &speck, verbose);
            }
            return Ok(0);
        }

        match build_beads_status_data(&speck, &name, &file, root_id, &beads_cli) {
            Ok((status_data, details_map)) => {
                if json_output {
                    let response = JsonResponse::ok("status", status_data);
                    println!("{}", serde_json::to_string_pretty(&response).unwrap());
                } else {
                    output_beads_text(&status_data, &speck, full, &details_map);
                }
                Ok(0)
            }
            Err(e) => {
                eprintln!("warning: beads query failed ({}), falling back to checkbox mode", e);
                // Fall back to checkbox mode
                let status_data = build_checkbox_status_data(&speck, &name);
                if json_output {
                    let response = JsonResponse::ok("status", status_data);
                    println!("{}", serde_json::to_string_pretty(&response).unwrap());
                } else {
                    output_text(&status_data, &speck, verbose);
                }
                Ok(0)
            }
        }
    } else {
        // No beads_root_id, use checkbox mode
        let status_data = build_checkbox_status_data(&speck, &name);
        if json_output {
            let response = JsonResponse::ok("status", status_data);
            println!("{}", serde_json::to_string_pretty(&response).unwrap());
        } else {
            output_text(&status_data, &speck, verbose);
        }
        Ok(0)
    }
}

/// Resolve a file path relative to the project
fn resolve_file_path(project_root: &Path, file: &str) -> PathBuf {
    let path = Path::new(file);
    if path.is_absolute() {
        path.to_path_buf()
    } else if file.starts_with(".specks/") || file.starts_with(".specks\\") {
        project_root.join(file)
    } else if file.starts_with("specks-") || file.ends_with(".md") {
        // Assume it's in .specks/
        let filename = if file.starts_with("specks-") && file.ends_with(".md") {
            file.to_string()
        } else if file.starts_with("specks-") {
            format!("{}.md", file)
        } else {
            format!("specks-{}.md", file)
        };
        project_root.join(".specks").join(filename)
    } else {
        // Try as-is first
        let as_is = project_root.join(file);
        if as_is.exists() {
            as_is
        } else {
            // Try in .specks/ with prefix
            project_root
                .join(".specks")
                .join(format!("specks-{}.md", file))
        }
    }
}

/// Build status data from a parsed speck using checkbox counting (fallback mode)
fn build_checkbox_status_data(speck: &Speck, name: &str) -> StatusData {
    let mut total_done = 0;
    let mut total_items = 0;

    let steps: Vec<StepStatus> = speck
        .steps
        .iter()
        .map(|step| {
            let step_done = step.completed_items();
            let step_total = step.total_items();
            total_done += step_done;
            total_items += step_total;

            let substeps: Vec<SubstepStatus> = step
                .substeps
                .iter()
                .map(|substep| {
                    let sub_done = substep.completed_items();
                    let sub_total = substep.total_items();
                    total_done += sub_done;
                    total_items += sub_total;

                    SubstepStatus {
                        title: substep.title.clone(),
                        anchor: format!("#{}", substep.anchor),
                        done: sub_done,
                        total: sub_total,
                    }
                })
                .collect();

            StepStatus {
                title: step.title.clone(),
                anchor: format!("#{}", step.anchor),
                done: step_done,
                total: step_total,
                substeps,
            }
        })
        .collect();

    let status = speck
        .metadata
        .status
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    // Build extended fields
    let all_steps: Vec<StepInfo> = speck
        .steps
        .iter()
        .map(|step| StepInfo {
            anchor: format!("#{}", step.anchor),
            title: step.title.clone(),
            number: step.number.clone(),
            bead_id: step.bead_id.clone(),
        })
        .collect();

    // A step is complete if all its items are checked (both tasks and substeps)
    let completed_steps: Vec<StepInfo> = speck
        .steps
        .iter()
        .filter(|step| {
            let step_done = step.completed_items();
            let step_total = step.total_items();
            step_total > 0 && step_done == step_total
        })
        .map(|step| StepInfo {
            anchor: format!("#{}", step.anchor),
            title: step.title.clone(),
            number: step.number.clone(),
            bead_id: step.bead_id.clone(),
        })
        .collect();

    // Remaining steps are those not in completed_steps
    let completed_anchors: std::collections::HashSet<String> =
        completed_steps.iter().map(|s| s.anchor.clone()).collect();

    let remaining_steps: Vec<StepInfo> = all_steps
        .iter()
        .filter(|step| !completed_anchors.contains(&step.anchor))
        .cloned()
        .collect();

    // Next step is the first remaining step
    let next_step = remaining_steps.first().cloned();

    // Build bead mapping (only include steps with bead_id)
    let bead_mapping: HashMap<String, String> = speck
        .steps
        .iter()
        .filter_map(|step| {
            step.bead_id
                .as_ref()
                .map(|bead_id| (format!("#{}", step.anchor), bead_id.clone()))
        })
        .collect();

    // Build dependencies mapping
    let dependencies: HashMap<String, Vec<String>> = speck
        .steps
        .iter()
        .map(|step| {
            let deps = step
                .depends_on
                .iter()
                .map(|dep| format!("#{}", dep))
                .collect();
            (format!("#{}", step.anchor), deps)
        })
        .collect();

    StatusData {
        name: name.to_string(),
        status,
        progress: Progress {
            done: total_done,
            total: total_items,
        },
        steps,
        all_steps: Some(all_steps),
        completed_steps: Some(completed_steps),
        remaining_steps: Some(remaining_steps),
        next_step,
        bead_mapping: Some(bead_mapping),
        dependencies: Some(dependencies),
        mode: Some("checkbox".to_string()),
        speck: None,
        phase_title: None,
        total_step_count: None,
        completed_step_count: None,
        ready_step_count: None,
        blocked_step_count: None,
        bead_steps: None,
    }
}

/// Classify steps based on bead data (pure function for testability)
fn classify_steps(
    speck: &Speck,
    children: &[IssueDetails],
    ready_ids: &HashSet<String>,
) -> (Vec<BeadStepStatus>, HashMap<String, IssueDetails>) {
    // Build bead_id -> IssueDetails map
    let bead_id_to_details: HashMap<String, &IssueDetails> =
        children.iter().map(|d| (d.id.clone(), d)).collect();

    // Build bead_id -> step anchor map (for blocked_by resolution)
    let bead_id_to_anchor: HashMap<String, String> = speck
        .steps
        .iter()
        .filter_map(|step| {
            step.bead_id
                .as_ref()
                .map(|bead_id| (bead_id.clone(), format!("#{}", step.anchor)))
        })
        .collect();

    // Build a map of bead ID to full IssueDetails for --full rendering
    let mut details_map: HashMap<String, IssueDetails> = HashMap::new();

    let bead_steps: Vec<BeadStepStatus> = speck
        .steps
        .iter()
        .map(|step| {
            let anchor = format!("#{}", step.anchor);
            let title = step.title.clone();
            let number = step.number.clone();

            match &step.bead_id {
                None => BeadStepStatus {
                    anchor,
                    title,
                    number,
                    bead_status: Some("pending".to_string()),
                    bead_id: None,
                    commit_hash: None,
                    commit_summary: None,
                    close_reason: None,
                    task_count: None,
                    test_count: None,
                    checkpoint_count: None,
                    blocked_by: None,
                },
                Some(bead_id) => {
                    if let Some(&details) = bead_id_to_details.get(bead_id) {
                        // Store full details for --full rendering
                        details_map.insert(anchor.clone(), details.clone());

                        if details.status == "closed" {
                            // Complete step
                            let parsed = details
                                .close_reason
                                .as_ref()
                                .map(|r| parse_close_reason(r))
                                .unwrap_or_else(|| parse_close_reason(""));

                            BeadStepStatus {
                                anchor,
                                title,
                                number,
                                bead_status: Some("complete".to_string()),
                                bead_id: Some(bead_id.clone()),
                                commit_hash: parsed.commit_hash,
                                commit_summary: parsed.commit_summary,
                                close_reason: Some(parsed.raw),
                                task_count: None,
                                test_count: None,
                                checkpoint_count: None,
                                blocked_by: None,
                            }
                        } else if ready_ids.contains(bead_id) {
                            // Ready step
                            BeadStepStatus {
                                anchor,
                                title,
                                number,
                                bead_status: Some("ready".to_string()),
                                bead_id: Some(bead_id.clone()),
                                commit_hash: None,
                                commit_summary: None,
                                close_reason: None,
                                task_count: Some(step.tasks.len()),
                                test_count: Some(step.tests.len()),
                                checkpoint_count: Some(step.checkpoints.len()),
                                blocked_by: None,
                            }
                        } else {
                            // Blocked step
                            // Compute blocked_by: find dependencies that are not closed
                            let blocked_by: Vec<String> = details
                                .dependencies
                                .iter()
                                .filter_map(|dep| {
                                    // Check if this dependency is still open
                                    if let Some(&dep_details) = bead_id_to_details.get(&dep.id) {
                                        if dep_details.status != "closed" {
                                            // Resolve to step anchor
                                            bead_id_to_anchor.get(&dep.id).cloned()
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                })
                                .collect();

                            BeadStepStatus {
                                anchor,
                                title,
                                number,
                                bead_status: Some("blocked".to_string()),
                                bead_id: Some(bead_id.clone()),
                                commit_hash: None,
                                commit_summary: None,
                                close_reason: None,
                                task_count: None,
                                test_count: None,
                                checkpoint_count: None,
                                blocked_by: if blocked_by.is_empty() {
                                    None
                                } else {
                                    Some(blocked_by)
                                },
                            }
                        }
                    } else {
                        // Bead ID present but not in children (edge case)
                        BeadStepStatus {
                            anchor,
                            title,
                            number,
                            bead_status: Some("pending".to_string()),
                            bead_id: Some(bead_id.clone()),
                            commit_hash: None,
                            commit_summary: None,
                            close_reason: None,
                            task_count: None,
                            test_count: None,
                            checkpoint_count: None,
                            blocked_by: None,
                        }
                    }
                }
            }
        })
        .collect();

    (bead_steps, details_map)
}

/// Build status data using beads integration
fn build_beads_status_data(
    speck: &Speck,
    name: &str,
    file_path: &str,
    root_id: &str,
    beads_cli: &BeadsCli,
) -> Result<(StatusData, HashMap<String, IssueDetails>), String> {
    // Query all child beads with details
    let children = beads_cli
        .list_children_detailed(root_id, None)
        .map_err(|e| format!("failed to query bead children: {}", e))?;

    // Query ready beads
    let ready_beads = beads_cli
        .ready(Some(root_id), None)
        .map_err(|e| format!("failed to query ready beads: {}", e))?;

    let ready_ids: HashSet<String> = ready_beads.iter().map(|b| b.id.clone()).collect();

    // Classify steps
    let (bead_steps, details_map) = classify_steps(speck, &children, &ready_ids);

    // Count step statuses
    let completed_count = bead_steps
        .iter()
        .filter(|s| s.bead_status.as_deref() == Some("complete"))
        .count();
    let ready_count = bead_steps
        .iter()
        .filter(|s| s.bead_status.as_deref() == Some("ready"))
        .count();
    let blocked_count = bead_steps
        .iter()
        .filter(|s| s.bead_status.as_deref() == Some("blocked"))
        .count();

    let status = speck
        .metadata
        .status
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    let status_data = StatusData {
        name: name.to_string(),
        status,
        progress: Progress {
            done: completed_count,
            total: speck.steps.len(),
        },
        steps: vec![], // Empty in beads mode
        all_steps: None,
        completed_steps: None,
        remaining_steps: None,
        next_step: None,
        bead_mapping: None,
        dependencies: None,
        mode: Some("beads".to_string()),
        speck: Some(file_path.to_string()),
        phase_title: speck.phase_title.clone(),
        total_step_count: Some(speck.steps.len()),
        completed_step_count: Some(completed_count),
        ready_step_count: Some(ready_count),
        blocked_step_count: Some(blocked_count),
        bead_steps: Some(bead_steps),
    };

    Ok((status_data, details_map))
}

/// Output beads-mode status in text format
fn output_beads_text(
    data: &StatusData,
    _speck: &Speck,
    full: bool,
    details_map: &HashMap<String, IssueDetails>,
) {
    // Print phase title or speck name
    if let Some(ref phase_title) = data.phase_title {
        println!("## {}", phase_title);
    } else {
        println!("## {}", data.name);
    }
    println!();

    // Print summary
    let completed = data.completed_step_count.unwrap_or(0);
    let total = data.total_step_count.unwrap_or(0);
    println!("Status: {} | {}/{} steps complete", data.status, completed, total);
    println!();

    // Print each step
    if let Some(ref bead_steps) = data.bead_steps {
        for step in bead_steps {
            let indicator = match step.bead_status.as_deref() {
                Some("complete") => "[✓]",
                Some("ready") => "[...]",
                Some("blocked") => "[⏳]",
                _ => "[ ]",
            };

            let status_label = step.bead_status.as_deref().unwrap_or("pending");

            println!(
                "Step {}: {}   {} {}",
                step.number, step.title, indicator, status_label
            );

            // Show close reason for completed steps
            if let Some(ref close_reason) = step.close_reason {
                println!("  {}", close_reason);
            }

            // Show task/test/checkpoint counts for ready steps
            if step.bead_status.as_deref() == Some("ready") {
                let tasks = step.task_count.unwrap_or(0);
                let tests = step.test_count.unwrap_or(0);
                let checkpoints = step.checkpoint_count.unwrap_or(0);
                println!(
                    "  Tasks: {} | Tests: {} | Checkpoints: {}",
                    tasks, tests, checkpoints
                );
            }

            // Show blocked_by for blocked steps
            if let Some(ref blocked_by) = step.blocked_by {
                let blocked_str = blocked_by
                    .iter()
                    .map(|anchor| {
                        // Convert #step-N to "Step N"
                        anchor.trim_start_matches('#').replace("step-", "Step ")
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("  Blocked by: {}", blocked_str);
            }

            // --full mode: show raw bead field content
            if full {
                if let Some(details) = details_map.get(&step.anchor) {
                    if !details.description.is_empty() {
                        println!("  --- Description ---");
                        for line in details.description.lines() {
                            println!("  {}", line);
                        }
                    }
                    if let Some(ref design) = details.design {
                        if !design.is_empty() {
                            println!("  --- Design ---");
                            for line in design.lines() {
                                println!("  {}", line);
                            }
                        }
                    }
                    if let Some(ref acceptance) = details.acceptance_criteria {
                        if !acceptance.is_empty() {
                            println!("  --- Acceptance Criteria ---");
                            for line in acceptance.lines() {
                                println!("  {}", line);
                            }
                        }
                    }
                    if let Some(ref notes) = details.notes {
                        if !notes.is_empty() {
                            println!("  --- Notes ---");
                            for line in notes.lines() {
                                println!("  {}", line);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Output status in text format
fn output_text(data: &StatusData, speck: &Speck, verbose: bool) {
    // Calculate percentage
    let percentage = if data.progress.total > 0 {
        (data.progress.done as f64 / data.progress.total as f64 * 100.0) as usize
    } else {
        0
    };

    println!(
        "{}.md: {} ({}% complete)",
        data.name, data.status, percentage
    );
    println!();

    for (i, step) in data.steps.iter().enumerate() {
        let check = if step.total > 0 && step.done == step.total {
            "[x]"
        } else {
            "[ ]"
        };

        let progress = format!("{}/{}", step.done, step.total);
        println!(
            "Step {}: {:<40} {} {}",
            speck.steps[i].number, step.title, check, progress
        );

        // Show substeps
        for (j, substep) in step.substeps.iter().enumerate() {
            let sub_check = if substep.total > 0 && substep.done == substep.total {
                "[x]"
            } else {
                "[ ]"
            };
            let sub_progress = format!("{}/{}", substep.done, substep.total);
            println!(
                "  Step {}: {:<38} {} {}",
                speck.steps[i].substeps[j].number, substep.title, sub_check, sub_progress
            );
        }

        // Verbose mode: show individual tasks
        if verbose {
            let step_data = &speck.steps[i];
            if !step_data.tasks.is_empty() {
                println!("    Tasks:");
                for task in &step_data.tasks {
                    let check = if task.checked { "[x]" } else { "[ ]" };
                    println!("      {} {}", check, task.text);
                }
            }
            if !step_data.tests.is_empty() {
                println!("    Tests:");
                for test in &step_data.tests {
                    let check = if test.checked { "[x]" } else { "[ ]" };
                    println!("      {} {}", check, test.text);
                }
            }
            if !step_data.checkpoints.is_empty() {
                println!("    Checkpoints:");
                for checkpoint in &step_data.checkpoints {
                    let check = if checkpoint.checked { "[x]" } else { "[ ]" };
                    println!("      {} {}", check, checkpoint.text);
                }
            }
            if let Some(ref refs) = step_data.references {
                println!("    References: {}", refs);
            }
            println!();
        }
    }

    println!();
    println!(
        "Total: {}/{} tasks complete",
        data.progress.done, data.progress.total
    );
}
