//! Implementation of the `specks setup` command
//!
//! Handles installation and verification of Claude Code skills.
//! Skills can be installed to:
//! - Per-project: `.claude/skills/` in the current project
//! - Global: `~/.claude/skills/` in the user's home directory

use std::path::Path;

use crate::output::{JsonIssue, JsonResponse, SetupData, SkillInfo};
use crate::share::{
    SkillInstallStatus, SkillVerifyStatus, find_share_dir, get_global_skills_dir,
    install_all_skills, install_all_skills_globally, verify_all_skills, verify_all_skills_globally,
};

/// Run the setup claude command.
///
/// If `global` is true, installs to ~/.claude/skills/ instead of per-project.
/// If `check` is true, verifies installation status without installing.
/// If `force` is true, overwrites existing skills even if they match.
pub fn run_setup_claude(
    global: bool,
    check: bool,
    force: bool,
    json_output: bool,
    quiet: bool,
) -> Result<i32, String> {
    if global {
        run_setup_claude_global(check, force, json_output, quiet)
    } else {
        // Get current directory as project directory
        let project_dir = std::env::current_dir()
            .map_err(|e| format!("failed to get current directory: {}", e))?;
        run_setup_claude_impl(check, force, json_output, quiet, &project_dir)
    }
}

/// Run the setup command for global installation (~/.claude/skills/).
fn run_setup_claude_global(
    check: bool,
    force: bool,
    json_output: bool,
    quiet: bool,
) -> Result<i32, String> {
    // Find share directory
    let share_dir = find_share_dir();

    if check {
        run_check_mode_global(&share_dir, json_output, quiet)
    } else {
        run_install_mode_global(&share_dir, force, json_output, quiet)
    }
}

/// Internal implementation that accepts a project directory.
/// This allows testing without changing the global current directory.
fn run_setup_claude_impl(
    check: bool,
    force: bool,
    json_output: bool,
    quiet: bool,
    project_dir: &Path,
) -> Result<i32, String> {
    // Find share directory
    let share_dir = find_share_dir();

    if check {
        // Verification mode - just check status
        run_check_mode(&share_dir, project_dir, json_output, quiet)
    } else {
        // Installation mode
        run_install_mode(&share_dir, project_dir, force, json_output, quiet)
    }
}

/// Output the share directory not found error.
fn output_share_dir_not_found(action: &str, json_output: bool, global: bool) {
    let location = if global {
        "~/.claude/skills/"
    } else {
        ".claude/skills/"
    };

    if json_output {
        let issues = vec![JsonIssue {
            code: "E025".to_string(),
            severity: "error".to_string(),
            message: "Skills not found: share directory not found".to_string(),
            file: None,
            line: None,
            anchor: None,
        }];
        let response: JsonResponse<SetupData> = JsonResponse::error(
            "setup",
            SetupData {
                subcommand: "claude".to_string(),
                action: action.to_string(),
                share_dir: None,
                target_dir: Some(location.to_string()),
                skills_installed: vec![],
            },
            issues,
        );
        println!("{}", serde_json::to_string_pretty(&response).unwrap());
    } else {
        eprintln!("error: E025: Skills not found: share directory not found");
        eprintln!();
        eprintln!(
            "If installed via Homebrew, skills should be at /opt/homebrew/share/specks/skills/"
        );
        eprintln!(
            "You can set SPECKS_SHARE_DIR environment variable to specify a custom location."
        );
    }
}

/// Run in check mode - verify skill installation status without installing (per-project).
fn run_check_mode(
    share_dir: &Option<std::path::PathBuf>,
    project_dir: &Path,
    json_output: bool,
    quiet: bool,
) -> Result<i32, String> {
    let Some(share_dir) = share_dir else {
        output_share_dir_not_found("check", json_output, false);
        return Ok(7);
    };

    let results = verify_all_skills(share_dir, project_dir);

    let skills_info: Vec<SkillInfo> = results
        .iter()
        .map(|(name, status)| SkillInfo {
            name: name.clone(),
            path: format!(".claude/skills/{}/SKILL.md", name),
            status: status.as_str().to_string(),
        })
        .collect();

    // Check if all skills are up to date
    let all_up_to_date = results
        .iter()
        .all(|(_, status)| *status == SkillVerifyStatus::UpToDate);

    if json_output {
        let response = JsonResponse::ok(
            "setup",
            SetupData {
                subcommand: "claude".to_string(),
                action: "check".to_string(),
                share_dir: Some(share_dir.display().to_string()),
                target_dir: Some(".claude/skills/".to_string()),
                skills_installed: skills_info,
            },
        );
        println!("{}", serde_json::to_string_pretty(&response).unwrap());
    } else if !quiet {
        println!("Claude Code skills status (per-project):");
        println!("  Share directory: {}", share_dir.display());
        println!("  Target: .claude/skills/");
        println!();

        for (name, status) in &results {
            let status_str = match status {
                SkillVerifyStatus::UpToDate => "✓ up to date",
                SkillVerifyStatus::Outdated => "⚠ outdated",
                SkillVerifyStatus::Missing => "✗ missing",
                SkillVerifyStatus::SourceMissing => "? source missing",
            };
            println!("  {}: {}", name, status_str);
        }

        if !all_up_to_date {
            println!();
            println!("Run `specks setup claude` to install/update skills.");
            println!("Run `specks setup claude --force` to overwrite all skills.");
        }
    }

    Ok(0)
}

