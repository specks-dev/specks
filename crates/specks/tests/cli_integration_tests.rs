//! CLI integration tests for specks commands

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Get the path to the specks binary
fn specks_binary() -> PathBuf {
    // Use the debug binary
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // crates
    path.pop(); // specks root
    path.push("target");
    path.push("debug");
    path.push("specks");
    path
}

/// Create a temp directory with .specks initialized
fn setup_test_project() -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("failed to create temp dir");

    // Run specks init
    let output = Command::new(specks_binary())
        .arg("init")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks init");

    assert!(
        output.status.success(),
        "specks init failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Copy agents from the workspace to the test project
    // This ensures execute preflight checks pass
    copy_test_agents(temp.path());

    temp
}

/// Find the workspace root (where agents/ directory exists)
fn find_workspace_root() -> std::path::PathBuf {
    // Start from the manifest directory and walk up until we find agents/
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if path.join("agents").is_dir() {
            return path;
        }
        if !path.pop() {
            panic!("Could not find workspace root with agents/ directory");
        }
    }
}

/// Copy agents to test project directory
fn copy_test_agents(project_path: &std::path::Path) {
    let workspace = find_workspace_root();
    let source_agents = workspace.join("agents");
    let dest_agents = project_path.join("agents");

    // Create agents directory
    fs::create_dir_all(&dest_agents).expect("failed to create agents directory");

    // Copy all .md files from source to dest
    for entry in fs::read_dir(&source_agents).expect("failed to read agents directory") {
        let entry = entry.expect("failed to read entry");
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "md") {
            let filename = path.file_name().unwrap();
            let dest_path = dest_agents.join(filename);
            fs::copy(&path, &dest_path).expect("failed to copy agent file");
        }
    }
}

/// Create a minimal valid speck in the test project
fn create_test_speck(temp_dir: &tempfile::TempDir, name: &str, content: &str) {
    let speck_path = temp_dir
        .path()
        .join(".specks")
        .join(format!("specks-{}.md", name));
    fs::write(&speck_path, content).expect("failed to write test speck");
}

const MINIMAL_SPECK: &str = r#"## Phase 1.0: Test Feature {#phase-1}

**Purpose:** Test speck for integration testing.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | active |
| Target branch | main |
| Tracking issue/PR | N/A |
| Last updated | 2026-02-04 |

---

### Phase Overview {#phase-overview}

#### Context {#context}

Test context paragraph.

---

### 1.0.0 Design Decisions {#design-decisions}

#### [D01] Test Decision (DECIDED) {#d01-test}

**Decision:** This is a test decision.

**Rationale:**
- Because testing

**Implications:**
- Tests work

---

### 1.0.5 Execution Steps {#execution-steps}

#### Step 0: Setup {#step-0}

**Commit:** `feat: setup`

**References:** [D01] Test Decision, (#context)

**Tasks:**
- [x] Create project
- [ ] Add tests

**Tests:**
- [ ] Unit test

**Checkpoint:**
- [x] Build passes

---

### 1.0.6 Deliverables and Checkpoints {#deliverables}

**Deliverable:** Working test feature.

#### Phase Exit Criteria {#exit-criteria}

- [ ] All tests pass
"#;

#[test]
fn test_init_creates_expected_files() {
    let temp = tempfile::tempdir().expect("failed to create temp dir");

    let output = Command::new(specks_binary())
        .arg("init")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks init");

    assert!(output.status.success(), "init should succeed");

    // Check files were created
    let specks_dir = temp.path().join(".specks");
    assert!(specks_dir.is_dir(), ".specks directory should exist");
    assert!(
        specks_dir.join("specks-skeleton.md").is_file(),
        "skeleton should exist"
    );
    assert!(
        specks_dir.join("config.toml").is_file(),
        "config should exist"
    );
    assert!(
        specks_dir.join("specks-implementation-log.md").is_file(),
        "implementation log should exist"
    );
}

#[test]
fn test_init_fails_without_force() {
    let temp = setup_test_project();

    let output = Command::new(specks_binary())
        .arg("init")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks init");

    assert!(
        !output.status.success(),
        "init without force should fail on existing project"
    );
}

#[test]
fn test_init_with_force_succeeds() {
    let temp = setup_test_project();

    let output = Command::new(specks_binary())
        .arg("init")
        .arg("--force")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks init --force");

    assert!(output.status.success(), "init --force should succeed");
}

#[test]
fn test_validate_valid_speck() {
    let temp = setup_test_project();
    create_test_speck(&temp, "test", MINIMAL_SPECK);

    let output = Command::new(specks_binary())
        .arg("validate")
        .arg("specks-test.md")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks validate");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "validate should succeed on valid speck: {}",
        stdout
    );
    assert!(stdout.contains("valid"), "output should say valid");
}

