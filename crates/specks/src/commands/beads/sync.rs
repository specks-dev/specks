//! Implementation of the `specks beads sync` command (Spec S06)

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use specks_core::{BeadsCli, Config, Speck, SpecksError, find_project_root, parse_speck};

use crate::output::{JsonIssue, JsonResponse};

/// Sync result data for JSON output
#[derive(Debug, serde::Serialize)]
pub struct SyncData {
    pub file: String,
    pub root_bead_id: Option<String>,
    pub steps_synced: usize,
    pub deps_added: usize,
    pub dry_run: bool,
}

/// Options for the sync command
pub struct SyncOptions {
    pub file: String,
    pub dry_run: bool,
    #[allow(dead_code)]
    pub update_title: bool,
    #[allow(dead_code)]
    pub update_body: bool,
    pub prune_deps: bool,
    pub substeps_mode: String,
    pub json_output: bool,
    pub quiet: bool,
}

/// Run the beads sync command
pub fn run_sync(opts: SyncOptions) -> Result<i32, String> {
    let SyncOptions {
        file,
        dry_run,
        update_title: _,
        update_body: _,
        prune_deps,
        substeps_mode,
        json_output,
        quiet,
    } = opts;
    // Find project root
    let project_root = match find_project_root() {
        Ok(root) => root,
        Err(_) => {
            return output_error(
                json_output,
                "E009",
                ".specks directory not initialized",
                &file,
                9,
            );
        }
    };

    // Load config
    let config = Config::load_from_project(&project_root).unwrap_or_default();
    let bd_path =
        std::env::var("SPECKS_BD_PATH").unwrap_or_else(|_| config.specks.beads.bd_path.clone());
    let beads = BeadsCli::new(bd_path);

    // Check if beads CLI is installed
    if !beads.is_installed() {
        return output_error(
            json_output,
            "E005",
            "beads CLI not installed or not found",
            &file,
            5,
        );
    }

    // Check if beads is initialized
    if !beads.is_initialized(&project_root) {
        return output_error(
            json_output,
            "E013",
            "beads not initialized (run `bd init`)",
            &file,
            13,
        );
    }

    // Resolve file path
    let path = resolve_file_path(&project_root, &file);
    if !path.exists() {
        return output_error(
            json_output,
            "E002",
            &format!("file not found: {}", file),
            &file,
            2,
        );
    }

    // Read and parse the speck
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            return output_error(
                json_output,
                "E002",
                &format!("failed to read file: {}", e),
                &file,
                2,
            );
        }
    };

    let speck = match parse_speck(&content) {
        Ok(s) => s,
        Err(e) => {
            return output_error(
                json_output,
                "E001",
                &format!("failed to parse speck: {}", e),
                &file,
                1,
            );
        }
    };

    // Perform sync
    let ctx = SyncContext {
        beads: &beads,
        config: &config,
        dry_run,
        prune_deps,
        substeps_mode: &substeps_mode,
        quiet,
    };
    let result = sync_speck_to_beads(&path, &speck, &content, &ctx);

    match result {
        Ok((root_id, steps_synced, deps_added, updated_content)) => {
            // Write updated content back to file (unless dry run)
            if !dry_run {
                if let Some(new_content) = updated_content {
                    if let Err(e) = fs::write(&path, new_content) {
                        return output_error(
                            json_output,
                            "E002",
                            &format!("failed to write file: {}", e),
                            &file,
                            1,
                        );
                    }
                }
            }

            if json_output {
                let data = SyncData {
                    file: file.clone(),
                    root_bead_id: root_id.clone(),
                    steps_synced,
                    deps_added,
                    dry_run,
                };
                let response = JsonResponse::ok("beads sync", data);
                println!("{}", serde_json::to_string_pretty(&response).unwrap());
            } else if !quiet {
                if dry_run {
                    println!("[dry-run] Would sync {} to beads:", file);
                } else {
                    println!("Synced {} to beads:", file);
                }
                if let Some(id) = root_id {
                    println!("  Root bead: {}", id);
                }
                println!("  Steps synced: {}", steps_synced);
                println!("  Dependencies added: {}", deps_added);
            }

            Ok(0)
        }
        Err(e) => {
            let code = match &e {
                SpecksError::BeadsNotInstalled => "E005",
                SpecksError::BeadsNotInitialized => "E013",
                _ => "E016",
            };
            let exit_code = e.exit_code();
            output_error(json_output, code, &e.to_string(), &file, exit_code)
        }
    }
}

