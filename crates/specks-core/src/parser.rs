//! Speck file parsing
//!
//! Parses speck files (structured plans for multi-agent implementation) from Markdown content.
//! The parser extracts:
//! - Plan metadata (owner, status, target branch, etc.)
//! - Anchors for cross-referencing
//! - Design decisions and open questions
//! - Execution steps with tasks, tests, and checkpoints

use crate::error::SpecksError;
use crate::types::{
    Anchor, BeadsHints, Checkpoint, CheckpointKind, Decision, Question, Speck, Step, Substep,
};
use std::collections::HashMap;

/// Regex patterns for parsing (compiled once)
mod patterns {
    use std::sync::LazyLock;

    pub static ANCHOR: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"\{#([a-z0-9-]+)\}").unwrap());

    pub static PHASE_HEADER: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"^##\s+Phase\s+[\d.]+:\s*(.+?)\s*(?:\{#([a-z0-9-]+)\})?\s*$").unwrap()
    });

    pub static STEP_HEADER: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(
            r"^#{3,5}\s+Step\s+(\d+(?:\.\d+)?):?\s*(.+?)\s*(?:\{#([a-z0-9-]+)\})?\s*$",
        )
        .unwrap()
    });

    pub static DECISION_HEADER: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(
            r"^####\s+\[([DQ]\d+)\]\s*(.+?)\s*(?:\((\w+)\))?\s*(?:\{#([a-z0-9-]+)\})?\s*$",
        )
        .unwrap()
    });

    pub static CHECKBOX: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"^-\s+\[([ xX])\]\s*(.+)$").unwrap());

    pub static METADATA_ROW: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"^\|\s*([^|]+?)\s*\|\s*([^|]*?)\s*\|").unwrap());

    pub static DEPENDS_ON: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"^\*\*Depends on:\*\*\s*(.+)$").unwrap());

    pub static BEAD_LINE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"^\*\*Bead:\*\*\s*`([^`]+)`").unwrap());

    pub static BEADS_HINTS: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"^\*\*Beads:\*\*\s*(.+)$").unwrap());

    pub static COMMIT_LINE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"^\*\*Commit:\*\*\s*`([^`]+)`").unwrap());

    pub static REFERENCES_LINE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"^\*\*References:\*\*\s*(.+)$").unwrap());

    pub static PURPOSE_LINE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"^\*\*Purpose:\*\*\s*(.+)$").unwrap());

    pub static BEADS_ROOT_ROW: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"^\|\s*Beads\s+Root\s*\|\s*`([^`]+)`").unwrap());

    pub static ANCHOR_REF: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"#([a-z0-9-]+)").unwrap());

    pub static SECTION_HEADER: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"^(#{1,6})\s+(.+?)\s*(?:\{#([a-z0-9-]+)\})?\s*$").unwrap()
    });
}

