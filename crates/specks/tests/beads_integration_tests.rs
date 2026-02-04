//! Beads integration tests using mock-bd
//!
//! These tests use the bd-fake script to simulate the beads CLI,
//! enabling deterministic testing without network dependencies.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Get the path to the specks binary
fn specks_binary() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // crates
    path.pop(); // specks root
    path.push("target");
    path.push("debug");
    path.push("specks");
    path
}

/// Get the path to the bd-fake script
fn bd_fake_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // crates
    path.pop(); // specks root
    path.push("tests");
    path.push("bin");
    path.push("bd-fake");
    path
}

/// Create a temp directory with .specks and .beads initialized
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

    // Create .beads directory (required for beads commands)
    fs::create_dir(temp.path().join(".beads")).expect("failed to create .beads dir");

    temp
}

/// Create a test speck in the project
fn create_test_speck(temp_dir: &tempfile::TempDir, name: &str, content: &str) {
    let speck_path = temp_dir
        .path()
        .join(".specks")
        .join(format!("specks-{}.md", name));
    fs::write(&speck_path, content).expect("failed to write test speck");
}

/// Minimal speck with one step
const SINGLE_STEP_SPECK: &str = r#"## Phase 1.0: Test Feature {#phase-1}

**Purpose:** Test speck for beads integration testing.

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

None.

---

### 1.0.5 Execution Steps {#execution-steps}

#### Step 0: Setup {#step-0}

**Commit:** `feat: setup`

**References:** (#context)

**Tasks:**
- [ ] Create project

**Checkpoint:**
- [ ] Build passes

---

### 1.0.6 Deliverables and Checkpoints {#deliverables}

**Deliverable:** Working feature.
"#;

/// Speck with multiple steps and dependencies
const MULTI_STEP_SPECK: &str = r#"## Phase 1.0: Multi-Step Feature {#phase-1}

**Purpose:** Test speck with dependencies for beads integration testing.

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

None.

---

### 1.0.5 Execution Steps {#execution-steps}

#### Step 0: Foundation {#step-0}

**Commit:** `feat: foundation`

**References:** (#context)

**Tasks:**
- [ ] Create base

**Checkpoint:**
- [ ] Base works

---

#### Step 1: Build on Foundation {#step-1}

**Depends on:** #step-0

**Commit:** `feat: build`

**References:** (#context)

**Tasks:**
- [ ] Build feature

**Checkpoint:**
- [ ] Feature works

---

#### Step 2: Final Integration {#step-2}

**Depends on:** #step-0, #step-1

**Commit:** `feat: integrate`

**References:** (#context)

**Tasks:**
- [ ] Integrate all

**Checkpoint:**
- [ ] All works

---

### 1.0.6 Deliverables and Checkpoints {#deliverables}

**Deliverable:** Complete feature.
"#;

// =============================================================================
// Mock-bd unit tests
// =============================================================================

#[test]
fn test_mock_bd_create_returns_valid_issue_json() {
    let temp_state = tempfile::tempdir().expect("failed to create temp state dir");

    let output = Command::new(bd_fake_path())
        .env("SPECKS_BD_STATE", temp_state.path())
        .args(["create", "Test Issue", "--json"])
        .output()
        .expect("failed to run bd-fake create");

    assert!(output.status.success(), "bd create should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");

    // Verify required fields per Beads JSON Contract
    assert!(json["id"].is_string(), "should have id");
    assert!(json["title"].is_string(), "should have title");
    assert!(json["status"].is_string(), "should have status");
    assert!(json["priority"].is_number(), "should have priority");
    assert!(json["issue_type"].is_string(), "should have issue_type");

    assert_eq!(json["title"], "Test Issue");
    assert_eq!(json["status"], "open");
}

#[test]
fn test_mock_bd_show_returns_issue_details() {
    let temp_state = tempfile::tempdir().expect("failed to create temp state dir");

    // Create an issue first
    Command::new(bd_fake_path())
        .env("SPECKS_BD_STATE", temp_state.path())
        .args(["create", "Test Issue", "--json"])
        .output()
        .expect("failed to run bd-fake create");

    // Show the issue
    let output = Command::new(bd_fake_path())
        .env("SPECKS_BD_STATE", temp_state.path())
        .args(["show", "bd-fake-1", "--json"])
        .output()
        .expect("failed to run bd-fake show");

    assert!(output.status.success(), "bd show should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");

    // Show returns single object (contract allows array or object)
    assert!(json["id"].is_string(), "should have id");
    assert!(json["dependencies"].is_array(), "should have dependencies array");
}

#[test]
fn test_mock_bd_dep_add_and_list_track_dependencies() {
    let temp_state = tempfile::tempdir().expect("failed to create temp state dir");

    // Create two issues
    Command::new(bd_fake_path())
        .env("SPECKS_BD_STATE", temp_state.path())
        .args(["create", "Parent", "--json"])
        .output()
        .expect("failed to create parent");

    Command::new(bd_fake_path())
        .env("SPECKS_BD_STATE", temp_state.path())
        .args(["create", "Child", "--json"])
        .output()
        .expect("failed to create child");

    // Add dependency: Child depends on Parent
    let add_output = Command::new(bd_fake_path())
        .env("SPECKS_BD_STATE", temp_state.path())
        .args(["dep", "add", "bd-fake-2", "bd-fake-1", "--json"])
        .output()
        .expect("failed to add dep");

    assert!(add_output.status.success(), "dep add should succeed");

    let add_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&add_output.stdout))
            .expect("should be valid JSON");
    assert_eq!(add_json["status"], "added");

    // List dependencies
    let list_output = Command::new(bd_fake_path())
        .env("SPECKS_BD_STATE", temp_state.path())
        .args(["dep", "list", "bd-fake-2", "--json"])
        .output()
        .expect("failed to list deps");

    assert!(list_output.status.success(), "dep list should succeed");

    let list_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&list_output.stdout))
            .expect("should be valid JSON");

    assert!(list_json.is_array(), "dep list should return array");
    assert_eq!(list_json.as_array().unwrap().len(), 1);
    assert_eq!(list_json[0]["id"], "bd-fake-1");
}