/// Run in check mode - verify skill installation status without installing (global).
fn run_check_mode_global(
    share_dir: &Option<std::path::PathBuf>,
    json_output: bool,
    quiet: bool,
) -> Result<i32, String> {
    let Some(share_dir) = share_dir else {
        output_share_dir_not_found("check", json_output, true);
        return Ok(7);
    };

    let global_dir = get_global_skills_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "~/.claude/skills/".to_string());

    let results = verify_all_skills_globally(share_dir);

    let skills_info: Vec<SkillInfo> = results
        .iter()
        .map(|(name, status)| SkillInfo {
            name: name.clone(),
            path: format!("~/.claude/skills/{}/SKILL.md", name),
            status: status.as_str().to_string(),
        })
        .collect();

    // Check if all skills are up to date
    let all_up_to_date = results
        .iter()
        .all(|(_, status)| *status == SkillVerifyStatus::UpToDate);

    if json_output {
        let response = JsonResponse::ok(
            "setup",
            SetupData {
                subcommand: "claude".to_string(),
                action: "check".to_string(),
                share_dir: Some(share_dir.display().to_string()),
                target_dir: Some(global_dir),
                skills_installed: skills_info,
            },
        );
        println!("{}", serde_json::to_string_pretty(&response).unwrap());
    } else if !quiet {
        let target_display = get_global_skills_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "~/.claude/skills/".to_string());

        println!("Claude Code skills status (global):");
        println!("  Share directory: {}", share_dir.display());
        println!("  Target: {}", target_display);
        println!();

        for (name, status) in &results {
            let status_str = match status {
                SkillVerifyStatus::UpToDate => "✓ up to date",
                SkillVerifyStatus::Outdated => "⚠ outdated",
                SkillVerifyStatus::Missing => "✗ missing",
                SkillVerifyStatus::SourceMissing => "? source missing",
            };
            println!("  {}: {}", name, status_str);
        }

        if !all_up_to_date {
            println!();
            println!("Run `specks setup claude --global` to install/update skills.");
            println!("Run `specks setup claude --global --force` to overwrite all skills.");
        }
    }

    Ok(0)
}