/// Parse a speck file from its contents
pub fn parse_speck(content: &str) -> Result<Speck, SpecksError> {
    let mut speck = Speck {
        raw_content: content.to_string(),
        ..Default::default()
    };

    let lines: Vec<&str> = content.lines().collect();

    // Track current parsing context
    let mut in_metadata_table = false;
    let mut in_step: Option<usize> = None; // Index into speck.steps
    let mut in_substep: Option<usize> = None; // Index into current step's substeps
    let mut current_section = CurrentSection::None;
    let mut anchor_locations: HashMap<String, usize> = HashMap::new();

    for (line_num, line) in lines.iter().enumerate() {
        let line_number = line_num + 1; // 1-indexed

        // Extract anchors from any line
        for cap in patterns::ANCHOR.captures_iter(line) {
            let anchor_name = cap.get(1).unwrap().as_str().to_string();

            // Check for duplicates
            if let Some(&_first_line) = anchor_locations.get(&anchor_name) {
                // We'll collect this as a warning/error later, but still record it
                speck.anchors.push(Anchor {
                    name: format!("{} (duplicate)", anchor_name),
                    line: line_number,
                });
            } else {
                anchor_locations.insert(anchor_name.clone(), line_number);
                speck.anchors.push(Anchor {
                    name: anchor_name,
                    line: line_number,
                });
            }
        }

        // Parse phase header
        if let Some(caps) = patterns::PHASE_HEADER.captures(line) {
            speck.phase_title = Some(caps.get(1).unwrap().as_str().to_string());
            if let Some(anchor) = caps.get(2) {
                speck.phase_anchor = Some(anchor.as_str().to_string());
            }
            continue;
        }

        // Parse purpose line
        if let Some(caps) = patterns::PURPOSE_LINE.captures(line) {
            speck.purpose = Some(caps.get(1).unwrap().as_str().to_string());
            continue;
        }

        // Detect metadata table start
        if line.contains("| Field | Value |") || line.contains("|------|-------|") {
            in_metadata_table = true;
            continue;
        }

        // Parse metadata table rows
        if in_metadata_table {
            if line.trim().is_empty() || !line.starts_with('|') {
                in_metadata_table = false;
            } else if let Some(caps) = patterns::METADATA_ROW.captures(line) {
                let field = caps.get(1).unwrap().as_str().trim();
                let value = caps.get(2).unwrap().as_str().trim();

                // Skip header separator row
                if field.contains("---") {
                    continue;
                }

                match field.to_lowercase().as_str() {
                    "owner" => speck.metadata.owner = non_empty_value(value),
                    "status" => speck.metadata.status = non_empty_value(value),
                    "target branch" => speck.metadata.target_branch = non_empty_value(value),
                    "tracking issue/pr" | "tracking issue" | "tracking" => {
                        speck.metadata.tracking = non_empty_value(value)
                    }
                    "last updated" => speck.metadata.last_updated = non_empty_value(value),
                    _ => {}
                }

                // Check for Beads Root in metadata
                if let Some(caps) = patterns::BEADS_ROOT_ROW.captures(line) {
                    speck.metadata.beads_root_id = Some(caps.get(1).unwrap().as_str().to_string());
                }
                continue;
            }
        }

        // Parse section headers to track context
        if let Some(caps) = patterns::SECTION_HEADER.captures(line) {
            let header_text = caps.get(2).unwrap().as_str();
            let header_lower = header_text.to_lowercase();

            if header_lower.contains("tasks:") || header_lower == "tasks" {
                current_section = CurrentSection::Tasks;
            } else if header_lower.contains("tests:") || header_lower == "tests" {
                current_section = CurrentSection::Tests;
            } else if header_lower.contains("checkpoint")
                || header_lower.contains("checkpoints:")
                || header_lower == "checkpoints"
            {
                current_section = CurrentSection::Checkpoints;
            } else if header_lower.contains("artifacts:") || header_lower == "artifacts" {
                current_section = CurrentSection::Artifacts;
            } else if header_lower.contains("references") || header_lower.contains("rollback") {
                current_section = CurrentSection::Other;
            }
        }

        // Check for **Tasks:**, **Tests:**, **Checkpoint:**, **Artifacts:** bold markers
        if line.starts_with("**Tasks:**") {
            current_section = CurrentSection::Tasks;
            continue;
        }
        if line.starts_with("**Tests:**") {
            current_section = CurrentSection::Tests;
            continue;
        }
        if line.starts_with("**Checkpoint:**") || line.starts_with("**Checkpoints:**") {
            current_section = CurrentSection::Checkpoints;
            continue;
        }
        if line.starts_with("**Artifacts:**") {
            current_section = CurrentSection::Artifacts;
            continue;
        }

        // Parse decision/question headers
        if let Some(caps) = patterns::DECISION_HEADER.captures(line) {
            let id = caps.get(1).unwrap().as_str();
            let title = caps.get(2).unwrap().as_str();
            let status = caps.get(3).map(|m| m.as_str().to_string());
            let anchor = caps.get(4).map(|m| m.as_str().to_string());

            if id.starts_with('D') {
                speck.decisions.push(Decision {
                    id: id.to_string(),
                    title: title.to_string(),
                    status,
                    anchor,
                    line: line_number,
                });
            } else if id.starts_with('Q') {
                speck.questions.push(Question {
                    id: id.to_string(),
                    title: title.to_string(),
                    resolution: status,
                    anchor,
                    line: line_number,
                });
            }
            continue;
        }

        // Parse step headers
        if let Some(caps) = patterns::STEP_HEADER.captures(line) {
            let number = caps.get(1).unwrap().as_str();
            let title = caps.get(2).unwrap().as_str();
            let anchor = caps
                .get(3)
                .map(|m| m.as_str().to_string())
                .unwrap_or_else(|| format!("step-{}", number.replace('.', "-")));

            // Check if this is a substep (contains a dot)
            if number.contains('.') {
                // This is a substep
                let substep = Substep {
                    number: number.to_string(),
                    title: title.to_string(),
                    anchor: anchor.clone(),
                    line: line_number,
                    ..Default::default()
                };

                if let Some(step_idx) = in_step {
                    let substep_idx = speck.steps[step_idx].substeps.len();
                    speck.steps[step_idx].substeps.push(substep);
                    in_substep = Some(substep_idx);
                }
            } else {
                // This is a main step
                let step = Step {
                    number: number.to_string(),
                    title: title.to_string(),
                    anchor: anchor.clone(),
                    line: line_number,
                    ..Default::default()
                };
                speck.steps.push(step);
                in_step = Some(speck.steps.len() - 1);
                in_substep = None;
            }

            current_section = CurrentSection::None;
            continue;
        }

        // Parse step metadata lines (when inside a step)
        if in_step.is_some() {
            // Parse **Depends on:** line
            if let Some(caps) = patterns::DEPENDS_ON.captures(line) {
                let deps_str = caps.get(1).unwrap().as_str();
                let deps: Vec<String> = patterns::ANCHOR_REF
                    .captures_iter(deps_str)
                    .map(|c| c.get(1).unwrap().as_str().to_string())
                    .collect();

                if let Some(substep_idx) = in_substep {
                    if let Some(step_idx) = in_step {
                        speck.steps[step_idx].substeps[substep_idx].depends_on = deps;
                    }
                } else if let Some(step_idx) = in_step {
                    speck.steps[step_idx].depends_on = deps;
                }
                continue;
            }

            // Parse **Bead:** line
            if let Some(caps) = patterns::BEAD_LINE.captures(line) {
                let bead_id = caps.get(1).unwrap().as_str().to_string();

                if let Some(substep_idx) = in_substep {
                    if let Some(step_idx) = in_step {
                        speck.steps[step_idx].substeps[substep_idx].bead_id = Some(bead_id);
                    }
                } else if let Some(step_idx) = in_step {
                    speck.steps[step_idx].bead_id = Some(bead_id);
                }
                continue;
            }

            // Parse **Beads:** hints line
            if let Some(caps) = patterns::BEADS_HINTS.captures(line) {
                let hints_str = caps.get(1).unwrap().as_str();
                let hints = parse_beads_hints(hints_str);

                if let Some(substep_idx) = in_substep {
                    if let Some(step_idx) = in_step {
                        speck.steps[step_idx].substeps[substep_idx].beads_hints = Some(hints);
                    }
                } else if let Some(step_idx) = in_step {
                    speck.steps[step_idx].beads_hints = Some(hints);
                }
                continue;
            }

            // Parse **Commit:** line
            if let Some(caps) = patterns::COMMIT_LINE.captures(line) {
                let commit_msg = caps.get(1).unwrap().as_str().to_string();

                if let Some(substep_idx) = in_substep {
                    if let Some(step_idx) = in_step {
                        speck.steps[step_idx].substeps[substep_idx].commit_message =
                            Some(commit_msg);
                    }
                } else if let Some(step_idx) = in_step {
                    speck.steps[step_idx].commit_message = Some(commit_msg);
                }
                continue;
            }

            // Parse **References:** line
            if let Some(caps) = patterns::REFERENCES_LINE.captures(line) {
                let refs = caps.get(1).unwrap().as_str().to_string();

                if let Some(substep_idx) = in_substep {
                    if let Some(step_idx) = in_step {
                        speck.steps[step_idx].substeps[substep_idx].references = Some(refs);
                    }
                } else if let Some(step_idx) = in_step {
                    speck.steps[step_idx].references = Some(refs);
                }
                continue;
            }

            // Parse checkbox items
            if let Some(caps) = patterns::CHECKBOX.captures(line) {
                let checked = caps.get(1).unwrap().as_str() != " ";
                let text = caps.get(2).unwrap().as_str().to_string();

                // Special handling for Artifacts section: capture text as plain artifact item
                if current_section == CurrentSection::Artifacts {
                    if let Some(substep_idx) = in_substep {
                        if let Some(step_idx) = in_step {
                            speck.steps[step_idx].substeps[substep_idx]
                                .artifacts
                                .push(text);
                        }
                    } else if let Some(step_idx) = in_step {
                        speck.steps[step_idx].artifacts.push(text);
                    }
                    continue;
                }

                let kind = match current_section {
                    CurrentSection::Tasks => CheckpointKind::Task,
                    CurrentSection::Tests => CheckpointKind::Test,
                    CurrentSection::Checkpoints => CheckpointKind::Checkpoint,
                    _ => CheckpointKind::Task, // Default to task
                };

                let checkpoint = Checkpoint {
                    checked,
                    text,
                    kind,
                    line: line_number,
                };

                if let Some(substep_idx) = in_substep {
                    if let Some(step_idx) = in_step {
                        match kind {
                            CheckpointKind::Task => {
                                speck.steps[step_idx].substeps[substep_idx]
                                    .tasks
                                    .push(checkpoint);
                            }
                            CheckpointKind::Test => {
                                speck.steps[step_idx].substeps[substep_idx]
                                    .tests
                                    .push(checkpoint);
                            }
                            CheckpointKind::Checkpoint => {
                                speck.steps[step_idx].substeps[substep_idx]
                                    .checkpoints
                                    .push(checkpoint);
                            }
                        }
                    }
                } else if let Some(step_idx) = in_step {
                    match kind {
                        CheckpointKind::Task => {
                            speck.steps[step_idx].tasks.push(checkpoint);
                        }
                        CheckpointKind::Test => {
                            speck.steps[step_idx].tests.push(checkpoint);
                        }
                        CheckpointKind::Checkpoint => {
                            speck.steps[step_idx].checkpoints.push(checkpoint);
                        }
                    }
                }
                continue;
            }

            // Parse plain bullet items in Artifacts section
            if current_section == CurrentSection::Artifacts && line.trim_start().starts_with("- ") {
                let text = line.trim_start().strip_prefix("- ").unwrap().to_string();
                if let Some(substep_idx) = in_substep {
                    if let Some(step_idx) = in_step {
                        speck.steps[step_idx].substeps[substep_idx]
                            .artifacts
                            .push(text);
                    }
                } else if let Some(step_idx) = in_step {
                    speck.steps[step_idx].artifacts.push(text);
                }
            }
        }
    }

    Ok(speck)
}

