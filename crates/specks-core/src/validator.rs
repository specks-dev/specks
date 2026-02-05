//! Validation logic and rules
//!
//! Implements structural validation for speck documents per the skeleton format.
//! Validation rules are organized by severity (Error, Warning, Info) and can be
//! configured via validation levels (lenient, normal, strict).

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use crate::types::Speck;

/// Regex for valid anchor format (only a-z, 0-9, - allowed)
static VALID_ANCHOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9][a-z0-9-]*$").unwrap());

/// Regex for bead ID format (fallback validation)
/// Pattern: ^[a-z0-9][a-z0-9-]*-[a-z0-9]+(\.[0-9]+)*$
static VALID_BEAD_ID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9][a-z0-9-]*-[a-z0-9]+(\.[0-9]+)*$").unwrap());

/// Regex for unfilled placeholder pattern (<...>)
static PLACEHOLDER_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^<[^>]+>$").unwrap());

/// Regex to detect prose-style dependencies (e.g., "Step 0" instead of "#step-0")
static PROSE_DEPENDENCY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bstep\s+\d+").unwrap());

/// Regex for valid **References:** format - must contain [DNN] decision citations
/// Valid: "[D01] Decision name, [D02] Another"
/// Also valid: "Spec S01", "Table T01", "(#anchor)"
static DECISION_CITATION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[D\d{2}\]").unwrap());

/// Regex for anchor citations in References (must be in parentheses with # prefix)
static ANCHOR_CITATION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\(#[a-z0-9-]+(,\s*#[a-z0-9-]+)*\)").unwrap());

/// Result of validating a speck
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the speck is valid (no errors)
    pub valid: bool,
    /// List of validation issues
    pub issues: Vec<ValidationIssue>,
}

impl ValidationResult {
    /// Create a new empty validation result
    pub fn new() -> Self {
        Self {
            valid: true,
            issues: vec![],
        }
    }

    /// Add an issue and update validity
    pub fn add_issue(&mut self, issue: ValidationIssue) {
        if issue.severity == Severity::Error {
            self.valid = false;
        }
        self.issues.push(issue);
    }

    /// Count errors
    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count()
    }

    /// Count warnings
    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .count()
    }

    /// Count info messages
    pub fn info_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Info)
            .count()
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// A single validation issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// Error/warning code (e.g., "E001", "W001")
    pub code: String,
    /// Severity level
    pub severity: Severity,
    /// Human-readable message
    pub message: String,
    /// Line number (if applicable)
    pub line: Option<usize>,
    /// Anchor reference (if applicable)
    pub anchor: Option<String>,
}

impl ValidationIssue {
    /// Create a new validation issue
    pub fn new(code: &str, severity: Severity, message: String) -> Self {
        Self {
            code: code.to_string(),
            severity,
            message,
            line: None,
            anchor: None,
        }
    }

    /// Set the line number
    pub fn at_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Set the anchor reference
    pub fn with_anchor(mut self, anchor: &str) -> Self {
        self.anchor = Some(format!("#{}", anchor));
        self
    }
}

/// Severity level for validation issues
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Must fix
    Error,
    /// Should fix
    Warning,
    /// Optional/informational
    Info,
}

/// Validation level (strictness)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ValidationLevel {
    /// Lenient: Only report errors
    Lenient,
    /// Normal: Report errors and warnings (default)
    #[default]
    Normal,
    /// Strict: Report errors, warnings, and info
    Strict,
}

impl ValidationLevel {
    /// Parse from string representation
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "lenient" => ValidationLevel::Lenient,
            "strict" => ValidationLevel::Strict,
            _ => ValidationLevel::Normal,
        }
    }

    /// Check if this level includes warnings
    pub fn include_warnings(&self) -> bool {
        matches!(self, ValidationLevel::Normal | ValidationLevel::Strict)
    }

    /// Check if this level includes info messages
    pub fn include_info(&self) -> bool {
        matches!(self, ValidationLevel::Strict)
    }
}

/// Validation configuration
#[derive(Debug, Clone, Default)]
pub struct ValidationConfig {
    /// Validation strictness level
    pub level: ValidationLevel,
    /// Whether beads integration is enabled
    pub beads_enabled: bool,
    /// Whether to validate bead IDs (requires beads CLI)
    pub validate_bead_ids: bool,
}

/// Validate a parsed speck
pub fn validate_speck(speck: &Speck) -> ValidationResult {
    validate_speck_with_config(speck, &ValidationConfig::default())
}

