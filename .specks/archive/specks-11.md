## Phase 11.0: CLI Consolidation for Agent Setup Tasks {#phase-cli-consolidation}

**Purpose:** Consolidate planner and implementer setup-agent tasks into atomic CLI commands, reducing agent complexity by ~85 lines and eliminating shell quoting, git command construction, and multi-step rollback error surfaces.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | specks |
| Status | draft |
| Target branch | main |
| Tracking issue/PR | TBD |
| Last updated | 2026-02-08 |
| Beads Root | `specks-0yy` |

---

### Phase Overview {#phase-overview}

#### Context {#context}

The planner and implementer skills currently construct shell commands inline for initialization checks, worktree creation with bead syncing, and step status parsing. This duplicated logic introduces error-prone patterns: shell quoting edge cases, git command construction, regex parsing of speck files, and multi-step rollback sequences. Moving these operations into the CLI creates a single source of truth with proper error handling and transactional semantics.

#### Strategy {#strategy}

- Add `--check` flag to `specks init` for lightweight initialization verification (exit code only)
- Enhance `specks worktree create` with `--sync-beads` flag that atomically creates worktree, syncs beads inside it, and commits annotations
- Extend `specks status` JSON output with `all_steps`, `completed_steps`, `remaining_steps`, `next_step`, `bead_mapping`, and `dependencies` fields
- Implement full rollback on failure for worktree operations (remove worktree and branch)
- Use existing `JsonResponse` envelope for all new JSON outputs
- Maintain backward compatibility with existing CLI behavior

#### Stakeholders / Primary Customers {#stakeholders}

1. Planner skill (uses init check and status for planning workflow)
2. Implementer skill (uses worktree create with beads sync and status for implementation workflow)

#### Success Criteria (Measurable) {#success-criteria}

- `specks init --check` returns exit code 0 in initialized project, non-zero otherwise, with no side effects
- `specks worktree create --sync-beads` creates worktree, syncs beads, commits annotations, returns JSON with bead_mapping in single invocation
- `specks status <path> --json` returns step state including all_steps, completed_steps, remaining_steps, next_step, bead_mapping in one call
- All new functionality has unit and integration tests
- Existing tests continue to pass

#### Scope {#scope}

1. `specks init --check` flag implementation
2. `specks worktree create --sync-beads` flag with transactional semantics
3. Extended `specks status --json` output structure
4. Unit tests for new functionality
5. Integration tests for transactional rollback behavior

#### Non-goals (Explicitly out of scope) {#non-goals}

- Modifying agent skill files (agents will be updated in a follow-on phase)
- Adding new subcommands (only extending existing commands)
- Changing non-JSON output formats
- Beads CLI wrapper modifications (using existing BeadsCli)

#### Dependencies / Prerequisites {#dependencies}

- Existing `specks init` command implementation
- Existing `specks worktree create` command implementation
- Existing `specks status` command implementation
- Existing `specks beads sync` functionality in specks-core

#### Constraints {#constraints}

- Must maintain backward compatibility with existing CLI behavior
- Exit codes must follow existing specks conventions (0=success, non-zero for specific errors)
- JSON output must use existing `JsonResponse` envelope structure
- Synchronous operations only (no background tasks)

#### Assumptions {#assumptions}

- The 85 lines of complexity reduction refers to shell command construction, output parsing, and error handling currently duplicated across setup agents
- Exit codes will follow existing specks conventions (0=success, non-zero specific codes for different errors)
- JSON output will use the existing JsonResponse envelope structure defined in output.rs
- The commands will be synchronous (no background tasks or async operations)
- Existing worktree integration tests will be extended to cover the new --sync-beads functionality
- Both setup agents will be updated to use the new CLI commands, reducing their tool dependencies (less Bash, more structured CLI calls)

---

### 11.0.0 Design Decisions {#design-decisions}

#### [D01] Init check uses flag on init command (DECIDED) {#d01-init-check-flag}

