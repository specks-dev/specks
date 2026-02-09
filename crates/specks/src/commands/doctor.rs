//! Doctor command - health checks for specks project

use std::path::Path;

use crate::output::{DoctorData, DoctorSummary, HealthCheck, JsonResponse};

/// Exit codes per Table T02
const EXIT_PASS: u8 = 0;
const EXIT_WARN: u8 = 1;
const EXIT_FAIL: u8 = 2;

/// Log size thresholds per Table T02
const LOG_LINE_WARN: usize = 400;
const LOG_LINE_FAIL: usize = 500;
const LOG_BYTE_WARN: usize = 80 * 1024; // 80KB
const LOG_BYTE_FAIL: usize = 100 * 1024; // 100KB

/// Run the doctor command
pub fn run_doctor(json_output: bool, quiet: bool) -> Result<i32, String> {
    // Run all health checks
    let checks = vec![
        check_initialized(),
        check_log_size(),
        check_worktrees(),
        check_broken_refs(),
    ];

    // Calculate summary
    let passed = checks.iter().filter(|c| c.status == "pass").count();
    let warnings = checks.iter().filter(|c| c.status == "warn").count();
    let failures = checks.iter().filter(|c| c.status == "fail").count();

    let summary = DoctorSummary {
        passed,
        warnings,
        failures,
    };

    let data = DoctorData { checks, summary };

    // Determine exit code
    let exit_code = if failures > 0 {
        EXIT_FAIL as i32
    } else if warnings > 0 {
        EXIT_WARN as i32
    } else {
        EXIT_PASS as i32
    };

    // Output results
    if json_output {
        let response = if exit_code == EXIT_PASS as i32 {
            JsonResponse::ok("doctor", data)
        } else {
            JsonResponse::error("doctor", data, vec![])
        };
        println!("{}", serde_json::to_string_pretty(&response).unwrap());
    } else if !quiet {
        print_doctor_results(&data);
    }

    Ok(exit_code)
}

/// Print doctor results in text format
fn print_doctor_results(data: &DoctorData) {
    println!("Health Check Results:");
    println!();

    for check in &data.checks {
        let icon = match check.status.as_str() {
            "pass" => "✓",
            "warn" => "⚠",
            "fail" => "✗",
            _ => "?",
        };
        println!("  {} {} - {}", icon, check.name, check.message);
    }

    println!();
    println!(
        "Summary: {} passed, {} warnings, {} failures",
        data.summary.passed, data.summary.warnings, data.summary.failures
    );
}