/// Validate a parsed speck with configuration
pub fn validate_speck_with_config(speck: &Speck, config: &ValidationConfig) -> ValidationResult {
    let mut result = ValidationResult::new();

    // Build anchor map for reference validation
    let anchor_map: HashMap<String, usize> = speck
        .anchors
        .iter()
        .filter(|a| !a.name.contains("(duplicate)"))
        .map(|a| (a.name.clone(), a.line))
        .collect();

    // === ERROR CHECKS ===

    // E001: Check for required sections
    check_required_sections(speck, &mut result);

    // E002: Check for required metadata fields
    check_required_metadata(speck, &mut result);

    // E003: Check metadata Status value
    check_metadata_status(speck, &mut result);

    // E004: Check steps have References line
    check_step_references(speck, &mut result);

    // E005: Check anchor format
    check_anchor_format(speck, &mut result);

    // E006: Check for duplicate anchors
    check_duplicate_anchors(speck, &mut result);

    // E010: Check dependency references
    check_dependency_references(speck, &anchor_map, &mut result);

    // E011: Check for circular dependencies
    check_circular_dependencies(speck, &mut result);

    // E017: Check **Depends on:** format (must use anchor refs like #step-N)
    check_depends_on_format(speck, &mut result);

    // E018: Check **References:** format (must have [DNN] decision citations)
    check_references_format(speck, &mut result);

    // E012: Check bead ID format (when beads enabled)
    if config.beads_enabled && config.validate_bead_ids {
        check_bead_id_format(speck, &mut result);
    }

    // E014: Check Beads Root exists (when beads enabled and set)
    // Note: Actual bead existence check requires beads CLI, so we just validate format here
    // The CLI layer can do the actual existence check

    // E015: Check step bead exists (when beads enabled)
    // Note: Same as above - format validation only, CLI does existence check

    // === WARNING CHECKS ===
    if config.level.include_warnings() {
        // W001: Decisions without DECIDED/OPEN status
        check_decision_status(speck, &mut result);

        // W002: Questions without resolution status
        check_question_resolution(speck, &mut result);

        // W003: Steps without checkpoint items
        check_step_checkpoints(speck, &mut result);

        // W004: Steps without test items
        check_step_tests(speck, &mut result);

        // W005: References citing non-existent anchors
        check_reference_anchors(speck, &anchor_map, &mut result);

        // W006: Metadata fields with unfilled placeholders
        check_metadata_placeholders(speck, &mut result);

        // W007: Step (other than Step 0) has no dependencies
        check_step_dependencies(speck, &mut result);

        // W008: Bead ID present but beads integration not enabled
        if !config.beads_enabled {
            check_bead_without_integration(speck, &mut result);
        }
    }

    // === INFO CHECKS ===
    if config.level.include_info() {
        // I001: Document exceeds recommended size (2000+ lines)
        check_document_size(speck, &mut result);

        // I002: Deep dive sections exceed 50% of document
        // Note: This would require parsing deep dive sections, which we don't currently track
        // Skipping for now as it's informational
    }

    result
}

// === ERROR CHECK IMPLEMENTATIONS ===

/// E001: Check for required sections
fn check_required_sections(speck: &Speck, result: &mut ValidationResult) {
    let anchor_names: HashSet<&str> = speck
        .anchors
        .iter()
        .filter(|a| !a.name.contains("(duplicate)"))
        .map(|a| a.name.as_str())
        .collect();

    let required_sections = [
        ("plan-metadata", "Plan Metadata"),
        ("phase-overview", "Phase Overview"),
        ("design-decisions", "Design Decisions"),
        ("execution-steps", "Execution Steps"),
        ("deliverables", "Deliverables"),
    ];

    for (anchor, name) in required_sections {
        // Check if section exists by looking for the anchor or a variant
        let has_section = anchor_names.contains(anchor)
            || anchor_names
                .iter()
                .any(|a| a.ends_with(&format!("-{}", anchor)))
            || (anchor == "execution-steps" && !speck.steps.is_empty());

        if !has_section {
            result.add_issue(ValidationIssue::new(
                "E001",
                Severity::Error,
                format!("Missing required section: {}", name),
            ));
        }
    }
}

/// E002: Check for required metadata fields
fn check_required_metadata(speck: &Speck, result: &mut ValidationResult) {
    // Required fields: Owner, Status, Last updated
    if speck.metadata.owner.is_none() {
        result.add_issue(ValidationIssue::new(
            "E002",
            Severity::Error,
            "Missing or empty required metadata field: Owner".to_string(),
        ));
    }

    if speck.metadata.status.is_none() {
        result.add_issue(ValidationIssue::new(
            "E002",
            Severity::Error,
            "Missing or empty required metadata field: Status".to_string(),
        ));
    }

    if speck.metadata.last_updated.is_none() {
        result.add_issue(ValidationIssue::new(
            "E002",
            Severity::Error,
            "Missing or empty required metadata field: Last updated".to_string(),
        ));
    }
}

/// E003: Check metadata Status value
fn check_metadata_status(speck: &Speck, result: &mut ValidationResult) {
    if let Some(status) = &speck.metadata.status {
        if !speck.metadata.is_valid_status() {
            result.add_issue(ValidationIssue::new(
                "E003",
                Severity::Error,
                format!(
                    "Invalid metadata Status value: {} (must be draft/active/done)",
                    status
                ),
            ));
        }
    }
}

/// E004: Check steps have References line
fn check_step_references(speck: &Speck, result: &mut ValidationResult) {
    for step in &speck.steps {
        if step.references.is_none() {
            result.add_issue(
                ValidationIssue::new(
                    "E004",
                    Severity::Error,
                    format!("Step {} missing References line", step.number),
                )
                .at_line(step.line)
                .with_anchor(&step.anchor),
            );
        }

        // Also check substeps
        for substep in &step.substeps {
            if substep.references.is_none() {
                result.add_issue(
                    ValidationIssue::new(
                        "E004",
                        Severity::Error,
                        format!("Step {} missing References line", substep.number),
                    )
                    .at_line(substep.line)
                    .with_anchor(&substep.anchor),
                );
            }
        }
    }
}