/// Parse beads hints from a hints string like "type=task, priority=1, labels=backend,api"
fn parse_beads_hints(hints_str: &str) -> BeadsHints {
    let mut hints = BeadsHints::default();

    // Split by comma followed by space and key= pattern to handle labels with commas
    // We need to find key=value pairs more carefully
    let mut remaining = hints_str.trim();

    while !remaining.is_empty() {
        // Find the next key=value pair
        if let Some(eq_pos) = remaining.find('=') {
            let key = remaining[..eq_pos].trim().to_lowercase();

            // Find the end of this value (next ", key=" pattern or end of string)
            let value_start = eq_pos + 1;
            let rest = &remaining[value_start..];

            // Look for next key= pattern (space + word + =)
            let value_end = find_next_key_start(rest);
            let value = rest[..value_end].trim().trim_end_matches(',').trim();

            match key.as_str() {
                "type" => hints.issue_type = Some(value.to_string()),
                "priority" => {
                    if let Ok(p) = value.parse::<u8>() {
                        hints.priority = Some(p);
                    }
                }
                "labels" => {
                    hints.labels = value.split(',').map(|s| s.trim().to_string()).collect();
                }
                "estimate_minutes" | "estimate" => {
                    if let Ok(e) = value.parse::<u32>() {
                        hints.estimate_minutes = Some(e);
                    }
                }
                _ => {}
            }

            remaining = rest[value_end..].trim().trim_start_matches(',').trim();
        } else {
            break;
        }
    }

    hints
}