/// Check if specks is initialized
fn check_initialized() -> HealthCheck {
    let specks_dir = Path::new(".specks");

    if !specks_dir.exists() {
        return HealthCheck {
            name: "initialized".to_string(),
            status: "fail".to_string(),
            message: "Specks is not initialized (.specks/ directory missing)".to_string(),
            details: None,
        };
    }

    // Check for required files
    let required_files = ["specks-skeleton.md", "config.toml"];
    let missing: Vec<_> = required_files
        .iter()
        .filter(|f| !specks_dir.join(f).exists())
        .collect();

    if !missing.is_empty() {
        return HealthCheck {
            name: "initialized".to_string(),
            status: "fail".to_string(),
            message: format!(
                "Specks directory missing required files: {}",
                missing
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            details: None,
        };
    }

    HealthCheck {
        name: "initialized".to_string(),
        status: "pass".to_string(),
        message: "Specks is initialized".to_string(),
        details: None,
    }
}

/// Check implementation log size
fn check_log_size() -> HealthCheck {
    let log_path = Path::new(".specks/specks-implementation-log.md");

    if !log_path.exists() {
        return HealthCheck {
            name: "log_size".to_string(),
            status: "pass".to_string(),
            message: "Implementation log not found (no history yet)".to_string(),
            details: None,
        };
    }

    // Read the file to get line count and byte size
    let content = match std::fs::read_to_string(log_path) {
        Ok(c) => c,
        Err(e) => {
            return HealthCheck {
                name: "log_size".to_string(),
                status: "fail".to_string(),
                message: format!("Failed to read implementation log: {}", e),
                details: None,
            }
        }
    };

    let lines = content.lines().count();
    let bytes = content.len();

    let details = serde_json::json!({
        "lines": lines,
        "bytes": bytes
    });

    // Determine status based on thresholds
    if lines > LOG_LINE_FAIL || bytes > LOG_BYTE_FAIL {
        HealthCheck {
            name: "log_size".to_string(),
            status: "fail".to_string(),
            message: format!(
                "Implementation log exceeds limits ({} lines, {} bytes)",
                lines, bytes
            ),
            details: Some(details),
        }
    } else if lines > LOG_LINE_WARN || bytes > LOG_BYTE_WARN {
        HealthCheck {
            name: "log_size".to_string(),
            status: "warn".to_string(),
            message: format!(
                "Implementation log approaching limits ({} lines, {} bytes)",
                lines, bytes
            ),
            details: Some(details),
        }
    } else {
        HealthCheck {
            name: "log_size".to_string(),
            status: "pass".to_string(),
            message: format!(
                "Implementation log size OK ({} lines, {} bytes)",
                lines, bytes
            ),
            details: Some(details),
        }
    }
}

/// Check worktree consistency
fn check_worktrees() -> HealthCheck {
    let worktrees_dir = Path::new(".specks-worktrees");

    if !worktrees_dir.exists() {
        return HealthCheck {
            name: "worktrees".to_string(),
            status: "pass".to_string(),
            message: "No worktrees directory (no implementations yet)".to_string(),
            details: None,
        };
    }

    // List all worktree directories
    let entries = match std::fs::read_dir(worktrees_dir) {
        Ok(e) => e,
        Err(e) => {
            return HealthCheck {
                name: "worktrees".to_string(),
                status: "fail".to_string(),
                message: format!("Failed to read worktrees directory: {}", e),
                details: None,
            }
        }
    };

    let mut valid_count = 0;
    let mut invalid_paths = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Check if path follows the pattern .specks-worktrees/specks__*
            let path_str = path.to_string_lossy();
            if !path_str.starts_with(".specks-worktrees/specks__") {
                invalid_paths.push(path_str.to_string());
            } else {
                valid_count += 1;
            }
        }
    }

    if !invalid_paths.is_empty() {
        let details = serde_json::json!({
            "invalid_paths": invalid_paths
        });
        HealthCheck {
            name: "worktrees".to_string(),
            status: "fail".to_string(),
            message: format!("{} invalid worktree path(s) found", invalid_paths.len()),
            details: Some(details),
        }
    } else if valid_count == 0 {
        HealthCheck {
            name: "worktrees".to_string(),
            status: "pass".to_string(),
            message: "No worktrees found".to_string(),
            details: None,
        }
    } else {
        HealthCheck {
            name: "worktrees".to_string(),
            status: "pass".to_string(),
            message: format!("{} worktree(s) found, all paths valid", valid_count),
            details: None,
        }
    }
}

/// Check for broken anchor references
fn check_broken_refs() -> HealthCheck {
    use specks_core::{parse_speck, validate_speck, Severity};

    let specks_dir = Path::new(".specks");
    if !specks_dir.exists() {
        return HealthCheck {
            name: "broken_refs".to_string(),
            status: "pass".to_string(),
            message: "No specks directory to check".to_string(),
            details: None,
        };
    }

    // Find all speck files
    let entries = match std::fs::read_dir(specks_dir) {
        Ok(e) => e,
        Err(e) => {
            return HealthCheck {
                name: "broken_refs".to_string(),
                status: "fail".to_string(),
                message: format!("Failed to read specks directory: {}", e),
                details: None,
            }
        }
    };

    let mut broken_refs = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            let filename = path.file_name().unwrap().to_string_lossy();
            // Skip skeleton, config, and log files
            if filename.starts_with("specks-")
                && filename.ends_with(".md")
                && filename != "specks-skeleton.md"
                && filename != "specks-implementation-log.md"
            {
                // Read and parse the speck
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        match parse_speck(&content) {
                            Ok(speck) => {
                                let result = validate_speck(&speck);
                                // Look for broken reference errors (E010)
                                for issue in &result.issues {
                                    if issue.code == "E010" && issue.severity == Severity::Error {
                                        broken_refs.push(format!(
                                            "{}: {}",
                                            filename,
                                            issue.message
                                        ));
                                    }
                                }
                            }
                            Err(_) => {
                                // Skip files that can't be parsed
                                continue;
                            }
                        }
                    }
                    Err(_) => {
                        // Skip files that can't be read
                        continue;
                    }
                }
            }
        }
    }

    if broken_refs.is_empty() {
        HealthCheck {
            name: "broken_refs".to_string(),
            status: "pass".to_string(),
            message: "No broken anchor references found".to_string(),
            details: None,
        }
    } else {
        let details = serde_json::json!({
            "refs": broken_refs
        });
        HealthCheck {
            name: "broken_refs".to_string(),
            status: "fail".to_string(),
            message: format!("{} broken anchor reference(s) found", broken_refs.len()),
            details: Some(details),
        }
    }
}