/// E005: Check anchor format (only a-z, 0-9, - allowed)
fn check_anchor_format(speck: &Speck, result: &mut ValidationResult) {
    for anchor in &speck.anchors {
        // Skip duplicate markers
        if anchor.name.contains("(duplicate)") {
            continue;
        }

        if !VALID_ANCHOR.is_match(&anchor.name) {
            result.add_issue(
                ValidationIssue::new(
                    "E005",
                    Severity::Error,
                    format!("Invalid anchor format: {{{}}}", anchor.name),
                )
                .at_line(anchor.line),
            );
        }
    }
}

/// E006: Check for duplicate anchors
fn check_duplicate_anchors(speck: &Speck, result: &mut ValidationResult) {
    let mut seen: HashMap<&str, usize> = HashMap::new();

    for anchor in &speck.anchors {
        // Extract original name from "(duplicate)" marker
        let name = if anchor.name.contains("(duplicate)") {
            anchor
                .name
                .split(" (duplicate)")
                .next()
                .unwrap_or(&anchor.name)
        } else {
            &anchor.name
        };

        if let Some(&first_line) = seen.get(name) {
            result.add_issue(
                ValidationIssue::new(
                    "E006",
                    Severity::Error,
                    format!("Duplicate anchor: {}", name),
                )
                .at_line(anchor.line)
                .with_anchor(name),
            );
            // Add note about first occurrence
            result.issues.last_mut().unwrap().message =
                format!("Duplicate anchor: {} (first at line {})", name, first_line);
        } else {
            seen.insert(name, anchor.line);
        }
    }
}

/// E010: Check dependency references point to existing step anchors
fn check_dependency_references(
    speck: &Speck,
    anchor_map: &HashMap<String, usize>,
    result: &mut ValidationResult,
) {
    // Collect all step anchors
    let step_anchors: HashSet<&str> = speck
        .steps
        .iter()
        .flat_map(|s| {
            std::iter::once(s.anchor.as_str()).chain(s.substeps.iter().map(|ss| ss.anchor.as_str()))
        })
        .collect();

    for step in &speck.steps {
        for dep in &step.depends_on {
            if !step_anchors.contains(dep.as_str()) && !anchor_map.contains_key(dep) {
                result.add_issue(
                    ValidationIssue::new(
                        "E010",
                        Severity::Error,
                        format!("Dependency references non-existent step anchor: {}", dep),
                    )
                    .at_line(step.line)
                    .with_anchor(&step.anchor),
                );
            }
        }

        for substep in &step.substeps {
            for dep in &substep.depends_on {
                if !step_anchors.contains(dep.as_str()) && !anchor_map.contains_key(dep) {
                    result.add_issue(
                        ValidationIssue::new(
                            "E010",
                            Severity::Error,
                            format!("Dependency references non-existent step anchor: {}", dep),
                        )
                        .at_line(substep.line)
                        .with_anchor(&substep.anchor),
                    );
                }
            }
        }
    }
}

/// E011: Check for circular dependencies using DFS
fn check_circular_dependencies(speck: &Speck, result: &mut ValidationResult) {
    // Build dependency graph
    let mut deps: HashMap<&str, Vec<&str>> = HashMap::new();

    for step in &speck.steps {
        deps.insert(
            step.anchor.as_str(),
            step.depends_on.iter().map(|s| s.as_str()).collect(),
        );

        for substep in &step.substeps {
            deps.insert(
                substep.anchor.as_str(),
                substep.depends_on.iter().map(|s| s.as_str()).collect(),
            );
        }
    }

    // DFS to detect cycles
    let mut visited: HashSet<&str> = HashSet::new();
    let mut rec_stack: HashSet<&str> = HashSet::new();
    let mut path: Vec<&str> = Vec::new();

    for start in deps.keys() {
        if !visited.contains(start) {
            if let Some(cycle) = detect_cycle(start, &deps, &mut visited, &mut rec_stack, &mut path)
            {
                result.add_issue(ValidationIssue::new(
                    "E011",
                    Severity::Error,
                    format!("Circular dependency detected: {}", cycle),
                ));
            }
        }
    }
}

/// Helper for cycle detection
fn detect_cycle<'a>(
    node: &'a str,
    deps: &HashMap<&'a str, Vec<&'a str>>,
    visited: &mut HashSet<&'a str>,
    rec_stack: &mut HashSet<&'a str>,
    path: &mut Vec<&'a str>,
) -> Option<String> {
    visited.insert(node);
    rec_stack.insert(node);
    path.push(node);

    if let Some(neighbors) = deps.get(node) {
        for &neighbor in neighbors {
            if !visited.contains(neighbor) {
                if let Some(cycle) = detect_cycle(neighbor, deps, visited, rec_stack, path) {
                    return Some(cycle);
                }
            } else if rec_stack.contains(neighbor) {
                // Found a cycle - construct cycle string
                let cycle_start = path.iter().position(|&n| n == neighbor).unwrap();
                let cycle_nodes: Vec<&str> = path[cycle_start..].to_vec();
                return Some(format!("{} -> {}", cycle_nodes.join(" -> "), neighbor));
            }
        }
    }

    path.pop();
    rec_stack.remove(node);
    None
}