/// Find the start of the next key=value pair in a hints string
fn find_next_key_start(s: &str) -> usize {
    // Look for patterns like ", key=" where key is a word
    let bytes = s.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // Look for comma
        if bytes[i] == b',' {
            // Skip whitespace
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] == b' ' {
                j += 1;
            }
            // Check if this looks like a key (word followed by =)
            let mut k = j;
            while k < bytes.len() && (bytes[k].is_ascii_alphanumeric() || bytes[k] == b'_') {
                k += 1;
            }
            if k < bytes.len() && bytes[k] == b'=' && k > j {
                // This is a new key
                return i;
            }
        }
        i += 1;
    }

    s.len()
}

/// Track which section we're currently parsing within a step
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CurrentSection {
    None,
    Tasks,
    Tests,
    Checkpoints,
    Artifacts,
    Other,
}

/// Convert a value to Option, returning None only if empty
/// Per spec: TBD is considered "present" for Owner and Tracking fields
/// Per spec: <...> placeholders are stored but generate a warning
fn non_empty_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        // Store all non-empty values including TBD and placeholders
        // Validation will handle warnings for placeholders
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_speck() {
        let content = r#"## Phase 1.0: Test Phase {#phase-1}

**Purpose:** Test purpose statement

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test Owner |
| Status | draft |
| Target branch | main |
| Tracking issue/PR | #123 |
| Last updated | 2026-02-03 |

---

### 1.0.5 Execution Steps {#execution-steps}

#### Step 0: Bootstrap {#step-0}

**Commit:** `feat: initial setup`

**References:** [D01] Test decision

**Tasks:**
- [ ] Task one
- [x] Task two

**Tests:**
- [ ] Test one

**Checkpoint:**
- [ ] Checkpoint one
- [x] Checkpoint two
"#;

        let speck = parse_speck(content).unwrap();

        assert_eq!(speck.phase_title, Some("Test Phase".to_string()));
        assert_eq!(speck.phase_anchor, Some("phase-1".to_string()));
        assert_eq!(speck.purpose, Some("Test purpose statement".to_string()));

        assert_eq!(speck.metadata.owner, Some("Test Owner".to_string()));
        assert_eq!(speck.metadata.status, Some("draft".to_string()));
        assert_eq!(speck.metadata.target_branch, Some("main".to_string()));
        assert_eq!(speck.metadata.tracking, Some("#123".to_string()));
        assert_eq!(speck.metadata.last_updated, Some("2026-02-03".to_string()));

        assert_eq!(speck.steps.len(), 1);
        let step = &speck.steps[0];
        assert_eq!(step.number, "0");
        assert_eq!(step.title, "Bootstrap");
        assert_eq!(step.anchor, "step-0");
        assert_eq!(step.commit_message, Some("feat: initial setup".to_string()));
        assert_eq!(step.references, Some("[D01] Test decision".to_string()));

        assert_eq!(step.tasks.len(), 2);
        assert!(!step.tasks[0].checked);
        assert!(step.tasks[1].checked);

        assert_eq!(step.tests.len(), 1);
        assert!(!step.tests[0].checked);

        assert_eq!(step.checkpoints.len(), 2);
        assert!(!step.checkpoints[0].checked);
        assert!(step.checkpoints[1].checked);
    }

    #[test]
    fn test_parse_depends_on() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | active |
