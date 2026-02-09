//! Implementation of the `specks init` command (Spec S01)

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::output::{InitData, JsonIssue, JsonResponse};

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

    // Ensure .specks-worktrees/ is in .gitignore
    let gitignore_path = Path::new(".gitignore");
    let gitignore_entry = ".specks-worktrees/";

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

        writeln!(
            file,
            "\n# Specks worktrees (isolated implementation environments)"
        )
        .map_err(|e| format!("failed to write to .gitignore: {}", e))?;
        writeln!(file, "{}", gitignore_entry)
            .map_err(|e| format!("failed to write to .gitignore: {}", e))?;
    }

    let files_created = vec![
        "specks-skeleton.md".to_string(),
        "config.toml".to_string(),
        "specks-implementation-log.md".to_string(),
    ];

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
        if should_add_entry {
            println!("  Updated: .gitignore (added .specks-worktrees/)");
            println!("  Updated: .gitignore (added .specks-worktrees/)");
        }

        // Note about plugin-based workflow
        println!();
        println!("To use specks with Claude Code:");
        println!("  claude --plugin-dir /path/to/specks");
        println!("  Then use /specks:planner and /specks:implementer");
    }

    Ok(0)
}