/// E017: Check **Depends on:** format
/// Must use anchor references like #step-0, not prose like "Step 0"
fn check_depends_on_format(speck: &Speck, result: &mut ValidationResult) {
    for step in &speck.steps {
        // Check if step has dependencies declared but in wrong format
        if !step.depends_on.is_empty() {
            // Dependencies should be anchor refs - check if any look like prose
            for dep in &step.depends_on {
                // Valid: "step-0", "step-1-2"
                // Invalid: "Step 0", "step 0", etc.
                if !dep.starts_with("step-") && !dep.contains('-') {
                    result.add_issue(
                        ValidationIssue::new(
                            "E017",
                            Severity::Error,
                            format!(
                                "Invalid dependency format: '{}' (must be anchor ref like 'step-0', not prose)",
                                dep
                            ),
                        )
                        .at_line(step.line)
                        .with_anchor(&step.anchor),
                    );
                }
            }
        }

        // Also check substeps
        for substep in &step.substeps {
            for dep in &substep.depends_on {
                if !dep.starts_with("step-") && !dep.contains('-') {
                    result.add_issue(
                        ValidationIssue::new(
                            "E017",
                            Severity::Error,
                            format!(
                                "Invalid dependency format: '{}' (must be anchor ref like 'step-0', not prose)",
                                dep
                            ),
                        )
                        .at_line(substep.line)
                        .with_anchor(&substep.anchor),
                    );
                }
            }
        }
    }
}

/// E018: Check **References:** format
/// Must contain decision citations in [DNN] format (e.g., [D01], [D02])
fn check_references_format(speck: &Speck, result: &mut ValidationResult) {
    for step in &speck.steps {
        if let Some(refs) = &step.references {
            // Check for decision citations [DNN]
            let has_decision_citation = DECISION_CITATION.is_match(refs);

            // Check for anchor citations (#anchor) in parentheses
            let has_anchor_citation = ANCHOR_CITATION.is_match(refs);

            // Check for vague references
            let is_vague = refs.to_lowercase().contains("see above")
                || refs.to_lowercase().contains("n/a")
                || refs.to_lowercase().contains("see below")
                || refs.to_lowercase().contains("see design")
                || refs.trim().is_empty();

            // References should have decision citations OR anchor citations, not be vague
            if is_vague && !has_decision_citation && !has_anchor_citation {
                result.add_issue(
                    ValidationIssue::new(
                        "E018",
                        Severity::Error,
                        format!(
                            "Step {} has vague References '{}' (must cite [DNN] decisions or (#anchor) refs)",
                            step.number, refs
                        ),
                    )
                    .at_line(step.line)
                    .with_anchor(&step.anchor),
                );
            }

            // Check for prose-style dependency mentions in References (should be in Depends on)
            if PROSE_DEPENDENCY.is_match(refs) && !has_decision_citation {
                result.add_issue(
                    ValidationIssue::new(
                        "E018",
                        Severity::Error,
                        format!(
                            "Step {} References contains prose step reference '{}' (use [DNN] format for decisions, (#anchor) for section refs)",
                            step.number, refs
                        ),
                    )
                    .at_line(step.line)
                    .with_anchor(&step.anchor),
                );
            }
        }

        // Also check substeps
        for substep in &step.substeps {
            if let Some(refs) = &substep.references {
                let has_decision_citation = DECISION_CITATION.is_match(refs);
                let has_anchor_citation = ANCHOR_CITATION.is_match(refs);
                let is_vague = refs.to_lowercase().contains("see above")
                    || refs.to_lowercase().contains("n/a")
                    || refs.to_lowercase().contains("see below")
                    || refs.trim().is_empty();

                if is_vague && !has_decision_citation && !has_anchor_citation {
                    result.add_issue(
                        ValidationIssue::new(
                            "E018",
                            Severity::Error,
                            format!(
                                "Step {} has vague References '{}' (must cite [DNN] decisions or (#anchor) refs)",
                                substep.number, refs
                            ),
                        )
                        .at_line(substep.line)
                        .with_anchor(&substep.anchor),
                    );
                }
            }
        }
    }
}