**Decision:** Add `--check` flag to `specks init` rather than creating a new subcommand.

**Rationale:**
- Keeps related functionality grouped together
- Avoids proliferation of subcommands
- Consistent with patterns like `git status` for checking state

**Implications:**
- `specks init --check` returns exit code only, no side effects
- Flag is mutually exclusive with `--force`
- JSON output when `--json` is also specified includes `{"initialized": true/false}`

#### [D02] Worktree create with sync-beads commits automatically (DECIDED) {#d02-sync-beads-commit}

**Decision:** The `--sync-beads` flag on `specks worktree create` handles the commit internally rather than leaving uncommitted changes.

**Rationale:**
- Atomic operation reduces error surface in agents
- Agents don't need to construct git commit commands
- Commit message can be standardized and consistent

**Implications:**
- Worktree is created with bead annotations already committed
- Rollback removes both worktree and branch on any failure
- Commit message: `chore(beads): sync bead annotations for <speck-name>`

#### [D03] Full rollback on worktree creation failure (DECIDED) {#d03-full-rollback}

**Decision:** If any step of `specks worktree create --sync-beads` fails, perform full rollback removing worktree and branch.

**Rationale:**
- Transactional semantics prevent partial state
- Agents don't need to implement their own cleanup
- Simplifies error recovery

**Implications:**
- On failure: remove worktree directory, delete branch, clean git state
- Return appropriate exit code and error message
- JSON output includes `rollback_performed: true` on failure

#### [D04] Extend existing status command (DECIDED) {#d04-extend-status}

**Decision:** Add new fields to existing `specks status` JSON output rather than creating a new command.

**Rationale:**
- Backward compatible (new fields are additive)
- Avoids command proliferation
- Single command for all step-related queries

**Implications:**
- Add `all_steps`, `completed_steps`, `remaining_steps`, `next_step` arrays to StatusData
- Add `bead_mapping` object mapping step anchors to bead IDs
- Add `dependencies` object mapping step anchors to their dependency anchors

---

### 11.0.1 Specification {#specification}

#### 11.0.1.1 Init Check Specification {#init-check-spec}