#[test]
fn test_mock_bd_ready_returns_unblocked_issues() {
    let temp_state = tempfile::tempdir().expect("failed to create temp state dir");

    // Create parent and children
    Command::new(bd_fake_path())
        .env("SPECKS_BD_STATE", temp_state.path())
        .args(["create", "Root", "--type", "epic", "--json"])
        .output()
        .expect("failed to create root");

    Command::new(bd_fake_path())
        .env("SPECKS_BD_STATE", temp_state.path())
        .args(["create", "Step 0", "--parent", "bd-fake-1", "--json"])
        .output()
        .expect("failed to create step 0");

    Command::new(bd_fake_path())
        .env("SPECKS_BD_STATE", temp_state.path())
        .args(["create", "Step 1", "--parent", "bd-fake-1", "--json"])
        .output()
        .expect("failed to create step 1");

    // Add dependency: Step 1 depends on Step 0
    Command::new(bd_fake_path())
        .env("SPECKS_BD_STATE", temp_state.path())
        .args(["dep", "add", "bd-fake-1.2", "bd-fake-1.1", "--json"])
        .output()
        .expect("failed to add dep");

    // Check ready - only Step 0 should be ready
    let ready_output = Command::new(bd_fake_path())
        .env("SPECKS_BD_STATE", temp_state.path())
        .args(["ready", "--parent", "bd-fake-1", "--json"])
        .output()
        .expect("failed to get ready");

    let ready_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&ready_output.stdout))
            .expect("should be valid JSON");

    assert!(ready_json.is_array());
    let ready_arr = ready_json.as_array().unwrap();
    assert_eq!(ready_arr.len(), 1, "only Step 0 should be ready");
    assert_eq!(ready_arr[0]["id"], "bd-fake-1.1");

    // Close Step 0
    Command::new(bd_fake_path())
        .env("SPECKS_BD_STATE", temp_state.path())
        .args(["close", "bd-fake-1.1"])
        .output()
        .expect("failed to close");

    // Now Step 1 should be ready
    let ready_after = Command::new(bd_fake_path())
        .env("SPECKS_BD_STATE", temp_state.path())
        .args(["ready", "--parent", "bd-fake-1", "--json"])
        .output()
        .expect("failed to get ready after close");

    let ready_after_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&ready_after.stdout))
            .expect("should be valid JSON");

    let ready_after_arr = ready_after_json.as_array().unwrap();
    assert_eq!(ready_after_arr.len(), 1);
    assert_eq!(ready_after_arr[0]["id"], "bd-fake-1.2");
}