/// E012: Check bead ID format
fn check_bead_id_format(speck: &Speck, result: &mut ValidationResult) {
    // Check beads root ID
    if let Some(root_id) = &speck.metadata.beads_root_id {
        if !VALID_BEAD_ID.is_match(root_id) {
            result.add_issue(ValidationIssue::new(
                "E012",
                Severity::Error,
                format!("Invalid bead ID format: {}", root_id),
            ));
        }
    }

    // Check step bead IDs
    for step in &speck.steps {
        if let Some(bead_id) = &step.bead_id {
            if !VALID_BEAD_ID.is_match(bead_id) {
                result.add_issue(
                    ValidationIssue::new(
                        "E012",
                        Severity::Error,
                        format!("Invalid bead ID format: {}", bead_id),
                    )
                    .at_line(step.line)
                    .with_anchor(&step.anchor),
                );
            }
        }

        for substep in &step.substeps {
            if let Some(bead_id) = &substep.bead_id {
                if !VALID_BEAD_ID.is_match(bead_id) {
                    result.add_issue(
                        ValidationIssue::new(
                            "E012",
                            Severity::Error,
                            format!("Invalid bead ID format: {}", bead_id),
                        )
                        .at_line(substep.line)
                        .with_anchor(&substep.anchor),
                    );
                }
            }
        }
    }
}

// === WARNING CHECK IMPLEMENTATIONS ===

/// W001: Decisions without DECIDED/OPEN status
fn check_decision_status(speck: &Speck, result: &mut ValidationResult) {
    for decision in &speck.decisions {
        if decision.status.is_none() {
            result.add_issue(
                ValidationIssue::new(
                    "W001",
                    Severity::Warning,
                    format!("Decision {} missing status", decision.id),
                )
                .at_line(decision.line),
            );
        }
    }
}

/// W002: Questions without resolution status
fn check_question_resolution(speck: &Speck, result: &mut ValidationResult) {
    for question in &speck.questions {
        if question.resolution.is_none() {
            result.add_issue(
                ValidationIssue::new(
                    "W002",
                    Severity::Warning,
                    format!("Question {} missing resolution", question.id),
                )
                .at_line(question.line),
            );
        }
    }
}

/// W003: Steps without checkpoint items
fn check_step_checkpoints(speck: &Speck, result: &mut ValidationResult) {
    for step in &speck.steps {
        if step.checkpoints.is_empty() {
            result.add_issue(
                ValidationIssue::new(
                    "W003",
                    Severity::Warning,
                    format!("Step {} has no checkpoint items", step.number),
                )
                .at_line(step.line)
                .with_anchor(&step.anchor),
            );
        }
    }
}

/// W004: Steps without test items
fn check_step_tests(speck: &Speck, result: &mut ValidationResult) {
    for step in &speck.steps {
        if step.tests.is_empty() {
            result.add_issue(
                ValidationIssue::new(
                    "W004",
                    Severity::Warning,
                    format!("Step {} has no test items", step.number),
                )
                .at_line(step.line)
                .with_anchor(&step.anchor),
            );
        }
    }
}

/// W005: References citing non-existent anchors
fn check_reference_anchors(
    speck: &Speck,
    anchor_map: &HashMap<String, usize>,
    result: &mut ValidationResult,
) {
    let anchor_ref_pattern = Regex::new(r"#([a-z0-9-]+)").unwrap();

    for step in &speck.steps {
        if let Some(refs) = &step.references {
            for cap in anchor_ref_pattern.captures_iter(refs) {
                let ref_anchor = cap.get(1).unwrap().as_str();
                if !anchor_map.contains_key(ref_anchor) {
                    result.add_issue(
                        ValidationIssue::new(
                            "W005",
                            Severity::Warning,
                            format!("Reference to non-existent anchor: #{}", ref_anchor),
                        )
                        .at_line(step.line)
                        .with_anchor(&step.anchor),
                    );
                }
            }
        }
    }
}

/// W006: Metadata fields with unfilled placeholders
fn check_metadata_placeholders(speck: &Speck, result: &mut ValidationResult) {
    let fields = [
        ("Owner", &speck.metadata.owner),
        ("Status", &speck.metadata.status),
        ("Target branch", &speck.metadata.target_branch),
        ("Tracking issue/PR", &speck.metadata.tracking),
        ("Last updated", &speck.metadata.last_updated),
    ];

    for (name, value) in fields {
        if let Some(v) = value {
            if PLACEHOLDER_PATTERN.is_match(v) {
                result.add_issue(ValidationIssue::new(
                    "W006",
                    Severity::Warning,
                    format!("Unfilled placeholder in metadata: {} contains {}", name, v),
                ));
            }
        }
    }
}

/// W007: Step (other than Step 0) has no dependencies
fn check_step_dependencies(speck: &Speck, result: &mut ValidationResult) {
    for step in &speck.steps {
        // Step 0 is allowed to have no dependencies
        if step.number != "0" && step.depends_on.is_empty() {
            result.add_issue(
                ValidationIssue::new(
                    "W007",
                    Severity::Warning,
                    format!("Step {} has no dependencies", step.number),
                )
                .at_line(step.line)
                .with_anchor(&step.anchor),
            );
        }
    }
}

