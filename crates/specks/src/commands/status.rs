//! Implementation of the `specks status` command (Spec S04)

use std::fs;
use std::path::{Path, PathBuf};

use specks_core::{Speck, find_project_root, parse_speck, speck_name_from_path};

use crate::output::{JsonIssue, JsonResponse, Progress, StatusData, StepStatus, SubstepStatus};

/// Run the status command
pub fn run_status(
    file: String,
    verbose: bool,
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
    let status_data = build_status_data(&speck, &name);

    if json_output {
        let response = JsonResponse::ok("status", status_data);
        println!("{}", serde_json::to_string_pretty(&response).unwrap());
    } else {
        output_text(&status_data, &speck, verbose);
    }

    Ok(0)
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

/// Build status data from a parsed speck
fn build_status_data(speck: &Speck, name: &str) -> StatusData {
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

    StatusData {
        name: name.to_string(),
        status,
        progress: Progress {
            done: total_done,
            total: total_items,
        },
        steps,
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