/// Internal context for sync operations
struct SyncContext<'a> {
    beads: &'a BeadsCli,
    config: &'a Config,
    dry_run: bool,
    prune_deps: bool,
    substeps_mode: &'a str,
    quiet: bool,
}

/// Sync a speck to beads
fn sync_speck_to_beads(
    path: &Path,
    speck: &Speck,
    content: &str,
    ctx: &SyncContext<'_>,
) -> Result<(Option<String>, usize, usize, Option<String>), SpecksError> {
    let mut updated_content = content.to_string();
    let mut steps_synced = 0;
    let mut deps_added = 0;

    // Get phase title for root bead
    let phase_title = speck
        .phase_title
        .clone()
        .unwrap_or_else(|| "Untitled Speck".to_string());
    let speck_path = path.to_string_lossy();

    // Phase 1: Collect all known bead IDs from speck for batch existence check
    let mut known_ids: Vec<String> = Vec::new();
    if let Some(ref root_id) = speck.metadata.beads_root_id {
        known_ids.push(root_id.clone());
    }
    for step in &speck.steps {
        if let Some(ref bead_id) = step.bead_id {
            known_ids.push(bead_id.clone());
        }
        if ctx.substeps_mode == "children" {
            for substep in &step.substeps {
                if let Some(ref bead_id) = substep.bead_id {
                    known_ids.push(bead_id.clone());
                }
            }
        }
    }

    // Phase 2: Single batch query to check which beads exist (major performance win)
    let existing_ids = if ctx.dry_run || known_ids.is_empty() {
        HashSet::new()
    } else {
        ctx.beads.list_by_ids(&known_ids).unwrap_or_default()
    };

    // Step 1: Ensure root bead exists
    let root_id = ensure_root_bead(
        speck,
        &phase_title,
        &speck_path,
        ctx,
        &existing_ids,
        &mut updated_content,
    )?;

    // Build a map of step anchors to bead IDs (existing)
    let mut anchor_to_bead: HashMap<String, String> = HashMap::new();

    // Step 2: Process each step
    for step in &speck.steps {
        let step_bead_id = ensure_step_bead(
            step,
            &root_id,
            &speck_path,
            ctx.beads,
            ctx.dry_run,
            ctx.quiet,
            &existing_ids,
            &mut updated_content,
        )?;

        anchor_to_bead.insert(step.anchor.clone(), step_bead_id.clone());
        steps_synced += 1;

        // Handle substeps if mode is "children"
        if ctx.substeps_mode == "children" {
            for substep in &step.substeps {
                let substep_bead_id = ensure_substep_bead(
                    substep,
                    &step_bead_id,
                    &speck_path,
                    ctx.beads,
                    ctx.dry_run,
                    ctx.quiet,
                    &existing_ids,
                    &mut updated_content,
                )?;

                anchor_to_bead.insert(substep.anchor.clone(), substep_bead_id);
                steps_synced += 1;
            }
        }
    }

    // Step 3: Create dependency edges
    // Optimization: if bead already existed (in existing_ids) and we're not pruning,
    // skip dependency sync entirely - deps were set when bead was first created.
    for step in &speck.steps {
        if let Some(bead_id) = anchor_to_bead.get(&step.anchor) {
            // Skip if bead already existed and not pruning (deps already set)
            let bead_existed = step
                .bead_id
                .as_ref()
                .is_some_and(|id| existing_ids.contains(id));
            if !bead_existed || ctx.prune_deps {
                let added = sync_dependencies(
                    bead_id,
                    &step.depends_on,
                    &anchor_to_bead,
                    ctx.beads,
                    ctx.prune_deps,
                    ctx.dry_run,
                )?;
                deps_added += added;
            }
        }

        // Handle substep dependencies
        if ctx.substeps_mode == "children" {
            for substep in &step.substeps {
                if let Some(bead_id) = anchor_to_bead.get(&substep.anchor) {
                    // Skip if bead already existed and not pruning
                    let bead_existed = substep
                        .bead_id
                        .as_ref()
                        .is_some_and(|id| existing_ids.contains(id));
                    if !bead_existed || ctx.prune_deps {
                        // Substeps inherit parent deps if no explicit deps
                        let deps = if substep.depends_on.is_empty() {
                            &step.depends_on
                        } else {
                            &substep.depends_on
                        };
                        let added = sync_dependencies(
                            bead_id,
                            deps,
                            &anchor_to_bead,
                            ctx.beads,
                            ctx.prune_deps,
                            ctx.dry_run,
                        )?;
                        deps_added += added;
                    }
                }
            }
        }
    }

    let content_changed = updated_content != content;
    Ok((
        Some(root_id),
        steps_synced,
        deps_added,
        if content_changed {
            Some(updated_content)
        } else {
            None
        },
    ))
}