/// W008: Bead ID present but beads integration not enabled
fn check_bead_without_integration(speck: &Speck, result: &mut ValidationResult) {
    if speck.metadata.beads_root_id.is_some() {
        result.add_issue(ValidationIssue::new(
            "W008",
            Severity::Warning,
            "Beads Root ID present but beads integration not enabled".to_string(),
        ));
    }

    for step in &speck.steps {
        if step.bead_id.is_some() {
            result.add_issue(
                ValidationIssue::new(
                    "W008",
                    Severity::Warning,
                    format!(
                        "Step {} has bead ID but beads integration not enabled",
                        step.number
                    ),
                )
                .at_line(step.line)
                .with_anchor(&step.anchor),
            );
        }

        for substep in &step.substeps {
            if substep.bead_id.is_some() {
                result.add_issue(
                    ValidationIssue::new(
                        "W008",
                        Severity::Warning,
                        format!(
                            "Step {} has bead ID but beads integration not enabled",
                            substep.number
                        ),
                    )
                    .at_line(substep.line)
                    .with_anchor(&substep.anchor),
                );
            }
        }
    }
}

// === INFO CHECK IMPLEMENTATIONS ===

/// I001: Document exceeds recommended size (2000+ lines)
fn check_document_size(speck: &Speck, result: &mut ValidationResult) {
    let line_count = speck.raw_content.lines().count();
    if line_count >= 2000 {
        result.add_issue(ValidationIssue::new(
            "I001",
            Severity::Info,
            format!("Document exceeds recommended size ({} lines)", line_count),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_speck;

    #[test]
    fn test_validate_minimal_valid_speck() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test Owner |
| Status | draft |
| Target branch | main |
| Last updated | 2026-02-03 |

### Phase Overview {#phase-overview}

Overview text.

### Design Decisions {#design-decisions}

#### [D01] Test Decision (DECIDED) {#d01-test}

Decision text.

### 1.0.5 Execution Steps {#execution-steps}

#### Step 0: Bootstrap {#step-0}

**References:** [D01] Test decision

**Tasks:**
- [ ] Task one

**Tests:**
- [ ] Test one

**Checkpoint:**
- [ ] Check one

### Deliverables {#deliverables}

Deliverable text.
"#;

        let speck = parse_speck(content).unwrap();
        let result = validate_speck(&speck);

        assert!(
            result.valid,
            "Expected valid speck, got issues: {:?}",
            result.issues
        );
    }

    #[test]
    fn test_e001_missing_section() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test Owner |
| Status | draft |
| Last updated | 2026-02-03 |

### 1.0.5 Execution Steps {#execution-steps}

#### Step 0: Test {#step-0}

**References:** Test

**Tasks:**
- [ ] Task
"#;

        let speck = parse_speck(content).unwrap();
        let result = validate_speck(&speck);

        assert!(!result.valid);
        let e001_issues: Vec<_> = result.issues.iter().filter(|i| i.code == "E001").collect();
        assert!(
            !e001_issues.is_empty(),
            "Expected E001 errors for missing sections"
        );
    }

    #[test]
    fn test_e002_missing_metadata() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | |
| Status | draft |
| Last updated | 2026-02-03 |
"#;

        let speck = parse_speck(content).unwrap();
        let result = validate_speck(&speck);

        let e002_issues: Vec<_> = result.issues.iter().filter(|i| i.code == "E002").collect();
        assert!(
            !e002_issues.is_empty(),
            "Expected E002 error for missing Owner"
        );
    }

    #[test]
    fn test_e003_invalid_status() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | invalid |
| Last updated | 2026-02-03 |
"#;

        let speck = parse_speck(content).unwrap();
        let result = validate_speck(&speck);

        let e003_issues: Vec<_> = result.issues.iter().filter(|i| i.code == "E003").collect();
        assert_eq!(e003_issues.len(), 1);
        assert!(e003_issues[0].message.contains("invalid"));
    }

    #[test]
    fn test_e004_missing_references() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | draft |
| Last updated | 2026-02-03 |

#### Step 0: Test {#step-0}

**Tasks:**
- [ ] Task without references
"#;

        let speck = parse_speck(content).unwrap();
        let result = validate_speck(&speck);

        let e004_issues: Vec<_> = result.issues.iter().filter(|i| i.code == "E004").collect();
        assert_eq!(e004_issues.len(), 1);
    }

    #[test]
    fn test_e006_duplicate_anchors() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Section One {#duplicate-anchor}

### Section Two {#duplicate-anchor}
"#;

        let speck = parse_speck(content).unwrap();
        let result = validate_speck(&speck);

        let e006_issues: Vec<_> = result.issues.iter().filter(|i| i.code == "E006").collect();
        assert_eq!(e006_issues.len(), 1);
    }

    #[test]
    fn test_e010_invalid_dependency() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | draft |
| Last updated | 2026-02-03 |

#### Step 0: First {#step-0}

**References:** Test

**Tasks:**
- [ ] Task

#### Step 1: Second {#step-1}

**Depends on:** #nonexistent-step

**References:** Test

**Tasks:**
- [ ] Task
"#;

        let speck = parse_speck(content).unwrap();
        let result = validate_speck(&speck);

        let e010_issues: Vec<_> = result.issues.iter().filter(|i| i.code == "E010").collect();
        assert_eq!(e010_issues.len(), 1);
        assert!(e010_issues[0].message.contains("nonexistent-step"));
    }

    #[test]
    fn test_e011_circular_dependency() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | draft |
| Last updated | 2026-02-03 |

#### Step 1: First {#step-1}

**Depends on:** #step-2

**References:** Test

