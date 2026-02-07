//! Agent integration tests
//!
//! These tests verify the agent definitions and their contracts.
//! Since agents are markdown files invoked by Claude Code, we test:
//! - Agent definitions exist and have correct frontmatter
//! - Agent contracts (inputs/outputs) are documented
//! - Inter-agent protocols are consistent
//!
//! Note: As of Phase 3.0, many agents became skills (clarifier, critic, reviewer,
//! auditor, logger, committer). Monitor was eliminated. Only 5 agents remain:
//! director, planner, interviewer, architect, implementer.

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

/// List of all agents (Phase 3.0: 5 agents, others became skills)
/// Per specks-3.md #agent-summary, these are the remaining agents.
const ALL_AGENTS: &[&str] = &[
    "director",
    "planner",
    "interviewer",
    "architect",
    "implementer",
];

/// Execution agents that write code (architect plans, implementer executes)
const EXECUTION_AGENTS: &[&str] = &["architect", "implementer"];

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
        // (Input Contract, Inputs You Receive, or "From the director" are all valid)
        assert!(
            content.contains("## Inputs You Receive")
                || content.contains("## Input Contract")
                || content.contains("From the director"),
            "Agent {} missing input documentation",
            agent
        );
    }
}

#[test]
fn test_only_expected_agents_exist() {
    let dir = agents_dir();
    let entries: Vec<_> = fs::read_dir(&dir)
        .expect("Failed to read agents directory")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
        .collect();

    assert_eq!(
        entries.len(),
        5,
        "Expected exactly 5 agent files, found {}",
        entries.len()
    );

    // Verify no specks-*.md files exist (should have been renamed)
    for entry in &entries {
        let filename = entry.file_name().to_string_lossy().to_string();
        assert!(
            !filename.starts_with("specks-"),
            "Found old-style agent file: {} (should be renamed to remove specks- prefix)",
            filename
        );
    }
}

// =============================================================================
// Implementer Agent Tests
// =============================================================================

#[test]
fn test_implementer_documents_drift_detection() {
    let path = agents_dir().join("implementer.md");
    let content = fs::read_to_string(&path).expect("Failed to read implementer agent");

    // Implementer must self-monitor for drift (per specks-3.md #implementer-agent-contract)
    assert!(
        content.contains("drift") || content.contains("Drift"),
        "Implementer agent must document drift detection"
    );

    // Implementer should document self-halt behavior
    assert!(
        content.contains("self-halt")
            || content.contains("Self-halt")
            || content.contains("halted_for_drift"),
        "Implementer agent must document self-halt behavior"
    );
}

#[test]
fn test_implementer_documents_output_contract() {
    let path = agents_dir().join("implementer.md");
    let content = fs::read_to_string(&path).expect("Failed to read implementer agent");

    // Implementer must document its output contract (per specks-3.md #implementer-agent-contract)
    assert!(
        content.contains("## Output Contract") || content.contains("drift_assessment"),
        "Implementer agent must document output contract with drift_assessment"
    );
}

#[test]
fn test_implementer_has_required_tools() {
    let path = agents_dir().join("implementer.md");
    let content = fs::read_to_string(&path).expect("Failed to read implementer agent");

    // Implementer must have Write, Edit, and Bash tools for code modification
    assert!(
        content.contains("Write") && content.contains("Edit") && content.contains("Bash"),
        "Implementer agent must have Write, Edit, and Bash tools"
    );
}

// =============================================================================
// Director Agent Tests
// =============================================================================

#[test]
fn test_director_documents_full_execution_loop() {
    let path = agents_dir().join("director.md");
    let content = fs::read_to_string(&path).expect("Failed to read director agent");

    // Director must document the execution loop
    assert!(
        content.contains("Execution Mode Workflow")
            || content.contains("Execution Phase")
            || content.contains("execution loop"),
        "Director agent must document execution workflow"
    );

    // Director must invoke implementer agent
    assert!(
        content.to_lowercase().contains("implementer"),
        "Director agent must mention implementer"
    );

    // Director must invoke architect agent
    assert!(
        content.to_lowercase().contains("architect"),
        "Director agent must mention architect"
    );
}

#[test]
fn test_director_uses_skill_tool() {
    let path = agents_dir().join("director.md");
    let content = fs::read_to_string(&path).expect("Failed to read director agent");

    // Director must use Skill tool (per D07)
    assert!(
        content.contains("Skill") && content.contains("tools:"),
        "Director agent must have Skill in tools"
    );

    // Director should invoke skills for analysis tasks
    assert!(
        content.contains("specks:clarifier")
            || content.contains("specks:critic")
            || content.contains("specks:reviewer"),
        "Director must invoke skills via Skill tool"
    );
}

#[test]
fn test_director_documents_halt_handling() {
    let path = agents_dir().join("director.md");
    let content = fs::read_to_string(&path).expect("Failed to read director agent");

    // Director must document halt handling (drift escalation)
    assert!(
        content.contains("Halt")
            || content.contains("HALT")
            || content.contains("halted_for_drift")
            || content.contains("drift"),
        "Director agent must document halt/drift handling"
    );
}

#[test]
fn test_director_documents_escalation() {
    let path = agents_dir().join("director.md");
    let content = fs::read_to_string(&path).expect("Failed to read director agent");

    // Director must document escalation (via interviewer for user decisions)
    assert!(
        content.contains("Escalation")
            || content.contains("escalation")
            || content.contains("interviewer"),
        "Director agent must document escalation paths"
    );
}