/// Ensure root bead exists and return its ID
fn ensure_root_bead(
    speck: &Speck,
    phase_title: &str,
    speck_path: &str,
    ctx: &SyncContext<'_>,
    existing_ids: &HashSet<String>,
    content: &mut String,
) -> Result<String, SpecksError> {
    // Check if we already have a root ID
    if let Some(ref root_id) = speck.metadata.beads_root_id {
        // Use pre-fetched existence check (no subprocess call)
        if existing_ids.contains(root_id) {
            return Ok(root_id.clone());
        }
        // Root bead was deleted, need to recreate
        if !ctx.quiet {
            eprintln!("warning: root bead {} not found, recreating", root_id);
        }
    }

    // Create root bead
    let description = format!("Specks: {}", speck_path);
    let issue_type = &ctx.config.specks.beads.root_issue_type;

    if ctx.dry_run {
        // Generate a fake ID for dry run
        let fake_id = "bd-dryrun-root".to_string();
        write_beads_root_to_content(content, &fake_id);
        return Ok(fake_id);
    }

    let issue = ctx.beads.create(
        phase_title,
        Some(&description),
        None,
        Some(issue_type),
        None,
    )?;

    // Write Beads Root to content
    write_beads_root_to_content(content, &issue.id);

    Ok(issue.id)
}

/// Ensure step bead exists and return its ID
#[allow(clippy::too_many_arguments)]
fn ensure_step_bead(
    step: &specks_core::Step,
    root_id: &str,
    speck_path: &str,
    beads: &BeadsCli,
    dry_run: bool,
    quiet: bool,
    existing_ids: &HashSet<String>,
    content: &mut String,
) -> Result<String, SpecksError> {
    // Check if step already has a bead ID
    if let Some(ref bead_id) = step.bead_id {
        // Use pre-fetched existence check (no subprocess call)
        if existing_ids.contains(bead_id) {
            return Ok(bead_id.clone());
        }
        // Bead was deleted, need to recreate
        if !quiet {
            eprintln!("warning: step bead {} not found, recreating", bead_id);
        }
    }

    // Create step bead
    let title = format!("Step {}: {}", step.number, step.title);
    let mut desc_parts = vec![format!("Specks: {}#{}", speck_path, step.anchor)];
    if let Some(ref commit) = step.commit_message {
        desc_parts.push(format!("Commit: {}", commit));
    }
    if !step.depends_on.is_empty() {
        desc_parts.push(format!("Depends on: {}", step.depends_on.join(", ")));
    }
    let description = desc_parts.join("\n");

    if dry_run {
        // Generate a fake ID for dry run
        let fake_id = format!("bd-dryrun-{}", step.anchor);
        write_bead_to_step(content, &step.anchor, &fake_id);
        return Ok(fake_id);
    }

    let issue = beads.create(&title, Some(&description), Some(root_id), None, None)?;

    // Write Bead ID to step in content
    write_bead_to_step(content, &step.anchor, &issue.id);

    Ok(issue.id)
}

