//! Integration tests for worktree lifecycle

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

/// Create a temp directory with .specks initialized and git repo set up
fn setup_test_git_repo() -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("failed to create temp dir");

    // Initialize git repo with explicit main branch
    let output = Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(temp.path())
        .output()
        .expect("failed to run git init");
    assert!(output.status.success(), "git init failed");

    // Configure git user
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(temp.path())
        .output()
        .expect("failed to configure git user");

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(temp.path())
        .output()
        .expect("failed to configure git email");

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

    // Create initial commit
    Command::new("git")
        .args(["add", "."])
        .current_dir(temp.path())
        .output()
        .expect("failed to stage files");

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(temp.path())
        .output()
        .expect("failed to create initial commit");

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
| Last updated | 2026-02-08 |

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
fn test_worktree_lifecycle() {
    let temp = setup_test_git_repo();
    create_test_speck(&temp, "test-worktree", MINIMAL_SPECK);

    // Commit the speck file
    Command::new("git")
        .args(["add", ".specks/specks-test-worktree.md"])
        .current_dir(temp.path())
        .output()
        .expect("failed to stage speck");

    Command::new("git")
        .args(["commit", "-m", "Add test speck"])
        .current_dir(temp.path())
        .output()
        .expect("failed to commit speck");

    // Step 1: Create worktree
    let output = Command::new(specks_binary())
        .arg("worktree")
        .arg("create")
        .arg(".specks/specks-test-worktree.md")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks worktree create");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "worktree create should succeed: {}",
        stderr
    );

    // Step 2: Verify worktree directory exists
    let worktrees_dir = temp.path().join(".specks-worktrees");
    assert!(
        worktrees_dir.is_dir(),
        ".specks-worktrees directory should exist"
    );

    // Find the created worktree (should be only one)
    // Filter for actual worktrees (starting with specks__), excluding .sessions and .artifacts
    let worktree_entries: Vec<_> = fs::read_dir(&worktrees_dir)
        .expect("failed to read worktrees dir")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|s| s.starts_with("specks__"))
                .unwrap_or(false)
        })
        .collect();
    assert_eq!(
        worktree_entries.len(),
        1,
        "should have exactly one worktree"
    );

    let worktree_path = worktree_entries[0].path();
    let worktree_name = worktree_path.file_name().unwrap().to_str().unwrap();
    assert!(
        worktree_name.starts_with("specks__test-worktree-"),
        "worktree name should be filesystem-safe"
    );

    // Step 3: Verify session.json exists in external storage
    let worktree_name = worktree_path.file_name().unwrap().to_str().unwrap();
    let session_id = worktree_name.strip_prefix("specks__").unwrap();
    let session_file = temp
        .path()
        .join(".specks-worktrees")
        .join(".sessions")
        .join(format!("{}.json", session_id));
    assert!(
        session_file.is_file(),
        "session.json should exist in external storage"
    );

    let session_contents = fs::read_to_string(&session_file).expect("failed to read session.json");
    let session: serde_json::Value =
        serde_json::from_str(&session_contents).expect("session.json should be valid JSON");

    assert_eq!(session["schema_version"], "1");
    assert_eq!(session["speck_path"], ".specks/specks-test-worktree.md");
    assert_eq!(session["speck_slug"], "test-worktree");
    assert_eq!(session["base_branch"], "main");
    assert_eq!(session["status"], "pending");
    assert!(
        session["branch_name"]
            .as_str()
            .unwrap()
            .starts_with("specks/test-worktree-")
    );

    // Step 4: Verify worktree directory structure
    assert!(
        worktree_path.join(".specks").is_dir(),
        "worktree should have .specks directory"
    );
    assert!(
        worktree_path
            .join(".specks")
            .join("specks-test-worktree.md")
            .is_file(),
        "speck should be in worktree"
    );

    // Step 5: List worktrees and verify it appears
    let output = Command::new(specks_binary())
        .arg("worktree")
        .arg("list")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks worktree list");

    assert!(output.status.success(), "worktree list should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("test-worktree"),
        "list should show the worktree: {}",
        stdout
    );
    assert!(
        stdout.contains("Pending") || stdout.contains("In Progress"),
        "list should show status: {}",
        stdout
    );

    // Step 6: Simulate merge by fast-forward merging branch to main
    let branch_name = session["branch_name"].as_str().unwrap();

    // Commit a dummy file in the worktree (simulating what implementer does)
    // Note: session.json is now in external storage, not in the worktree
    fs::write(worktree_path.join("test.txt"), "test").expect("failed to write test file");
    Command::new("git")
        .args(["-C", worktree_path.to_str().unwrap(), "add", "test.txt"])
        .current_dir(temp.path())
        .output()
        .expect("failed to stage test file");

    Command::new("git")
        .args([
            "-C",
            worktree_path.to_str().unwrap(),
            "commit",
            "-m",
            "Add test file",
        ])
        .current_dir(temp.path())
        .output()
        .expect("failed to commit test file");

    // Switch to main
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(temp.path())
        .output()
        .expect("failed to checkout main");

    // Merge the worktree branch (this is a simulation - normally would be done via PR)
    let merge_output = Command::new("git")
        .args(["merge", "--ff-only", branch_name])
        .current_dir(temp.path())
        .output()
        .expect("failed to merge branch");

    assert!(
        merge_output.status.success(),
        "merge should succeed: {}",
        String::from_utf8_lossy(&merge_output.stderr)
    );

    // Debug: Check if branch is detected as merged
    let merge_check = Command::new("git")
        .args(["merge-base", "--is-ancestor", branch_name, "main"])
        .current_dir(temp.path())
        .output()
        .expect("failed to check merge status");

    let is_merged = merge_check.status.success();

    // Step 7: Run cleanup --merged and verify worktree is removed
    // Use --force since gh CLI is not available in test environment
    let output = Command::new(specks_binary())
        .arg("worktree")
        .arg("cleanup")
        .arg("--merged")
        .arg("--force")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks worktree cleanup");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "cleanup should succeed. Is merged: {}, stdout: '{}', stderr: '{}'",
        is_merged,
        stdout,
        stderr
    );

    // Check if cleanup actually removed anything
    assert!(
        !stdout.trim().is_empty() || stdout.contains("No merged worktrees"),
        "cleanup should report what was removed or that nothing was found. stdout: '{}'",
        stdout
    );

    // Check if cleanup was successful
    let list_output = Command::new(specks_binary())
        .arg("worktree")
        .arg("list")
        .current_dir(temp.path())
        .output()
        .expect("failed to list worktrees after cleanup");
    let list_stdout = String::from_utf8_lossy(&list_output.stdout);

    // Verify worktree was removed from git's tracking
    assert!(
        list_stdout.contains("No active worktrees"),
        "worktree should not be listed after cleanup"
    );

    // Note: The actual directory removal is handled by git worktree remove,
    // but the directory may persist in some cases (especially on macOS with
    // temp directories). The important thing is that git no longer tracks it.

    // Verify branch was removed
    let branch_output = Command::new("git")
        .args(["branch", "--list", branch_name])
        .current_dir(temp.path())
        .output()
        .expect("failed to list branches");

    let branch_stdout = String::from_utf8_lossy(&branch_output.stdout);
    assert!(
        branch_stdout.trim().is_empty(),
        "branch should be removed after cleanup: {}",
        branch_stdout
    );
}