/// Run in install mode - install skills to project.
fn run_install_mode(
    share_dir: &Option<std::path::PathBuf>,
    project_dir: &Path,
    force: bool,
    json_output: bool,
    quiet: bool,
) -> Result<i32, String> {
    let Some(share_dir) = share_dir else {
        output_share_dir_not_found("install", json_output, false);
        return Ok(7);
    };

    let results = install_all_skills(share_dir, project_dir, force);

    let mut skills_info: Vec<SkillInfo> = vec![];
    let mut had_errors = false;

    for (name, status_result) in &results {
        match status_result {
            Ok(status) => {
                skills_info.push(SkillInfo {
                    name: name.clone(),
                    path: format!(".claude/skills/{}/SKILL.md", name),
                    status: status.as_str().to_string(),
                });
            }
            Err(e) => {
                had_errors = true;
                skills_info.push(SkillInfo {
                    name: name.clone(),
                    path: format!(".claude/skills/{}/SKILL.md", name),
                    status: format!("error: {}", e),
                });
            }
        }
    }

    if json_output {
        let response = if had_errors {
            let issues = skills_info
                .iter()
                .filter(|s| s.status.starts_with("error:"))
                .map(|s| JsonIssue {
                    code: "E025".to_string(),
                    severity: "error".to_string(),
                    message: format!("Failed to install skill {}: {}", s.name, s.status),
                    file: Some(s.path.clone()),
                    line: None,
                    anchor: None,
                })
                .collect();

            JsonResponse::error(
                "setup",
                SetupData {
                    subcommand: "claude".to_string(),
                    action: "install".to_string(),
                    share_dir: Some(share_dir.display().to_string()),
                    target_dir: Some(".claude/skills/".to_string()),
                    skills_installed: skills_info,
                },
                issues,
            )
        } else {
            JsonResponse::ok(
                "setup",
                SetupData {
                    subcommand: "claude".to_string(),
                    action: "install".to_string(),
                    share_dir: Some(share_dir.display().to_string()),
                    target_dir: Some(".claude/skills/".to_string()),
                    skills_installed: skills_info,
                },
            )
        };
        println!("{}", serde_json::to_string_pretty(&response).unwrap());
    } else if !quiet {
        println!("Installing Claude Code skills (per-project):");
        println!("  Share directory: {}", share_dir.display());
        println!("  Target: .claude/skills/");
        println!();

        for (name, status_result) in &results {
            match status_result {
                Ok(status) => {
                    let status_str = match status {
                        SkillInstallStatus::Installed => "installed",
                        SkillInstallStatus::Updated => "updated",
                        SkillInstallStatus::Unchanged => "unchanged",
                        SkillInstallStatus::SourceMissing => "skipped (source missing)",
                    };
                    println!("  {}: {}", name, status_str);
                }
                Err(e) => {
                    println!("  {}: error: {}", name, e);
                }
            }
        }

        // Summary message
        let installed_count = results
            .iter()
            .filter(|(_, r)| matches!(r, Ok(SkillInstallStatus::Installed)))
            .count();
        let updated_count = results
            .iter()
            .filter(|(_, r)| matches!(r, Ok(SkillInstallStatus::Updated)))
            .count();
        let unchanged_count = results
            .iter()
            .filter(|(_, r)| matches!(r, Ok(SkillInstallStatus::Unchanged)))
            .count();

        if installed_count > 0 || updated_count > 0 {
            println!();
            if installed_count > 0 {
                println!(
                    "Skills are now available at .claude/skills/ for use with /specks-plan and /specks-execute"
                );
            }
        } else if unchanged_count == results.len() {
            println!();
            println!("All skills already up to date.");
        }
    }

    if had_errors { Ok(1) } else { Ok(0) }
}

