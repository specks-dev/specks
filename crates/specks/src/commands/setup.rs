//! Implementation of the `specks setup` command
//!
//! Handles installation and verification of Claude Code skills.

use std::path::Path;

use crate::output::{JsonIssue, JsonResponse, SetupData, SkillInfo};
use crate::share::{
    find_share_dir, install_all_skills, verify_all_skills, SkillInstallStatus, SkillVerifyStatus,
};

/// Run the setup claude command.
///
/// If `check` is true, verifies installation status without installing.
/// If `force` is true, overwrites existing skills even if they match.
pub fn run_setup_claude(
    check: bool,
    force: bool,
    json_output: bool,
    quiet: bool,
) -> Result<i32, String> {
    // Find share directory
    let share_dir = find_share_dir();

    // Get current directory as project directory
    let project_dir = std::env::current_dir()
        .map_err(|e| format!("failed to get current directory: {}", e))?;

    if check {
        // Verification mode - just check status
        run_check_mode(&share_dir, &project_dir, json_output, quiet)
    } else {
        // Installation mode
        run_install_mode(&share_dir, &project_dir, force, json_output, quiet)
    }
}

/// Run in check mode - verify skill installation status without installing.
fn run_check_mode(
    share_dir: &Option<std::path::PathBuf>,
    project_dir: &Path,
    json_output: bool,
    quiet: bool,
) -> Result<i32, String> {
    let Some(share_dir) = share_dir else {
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
                    action: "check".to_string(),
                    share_dir: None,
                    skills_installed: vec![],
                },
                issues,
            );
            println!("{}", serde_json::to_string_pretty(&response).unwrap());
        } else {
            eprintln!("error: E025: Skills not found: share directory not found");
            eprintln!();
            eprintln!("If installed via Homebrew, skills should be at /opt/homebrew/share/specks/skills/");
            eprintln!("You can set SPECKS_SHARE_DIR environment variable to specify a custom location.");
        }
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
                skills_installed: skills_info,
            },
        );
        println!("{}", serde_json::to_string_pretty(&response).unwrap());
    } else if !quiet {
        println!("Claude Code skills status:");
        println!("  Share directory: {}", share_dir.display());
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

/// Run in install mode - install skills to project.
fn run_install_mode(
    share_dir: &Option<std::path::PathBuf>,
    project_dir: &Path,
    force: bool,
    json_output: bool,
    quiet: bool,
) -> Result<i32, String> {
    let Some(share_dir) = share_dir else {
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
                    action: "install".to_string(),
                    share_dir: None,
                    skills_installed: vec![],
                },
                issues,
            );
            println!("{}", serde_json::to_string_pretty(&response).unwrap());
        } else {
            eprintln!("error: E025: Skills not found: share directory not found");
            eprintln!();
            eprintln!("If installed via Homebrew, skills should be at /opt/homebrew/share/specks/skills/");
            eprintln!("You can set SPECKS_SHARE_DIR environment variable to specify a custom location.");
        }
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
                    skills_installed: skills_info,
                },
            )
        };
        println!("{}", serde_json::to_string_pretty(&response).unwrap());
    } else if !quiet {
        println!("Installing Claude Code skills:");
        println!("  Share directory: {}", share_dir.display());
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

    if had_errors {
        Ok(1)
    } else {
        Ok(0)
    }
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
        let saved_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(project.path()).unwrap();

        let result = run_setup_claude(true, false, false, true);

        // Restore
        std::env::set_current_dir(saved_dir).unwrap();
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
        let saved_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(project.path()).unwrap();

        let result = run_setup_claude(false, false, false, true);

        // Restore
        std::env::set_current_dir(saved_dir).unwrap();
        unsafe {
            std::env::remove_var(crate::share::SHARE_DIR_ENV_VAR);
        }

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        // Verify skills were installed
        assert!(project
            .path()
            .join(".claude/skills/specks-plan/SKILL.md")
            .exists());
        assert!(project
            .path()
            .join(".claude/skills/specks-execute/SKILL.md")
            .exists());
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
        let saved_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(project.path()).unwrap();

        // First install
        let result1 = run_setup_claude(false, false, false, true);
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), 0);

        // Second install should also succeed
        let result2 = run_setup_claude(false, false, false, true);

        // Restore
        std::env::set_current_dir(saved_dir).unwrap();
        unsafe {
            std::env::remove_var(crate::share::SHARE_DIR_ENV_VAR);
        }

        assert!(result2.is_ok());
        assert_eq!(result2.unwrap(), 0);
    }
}