**Tasks:**
- [ ] Task

#### Step 2: Second {#step-2}

**Depends on:** #step-1

**References:** Test

**Tasks:**
- [ ] Task
"#;

        let speck = parse_speck(content).unwrap();
        let result = validate_speck(&speck);

        let e011_issues: Vec<_> = result.issues.iter().filter(|i| i.code == "E011").collect();
        assert_eq!(e011_issues.len(), 1);
        assert!(e011_issues[0].message.contains("Circular dependency"));
    }

    #[test]
    fn test_e012_invalid_bead_id() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | draft |
| Last updated | 2026-02-03 |
| Beads Root | `invalid bead id` |
"#;

        let speck = parse_speck(content).unwrap();
        let config = ValidationConfig {
            beads_enabled: true,
            validate_bead_ids: true,
            ..Default::default()
        };
        let result = validate_speck_with_config(&speck, &config);

        let e012_issues: Vec<_> = result.issues.iter().filter(|i| i.code == "E012").collect();
        assert_eq!(e012_issues.len(), 1);
    }

    #[test]
    fn test_w001_decision_missing_status() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | draft |
| Last updated | 2026-02-03 |

### Design Decisions {#design-decisions}

#### [D01] Test Decision {#d01-test}

Decision without status.
"#;

        let speck = parse_speck(content).unwrap();
        let result = validate_speck(&speck);

        let w001_issues: Vec<_> = result.issues.iter().filter(|i| i.code == "W001").collect();
        assert_eq!(w001_issues.len(), 1);
    }

    #[test]
    fn test_w002_question_missing_resolution() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | draft |
| Last updated | 2026-02-03 |

### Open Questions {#open-questions}

#### [Q01] Test Question {#q01-test}

Question without resolution.
"#;

        let speck = parse_speck(content).unwrap();
        let result = validate_speck(&speck);

        let w002_issues: Vec<_> = result.issues.iter().filter(|i| i.code == "W002").collect();
        assert_eq!(w002_issues.len(), 1);
    }

    #[test]
    fn test_w003_step_missing_checkpoints() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | draft |
| Last updated | 2026-02-03 |

#### Step 0: Test {#step-0}

**References:** Test

**Tasks:**
- [ ] Task only, no checkpoint
"#;

        let speck = parse_speck(content).unwrap();
        let result = validate_speck(&speck);

        let w003_issues: Vec<_> = result.issues.iter().filter(|i| i.code == "W003").collect();
        assert_eq!(w003_issues.len(), 1);
    }

    #[test]
    fn test_w004_step_missing_tests() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | draft |
| Last updated | 2026-02-03 |

#### Step 0: Test {#step-0}

**References:** Test

**Tasks:**
- [ ] Task only, no tests
"#;

        let speck = parse_speck(content).unwrap();
        let result = validate_speck(&speck);

        let w004_issues: Vec<_> = result.issues.iter().filter(|i| i.code == "W004").collect();
        assert_eq!(w004_issues.len(), 1);
    }

    #[test]
    fn test_w006_placeholder_in_metadata() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | <your name> |
| Status | draft |
| Last updated | 2026-02-03 |
"#;

        let speck = parse_speck(content).unwrap();
        let result = validate_speck(&speck);

        let w006_issues: Vec<_> = result.issues.iter().filter(|i| i.code == "W006").collect();
        assert_eq!(w006_issues.len(), 1);
        assert!(w006_issues[0].message.contains("<your name>"));
    }

    #[test]
    fn test_w007_step_no_dependencies() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | draft |
| Last updated | 2026-02-03 |

#### Step 0: First {#step-0}

**References:** Test

**Tasks:**
- [ ] Task

#### Step 1: Second no deps {#step-1}

**References:** Test

**Tasks:**
- [ ] Task without depends on
"#;

        let speck = parse_speck(content).unwrap();
        let result = validate_speck(&speck);

        let w007_issues: Vec<_> = result.issues.iter().filter(|i| i.code == "W007").collect();
        assert_eq!(w007_issues.len(), 1);
        assert!(w007_issues[0].message.contains("Step 1"));
    }

    #[test]
    fn test_w008_bead_without_integration() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | draft |
| Last updated | 2026-02-03 |

#### Step 0: Test {#step-0}

**Bead:** `bd-test123`

**References:** Test

**Tasks:**
- [ ] Task
"#;

        let speck = parse_speck(content).unwrap();
        // Default config has beads_enabled = false
        let result = validate_speck(&speck);

        let w008_issues: Vec<_> = result.issues.iter().filter(|i| i.code == "W008").collect();
        assert_eq!(w008_issues.len(), 1);
    }

    #[test]
    fn test_i001_document_size() {
        // Create a speck with 2000+ lines
        let mut content = String::from(
            r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | draft |
| Last updated | 2026-02-03 |

"#,
        );

        for i in 0..2000 {
            content.push_str(&format!("Line {}\n", i));
        }

        let speck = parse_speck(&content).unwrap();
        let config = ValidationConfig {
            level: ValidationLevel::Strict,
            ..Default::default()
        };
        let result = validate_speck_with_config(&speck, &config);

        let i001_issues: Vec<_> = result.issues.iter().filter(|i| i.code == "I001").collect();
        assert_eq!(i001_issues.len(), 1);
    }

    #[test]
    fn test_validation_levels() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | draft |