#[test]
fn test_validate_invalid_speck() {
    let temp = setup_test_project();

    // Create an invalid speck (missing metadata)
    let invalid = r#"## Phase 1.0: Test {#phase-1}

**Purpose:** Test

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | |
| Status | invalid_status |

---

### Phase Overview {#phase-overview}

Test

---

### 1.0.0 Design Decisions {#design-decisions}

None

---

### 1.0.5 Execution Steps {#execution-steps}

None

---

### 1.0.6 Deliverables and Checkpoints {#deliverables}

None
"#;

    create_test_speck(&temp, "invalid", invalid);

    let output = Command::new(specks_binary())
        .arg("validate")
        .arg("specks-invalid.md")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks validate");

    assert!(
        !output.status.success(),
        "validate should fail on invalid speck"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("error"), "output should contain error");
}

#[test]
fn test_list_shows_specks() {
    let temp = setup_test_project();
    create_test_speck(&temp, "test", MINIMAL_SPECK);

    let output = Command::new(specks_binary())
        .arg("list")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks list");

    assert!(output.status.success(), "list should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test"), "output should contain speck name");
    assert!(stdout.contains("active"), "output should contain status");
}

#[test]
fn test_status_shows_step_breakdown() {
    let temp = setup_test_project();
    create_test_speck(&temp, "test", MINIMAL_SPECK);

    let output = Command::new(specks_binary())
        .arg("status")
        .arg("specks-test.md")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks status");

    assert!(output.status.success(), "status should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Step 0"), "output should contain step");
    assert!(stdout.contains("Setup"), "output should contain step title");
    assert!(stdout.contains("Total:"), "output should contain total");
}

