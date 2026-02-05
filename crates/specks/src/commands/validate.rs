//! Implementation of the `specks validate` command (Spec S02)

use std::fs;
use std::path::{Path, PathBuf};

use specks_core::{
    Config, Severity, ValidationConfig, ValidationLevel, ValidationResult, find_project_root,
    find_specks, parse_speck, speck_name_from_path, validate_speck_with_config,
};

use crate::output::{JsonIssue, JsonResponse, ValidateData, ValidatedFile};

/// Run the validate command
pub fn run_validate(
    file: Option<String>,
    strict: bool,
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
                let response: JsonResponse<ValidateData> =
                    JsonResponse::error("validate", ValidateData { files: vec![] }, issues);
                println!("{}", serde_json::to_string_pretty(&response).unwrap());
            } else {
                eprintln!("error: {}", message);
            }
            return Ok(9); // E009 exit code
        }
    };

    // Load configuration
    let config = Config::load_from_project(&project_root).unwrap_or_default();

    // Determine validation level
    let level = if strict {
        ValidationLevel::Strict
    } else {
        ValidationLevel::parse(&config.specks.validation_level)
    };

    let validation_config = ValidationConfig {
        level,
        ..Default::default()
    };

    // Get files to validate
    let files_to_validate = match file {
        Some(f) => {
            // Single file validation
            let path = resolve_file_path(&project_root, &f);
            if !path.exists() {
                let message = format!("file not found: {}", f);
                if json_output {
                    let issues = vec![JsonIssue {
                        code: "E002".to_string(),
                        severity: "error".to_string(),
                        message: message.clone(),
                        file: Some(f),
                        line: None,
                        anchor: None,
                    }];
                    let response: JsonResponse<ValidateData> =
                        JsonResponse::error("validate", ValidateData { files: vec![] }, issues);
                    println!("{}", serde_json::to_string_pretty(&response).unwrap());
                } else {
                    eprintln!("error: {}", message);
                }
                return Ok(2); // File not found exit code
            }
            vec![path]
        }
        None => {
            // Validate all specks
            match find_specks(&project_root) {
                Ok(specks) => specks,
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
                        let response: JsonResponse<ValidateData> =
                            JsonResponse::error("validate", ValidateData { files: vec![] }, issues);
                        println!("{}", serde_json::to_string_pretty(&response).unwrap());
                    } else {
                        eprintln!("error: {}", message);
                    }
                    return Ok(9);
                }
            }
        }
    };

    if files_to_validate.is_empty() {
        if json_output {
            let response = JsonResponse::ok("validate", ValidateData { files: vec![] });
            println!("{}", serde_json::to_string_pretty(&response).unwrap());
        } else if !quiet {
            println!("No speck files found to validate");
        }
        return Ok(0);
    }

    let mut all_results: Vec<(PathBuf, ValidationResult)> = Vec::new();
    let mut has_errors = false;

    for path in &files_to_validate {
        let result = validate_file(path, &validation_config);
        if !result.valid {
            has_errors = true;
        }
        all_results.push((path.clone(), result));
    }

    if json_output {
        output_json(&project_root, &all_results, has_errors);
    } else if !quiet {
        output_text(&project_root, &all_results);
    }

    Ok(if has_errors { 1 } else { 0 })
}

/// Resolve a file path relative to the project
fn resolve_file_path(project_root: &Path, file: &str) -> PathBuf {
    let path = Path::new(file);
    if path.is_absolute() {
        path.to_path_buf()
    } else if file.starts_with(".specks/") || file.starts_with(".specks\\") {
        project_root.join(file)
    } else if file.starts_with("specks-") {
        // Assume it's in .specks/
        project_root.join(".specks").join(file)
    } else {
        // Try as-is first
        let as_is = project_root.join(file);
        if as_is.exists() {
            as_is
        } else {
            // Try in .specks/
            project_root.join(".specks").join(file)
        }
    }
}

/// Validate a single file
fn validate_file(path: &Path, config: &ValidationConfig) -> ValidationResult {
    // Read the file
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            let mut result = ValidationResult::new();
            result.add_issue(specks_core::ValidationIssue::new(
                "E002",
                Severity::Error,
                format!("failed to read file: {}", e),
            ));
            return result;
        }
    };

    // Parse the speck
    let speck = match parse_speck(&content) {
        Ok(mut s) => {
            s.path = path.to_str().map(|s| s.to_string());
            s
        }
        Err(e) => {
            let mut result = ValidationResult::new();
            result.add_issue(specks_core::ValidationIssue::new(
                "E001",
                Severity::Error,
                format!("failed to parse speck: {}", e),
            ));
            return result;
        }
    };

    // Validate
    validate_speck_with_config(&speck, config)
}

/// Output results in JSON format
fn output_json(project_root: &Path, results: &[(PathBuf, ValidationResult)], has_errors: bool) {
    let mut files = Vec::new();
    let mut all_issues = Vec::new();

    for (path, result) in results {
        let relative_path = make_relative_path(project_root, path);

        files.push(ValidatedFile {
            path: relative_path.clone(),
            valid: result.valid,
            error_count: result.error_count(),
            warning_count: result.warning_count(),
        });

        for issue in &result.issues {
            all_issues.push(JsonIssue::from(issue).with_file(&relative_path));
        }
    }

    let response = if has_errors {
        JsonResponse::error("validate", ValidateData { files }, all_issues)
    } else {
        JsonResponse::ok_with_issues("validate", ValidateData { files }, all_issues)
    };

    println!("{}", serde_json::to_string_pretty(&response).unwrap());
}

/// Output results in text format
fn output_text(project_root: &Path, results: &[(PathBuf, ValidationResult)]) {
    for (path, result) in results {
        let relative_path = make_relative_path(project_root, path);
        let name = speck_name_from_path(path).unwrap_or_else(|| relative_path.clone());

        let error_count = result.error_count();
        let warning_count = result.warning_count();

        if result.valid && warning_count == 0 {
            println!("{}: valid", name);
        } else {
            println!(
                "{}: {} error{}, {} warning{}",
                name,
                error_count,
                if error_count == 1 { "" } else { "s" },
                warning_count,
                if warning_count == 1 { "" } else { "s" }
            );
        }

        // Group issues by severity
        let errors: Vec<_> = result
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .collect();
        let warnings: Vec<_> = result
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .collect();
        let infos: Vec<_> = result
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Info)
            .collect();

        if !errors.is_empty() {
            println!("\nErrors:");
            for issue in errors {
                print_issue(issue);
            }
        }

        if !warnings.is_empty() {
            println!("\nWarnings:");
            for issue in warnings {
                print_issue(issue);
            }
        }

        if !infos.is_empty() {
            println!("\nInfo:");
            for issue in infos {
                print_issue(issue);
            }
        }

        println!();
    }
}

/// Print a single issue
fn print_issue(issue: &specks_core::ValidationIssue) {
    let line_info = issue
        .line
        .map(|l| format!("Line {}: ", l))
        .unwrap_or_default();
    println!("  {}{}", line_info, issue.message);
}

/// Make a path relative to the project root using forward slashes
fn make_relative_path(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
