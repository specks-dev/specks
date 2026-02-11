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

    temp
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
fn test_init_idempotent_on_existing_project() {
    let temp = setup_test_project();

    // Running init again should succeed (idempotent — creates missing files only)
    let output = Command::new(specks_binary())
        .arg("init")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks init");

    assert!(
        output.status.success(),
        "init should succeed idempotently on existing project"
    );

    // All files should still exist
    let specks_dir = temp.path().join(".specks");
    assert!(specks_dir.join("specks-skeleton.md").is_file());
    assert!(specks_dir.join("config.toml").is_file());
    assert!(specks_dir.join("specks-implementation-log.md").is_file());
}

#[test]
fn test_init_creates_missing_files() {
    let temp = tempfile::tempdir().expect("failed to create temp dir");

    // Create .specks/ with only a speck file (simulates worktree scenario)
    let specks_dir = temp.path().join(".specks");
    std::fs::create_dir_all(&specks_dir).expect("failed to create .specks");
    std::fs::write(specks_dir.join("specks-1.md"), "# My Speck\n").expect("failed to write speck");

    // Running init should create the missing infrastructure files
    let output = Command::new(specks_binary())
        .arg("init")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks init");

    assert!(
        output.status.success(),
        "init should succeed and create missing files"
    );

    // Infrastructure files should now exist
    assert!(specks_dir.join("specks-skeleton.md").is_file());
    assert!(specks_dir.join("config.toml").is_file());
    assert!(specks_dir.join("specks-implementation-log.md").is_file());

    // Original speck file should be untouched
    let content =
        std::fs::read_to_string(specks_dir.join("specks-1.md")).expect("failed to read speck");
    assert_eq!(content, "# My Speck\n");
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
fn test_init_check_uninitialized_project() {
    let temp = tempfile::tempdir().expect("failed to create temp dir");

    let output = Command::new(specks_binary())
        .arg("init")
        .arg("--check")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks init --check");

    // Should return exit code 9 for uninitialized project
    assert_eq!(
        output.status.code(),
        Some(9),
        "init --check should return exit code 9 for uninitialized project"
    );
}

#[test]
fn test_init_check_initialized_project() {
    let temp = setup_test_project();

    let output = Command::new(specks_binary())
        .arg("init")
        .arg("--check")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks init --check");

    // Should return exit code 0 for initialized project
    assert!(
        output.status.success(),
        "init --check should succeed on initialized project"
    );
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn test_init_check_json_uninitialized() {
    let temp = tempfile::tempdir().expect("failed to create temp dir");

    let output = Command::new(specks_binary())
        .arg("init")
        .arg("--check")
        .arg("--json")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks init --check --json");

    assert_eq!(output.status.code(), Some(9));
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert_eq!(json["schema_version"], "1");
    assert_eq!(json["command"], "init");
    assert_eq!(json["status"], "ok");
    assert_eq!(json["data"]["initialized"], false);
    assert_eq!(json["data"]["path"], ".specks/");
}

#[test]
fn test_init_check_json_initialized() {
    let temp = setup_test_project();

    let output = Command::new(specks_binary())
        .arg("init")
        .arg("--check")
        .arg("--json")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks init --check --json");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert_eq!(json["schema_version"], "1");
    assert_eq!(json["command"], "init");
    assert_eq!(json["status"], "ok");
    assert_eq!(json["data"]["initialized"], true);
    assert_eq!(json["data"]["path"], ".specks/");
}

#[test]
fn test_init_check_force_mutually_exclusive() {
    let temp = tempfile::tempdir().expect("failed to create temp dir");

    let output = Command::new(specks_binary())
        .arg("init")
        .arg("--check")
        .arg("--force")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks init --check --force");

    // Should fail due to mutually exclusive flags
    assert!(
        !output.status.success(),
        "init --check --force should fail due to mutually exclusive flags"
    );
}