| Last updated | 2026-02-03 |

#### Step 0: First {#step-0}

**Tasks:**
- [ ] Task

#### Step 1: Second {#step-1}

**Depends on:** #step-0

**Tasks:**
- [ ] Task

#### Step 2: Third {#step-2}

**Depends on:** #step-0, #step-1

**Tasks:**
- [ ] Task
"#;

        let speck = parse_speck(content).unwrap();

        assert_eq!(speck.steps.len(), 3);
        assert!(speck.steps[0].depends_on.is_empty());
        assert_eq!(speck.steps[1].depends_on, vec!["step-0"]);
        assert_eq!(speck.steps[2].depends_on, vec!["step-0", "step-1"]);
    }

    #[test]
    fn test_parse_bead_line() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | active |
| Last updated | 2026-02-03 |
| Beads Root | `bd-root123` |

#### Step 0: First {#step-0}

**Bead:** `bd-step0`

**Tasks:**
- [ ] Task
"#;

        let speck = parse_speck(content).unwrap();

        assert_eq!(speck.metadata.beads_root_id, Some("bd-root123".to_string()));
        assert_eq!(speck.steps[0].bead_id, Some("bd-step0".to_string()));
    }

    #[test]
    fn test_parse_beads_hints() {
        let hints =
            parse_beads_hints("type=task, priority=2, labels=backend,api, estimate_minutes=60");

        assert_eq!(hints.issue_type, Some("task".to_string()));
        assert_eq!(hints.priority, Some(2));
        assert_eq!(hints.labels, vec!["backend", "api"]);
        assert_eq!(hints.estimate_minutes, Some(60));
    }

    #[test]
    fn test_parse_substeps() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | active |
