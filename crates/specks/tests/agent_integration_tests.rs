//! Agent integration tests
//!
//! These tests verify the agent definitions and their contracts.
//! Since agents are markdown files invoked by Claude Code, we test:
//! - Agent definitions exist and have correct frontmatter
//! - Agent contracts (inputs/outputs) are documented
//! - Inter-agent protocols are consistent

use std::fs;
use std::path::PathBuf;

/// Get the path to the agents directory
fn agents_dir() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // crates
    path.pop(); // specks root
    path.push("agents");
    path
}

/// Get the path to the .specks/runs directory (for testing run persistence)
fn runs_dir(temp_dir: &tempfile::TempDir) -> PathBuf {
    temp_dir.path().join(".specks").join("runs")
}

/// Parse agent frontmatter from markdown
fn parse_agent_frontmatter(content: &str) -> Option<(String, String, String, String)> {
    // Agent files have YAML frontmatter between --- markers
    let lines: Vec<&str> = content.lines().collect();
    if lines.first() != Some(&"---") {
        return None;
    }

    let mut name = String::new();
    let mut description = String::new();
    let mut tools = String::new();
    let mut model = String::new();

    for line in lines.iter().skip(1) {
        if *line == "---" {
            break;
        }
        {
            if let Some(value) = line.strip_prefix("name: ") {
                name = value.to_string();
            } else if let Some(value) = line.strip_prefix("description: ") {
                description = value.to_string();
            } else if let Some(value) = line.strip_prefix("tools: ") {
                tools = value.to_string();
            } else if let Some(value) = line.strip_prefix("model: ") {
                model = value.to_string();
            }
        }
    }

    if name.is_empty() {
        None
    } else {
        Some((name, description, tools, model))
    }
}

/// List of all expected execution agents
const EXECUTION_AGENTS: &[&str] = &[
    "specks-implementer",
    "specks-monitor",
    "specks-reviewer",
    "specks-auditor",
    "specks-logger",
    "specks-committer",
];

/// List of all agents (planning + execution + orchestration)
const ALL_AGENTS: &[&str] = &[
    "specks-director",
    "specks-clarifier",
    "specks-planner",
    "specks-critic",
    "specks-architect",
    "specks-implementer",
    "specks-monitor",
    "specks-reviewer",
    "specks-auditor",
    "specks-logger",
    "specks-committer",
    "specks-interviewer",
];

// =============================================================================
// Agent Definition Tests
// =============================================================================

#[test]
fn test_all_agent_definitions_exist() {
    let dir = agents_dir();
    for agent in ALL_AGENTS {
        let path = dir.join(format!("{}.md", agent));
        assert!(
            path.exists(),
            "Agent definition missing: {}",
            path.display()
        );
    }
}

#[test]
fn test_agent_definitions_have_valid_frontmatter() {
    let dir = agents_dir();
    for agent in ALL_AGENTS {
        let path = dir.join(format!("{}.md", agent));
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("Failed to read agent: {}", path.display()));

        let frontmatter = parse_agent_frontmatter(&content);
        assert!(
            frontmatter.is_some(),
            "Agent {} has invalid frontmatter",
            agent
        );

        let (name, description, tools, model) = frontmatter.unwrap();
        assert_eq!(name, *agent, "Agent name mismatch for {}", agent);
        assert!(
            !description.is_empty(),
            "Agent {} missing description",
            agent
        );
        assert!(!tools.is_empty(), "Agent {} missing tools", agent);
        assert!(!model.is_empty(), "Agent {} missing model", agent);
    }
}

#[test]
fn test_execution_agents_have_required_sections() {
    let dir = agents_dir();
    for agent in EXECUTION_AGENTS {
        let path = dir.join(format!("{}.md", agent));
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("Failed to read agent: {}", path.display()));

        // All execution agents should document their role
        assert!(
            content.contains("## Your Role"),
            "Agent {} missing 'Your Role' section",
            agent
        );

        // All execution agents should document inputs they receive
        assert!(
            content.contains("## Inputs You Receive") || content.contains("From the director"),
            "Agent {} missing input documentation",
            agent
        );
    }
}

// =============================================================================
// Critic Agent Tests
// =============================================================================

