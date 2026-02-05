//! Implementation of the `specks init` command (Spec S01)

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::output::{InitData, JsonIssue, JsonResponse};
use crate::share::{find_share_dir, install_all_skills, SkillInstallStatus};

/// Embedded skeleton content
const SKELETON_CONTENT: &str = include_str!("../../../../.specks/specks-skeleton.md");

/// Default config.toml content
const DEFAULT_CONFIG: &str = r#"[specks]
# Validation strictness: "lenient", "normal", "strict"
validation_level = "normal"

# Include info-level messages in validation output
show_info = false

[specks.naming]
# Speck file prefix (default: "specks-")
prefix = "specks-"

# Allowed name pattern (regex)
name_pattern = "^[a-z][a-z0-9-]{1,49}$"

[specks.beads]
# Enable beads integration
enabled = true

# Validate bead IDs when present
validate_bead_ids = true

# Path to beads CLI binary (default: "bd" on PATH)
bd_path = "bd"

# Sync behavior defaults (safe, non-destructive)
update_title = false
update_body = false
prune_deps = false

# Root bead type (epic recommended for bd ready --parent)
root_issue_type = "epic"

# Substep mapping: "none" (default) or "children"
substeps = "none"

# Pull behavior: which checkboxes to update when a bead is complete
# - "checkpoints": update only **Checkpoint:** items (default)
# - "all": update Tasks/Tests/Checkpoints
pull_checkbox_mode = "checkpoints"

# Warn when checkboxes and bead status disagree
pull_warn_on_conflict = true
"#;

/// Empty implementation log template
const IMPLEMENTATION_LOG_CONTENT: &str = r#"# Specks Implementation Log

This file documents the implementation progress for this project.

**Format:** Each entry records a completed step with tasks, files, and verification results.

Entries are sorted newest-first.

---

"#;