/// Ensure substep bead exists and return its ID
#[allow(clippy::too_many_arguments)]
fn ensure_substep_bead(
    substep: &specks_core::Substep,
    parent_bead_id: &str,
    speck_path: &str,
    beads: &BeadsCli,
    dry_run: bool,
    quiet: bool,
    existing_ids: &HashSet<String>,
    content: &mut String,
) -> Result<String, SpecksError> {
    // Check if substep already has a bead ID
    if let Some(ref bead_id) = substep.bead_id {
        // Use pre-fetched existence check (no subprocess call)
        if existing_ids.contains(bead_id) {
            return Ok(bead_id.clone());
        }
        // Bead was deleted, need to recreate
        if !quiet {
            eprintln!("warning: substep bead {} not found, recreating", bead_id);
        }
    }

    // Create substep bead
    let title = format!("Step {}: {}", substep.number, substep.title);
    let mut desc_parts = vec![format!("Specks: {}#{}", speck_path, substep.anchor)];
    if let Some(ref commit) = substep.commit_message {
        desc_parts.push(format!("Commit: {}", commit));
    }
    if !substep.depends_on.is_empty() {
        desc_parts.push(format!("Depends on: {}", substep.depends_on.join(", ")));
    }
    let description = desc_parts.join("\n");

    if dry_run {
        let fake_id = format!("bd-dryrun-{}", substep.anchor);
        write_bead_to_step(content, &substep.anchor, &fake_id);
        return Ok(fake_id);
    }

    let issue = beads.create(&title, Some(&description), Some(parent_bead_id), None, None)?;

    // Write Bead ID to substep in content
    write_bead_to_step(content, &substep.anchor, &issue.id);

    Ok(issue.id)
}

/// Sync dependencies for a bead
fn sync_dependencies(
    bead_id: &str,
    depends_on: &[String],
    anchor_to_bead: &HashMap<String, String>,
    beads: &BeadsCli,
    prune_deps: bool,
    dry_run: bool,
) -> Result<usize, SpecksError> {
    if dry_run {
        return Ok(depends_on.len());
    }

    let mut added = 0;

    // Get current dependencies
    let current_deps = beads.dep_list(bead_id).unwrap_or_default();
    let current_dep_ids: std::collections::HashSet<String> =
        current_deps.iter().map(|d| d.id.clone()).collect();

    // Add missing dependencies
    for dep_anchor in depends_on {
        if let Some(dep_bead_id) = anchor_to_bead.get(dep_anchor) {
            if !current_dep_ids.contains(dep_bead_id) {
                beads.dep_add(bead_id, dep_bead_id)?;
                added += 1;
            }
        }
    }

    // Prune extra dependencies if requested
    if prune_deps {
        let desired_dep_ids: std::collections::HashSet<String> = depends_on
            .iter()
            .filter_map(|a| anchor_to_bead.get(a).cloned())
            .collect();

        for dep in current_deps {
            if !desired_dep_ids.contains(&dep.id) {
                beads.dep_remove(bead_id, &dep.id)?;
            }
        }
    }

    Ok(added)
}

/// Write Beads Root ID to content (in Plan Metadata)
fn write_beads_root_to_content(content: &mut String, bead_id: &str) {
    // Check if Beads Root already exists in content
    let beads_root_pattern = regex::Regex::new(r"\*\*Beads Root:\*\*\s*`[^`]*`").unwrap();
    let beads_root_line = format!("**Beads Root:** `{}`", bead_id);

    if beads_root_pattern.is_match(content) {
        // Replace existing
        *content = beads_root_pattern
            .replace(content, beads_root_line.as_str())
            .to_string();
    } else {
        // Add after Plan Metadata table
        // Look for the table end (line starting with | followed by blank line or ---)
        let lines: Vec<&str> = content.lines().collect();
        let mut insert_pos = None;
        let mut in_metadata = false;

        for (i, line) in lines.iter().enumerate() {
            if line.contains("Plan Metadata") {
                in_metadata = true;
            }
            if in_metadata && line.starts_with('|') && line.contains("Last updated") {
                // Insert after the table row containing Last updated
                for (j, next_line) in lines.iter().enumerate().skip(i + 1) {
                    if !next_line.starts_with('|') {
                        insert_pos = Some(j);
                        break;
                    }
                }
                break;
            }
        }

        if let Some(pos) = insert_pos {
            let mut new_lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
            // Insert Beads Root row in table format
            let table_row = format!("| Beads Root | `{}` |", bead_id);
            new_lines.insert(pos, table_row);
            *content = new_lines.join("\n");
        }
    }
}