// =============================================================================
// Beads sync integration tests
// =============================================================================

#[test]
fn test_beads_sync_creates_root_and_step_beads() {
    let temp = setup_test_project();
    let temp_state = tempfile::tempdir().expect("failed to create temp state dir");
    create_test_speck(&temp, "test", SINGLE_STEP_SPECK);

    // Run specks beads sync with mock bd
    let output = Command::new(specks_binary())
        .env("SPECKS_BD_PATH", bd_fake_path())
        .env("SPECKS_BD_STATE", temp_state.path())
        .args(["beads", "sync", "specks-test.md", "--json"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks beads sync");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "beads sync should succeed. stdout: {}, stderr: {}",
        stdout,
        stderr
    );

    let json: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");

    assert_eq!(json["status"], "ok");
    assert!(json["data"]["root_bead_id"].is_string(), "should have root bead ID");
    assert!(json["data"]["steps_synced"].as_u64().unwrap() >= 1, "should sync at least 1 step");

    // Verify the speck file was updated with bead IDs
    let speck_content = fs::read_to_string(temp.path().join(".specks/specks-test.md"))
        .expect("failed to read speck");

    assert!(
        speck_content.contains("**Bead:**") || speck_content.contains("Beads Root"),
        "speck should contain bead ID references"
    );
}

#[test]
fn test_beads_sync_is_idempotent() {
    let temp = setup_test_project();
    let temp_state = tempfile::tempdir().expect("failed to create temp state dir");
    create_test_speck(&temp, "test", SINGLE_STEP_SPECK);

    // Run sync twice
    for i in 1..=2 {
        let output = Command::new(specks_binary())
            .env("SPECKS_BD_PATH", bd_fake_path())
            .env("SPECKS_BD_STATE", temp_state.path())
            .args(["beads", "sync", "specks-test.md", "--json"])
            .current_dir(temp.path())
            .output()
            .expect("failed to run specks beads sync");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "beads sync run {} should succeed: {}",
            i,
            stdout
        );
    }

    // After two syncs, there should still be only one root bead
    let issues_file = temp_state.path().join("issues.json");
    let issues_content = fs::read_to_string(&issues_file).expect("failed to read issues");
    let issues: serde_json::Value =
        serde_json::from_str(&issues_content).expect("should be valid JSON");

    // Count root-level issues (no dots in ID)
    let root_count = issues
        .as_object()
        .unwrap()
        .keys()
        .filter(|k| !k.contains('.'))
        .count();

    assert_eq!(root_count, 1, "should have exactly one root bead after idempotent syncs");
}