#[test]
fn test_critic_reviews_plan_quality() {
    let path = agents_dir().join("specks-critic.md");
    let content = fs::read_to_string(&path).expect("Failed to read critic agent");

    // Critic must check completeness
    assert!(
        content.contains("completeness") || content.contains("Completeness"),
        "Critic agent must check plan completeness"
    );

    // Critic must check implementability
    assert!(
        content.contains("implementab") || content.contains("Implementab"),
        "Critic agent must check implementability"
    );

    // Critic must check sequencing
    assert!(
        content.contains("sequenc") || content.contains("Sequenc"),
        "Critic agent must check sequencing"
    );

    // Critic must write report
    assert!(
        content.contains("critic-report.md"),
        "Critic agent must write critic-report.md"
    );
}

#[test]
fn test_critic_provides_actionable_recommendations() {
    let path = agents_dir().join("specks-critic.md");
    let content = fs::read_to_string(&path).expect("Failed to read critic agent");

    // Critic must document APPROVE/REVISE/REJECT
    assert!(
        content.contains("APPROVE") && content.contains("REVISE") && content.contains("REJECT"),
        "Critic agent must document all recommendation types"
    );
}

#[test]
fn test_critic_complements_reviewer_and_auditor() {
    let path = agents_dir().join("specks-critic.md");
    let content = fs::read_to_string(&path).expect("Failed to read critic agent");

    // Critic should explain how it differs from reviewer and auditor
    assert!(
        content.contains("Reviewer") && content.contains("Auditor"),
        "Critic agent must explain relationship to reviewer and auditor"
    );

    // Critic focuses on plans, not code
    assert!(
        content.contains("before implementation") || content.contains("planning phase"),
        "Critic agent must clarify it runs before implementation"
    );
}

// =============================================================================
// Implementer Agent Tests
// =============================================================================

#[test]
fn test_implementer_documents_halt_signal_checking() {
    let path = agents_dir().join("specks-implementer.md");
    let content = fs::read_to_string(&path).expect("Failed to read implementer agent");

    // Implementer must check for halt signals
    assert!(
        content.contains(".halt") || content.contains("halt signal"),
        "Implementer agent must document halt signal checking"
    );

    // Implementer should document partial completion
    assert!(
        content.contains("partial") || content.contains("Partial"),
        "Implementer agent must document partial completion handling"
    );
}

#[test]
fn test_implementer_invokes_implement_plan_skill() {
    let path = agents_dir().join("specks-implementer.md");
    let content = fs::read_to_string(&path).expect("Failed to read implementer agent");

    assert!(
        content.contains("implement-plan"),
        "Implementer agent must invoke implement-plan skill"
    );
}

// =============================================================================
// Monitor Agent Tests
// =============================================================================

#[test]
fn test_monitor_documents_drift_detection() {
    let path = agents_dir().join("specks-monitor.md");
    let content = fs::read_to_string(&path).expect("Failed to read monitor agent");

    // Monitor must document drift detection
    assert!(
        content.contains("drift") || content.contains("Drift"),
        "Monitor agent must document drift detection"
    );

    // Monitor must document expected_touch_set
    assert!(
        content.contains("expected_touch_set"),
        "Monitor agent must document expected_touch_set usage"
    );

    // Monitor must document halt signal writing
    assert!(
        content.contains(".halt") || content.contains("halt signal"),
        "Monitor agent must document halt signal writing"
    );
}

#[test]
fn test_monitor_documents_return_format() {
    let path = agents_dir().join("specks-monitor.md");
    let content = fs::read_to_string(&path).expect("Failed to read monitor agent");

    // Monitor must document CONTINUE/PAUSE/HALT
    assert!(
        content.contains("CONTINUE") && content.contains("PAUSE") && content.contains("HALT"),
        "Monitor agent must document all status values"
    );
}

// =============================================================================
// Reviewer Agent Tests
// =============================================================================

#[test]
fn test_reviewer_checks_plan_adherence() {
    let path = agents_dir().join("specks-reviewer.md");
    let content = fs::read_to_string(&path).expect("Failed to read reviewer agent");

    // Reviewer must check tasks
    assert!(
        content.contains("tasks") || content.contains("Tasks"),
        "Reviewer agent must check task completion"
    );

    // Reviewer must check tests
    assert!(
        content.contains("tests") || content.contains("Tests"),
        "Reviewer agent must verify tests"
    );

    // Reviewer must write report
    assert!(
        content.contains("reviewer-report.md"),
        "Reviewer agent must write reviewer-report.md"
    );
}

// =============================================================================
// Auditor Agent Tests
// =============================================================================