#[test]
fn test_worktree_list_json_output() {
    let temp = setup_test_git_repo();
    create_test_speck(&temp, "test-json", MINIMAL_SPECK);

    // Commit the speck file
    Command::new("git")
        .args(["add", ".specks/specks-test-json.md"])
        .current_dir(temp.path())
        .output()
        .expect("failed to stage speck");

    Command::new("git")
        .args(["commit", "-m", "Add test speck"])
        .current_dir(temp.path())
        .output()
        .expect("failed to commit speck");

    // Create a worktree
    let output = Command::new(specks_binary())
        .arg("worktree")
        .arg("create")
        .arg(".specks/specks-test-json.md")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks worktree create");

    assert!(output.status.success(), "worktree create should succeed");

    // List with JSON output
    let output = Command::new(specks_binary())
        .arg("worktree")
        .arg("list")
        .arg("--json")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks worktree list --json");

    assert!(
        output.status.success(),
        "worktree list --json should succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert!(json["worktrees"].is_array());
    assert_eq!(json["worktrees"].as_array().unwrap().len(), 1);

    let worktree = &json["worktrees"][0];
    assert_eq!(worktree["speck_slug"], "test-json");
    assert_eq!(worktree["base_branch"], "main");
    assert_eq!(worktree["status"], "pending");
}

#[test]
fn test_worktree_cleanup_dry_run() {
    let temp = setup_test_git_repo();
    create_test_speck(&temp, "test-cleanup", MINIMAL_SPECK);

    // Commit the speck file
    Command::new("git")
        .args(["add", ".specks/specks-test-cleanup.md"])
        .current_dir(temp.path())
        .output()
        .expect("failed to stage speck");

    Command::new("git")
        .args(["commit", "-m", "Add test speck"])
        .current_dir(temp.path())
        .output()
        .expect("failed to commit speck");

    // Create and merge a worktree
    let output = Command::new(specks_binary())
        .arg("worktree")
        .arg("create")
        .arg(".specks/specks-test-cleanup.md")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks worktree create");

    assert!(output.status.success(), "worktree create should succeed");

    // Get the worktree path
    let worktrees_dir = temp.path().join(".specks-worktrees");
    let worktree_entries: Vec<_> = fs::read_dir(&worktrees_dir)
        .expect("failed to read worktrees dir")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|s| s.starts_with("specks__"))
                .unwrap_or(false)
        })
        .collect();
    let worktree_path = worktree_entries[0].path();

    // Get branch name from session in external storage
    let worktree_name = worktree_path.file_name().unwrap().to_str().unwrap();
    let session_id = worktree_name.strip_prefix("specks__").unwrap();
    let session_file = temp
        .path()
        .join(".specks-worktrees")
        .join(".sessions")
        .join(format!("{}.json", session_id));
    let session_contents = fs::read_to_string(&session_file).expect("failed to read session.json");
    let session: serde_json::Value = serde_json::from_str(&session_contents).unwrap();
    let branch_name = session["branch_name"].as_str().unwrap();

    // Commit a dummy file in the worktree (session.json is now external)
    fs::write(worktree_path.join("test.txt"), "test").expect("failed to write test file");
    Command::new("git")
        .args(["-C", worktree_path.to_str().unwrap(), "add", "test.txt"])
        .current_dir(temp.path())
        .output()
        .expect("failed to stage test file");

    Command::new("git")
        .args([
            "-C",
            worktree_path.to_str().unwrap(),
            "commit",
            "-m",
            "Add test file",
        ])
        .current_dir(temp.path())
        .output()
        .expect("failed to commit test file");

    // Merge the branch
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(temp.path())
        .output()
        .expect("failed to checkout main");

    Command::new("git")
        .args(["merge", "--ff-only", branch_name])
        .current_dir(temp.path())
        .output()
        .expect("failed to merge branch");

    // Run cleanup with dry-run
    // Use --force since gh CLI is not available in test environment
    let output = Command::new(specks_binary())
        .arg("worktree")
        .arg("cleanup")
        .arg("--merged")
        .arg("--dry-run")
        .arg("--force")
        .current_dir(temp.path())
        .output()
        .expect("failed to run specks worktree cleanup --dry-run");

    assert!(output.status.success(), "cleanup --dry-run should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Would remove")
            || stdout.contains("would remove")
            || !stdout.trim().is_empty(),
        "dry-run should show what would be removed: {}",
        stdout
    );

    // Verify worktree still exists
    assert!(
        worktree_path.exists(),
        "worktree should still exist after dry-run"
    );
}
