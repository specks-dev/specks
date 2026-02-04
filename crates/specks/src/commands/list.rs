//! Implementation of the `specks list` command (Spec S03)

use std::fs;
use std::path::Path;

use specks_core::{find_project_root, find_specks, parse_speck, speck_name_from_path, Speck};

use crate::output::{JsonIssue, JsonResponse, ListData, Progress, SpeckSummary};

/// Run the list command
pub fn run_list(
    status_filter: Option<String>,
    json_output: bool,
    quiet: bool,
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
                let response: JsonResponse<ListData> = JsonResponse::error(
                    "list",
                    ListData { specks: vec![] },
                    issues,
                );
                println!("{}", serde_json::to_string_pretty(&response).unwrap());
            } else {
                eprintln!("error: {}", message);
            }
            return Ok(9); // E009 exit code
        }
    };

    // Find all speck files
    let speck_files = match find_specks(&project_root) {
        Ok(files) => files,
        Err(e) => {
            let message = format!("failed to find specks: {}", e);
            if json_output {
                let issues = vec![JsonIssue {
                    code: "E009".to_string(),
                    severity: "error".to_string(),
                    message: message.clone(),
                    file: None,
                    line: None,
                    anchor: None,
                }];
                let response: JsonResponse<ListData> = JsonResponse::error(
                    "list",
                    ListData { specks: vec![] },
                    issues,
                );
                println!("{}", serde_json::to_string_pretty(&response).unwrap());
            } else {
                eprintln!("error: {}", message);
            }
            return Ok(9);
        }
    };

    // Parse and collect speck summaries
    let mut summaries: Vec<SpeckSummary> = Vec::new();

    for path in &speck_files {
        if let Some(summary) = parse_speck_summary(path) {
            // Apply status filter if specified
            if let Some(ref filter) = status_filter {
                if !summary.status.eq_ignore_ascii_case(filter) {
                    continue;
                }
            }
            summaries.push(summary);
        }
    }

    if json_output {
        let response = JsonResponse::ok("list", ListData { specks: summaries });
        println!("{}", serde_json::to_string_pretty(&response).unwrap());
    } else if !quiet {
        if summaries.is_empty() {
            println!("No specks found");
        } else {
            output_table(&summaries);
        }
    }

    Ok(0)
}

/// Parse a speck file and return a summary
fn parse_speck_summary(path: &Path) -> Option<SpeckSummary> {
    let content = fs::read_to_string(path).ok()?;
    let speck = parse_speck(&content).ok()?;
    let name = speck_name_from_path(path)?;

    let (done, total) = count_checkboxes(&speck);
    let status = speck
        .metadata
        .status
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    let updated = speck
        .metadata
        .last_updated
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    Some(SpeckSummary {
        name,
        status,
        progress: Progress { done, total },
        updated,
    })
}

/// Count completed and total checkboxes in execution steps
fn count_checkboxes(speck: &Speck) -> (usize, usize) {
    let mut done = 0;
    let mut total = 0;

    for step in &speck.steps {
        total += step.total_items();
        done += step.completed_items();

        // Count substeps
        for substep in &step.substeps {
            total += substep.total_items();
            done += substep.completed_items();
        }
    }

    (done, total)
}

/// Output a formatted table
fn output_table(summaries: &[SpeckSummary]) {
    // Calculate column widths
    let name_width = summaries
        .iter()
        .map(|s| s.name.len())
        .max()
        .unwrap_or(5)
        .max(5);
    let status_width = summaries
        .iter()
        .map(|s| s.status.len())
        .max()
        .unwrap_or(6)
        .max(6);

    // Print header
    println!(
        "{:<name_width$}  {:<status_width$}  {:>10}  {:>10}",
        "SPECK",
        "STATUS",
        "PROGRESS",
        "UPDATED",
        name_width = name_width,
        status_width = status_width
    );

    // Print rows
    for summary in summaries {
        let progress = format!("{}/{}", summary.progress.done, summary.progress.total);
        println!(
            "{:<name_width$}  {:<status_width$}  {:>10}  {:>10}",
            summary.name,
            summary.status,
            progress,
            summary.updated,
            name_width = name_width,
            status_width = status_width
        );
    }
}