#[test]
fn test_beads_sync_creates_dependency_edges() {
    let temp = setup_test_project();
    let temp_state = tempfile::tempdir().expect("failed to create temp state dir");
    create_test_speck(&temp, "multi", MULTI_STEP_SPECK);

    // Run sync
    let output = Command::new(specks_binary())
        .env("SPECKS_BD_PATH", bd_fake_path())
        .env("SPECKS_BD_STATE", temp_state.path())
        .args(["beads", "sync", "specks-multi.md", "--json"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks beads sync");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "sync should succeed: {}", stdout);

    let json: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert!(
        json["data"]["deps_added"].as_u64().unwrap() >= 2,
        "should add dependency edges for step-1 and step-2"
    );

    // Verify dependencies in mock state
    let deps_file = temp_state.path().join("deps.json");
    let deps_content = fs::read_to_string(&deps_file).expect("failed to read deps");
    let deps: serde_json::Value = serde_json::from_str(&deps_content).expect("should be valid JSON");

    assert!(deps.as_array().unwrap().len() >= 2, "should have at least 2 dependency edges");
}

// =============================================================================
// Beads status integration tests
// =============================================================================

#[test]
fn test_beads_status_computes_readiness() {
    let temp = setup_test_project();
    let temp_state = tempfile::tempdir().expect("failed to create temp state dir");
    create_test_speck(&temp, "multi", MULTI_STEP_SPECK);

    // Run sync first to create beads
    Command::new(specks_binary())
        .env("SPECKS_BD_PATH", bd_fake_path())
        .env("SPECKS_BD_STATE", temp_state.path())
        .args(["beads", "sync", "specks-multi.md"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run sync");

    // Check status
    let output = Command::new(specks_binary())
        .env("SPECKS_BD_PATH", bd_fake_path())
        .env("SPECKS_BD_STATE", temp_state.path())
        .args(["beads", "status", "specks-multi.md", "--json"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run status");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "status should succeed: {}", stdout);

    let json: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");

    // All steps should initially be ready (open with deps not closed)
    // Step 0: ready (no deps)
    // Step 1: blocked (depends on step-0 which is open)
    // Step 2: blocked (depends on step-0 and step-1)
    let steps = json["data"]["files"][0]["steps"].as_array().unwrap();

    // Find step statuses
    let step_0_status = steps.iter().find(|s| s["anchor"] == "step-0").unwrap();
    let step_1_status = steps.iter().find(|s| s["anchor"] == "step-1").unwrap();
    let step_2_status = steps.iter().find(|s| s["anchor"] == "step-2").unwrap();

    assert_eq!(step_0_status["status"], "ready", "Step 0 has no deps, should be ready");
    assert_eq!(step_1_status["status"], "blocked", "Step 1 depends on Step 0, should be blocked");
    assert_eq!(step_2_status["status"], "blocked", "Step 2 depends on Step 0 and 1, should be blocked");
}

// =============================================================================
// Beads pull integration tests
// =============================================================================

#[test]
fn test_beads_pull_updates_checkboxes() {
    let temp = setup_test_project();
    let temp_state = tempfile::tempdir().expect("failed to create temp state dir");
    create_test_speck(&temp, "test", SINGLE_STEP_SPECK);

    // Run sync to create beads
    Command::new(specks_binary())
        .env("SPECKS_BD_PATH", bd_fake_path())
        .env("SPECKS_BD_STATE", temp_state.path())
        .args(["beads", "sync", "specks-test.md"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run sync");

    // Get the step bead ID from the mock state
    let issues_content =
        fs::read_to_string(temp_state.path().join("issues.json")).expect("failed to read issues");
    let issues: serde_json::Value = serde_json::from_str(&issues_content).unwrap();

    // Find the step bead (child of root)
    let step_bead_id = issues
        .as_object()
        .unwrap()
        .keys()
        .find(|k| k.contains('.'))
        .expect("should have a step bead");

    // Close the step bead in mock
    Command::new(bd_fake_path())
        .env("SPECKS_BD_STATE", temp_state.path())
        .args(["close", step_bead_id])
        .output()
        .expect("failed to close bead");

    // Run pull to update checkboxes
    let output = Command::new(specks_binary())
        .env("SPECKS_BD_PATH", bd_fake_path())
        .env("SPECKS_BD_STATE", temp_state.path())
        .args(["beads", "pull", "specks-test.md", "--json"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run pull");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "pull should succeed: {}", stdout);

    let json: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");

    // Verify checkboxes were updated
    assert!(
        json["data"]["total_updated"].as_u64().unwrap() >= 1,
        "should update at least one checkbox"
    );

    // Verify the speck file has checked checkboxes
    let speck_content = fs::read_to_string(temp.path().join(".specks/specks-test.md"))
        .expect("failed to read speck");

    // The checkpoint checkbox should be checked
    assert!(
        speck_content.contains("[x] Build passes") || speck_content.contains("[X] Build passes"),
        "checkpoint checkbox should be checked after pull"
    );
}

// =============================================================================
// Full workflow integration test (as documented in README)
// =============================================================================

#[test]
fn test_full_beads_workflow_sync_work_pull() {
    // This test exercises the complete workflow documented in README:
    // 1. specks beads sync (Plan → Beads)
    // 2. bd close (Work in Beads)
    // 3. specks beads status (Check readiness)
    // 4. specks beads pull (Beads → Plan)

    let temp = setup_test_project();
    let temp_state = tempfile::tempdir().expect("failed to create temp state dir");
    create_test_speck(&temp, "workflow", MULTI_STEP_SPECK);

    // Step 1: Sync speck to beads
    let sync_output = Command::new(specks_binary())
        .env("SPECKS_BD_PATH", bd_fake_path())
        .env("SPECKS_BD_STATE", temp_state.path())
        .args(["beads", "sync", "specks-workflow.md", "--json"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run sync");

    assert!(sync_output.status.success(), "sync should succeed");
    let sync_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&sync_output.stdout)).unwrap();
    assert_eq!(sync_json["data"]["steps_synced"], 3, "should sync 3 steps");

    // Verify bead IDs were written to speck
    let speck_after_sync = fs::read_to_string(temp.path().join(".specks/specks-workflow.md"))
        .expect("failed to read speck");
    assert!(
        speck_after_sync.contains("**Bead:**"),
        "speck should have Bead IDs after sync"
    );

    // Step 2: Check initial status - Step 0 should be ready, others blocked
    let status_output = Command::new(specks_binary())
        .env("SPECKS_BD_PATH", bd_fake_path())
        .env("SPECKS_BD_STATE", temp_state.path())
        .args(["beads", "status", "specks-workflow.md", "--json"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run status");

    let status_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&status_output.stdout)).unwrap();
    let steps = status_json["data"]["files"][0]["steps"].as_array().unwrap();
    let step_0 = steps.iter().find(|s| s["anchor"] == "step-0").unwrap();
    assert_eq!(step_0["status"], "ready", "Step 0 should be ready initially");

    // Step 3: Simulate work completion - close Step 0's bead
    let step_0_bead_id = step_0["bead_id"].as_str().unwrap();
    Command::new(bd_fake_path())
        .env("SPECKS_BD_STATE", temp_state.path())
        .args(["close", step_0_bead_id])
        .output()
        .expect("failed to close step 0 bead");

    // Step 4: Check status again - Step 0 complete, Step 1 now ready
    let status_after_work = Command::new(specks_binary())
        .env("SPECKS_BD_PATH", bd_fake_path())
        .env("SPECKS_BD_STATE", temp_state.path())
        .args(["beads", "status", "specks-workflow.md", "--json"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run status after work");

    let status_after_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&status_after_work.stdout)).unwrap();
    let steps_after = status_after_json["data"]["files"][0]["steps"].as_array().unwrap();

    let step_0_after = steps_after.iter().find(|s| s["anchor"] == "step-0").unwrap();
    let step_1_after = steps_after.iter().find(|s| s["anchor"] == "step-1").unwrap();

    assert_eq!(step_0_after["status"], "complete", "Step 0 should be complete after closing bead");
    assert_eq!(step_1_after["status"], "ready", "Step 1 should be ready after Step 0 complete");

    // Step 5: Pull completion back to checkboxes
    let pull_output = Command::new(specks_binary())
        .env("SPECKS_BD_PATH", bd_fake_path())
        .env("SPECKS_BD_STATE", temp_state.path())
        .args(["beads", "pull", "specks-workflow.md", "--json"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run pull");

    assert!(pull_output.status.success(), "pull should succeed");
    let pull_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&pull_output.stdout)).unwrap();
    assert!(
        pull_json["data"]["total_updated"].as_u64().unwrap() >= 1,
        "should update checkboxes from completed bead"
    );

    // Verify checkboxes were updated in speck file
    let speck_after_pull = fs::read_to_string(temp.path().join(".specks/specks-workflow.md"))
        .expect("failed to read speck after pull");
    assert!(
        speck_after_pull.contains("[x] Base works") || speck_after_pull.contains("[X] Base works"),
        "Step 0 checkpoint should be checked after pull"
    );
}