#[test]
fn test_json_output_init() {
    let temp = tempfile::tempdir().expect("failed to create temp dir");

    let output = Command::new(specks_binary())
        .arg("init")
        .arg("--json")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks init --json");

    assert!(output.status.success(), "init --json should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert_eq!(json["schema_version"], "1");
    assert_eq!(json["command"], "init");
    assert_eq!(json["status"], "ok");
    assert!(json["data"]["files_created"].is_array());
}

#[test]
fn test_json_output_list() {
    let temp = setup_test_project();
    create_test_speck(&temp, "test", MINIMAL_SPECK);

    let output = Command::new(specks_binary())
        .arg("list")
        .arg("--json")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks list --json");

    assert!(output.status.success(), "list --json should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert_eq!(json["schema_version"], "1");
    assert_eq!(json["command"], "list");
    assert_eq!(json["status"], "ok");
    assert!(json["data"]["specks"].is_array());
    assert_eq!(json["data"]["specks"][0]["name"], "test");
}

#[test]
fn test_json_output_validate() {
    let temp = setup_test_project();
    create_test_speck(&temp, "test", MINIMAL_SPECK);

    let output = Command::new(specks_binary())
        .arg("validate")
        .arg("specks-test.md")
        .arg("--json")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks validate --json");

    assert!(
        output.status.success(),
        "validate --json should succeed on valid speck"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert_eq!(json["schema_version"], "1");
    assert_eq!(json["command"], "validate");
    assert_eq!(json["status"], "ok");
    assert!(json["data"]["files"].is_array());
    assert_eq!(json["data"]["files"][0]["valid"], true);
}

#[test]
fn test_json_output_status() {
    let temp = setup_test_project();
    create_test_speck(&temp, "test", MINIMAL_SPECK);

    let output = Command::new(specks_binary())
        .arg("status")
        .arg("specks-test.md")
        .arg("--json")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks status --json");

    assert!(output.status.success(), "status --json should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert_eq!(json["schema_version"], "1");
    assert_eq!(json["command"], "status");
    assert_eq!(json["status"], "ok");
    assert_eq!(json["data"]["name"], "test");
    assert!(json["data"]["steps"].is_array());
    assert!(json["data"]["progress"]["done"].is_number());
    assert!(json["data"]["progress"]["total"].is_number());
}

#[test]
fn test_execute_dry_run() {
    let temp = setup_test_project();
    create_test_speck(&temp, "test", MINIMAL_SPECK);

    let output = Command::new(specks_binary())
        .arg("execute")
        .arg("specks-test.md")
        .arg("--dry-run")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks execute --dry-run");

    assert!(
        output.status.success(),
        "execute --dry-run should succeed on active speck"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Dry run"),
        "output should contain 'Dry run'"
    );
    assert!(
        stdout.contains("step-0"),
        "output should contain step anchor"
    );
}

#[test]
fn test_execute_dry_run_json() {
    let temp = setup_test_project();
    create_test_speck(&temp, "test", MINIMAL_SPECK);

    let output = Command::new(specks_binary())
        .arg("--json")
        .arg("execute")
        .arg("specks-test.md")
        .arg("--dry-run")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks execute --dry-run --json");

    assert!(
        output.status.success(),
        "execute --dry-run --json should succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert_eq!(json["schema_version"], "1");
    assert_eq!(json["command"], "execute");
    assert_eq!(json["status"], "ok");
    assert_eq!(json["data"]["outcome"], "dry_run");
    assert!(json["data"]["steps_remaining"].is_array());
    assert!(json["data"]["run_id"].is_string());
}

#[test]
fn test_execute_rejects_draft_speck() {
    let temp = setup_test_project();

    // Create a draft speck (status = draft)
    let draft_speck = MINIMAL_SPECK.replace("Status | active", "Status | draft");
    create_test_speck(&temp, "draft", &draft_speck);

    let output = Command::new(specks_binary())
        .arg("execute")
        .arg("specks-draft.md")
        .arg("--dry-run")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks execute");

    assert!(
        !output.status.success(),
        "execute should fail on draft speck"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("draft"),
        "error should mention draft status"
    );
    assert!(
        stderr.contains("active"),
        "error should mention required active status"
    );
}

#[test]
fn test_execute_with_step_filtering() {
    let temp = setup_test_project();

    // Create a speck with two steps
    let two_step_speck = r#"## Phase 1.0: Test Feature {#phase-1}

**Purpose:** Test speck with two steps.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | Test |
| Status | active |
| Target branch | main |
| Tracking issue/PR | N/A |
| Last updated | 2026-02-04 |

---

### Phase Overview {#phase-overview}

#### Context {#context}

Test context.

---

### 1.0.0 Design Decisions {#design-decisions}

#### [D01] Test (DECIDED) {#d01-test}

**Decision:** Test.

**Rationale:**
- Test

**Implications:**
- Test

---

### 1.0.5 Execution Steps {#execution-steps}

#### Step 0: First {#step-0}

**Commit:** `feat: first`

**References:** [D01] Test, (#context)

**Tasks:**
- [ ] Task 1

---

#### Step 1: Second {#step-1}

**Depends on:** #step-0

**Commit:** `feat: second`

**References:** [D01] Test, (#context)

**Tasks:**
- [ ] Task 2

---

### 1.0.6 Deliverables and Checkpoints {#deliverables}

**Deliverable:** Test.

#### Phase Exit Criteria {#exit-criteria}

- [ ] Done
"#;

    create_test_speck(&temp, "twostep", two_step_speck);

    // Test start-step filter
    let output = Command::new(specks_binary())
        .arg("execute")
        .arg("specks-twostep.md")
        .arg("--dry-run")
        .arg("--start-step")
        .arg("step-1")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks execute with start-step");

    assert!(
        output.status.success(),
        "execute with --start-step should succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("step-1"), "output should contain step-1");
    assert!(
        stdout.contains("skipped"),
        "output should show skipped steps"
    );
    assert!(
        stdout.contains("step-0"),
        "output should mention step-0 as skipped"
    );
}

#[test]
fn test_execute_not_initialized() {
    let temp = tempfile::tempdir().expect("failed to create temp dir");

    // Don't initialize - just try to run execute
    let output = Command::new(specks_binary())
        .arg("execute")
        .arg("specks-test.md")
        .arg("--dry-run")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks execute");

    assert!(
        !output.status.success(),
        "execute should fail when not initialized"
    );
    assert_eq!(
        output.status.code(),
        Some(9),
        "exit code should be 9 (not initialized)"
    );
}