**Spec S01: specks init --check** {#s01-init-check}

**Behavior:**
- Check if `.specks/specks-skeleton.md` exists
- Return exit code 0 if initialized, exit code 9 (E009) if not
- No file modifications, no directory creation
- When combined with `--json`, output `{"initialized": true/false}`

**Exit Codes:**
| Code | Meaning |
|------|---------|
| 0 | Project is initialized |
| 9 | Project not initialized (E009) |

**JSON Response (when --json specified):**
```json
{
  "schema_version": "1",
  "command": "init",
  "status": "ok",
  "data": {
    "initialized": true,
    "path": ".specks/"
  },
  "issues": []
}
```

#### 11.0.1.2 Worktree Create with Sync-Beads Specification {#worktree-sync-beads-spec}

**Spec S02: specks worktree create --sync-beads** {#s02-worktree-sync-beads}

**Behavior:**
1. Validate speck file exists and has execution steps
2. Create git worktree with new branch from base
3. Inside worktree: run `specks beads sync` to create/update beads and write IDs to speck
4. Stage and commit bead annotation changes in worktree
5. Return JSON with worktree path, branch name, and bead mapping
6. On any failure: rollback by removing worktree and branch

**Commit Message:** `chore(beads): sync bead annotations for <speck-name>`

**Exit Codes:**
| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 3 | Worktree already exists |
| 4 | Git version insufficient |
| 5 | Not a git repository |
| 6 | Base branch not found |
| 7 | Speck file not found |
| 8 | Speck has no steps |
| 10 | Beads sync failed (new) |
| 11 | Bead commit failed (new) |

**JSON Response:**
```json
{
  "schema_version": "1",
  "command": "worktree create",
  "status": "ok",
  "data": {
    "worktree_path": ".specks-worktrees/specks__auth-20260208-120000",
    "branch_name": "specks/auth-20260208-120000",
    "base_branch": "main",
    "speck_path": ".specks/specks-auth.md",
    "total_steps": 5,
    "bead_mapping": {
      "#step-0": "bd-abc123",
      "#step-1": "bd-def456",
      "#step-2": "bd-ghi789"
    },
    "root_bead_id": "bd-root42"
  },
  "issues": []
}
```

**Rollback Response (on failure):**
```json
{
  "schema_version": "1",
  "command": "worktree create",
  "status": "error",
  "data": {
    "worktree_path": "",
    "branch_name": "",
    "rollback_performed": true
  },
  "issues": [
    {
      "code": "E010",
      "severity": "error",
      "message": "beads sync failed: bd not found"
    }
  ]
}
```

#### 11.0.1.3 Extended Status Specification {#extended-status-spec}

**Spec S03: specks status --json extended fields** {#s03-status-extended}

**New Fields in StatusData:**

| Field | Type | Description |
|-------|------|-------------|
| `all_steps` | `StepInfo[]` | All steps with anchor, title, number |
| `completed_steps` | `StepInfo[]` | Steps where all checkboxes are checked |
| `remaining_steps` | `StepInfo[]` | Steps with unchecked checkboxes |
| `next_step` | `StepInfo \| null` | First remaining step, or null if done |
| `bead_mapping` | `object` | Map of step anchor to bead ID |
| `dependencies` | `object` | Map of step anchor to dependency anchors |

**StepInfo Type:**
```json
{
  "anchor": "#step-0",
  "title": "Setup project structure",
  "number": "0",
  "bead_id": "bd-abc123"
}
```

**Full JSON Response Example:**
```json
{
  "schema_version": "1",
  "command": "status",
  "status": "ok",
  "data": {
    "name": "specks-auth",
    "status": "active",
    "progress": { "done": 5, "total": 12 },
    "steps": [ ... ],
    "all_steps": [
      { "anchor": "#step-0", "title": "Setup", "number": "0", "bead_id": "bd-abc" },
      { "anchor": "#step-1", "title": "Implement", "number": "1", "bead_id": "bd-def" }
    ],
    "completed_steps": [
      { "anchor": "#step-0", "title": "Setup", "number": "0", "bead_id": "bd-abc" }
    ],
    "remaining_steps": [
      { "anchor": "#step-1", "title": "Implement", "number": "1", "bead_id": "bd-def" }
    ],
    "next_step": { "anchor": "#step-1", "title": "Implement", "number": "1", "bead_id": "bd-def" },
    "bead_mapping": {
      "#step-0": "bd-abc",
      "#step-1": "bd-def"
    },
    "dependencies": {
      "#step-0": [],
      "#step-1": ["#step-0"]
    }
  },
  "issues": []
}
```

---

### 11.0.2 Symbol Inventory {#symbol-inventory}

#### 11.0.2.1 Modified Files {#modified-files}

| File | Purpose |
|------|---------|
| `crates/specks/src/cli.rs` | Add `--check` flag to Init, `--sync-beads` flag to Worktree Create |
| `crates/specks/src/commands/init.rs` | Implement init check logic |
| `crates/specks/src/commands/worktree.rs` | Implement sync-beads with rollback |
| `crates/specks/src/commands/status.rs` | Add extended JSON fields |
| `crates/specks/src/output.rs` | Add InitCheckData, extended CreateData, StepInfo types |

#### 11.0.2.2 New Types {#new-types}

| Symbol | Kind | Location | Notes |
|--------|------|----------|-------|
| `InitCheckData` | struct | `output.rs` | Data for init --check response |
| `StepInfo` | struct | `output.rs` | Lightweight step info for status |
| `ExtendedCreateData` | struct | `commands/worktree.rs` | Response for --sync-beads |

---

### 11.0.3 Test Plan Concepts {#test-plan-concepts}

#### Test Categories {#test-categories}

| Category | Purpose | When to use |
|----------|---------|-------------|
| **Unit** | Test individual functions in isolation | Init check logic, status field building |
| **Integration** | Test full command execution | Worktree create with sync, rollback behavior |
| **Golden** | Compare JSON output against snapshots | All JSON response formats |

---

### 11.0.4 Execution Steps {#execution-steps}

#### Step 0: Add init --check flag {#step-0}

**Bead:** `specks-0yy.1`

**Commit:** `feat(cli): add init --check flag for initialization verification`

**References:** [D01] Init check uses flag, Spec S01, (#init-check-spec, #context)

**Artifacts:**
- Modified `cli.rs` with `--check` flag on Init command
- Modified `init.rs` with check logic
- New `InitCheckData` type in `output.rs`

**Tasks:**
- [ ] Add `check: bool` field to Init command in cli.rs
- [ ] Make `--check` and `--force` mutually exclusive
- [ ] Implement `run_init_check()` function in init.rs
- [ ] Add `InitCheckData` struct to output.rs
- [ ] Update `run_init()` to call check when flag is set

**Tests:**
- [ ] Unit test: init check returns true when .specks/specks-skeleton.md exists
- [ ] Unit test: init check returns false when .specks does not exist
- [ ] Unit test: --check and --force are mutually exclusive
- [ ] Integration test: exit code 0 for initialized project
- [ ] Integration test: exit code 9 for uninitialized project
- [ ] Golden test: JSON output for init --check --json

**Checkpoint:**
- [ ] `cargo nextest run -p specks init`
- [ ] `specks init --check` in initialized project returns exit code 0
- [ ] `specks init --check --json` outputs correct JSON structure

**Rollback:**
- Revert changes to cli.rs, init.rs, output.rs

---

#### Step 1: Add extended status fields {#step-1}

**Depends on:** #step-0

**Bead:** `specks-0yy.2`

**Commit:** `feat(cli): extend status --json with step arrays and bead mapping`

**References:** [D04] Extend existing status, Spec S03, (#extended-status-spec, #strategy)

**Artifacts:**
- Modified `output.rs` with StepInfo and extended StatusData
- Modified `status.rs` with extended field building

**Tasks:**
- [ ] Add `StepInfo` struct to output.rs
- [ ] Add `all_steps`, `completed_steps`, `remaining_steps`, `next_step` fields to StatusData
- [ ] Add `bead_mapping` and `dependencies` fields to StatusData
- [ ] Implement `build_extended_status_data()` in status.rs
- [ ] Update `run_status()` to populate new fields when --json is used

**Tests:**
- [ ] Unit test: all_steps contains all step anchors
- [ ] Unit test: completed_steps only contains fully completed steps
- [ ] Unit test: remaining_steps excludes completed steps
- [ ] Unit test: next_step is first of remaining_steps
- [ ] Unit test: bead_mapping correctly maps anchors to bead IDs
- [ ] Unit test: dependencies correctly maps step dependencies
- [ ] Golden test: full JSON output with all new fields

**Checkpoint:**
- [ ] `cargo nextest run -p specks status`
- [ ] `specks status specks-1.md --json` includes all new fields
- [ ] New fields are empty arrays/null when appropriate

**Rollback:**
- Revert changes to output.rs, status.rs

---

#### Step 2: Implement worktree create --sync-beads {#step-2}

**Depends on:** #step-1

**Bead:** `specks-0yy.3`

**Commit:** `feat(cli): add worktree create --sync-beads with atomic commit and rollback`

**References:** [D02] Sync-beads commits automatically, [D03] Full rollback, Spec S02, (#worktree-sync-beads-spec, #d03-full-rollback)

**Artifacts:**
- Modified `cli.rs` with `--sync-beads` flag on Worktree Create
- Modified `worktree.rs` with sync-beads implementation
- Extended `CreateData` with bead_mapping and rollback_performed

**Tasks:**
- [ ] Add `sync_beads: bool` flag to Worktree Create in cli.rs
- [ ] Add `bead_mapping` and `root_bead_id` fields to CreateData
- [ ] Add `rollback_performed` field for error responses
- [ ] Implement `sync_beads_in_worktree()` helper function
- [ ] Implement `commit_bead_annotations()` helper function
- [ ] Implement `rollback_worktree_creation()` helper function
- [ ] Update `run_worktree_create()` to handle --sync-beads flag
- [ ] Parse bead sync output to build bead_mapping

**Tests:**
- [ ] Integration test: worktree created with beads synced and committed
- [ ] Integration test: bead_mapping in JSON output matches synced beads
- [ ] Integration test: rollback removes worktree on beads sync failure
- [ ] Integration test: rollback removes worktree on commit failure
- [ ] Integration test: rollback removes branch on any failure
- [ ] Unit test: sync_beads_in_worktree calls beads sync correctly
- [ ] Unit test: commit message follows expected format
- [ ] Golden test: JSON output for successful --sync-beads

**Checkpoint:**
- [ ] `cargo nextest run -p specks worktree`
- [ ] `specks worktree create .specks/specks-test.md --sync-beads` creates worktree with beads committed
- [ ] JSON output includes populated bead_mapping
- [ ] Simulate failure and verify rollback removes worktree and branch

**Rollback:**
- Revert changes to cli.rs, worktree.rs
- Clean up any test worktrees left behind

---

#### Step 3: Add new error codes and documentation {#step-3}

**Depends on:** #step-2

**Bead:** `specks-0yy.4`

**Commit:** `docs(cli): add error codes E010, E011 and update help text`

**References:** Spec S02 exit codes, (#specification, #strategy)

**Artifacts:**
- Updated error.rs with E010, E011 codes if needed
- Updated CLI help text for new flags

**Tasks:**
- [ ] Add E010 (BeadsSyncFailed) and E011 (BeadCommitFailed) to error types if not exists
- [ ] Update long_about text for init command mentioning --check
- [ ] Update long_about text for worktree create mentioning --sync-beads
- [ ] Verify all new exit codes are documented in --help output

**Tests:**
- [ ] Unit test: verify_cli() passes with new flags
- [ ] Integration test: --help includes new flag documentation

**Checkpoint:**
- [ ] `cargo nextest run -p specks cli`
- [ ] `specks init --help` documents --check flag
- [ ] `specks worktree create --help` documents --sync-beads flag

**Rollback:**
- Revert changes to error.rs, cli.rs

---

### 11.0.5 Deliverables and Checkpoints {#deliverables}

**Deliverable:** Three enhanced CLI commands that consolidate setup-agent logic: `specks init --check`, `specks worktree create --sync-beads`, and extended `specks status --json`.

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

- [ ] `specks init --check` returns correct exit code with no side effects
- [ ] `specks worktree create --sync-beads` atomically creates worktree with synced and committed beads
- [ ] `specks worktree create --sync-beads` performs full rollback on any failure
- [ ] `specks status --json` includes all_steps, completed_steps, remaining_steps, next_step, bead_mapping, dependencies
- [ ] All new functionality has unit and integration tests
- [ ] `cargo nextest run` passes with no warnings

**Acceptance tests:**
- [ ] Integration test: full workflow from init check through worktree create with sync-beads
- [ ] Integration test: rollback cleans up completely on failure
- [ ] Golden test: all JSON responses match expected schemas

#### Roadmap / Follow-ons (Explicitly Not Required for Phase Close) {#roadmap}

- [ ] Update planner skill to use `specks init --check` instead of shell test
- [ ] Update implementer skill to use `specks worktree create --sync-beads`
- [ ] Update implementer skill to use extended `specks status --json`
- [ ] Measure actual complexity reduction in agent code after CLI adoption