| Last updated | 2026-02-03 |

#### Step 0: Test {#step-0}

**References:** Test

**Tasks:**
- [ ] Task only
"#;

        let speck = parse_speck(content).unwrap();

        // Lenient: Only errors
        let lenient_config = ValidationConfig {
            level: ValidationLevel::Lenient,
            ..Default::default()
        };
        let lenient_result = validate_speck_with_config(&speck, &lenient_config);
        let lenient_warnings: Vec<_> = lenient_result
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .collect();
        assert!(
            lenient_warnings.is_empty(),
            "Lenient should not include warnings"
        );

        // Normal: Errors + warnings
        let normal_config = ValidationConfig {
            level: ValidationLevel::Normal,
            ..Default::default()
        };
        let normal_result = validate_speck_with_config(&speck, &normal_config);
        let normal_warnings: Vec<_> = normal_result
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .collect();
        assert!(
            !normal_warnings.is_empty(),
            "Normal should include warnings"
        );
    }

    #[test]
    fn test_valid_bead_id_format() {
        // Test valid bead IDs
        assert!(VALID_BEAD_ID.is_match("bd-abc123"));
        assert!(VALID_BEAD_ID.is_match("bd-root-1"));
        assert!(VALID_BEAD_ID.is_match("bd-test-1.2"));
        assert!(VALID_BEAD_ID.is_match("bd-test-1.2.3"));
        assert!(VALID_BEAD_ID.is_match("proj-feature-a1"));

        // Test invalid bead IDs
        assert!(!VALID_BEAD_ID.is_match("invalid"));
        assert!(!VALID_BEAD_ID.is_match("Invalid-ID"));
        assert!(!VALID_BEAD_ID.is_match("bd_underscore_1"));
        assert!(!VALID_BEAD_ID.is_match(""));
    }

    #[test]
    fn test_validation_result_counts() {
        let mut result = ValidationResult::new();

        result.add_issue(ValidationIssue::new(
            "E001",
            Severity::Error,
            "Error 1".to_string(),
        ));
        result.add_issue(ValidationIssue::new(
            "E002",
            Severity::Error,
            "Error 2".to_string(),
        ));
        result.add_issue(ValidationIssue::new(
            "W001",
            Severity::Warning,
            "Warning 1".to_string(),
        ));
        result.add_issue(ValidationIssue::new(
            "I001",
            Severity::Info,
            "Info 1".to_string(),
        ));

        assert_eq!(result.error_count(), 2);
        assert_eq!(result.warning_count(), 1);
        assert_eq!(result.info_count(), 1);
        assert!(!result.valid);
    }

    #[test]
    fn test_e018_vague_references() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | draft |
| Last updated | 2026-02-03 |

#### Step 0: Test {#step-0}

**References:** See above

**Tasks:**
- [ ] Task
"#;

        let speck = parse_speck(content).unwrap();
        let result = validate_speck(&speck);

        let e018_issues: Vec<_> = result.issues.iter().filter(|i| i.code == "E018").collect();
        assert_eq!(
            e018_issues.len(),
            1,
            "Expected E018 error for vague reference"
        );
        assert!(e018_issues[0].message.contains("vague"));
    }

    #[test]
    fn test_e018_valid_references() {
        let content = r#"## Phase 1.0: Test {#phase-1}

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | draft |
| Last updated | 2026-02-03 |

### Design Decisions {#design-decisions}

#### [D01] Test Decision (DECIDED) {#d01-test}

Decision text.

#### Step 0: Test {#step-0}

**References:** [D01] Test Decision, (#context, #strategy)

**Tasks:**
- [ ] Task

**Checkpoint:**
- [ ] Check
"#;

        let speck = parse_speck(content).unwrap();
        let result = validate_speck(&speck);

        let e018_issues: Vec<_> = result.issues.iter().filter(|i| i.code == "E018").collect();
        assert!(
            e018_issues.is_empty(),
            "Expected no E018 errors for valid references: {:?}",
            e018_issues
        );
    }

    #[test]
    fn test_decision_citation_regex() {
        assert!(DECISION_CITATION.is_match("[D01] Test"));
        assert!(DECISION_CITATION.is_match("[D99] Another"));
        assert!(DECISION_CITATION.is_match("Some text [D01] more text"));
        assert!(!DECISION_CITATION.is_match("D01 Test")); // Missing brackets
        assert!(!DECISION_CITATION.is_match("[D1] Test")); // Single digit
    }

    #[test]
    fn test_anchor_citation_regex() {
        assert!(ANCHOR_CITATION.is_match("(#context)"));
        assert!(ANCHOR_CITATION.is_match("(#context, #strategy)"));
        assert!(ANCHOR_CITATION.is_match("(#step-0, #step-1, #step-2)"));
        assert!(!ANCHOR_CITATION.is_match("#context")); // Missing parens
        assert!(!ANCHOR_CITATION.is_match("(context)")); // Missing #
    }
}