#[test]
fn test_director_is_pure_orchestrator() {
    let path = agents_dir().join("director.md");
    let content = fs::read_to_string(&path).expect("Failed to read director agent");

    let frontmatter = parse_agent_frontmatter(&content).expect("Failed to parse frontmatter");
    let tools = frontmatter.2;

    // Director should NOT have AskUserQuestion (interviewer handles user interaction)
    assert!(
        !tools.contains("AskUserQuestion"),
        "Director should not have AskUserQuestion (D02: pure orchestrator)"
    );

    // Director should NOT have Edit (only Write for audit trail)
    assert!(
        !tools.contains("Edit"),
        "Director should not have Edit tool (D02: pure orchestrator)"
    );

    // Director SHOULD have Write for audit trail
    assert!(
        tools.contains("Write"),
        "Director should have Write tool for audit trail"
    );
}

// =============================================================================
// Interviewer Agent Tests
// =============================================================================

#[test]
fn test_interviewer_handles_user_interaction() {
    let path = agents_dir().join("interviewer.md");
    let content = fs::read_to_string(&path).expect("Failed to read interviewer agent");

    let frontmatter = parse_agent_frontmatter(&content).expect("Failed to parse frontmatter");
    let tools = frontmatter.2;

    // Interviewer must have AskUserQuestion
    assert!(
        tools.contains("AskUserQuestion"),
        "Interviewer must have AskUserQuestion tool"
    );

    // Interviewer should document user interaction
    assert!(
        content.contains("user interaction") || content.contains("single point"),
        "Interviewer must document its role as user interaction point"
    );
}

#[test]
fn test_interviewer_documents_contexts() {
    let path = agents_dir().join("interviewer.md");
    let content = fs::read_to_string(&path).expect("Failed to read interviewer agent");

    // Interviewer handles 4 contexts per specks-3.md #interviewer-contract
    let contexts = ["clarifier", "critic", "drift", "review"];
    for context in contexts {
        assert!(
            content.contains(context),
            "Interviewer must document {} context",
            context
        );
    }
}

// =============================================================================
// Planner Agent Tests
// =============================================================================

#[test]
fn test_planner_no_user_interaction() {
    let path = agents_dir().join("planner.md");
    let content = fs::read_to_string(&path).expect("Failed to read planner agent");

    let frontmatter = parse_agent_frontmatter(&content).expect("Failed to parse frontmatter");
    let tools = frontmatter.2;

    // Planner should NOT have AskUserQuestion (interviewer handles this)
    assert!(
        !tools.contains("AskUserQuestion"),
        "Planner should not have AskUserQuestion (D04: interviewer handles user interaction)"
    );
}

#[test]
fn test_planner_documents_input_contract() {
    let path = agents_dir().join("planner.md");
    let content = fs::read_to_string(&path).expect("Failed to read planner agent");

    // Planner must document input contract
    assert!(
        content.contains("## Input Contract") || content.contains("Inputs You Receive"),
        "Planner must document input contract"
    );

    // Planner receives clarifier assumptions from director
    assert!(
        content.contains("clarifier") || content.contains("assumptions"),
        "Planner must receive clarifier data from director"
    );
}

// =============================================================================
// Architect Agent Tests
// =============================================================================

#[test]
fn test_architect_documents_output_format() {
    let path = agents_dir().join("architect.md");
    let content = fs::read_to_string(&path).expect("Failed to read architect agent");

    // Architect must document expected_touch_set
    assert!(
        content.contains("expected_touch_set"),
        "Architect must document expected_touch_set in output"
    );

    // Architect must document implementation strategy
    assert!(
        content.contains("strategy") || content.contains("Strategy"),
        "Architect must document implementation strategy output"
    );
}

// =============================================================================
// Run Directory Tests
// =============================================================================

#[test]
fn test_run_directory_structure_documented() {
    let path = agents_dir().join("director.md");
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

    // Create a session-id based run directory (new format per specks-3.md)
    let session_id = "20260206-143022-plan-a1b2c3";
    let run_dir = runs.join(session_id);
    fs::create_dir(&run_dir).expect("failed to create run dir");

    // Create metadata.json (new format per specks-3.md #run-metadata)
    let metadata = serde_json::json!({
        "session_id": session_id,
        "mode": "plan",
        "started_at": "2026-02-06T14:30:22Z",
        "speck_path": ".specks/specks-test.md",
        "status": "in_progress",
        "completed_at": null
    });
    fs::write(
        run_dir.join("metadata.json"),
        serde_json::to_string_pretty(&metadata).unwrap(),
    )
    .expect("failed to write metadata.json");

    // Verify structure
    assert!(run_dir.join("metadata.json").exists());

    // Create planning subdirectory
    let planning_dir = run_dir.join("planning");
    fs::create_dir(&planning_dir).expect("failed to create planning dir");

    // Create skill outputs with sequential numbering
    let clarifier_output = serde_json::json!({
        "analysis": {"understood_intent": "test"},
        "questions": [],
        "assumptions": ["test assumption"]
    });
    fs::write(
        planning_dir.join("001-clarifier.json"),
        serde_json::to_string_pretty(&clarifier_output).unwrap(),
    )
    .expect("failed to write clarifier output");

    assert!(planning_dir.join("001-clarifier.json").exists());
}