| Last updated | 2026-02-03 |

#### Step 2: Big Step {#step-2}

**Depends on:** #step-1

**Tasks:**
- [ ] Parent task

##### Step 2.1: First Substep {#step-2-1}

**Commit:** `feat: substep 1`

**Tasks:**
- [ ] Substep task

##### Step 2.2: Second Substep {#step-2-2}

**Depends on:** #step-2-1

**Tasks:**
- [x] Another substep task
"#;

        let speck = parse_speck(content).unwrap();

        assert_eq!(speck.steps.len(), 1);
        let step = &speck.steps[0];
        assert_eq!(step.number, "2");
        assert_eq!(step.substeps.len(), 2);

        assert_eq!(step.substeps[0].number, "2.1");
        assert_eq!(step.substeps[0].title, "First Substep");
        assert_eq!(
            step.substeps[0].commit_message,
            Some("feat: substep 1".to_string())
        );

        assert_eq!(step.substeps[1].number, "2.2");
        assert_eq!(step.substeps[1].depends_on, vec!["step-2-1"]);
        assert!(step.substeps[1].tasks[0].checked);
    }

    #[test]
    fn test_parse_decisions() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | draft |
| Last updated | 2026-02-03 |

### Design Decisions {#design-decisions}

#### [D01] Use Rust (DECIDED) {#d01-use-rust}

**Decision:** Build in Rust.

#### [D02] Use clap (OPEN) {#d02-use-clap}

**Decision:** Consider clap for CLI.
"#;

        let speck = parse_speck(content).unwrap();

        assert_eq!(speck.decisions.len(), 2);
        assert_eq!(speck.decisions[0].id, "D01");
        assert_eq!(speck.decisions[0].title, "Use Rust");
        assert_eq!(speck.decisions[0].status, Some("DECIDED".to_string()));
        assert_eq!(speck.decisions[0].anchor, Some("d01-use-rust".to_string()));

        assert_eq!(speck.decisions[1].id, "D02");
        assert_eq!(speck.decisions[1].status, Some("OPEN".to_string()));
    }

    #[test]
    fn test_parse_questions() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | draft |
| Last updated | 2026-02-03 |

### Open Questions {#open-questions}