#[test]
fn test_auditor_checks_code_quality() {
    let path = agents_dir().join("specks-auditor.md");
    let content = fs::read_to_string(&path).expect("Failed to read auditor agent");

    // Auditor must check structure
    assert!(
        content.contains("structure") || content.contains("Structure"),
        "Auditor agent must check code structure"
    );

    // Auditor must check security
    assert!(
        content.contains("security") || content.contains("Security"),
        "Auditor agent must check security"
    );

    // Auditor must write report
    assert!(
        content.contains("auditor-report.md"),
        "Auditor agent must write auditor-report.md"
    );
}

#[test]
fn test_auditor_runs_at_different_granularities() {
    let path = agents_dir().join("specks-auditor.md");
    let content = fs::read_to_string(&path).expect("Failed to read auditor agent");

    // Auditor must document when it runs
    assert!(
        content.contains("step") && content.contains("milestone") && content.contains("completion"),
        "Auditor agent must document step/milestone/completion triggers"
    );
}

// =============================================================================
// Logger Agent Tests
// =============================================================================

#[test]
fn test_logger_invokes_update_log_skill() {
    let path = agents_dir().join("specks-logger.md");
    let content = fs::read_to_string(&path).expect("Failed to read logger agent");

    assert!(
        content.contains("update-specks-implementation-log")
            || content.contains("update-plan-implementation-log"),
        "Logger agent must invoke implementation log skill"
    );
}

// =============================================================================
// Committer Agent Tests
// =============================================================================

#[test]
fn test_committer_respects_commit_policy() {
    let path = agents_dir().join("specks-committer.md");
    let content = fs::read_to_string(&path).expect("Failed to read committer agent");

    // Committer must document both policies
    assert!(
        content.contains("manual") && content.contains("auto"),
        "Committer agent must document manual and auto policies"
    );

    // Committer must invoke prepare-git-commit-message skill
    assert!(
        content.contains("prepare-git-commit-message"),
        "Committer agent must invoke prepare-git-commit-message skill"
    );

    // Committer must write committer-prep.md
    assert!(
        content.contains("committer-prep.md"),
        "Committer agent must write committer-prep.md"
    );
}

// =============================================================================
// Director Agent Tests
// =============================================================================

#[test]
fn test_director_documents_full_execution_loop() {
    let path = agents_dir().join("specks-director.md");
    let content = fs::read_to_string(&path).expect("Failed to read director agent");

    // Director must document the execution loop
    assert!(
        content.contains("Execution Mode Workflow") || content.contains("execution loop"),
        "Director agent must document execution workflow"
    );

    // Director must invoke all execution agents
    for agent in &[
        "architect",
        "implementer",
        "monitor",
        "reviewer",
        "auditor",
        "logger",
        "committer",
    ] {
        assert!(
            content.to_lowercase().contains(agent),
            "Director agent must mention {}",
            agent
        );
    }
}

#[test]
fn test_director_uses_critic_in_planning_mode() {
    let path = agents_dir().join("specks-director.md");
    let content = fs::read_to_string(&path).expect("Failed to read director agent");

    // Director must document planning mode workflow
    assert!(
        content.contains("Planning Mode Workflow"),
        "Director agent must document planning mode"
    );

    // Director must use critic (not auditor) for plan review
    assert!(
        content.contains("Invoke CRITIC") || content.contains("CRITIC to review plan"),
        "Director must use CRITIC for plan review in planning mode"
    );

    // Director should mention critic in agent list
    assert!(
        content.contains("critic"),
        "Director must list critic in orchestrated agents"
    );
}

#[test]
fn test_director_documents_halt_handling() {
    let path = agents_dir().join("specks-director.md");
    let content = fs::read_to_string(&path).expect("Failed to read director agent");

    // Director must document halt handling
    assert!(
        content.contains("Halt") || content.contains("HALT"),
        "Director agent must document halt handling"
    );

    // Director must document halt signal file
    assert!(
        content.contains(".halt"),
        "Director agent must reference .halt signal file"
    );
}

#[test]
fn test_director_documents_escalation() {
    let path = agents_dir().join("specks-director.md");
    let content = fs::read_to_string(&path).expect("Failed to read director agent");

    // Director must document escalation
    assert!(
        content.contains("Escalation") || content.contains("escalation"),
        "Director agent must document escalation paths"
    );
}

// =============================================================================
// Halt Signal Protocol Tests
// =============================================================================