/// Run in install mode - install skills globally (~/.claude/skills/).
fn run_install_mode_global(
    share_dir: &Option<std::path::PathBuf>,
    force: bool,
    json_output: bool,
    quiet: bool,
) -> Result<i32, String> {
    let Some(share_dir) = share_dir else {
        output_share_dir_not_found("install", json_output, true);
        return Ok(7);
    };

    let global_dir = get_global_skills_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "~/.claude/skills/".to_string());

    let results = install_all_skills_globally(share_dir, force);

    let mut skills_info: Vec<SkillInfo> = vec![];
    let mut had_errors = false;

    for (name, status_result) in &results {
        match status_result {
            Ok(status) => {
                skills_info.push(SkillInfo {
                    name: name.clone(),
                    path: format!("~/.claude/skills/{}/SKILL.md", name),
                    status: status.as_str().to_string(),
                });
            }
            Err(e) => {
                had_errors = true;
                skills_info.push(SkillInfo {
                    name: name.clone(),
                    path: format!("~/.claude/skills/{}/SKILL.md", name),
                    status: format!("error: {}", e),
                });
            }
        }
    }

    if json_output {
        let response = if had_errors {
            let issues = skills_info
                .iter()
                .filter(|s| s.status.starts_with("error:"))
                .map(|s| JsonIssue {
                    code: "E025".to_string(),
                    severity: "error".to_string(),
                    message: format!("Failed to install skill {}: {}", s.name, s.status),
                    file: Some(s.path.clone()),
                    line: None,
                    anchor: None,
                })
                .collect();

            JsonResponse::error(
                "setup",
                SetupData {
                    subcommand: "claude".to_string(),
                    action: "install".to_string(),
                    share_dir: Some(share_dir.display().to_string()),
                    target_dir: Some(global_dir),
                    skills_installed: skills_info,
                },
                issues,
            )
        } else {
            JsonResponse::ok(
                "setup",
                SetupData {
                    subcommand: "claude".to_string(),
                    action: "install".to_string(),
                    share_dir: Some(share_dir.display().to_string()),
                    target_dir: Some(global_dir),
                    skills_installed: skills_info,
                },
            )
        };
        println!("{}", serde_json::to_string_pretty(&response).unwrap());
    } else if !quiet {
        let target_display = get_global_skills_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "~/.claude/skills/".to_string());

        println!("Installing Claude Code skills (global):");
        println!("  Share directory: {}", share_dir.display());
        println!("  Target: {}", target_display);
        println!();

        for (name, status_result) in &results {
            match status_result {
                Ok(status) => {
                    let status_str = match status {
                        SkillInstallStatus::Installed => "installed",
                        SkillInstallStatus::Updated => "updated",
                        SkillInstallStatus::Unchanged => "unchanged",
                        SkillInstallStatus::SourceMissing => "skipped (source missing)",
                    };
                    println!("  {}: {}", name, status_str);
                }
                Err(e) => {
                    println!("  {}: error: {}", name, e);
                }
            }
        }

        // Summary message
        let installed_count = results
            .iter()
            .filter(|(_, r)| matches!(r, Ok(SkillInstallStatus::Installed)))
            .count();
        let updated_count = results
            .iter()
            .filter(|(_, r)| matches!(r, Ok(SkillInstallStatus::Updated)))
            .count();
        let unchanged_count = results
            .iter()
            .filter(|(_, r)| matches!(r, Ok(SkillInstallStatus::Unchanged)))
            .count();

        if installed_count > 0 || updated_count > 0 {
            println!();
            if installed_count > 0 {
                println!(
                    "Skills are now available globally for use with /specks-plan and /specks-execute"
                );
            }
        } else if unchanged_count == results.len() {
            println!();
            println!("All skills already up to date.");
        }
    }

    if had_errors { Ok(1) } else { Ok(0) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_share_dir() -> TempDir {
        let temp = TempDir::new().unwrap();

        // Create skills directory structure
        let skills_dir = temp.path().join("skills");
        fs::create_dir_all(&skills_dir).unwrap();

        // Create specks-plan skill
        let plan_dir = skills_dir.join("specks-plan");
        fs::create_dir_all(&plan_dir).unwrap();
        fs::write(plan_dir.join("SKILL.md"), "# specks-plan skill").unwrap();

        // Create specks-execute skill
        let exec_dir = skills_dir.join("specks-execute");
        fs::create_dir_all(&exec_dir).unwrap();
        fs::write(exec_dir.join("SKILL.md"), "# specks-execute skill").unwrap();

        temp
    }

    #[test]
    fn test_check_mode_with_missing_skills() {
        let share = create_test_share_dir();
        let project = TempDir::new().unwrap();

        // Set share dir via env var
        // SAFETY: Tests run single-threaded for env var tests
        unsafe {
            std::env::set_var(crate::share::SHARE_DIR_ENV_VAR, share.path());
        }

        // Use the internal implementation directly to avoid changing current_dir
        // (which causes race conditions in parallel tests)
        let result = run_setup_claude_impl(true, false, false, true, project.path());

        unsafe {
            std::env::remove_var(crate::share::SHARE_DIR_ENV_VAR);
        }

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_install_mode() {
        let share = create_test_share_dir();
        let project = TempDir::new().unwrap();

        // Set share dir via env var
        // SAFETY: Tests run single-threaded for env var tests
        unsafe {
            std::env::set_var(crate::share::SHARE_DIR_ENV_VAR, share.path());
        }

        // Use the internal implementation directly to avoid changing current_dir
        let result = run_setup_claude_impl(false, false, false, true, project.path());

        unsafe {
            std::env::remove_var(crate::share::SHARE_DIR_ENV_VAR);
        }

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        // Verify skills were installed
        assert!(
            project
                .path()
                .join(".claude/skills/specks-plan/SKILL.md")
                .exists()
        );
        assert!(
            project
                .path()
                .join(".claude/skills/specks-execute/SKILL.md")
                .exists()
        );
    }

    #[test]
    fn test_install_mode_idempotent() {
        let share = create_test_share_dir();
        let project = TempDir::new().unwrap();

        // Set share dir via env var
        // SAFETY: Tests run single-threaded for env var tests
        unsafe {
            std::env::set_var(crate::share::SHARE_DIR_ENV_VAR, share.path());
        }

        // Use the internal implementation directly to avoid changing current_dir
        // First install
        let result1 = run_setup_claude_impl(false, false, false, true, project.path());
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), 0);

        // Second install should also succeed
        let result2 = run_setup_claude_impl(false, false, false, true, project.path());

        unsafe {
            std::env::remove_var(crate::share::SHARE_DIR_ENV_VAR);
        }

        assert!(result2.is_ok());
        assert_eq!(result2.unwrap(), 0);
    }
}