/// Run the init command
pub fn run_init(force: bool, json_output: bool, quiet: bool) -> Result<i32, String> {
    let specks_dir = Path::new(".specks");

    // Check if already exists
    if specks_dir.exists() && !force {
        let message = ".specks directory already exists (use --force to overwrite)".to_string();
        if json_output {
            let issues = vec![JsonIssue {
                code: "E009".to_string(),
                severity: "error".to_string(),
                message: message.clone(),
                file: Some(".specks/".to_string()),
                line: None,
                anchor: None,
            }];
            let response: JsonResponse<InitData> = JsonResponse::error(
                "init",
                InitData {
                    path: ".specks/".to_string(),
                    files_created: vec![],
                },
                issues,
            );
            println!("{}", serde_json::to_string_pretty(&response).unwrap());
        } else {
            eprintln!("error: {}", message);
        }
        return Ok(1);
    }

    // Create directory (remove first if force)
    if force && specks_dir.exists() {
        fs::remove_dir_all(specks_dir)
            .map_err(|e| format!("failed to remove existing .specks directory: {}", e))?;
    }

    fs::create_dir_all(specks_dir)
        .map_err(|e| format!("failed to create .specks directory: {}", e))?;

    // Create skeleton
    let skeleton_path = specks_dir.join("specks-skeleton.md");
    fs::write(&skeleton_path, SKELETON_CONTENT)
        .map_err(|e| format!("failed to write specks-skeleton.md: {}", e))?;

    // Create config.toml
    let config_path = specks_dir.join("config.toml");
    fs::write(&config_path, DEFAULT_CONFIG)
        .map_err(|e| format!("failed to write config.toml: {}", e))?;

    // Create implementation log
    let log_path = specks_dir.join("specks-implementation-log.md");
    fs::write(&log_path, IMPLEMENTATION_LOG_CONTENT)
        .map_err(|e| format!("failed to write specks-implementation-log.md: {}", e))?;

    // Create runs directory for agent reports (D15)
    let runs_dir = specks_dir.join("runs");
    fs::create_dir_all(&runs_dir)
        .map_err(|e| format!("failed to create runs directory: {}", e))?;

    // Ensure .specks/runs/ is in .gitignore
    let gitignore_path = Path::new(".gitignore");
    let gitignore_entry = ".specks/runs/";

    let should_add_entry = if gitignore_path.exists() {
        let content = fs::read_to_string(gitignore_path)
            .map_err(|e| format!("failed to read .gitignore: {}", e))?;
        !content.lines().any(|line| line.trim() == gitignore_entry)
    } else {
        true
    };

    if should_add_entry {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(gitignore_path)
            .map_err(|e| format!("failed to open .gitignore: {}", e))?;

        // Add newline before entry if file exists and doesn't end with newline
        if gitignore_path.exists() {
            let content = fs::read_to_string(gitignore_path).unwrap_or_default();
            if !content.is_empty() && !content.ends_with('\n') {
                writeln!(file).map_err(|e| format!("failed to write to .gitignore: {}", e))?;
            }
        }

        writeln!(file, "\n# Specks run artifacts (never commit)")
            .map_err(|e| format!("failed to write to .gitignore: {}", e))?;
        writeln!(file, "{}", gitignore_entry)
            .map_err(|e| format!("failed to write to .gitignore: {}", e))?;
    }

    let mut files_created = vec![
        "specks-skeleton.md".to_string(),
        "config.toml".to_string(),
        "specks-implementation-log.md".to_string(),
        "runs/".to_string(),
    ];

    // Try to install Claude Code skills (optional - warn but continue if not available)
    let project_dir = std::env::current_dir()
        .map_err(|e| format!("failed to get current directory: {}", e))?;
    let skills_installed = if let Some(share_dir) = find_share_dir() {
        let results = install_all_skills(&share_dir, &project_dir, false);
        let mut installed_skills: Vec<String> = vec![];
        let mut skill_warnings: Vec<String> = vec![];

        for (skill_name, result) in results {
            match result {
                Ok(SkillInstallStatus::Installed) | Ok(SkillInstallStatus::Updated) => {
                    let path = format!(".claude/skills/{}/SKILL.md", skill_name);
                    files_created.push(path.clone());
                    installed_skills.push(skill_name);
                }
                Ok(SkillInstallStatus::Unchanged) => {
                    // Skill already exists and is up to date - still count it
                    let path = format!(".claude/skills/{}/SKILL.md", skill_name);
                    files_created.push(path);
                    installed_skills.push(skill_name);
                }
                Ok(SkillInstallStatus::SourceMissing) => {
                    skill_warnings.push(format!(
                        "Skill {} not found in share directory",
                        skill_name
                    ));
                }
                Err(e) => {
                    skill_warnings.push(format!("Failed to install skill {}: {}", skill_name, e));
                }
            }
        }
        Some((installed_skills, skill_warnings))
    } else {
        None
    };

    if json_output {
        let response = JsonResponse::ok(
            "init",
            InitData {
                path: ".specks/".to_string(),
                files_created,
            },
        );
        println!("{}", serde_json::to_string_pretty(&response).unwrap());
    } else if !quiet {
        println!("Initialized specks project in .specks/");
        println!("  Created: specks-skeleton.md");
        println!("  Created: config.toml");
        println!("  Created: specks-implementation-log.md");
        println!("  Created: runs/");
        if should_add_entry {
            println!("  Updated: .gitignore (added .specks/runs/)");
        }

        // Report skill installation status
        match skills_installed {
            Some((installed, warnings)) => {
                if !installed.is_empty() {
                    println!();
                    println!("Claude Code skills installed:");
                    for skill in &installed {
                        println!("  Created: .claude/skills/{}/SKILL.md", skill);
                    }
                }
                for warning in warnings {
                    eprintln!("  Warning: {}", warning);
                }
            }
            None => {
                println!();
                println!(
                    "Note: Claude Code skills not installed (share directory not found)."
                );
                println!("      Run `specks setup claude` after ensuring specks is properly installed.");
            }
        }
    }

    Ok(0)
}