/// Write Bead ID to a step in content
///
/// Uses anchor-based matching only (not line numbers) to avoid issues with
/// stale line numbers after content modifications.
fn write_bead_to_step(content: &mut String, anchor: &str, bead_id: &str) {
    let lines: Vec<&str> = content.lines().collect();
    let mut new_lines: Vec<String> = Vec::new();
    let bead_line = format!("**Bead:** `{}`", bead_id);
    let bead_pattern = regex::Regex::new(r"^\*\*Bead:\*\*\s*`[^`]*`").unwrap();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        new_lines.push(line.to_string());

        // Only match on anchor - line numbers become stale after edits
        if line.contains(&format!("{{#{}}}", anchor)) {
            // Look ahead to find where to insert/update Bead line
            // Placement: after **Depends on:** if present, before **Commit:**
            let mut found_bead = false;
            let mut insert_before = None;

            for j in (i + 1)..std::cmp::min(i + 20, lines.len()) {
                let next_line = lines[j];

                // Check if bead line already exists
                if bead_pattern.is_match(next_line) {
                    // Copy intermediate lines first, then replace the bead
                    for line in lines.iter().skip(i + 1).take(j - i - 1) {
                        new_lines.push(line.to_string());
                    }
                    new_lines.push(bead_line.clone());
                    found_bead = true;
                    i = j;
                    break;
                }

                // Check for **Commit:** or next step/section header
                if next_line.starts_with("**Commit:**")
                    || next_line.starts_with("####")
                    || next_line.starts_with("---")
                {
                    insert_before = Some(j);
                    break;
                }
            }

            if !found_bead {
                if let Some(pos) = insert_before {
                    // Copy lines up to insert position
                    for line in lines.iter().skip(i + 1).take(pos - i - 1) {
                        new_lines.push(line.to_string());
                    }
                    // Insert bead line
                    new_lines.push(bead_line.clone());
                    new_lines.push(String::new()); // Blank line before next section
                    i = pos - 1;
                }
            }
        }

        i += 1;
    }

    *content = new_lines.join("\n");
}

/// Resolve file path relative to project
fn resolve_file_path(project_root: &Path, file: &str) -> std::path::PathBuf {
    let path = Path::new(file);
    if path.is_absolute() {
        path.to_path_buf()
    } else if file.starts_with(".specks/") || file.starts_with(".specks\\") {
        project_root.join(file)
    } else if file.starts_with("specks-") && file.ends_with(".md") {
        project_root.join(".specks").join(file)
    } else if file.ends_with(".md") {
        // Try as-is first
        let as_is = project_root.join(file);
        if as_is.exists() {
            as_is
        } else {
            project_root
                .join(".specks")
                .join(format!("specks-{}", file))
        }
    } else {
        project_root
            .join(".specks")
            .join(format!("specks-{}.md", file))
    }
}

/// Output an error in JSON or text format
fn output_error(
    json_output: bool,
    code: &str,
    message: &str,
    file: &str,
    exit_code: i32,
) -> Result<i32, String> {
    if json_output {
        let issues = vec![JsonIssue {
            code: code.to_string(),
            severity: "error".to_string(),
            message: message.to_string(),
            file: Some(file.to_string()),
            line: None,
            anchor: None,
        }];
        let response: JsonResponse<SyncData> = JsonResponse::error(
            "beads sync",
            SyncData {
                file: file.to_string(),
                root_bead_id: None,
                steps_synced: 0,
                deps_added: 0,
                dry_run: false,
            },
            issues,
        );
        println!("{}", serde_json::to_string_pretty(&response).unwrap());
    } else {
        eprintln!("error: {}", message);
    }
    Ok(exit_code)
}