#### [Q01] Distribution strategy (DEFERRED) {#q01-distribution}

**Question:** How to distribute?
"#;

        let speck = parse_speck(content).unwrap();

        assert_eq!(speck.questions.len(), 1);
        assert_eq!(speck.questions[0].id, "Q01");
        assert_eq!(speck.questions[0].title, "Distribution strategy");
        assert_eq!(speck.questions[0].resolution, Some("DEFERRED".to_string()));
    }

    #[test]
    fn test_parse_anchors() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

### Section One {#section-one}

### Section Two {#section-two}

#### Subsection {#subsection}
"#;

        let speck = parse_speck(content).unwrap();

        let anchor_names: Vec<&str> = speck.anchors.iter().map(|a| a.name.as_str()).collect();
        assert!(anchor_names.contains(&"phase-1"));
        assert!(anchor_names.contains(&"plan-metadata"));
        assert!(anchor_names.contains(&"section-one"));
        assert!(anchor_names.contains(&"section-two"));
        assert!(anchor_names.contains(&"subsection"));
    }

    #[test]
    fn test_checkbox_states() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | active |
| Last updated | 2026-02-03 |

#### Step 0: Test {#step-0}

**Tasks:**
- [ ] Unchecked lowercase
- [x] Checked lowercase
- [X] Checked uppercase
"#;

        let speck = parse_speck(content).unwrap();

        assert_eq!(speck.steps[0].tasks.len(), 3);
        assert!(!speck.steps[0].tasks[0].checked);
        assert!(speck.steps[0].tasks[1].checked);
        assert!(speck.steps[0].tasks[2].checked);
    }

    #[test]
    fn test_malformed_markdown_graceful() {
        // Parser should not panic on malformed content
        let content = "This is not a valid speck at all\n\nJust random text";
        let result = parse_speck(content);
        assert!(result.is_ok());

        let speck = result.unwrap();
        assert!(speck.steps.is_empty());
        assert!(speck.metadata.owner.is_none());
    }

    #[test]
    fn test_parse_artifacts_bold_marker() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | active |
| Last updated | 2026-02-03 |

#### Step 0: Test {#step-0}

**Tasks:**
- [ ] Task one

**Artifacts:**
- [ ] New file: src/main.rs
- [ ] Modified: Cargo.toml

**Tests:**
- [ ] Test one
"#;

        let speck = parse_speck(content).unwrap();

        assert_eq!(speck.steps.len(), 1);
        let step = &speck.steps[0];
        assert_eq!(step.artifacts.len(), 2);
        assert_eq!(step.artifacts[0], "New file: src/main.rs");
        assert_eq!(step.artifacts[1], "Modified: Cargo.toml");
    }

    #[test]
    fn test_parse_artifacts_heading_style() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | active |
| Last updated | 2026-02-03 |

#### Step 0: Test {#step-0}

##### Tasks

- [ ] Task one

##### Artifacts

- New file: src/main.rs
- Modified: Cargo.toml

##### Tests

- [ ] Test one
"#;

        let speck = parse_speck(content).unwrap();

        assert_eq!(speck.steps.len(), 1);
        let step = &speck.steps[0];
        assert_eq!(step.artifacts.len(), 2);
        assert_eq!(step.artifacts[0], "New file: src/main.rs");
        assert_eq!(step.artifacts[1], "Modified: Cargo.toml");
    }

    #[test]
    fn test_parse_artifacts_with_checkboxes() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | active |
| Last updated | 2026-02-03 |

#### Step 0: Test {#step-0}

**Artifacts:**
- [ ] New file: src/main.rs
- Modified: Cargo.toml
"#;

        let speck = parse_speck(content).unwrap();

        assert_eq!(speck.steps.len(), 1);
        let step = &speck.steps[0];
        assert_eq!(step.artifacts.len(), 2);
        assert_eq!(step.artifacts[0], "New file: src/main.rs");
        assert_eq!(step.artifacts[1], "Modified: Cargo.toml");
    }
}