#[test]
fn test_halt_signal_file_format_documented() {
    let dir = agents_dir();

    // Monitor should document halt file format
    let monitor_content =
        fs::read_to_string(dir.join("specks-monitor.md")).expect("Failed to read monitor agent");

    assert!(
        monitor_content.contains("reason") && monitor_content.contains("drift_type"),
        "Monitor must document halt file format"
    );

    // Director should document halt file format
    let director_content =
        fs::read_to_string(dir.join("specks-director.md")).expect("Failed to read director agent");

    assert!(
        director_content.contains("reason") && director_content.contains("drift_type"),
        "Director must document halt file format"
    );
}

// =============================================================================
// Run Directory Tests
// =============================================================================

#[test]
fn test_run_directory_structure_documented() {
    let path = agents_dir().join("specks-director.md");
    let content = fs::read_to_string(&path).expect("Failed to read director agent");

    // Director must document run directory structure
    assert!(
        content.contains("runs/") || content.contains(".specks/runs"),
        "Director must document run directory location"
    );

    // Director must document expected files (per specks-3.md #run-structure)
    let expected_files = [
        "metadata.json",
        "architect.json",
        "reviewer.json",
        "auditor.json",
    ];

    for file in expected_files {
        assert!(
            content.contains(file),
            "Director must document {} in run directory",
            file
        );
    }
}

// =============================================================================
// Integration Test: Simulated Workflow
// =============================================================================

#[test]
fn test_create_run_directory_structure() {
    let temp = tempfile::tempdir().expect("failed to create temp dir");

    // Create .specks directory
    fs::create_dir(temp.path().join(".specks")).expect("failed to create .specks");

    // Create runs directory
    let runs = runs_dir(&temp);
    fs::create_dir_all(&runs).expect("failed to create runs dir");

    // Create a UUID-based run directory
    let run_uuid = "test-run-12345";
    let run_dir = runs.join(run_uuid);
    fs::create_dir(&run_dir).expect("failed to create run dir");

    // Create invocation.json
    let invocation = serde_json::json!({
        "uuid": run_uuid,
        "timestamp": "2026-02-04T12:00:00Z",
        "speck": ".specks/specks-test.md",
        "mode": "execute",
        "commit_policy": "manual",
        "checkpoint_mode": "step",
        "start_step": null,
        "end_step": null
    });
    fs::write(
        run_dir.join("invocation.json"),
        serde_json::to_string_pretty(&invocation).unwrap(),
    )
    .expect("failed to write invocation.json");

    // Verify structure
    assert!(run_dir.join("invocation.json").exists());

    // Create halt signal file
    let halt = serde_json::json!({
        "reason": "drift_detected",
        "drift_type": "wrong_files",
        "drift_severity": "high",
        "timestamp": "2026-02-04T12:34:56Z",
        "description": "Test halt",
        "files_of_concern": ["test.rs"],
        "recommendation": "return_to_architect"
    });
    fs::write(
        run_dir.join(".halt"),
        serde_json::to_string_pretty(&halt).unwrap(),
    )
    .expect("failed to write .halt");

    assert!(run_dir.join(".halt").exists());

    // Create status.json
    let status = serde_json::json!({
        "uuid": run_uuid,
        "outcome": "halted",
        "steps_completed": [],
        "steps_remaining": ["#step-0"],
        "current_step": "#step-0",
        "halt_reason": "drift_detected",
        "errors": [],
        "timestamp_start": "2026-02-04T12:00:00Z",
        "timestamp_end": "2026-02-04T12:34:56Z"
    });
    fs::write(
        run_dir.join("status.json"),
        serde_json::to_string_pretty(&status).unwrap(),
    )
    .expect("failed to write status.json");

    assert!(run_dir.join("status.json").exists());
}

#[test]
fn test_reviewer_and_auditor_produce_complementary_reports() {
    // This test verifies the conceptual separation documented in D13
    let dir = agents_dir();

    let reviewer_content =
        fs::read_to_string(dir.join("specks-reviewer.md")).expect("Failed to read reviewer agent");
    let auditor_content =
        fs::read_to_string(dir.join("specks-auditor.md")).expect("Failed to read auditor agent");

    // Reviewer focuses on plan adherence
    assert!(
        reviewer_content.contains("plan adherence")
            || reviewer_content.contains("Did the implementation match"),
        "Reviewer should focus on plan adherence"
    );

    // Auditor focuses on code quality
    assert!(
        auditor_content.contains("code quality") || auditor_content.contains("quality"),
        "Auditor should focus on code quality"
    );

    // They should complement each other, not overlap
    assert!(
        !reviewer_content.contains("security issues") && auditor_content.contains("security"),
        "Security should be auditor's concern, not reviewer's"
    );
}
