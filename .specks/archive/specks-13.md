## Phase 1.0: Agent Execution Robustness Improvements {#phase-robustness}

**Purpose:** Enhance agent execution reliability by adding log rotation, health checks, size validation, and JSON contract enforcement to prevent unbounded log growth and detect configuration drift.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | kocienda |
| Status | draft |
| Target branch | main |
| Tracking issue/PR | TBD |
| Last updated | 2026-02-09 |

---

### Phase Overview {#phase-overview}

#### Context {#context}

As the specks project matures, implementation logs grow unboundedly, worktree configurations can drift, and agent JSON output contracts lack formal validation. These issues create operational friction: large logs slow down parsing and reading, worktree path misconfigurations cause silent failures, and malformed agent JSON can crash orchestrators.

This phase introduces defensive infrastructure: CLI commands for log rotation, a `specks doctor` health check command, size validation in the committer agent, and JSON contract enforcement at both the agent and orchestrator levels.

#### Strategy {#strategy}

- Add `specks log rotate` and `specks log prepend` CLI commands for log management
- Implement automatic log rotation as a hook in `specks beads close`
- Create `specks doctor` command with pluggable health checks
- Add size validation to committer-agent with auto-rotation on oversized logs
- Enforce JSON output validation in agent system prompts and orchestrator parsing
- Add worktree path verification patterns to detect misconfigured worktrees

#### Stakeholders / Primary Customers {#stakeholders}

1. Developers using specks for long-running projects with many implementation cycles
2. Automation systems that depend on predictable log sizes and valid JSON

#### Success Criteria (Measurable) {#success-criteria}

- `specks log rotate` archives logs over 500 lines or 100KB (whichever hits first)
- `specks doctor` detects at least: log size issues, worktree inconsistencies, broken refs
- Committer-agent auto-rotates log when size threshold exceeded
- Agent JSON output is validated at both agent and orchestrator levels
- All new commands follow existing CLI patterns (--json, exit codes, JsonResponse)

#### Scope {#scope}

1. `specks log rotate` command with threshold-based archival
2. `specks log prepend` command for atomic log entry insertion
3. Auto-rotation hook in `specks beads close`
4. `specks doctor` command with standard health checks
5. Size checks in committer-agent with auto-rotation behavior
6. JSON validation enforcement in agent contracts and orchestrator

#### Non-goals (Explicitly out of scope) {#non-goals}

- Log compression or deduplication
- Remote log shipping or centralized logging
- User-configurable rotation thresholds (hardcoded for v1)
- Interactive doctor repair commands (report only for v1)

#### Dependencies / Prerequisites {#dependencies}

- Existing `.specks/archive/` directory pattern
- Existing implementation log format with YAML frontmatter entries
- Existing `specks beads close` command infrastructure

#### Constraints {#constraints}

- Rotation must be atomic (no partial archives)
- Auto-rotation must not block step completion (warn on failure)
- Doctor checks must complete in under 5 seconds for typical projects
- All changes must be backward compatible with existing logs

#### Assumptions {#assumptions}

- Log rotation will archive to `.specks/archive/` directory (pattern already exists)
- Rotation format will match existing `archive/implementation-log-2026-02-pre.md` pattern
- Size checks will warn but not block by default to avoid breaking existing workflows
- JSON validation will use serde_json parsing to verify contract compliance
- Worktree path verification will check that paths start with `.specks-worktrees/` and exist
- Doctor command will follow existing CLI patterns (--json flag, exit codes, JsonResponse)
- All new commands will have integration tests following nextest conventions

---

### 1.0.0 Design Decisions {#design-decisions}

#### [D01] Combined rotation threshold: 500 lines OR 100KB (DECIDED) {#d01-rotation-threshold}

**Decision:** Log rotation triggers when the implementation log exceeds 500 lines OR 100KB, whichever threshold is hit first.

**Rationale:**
- Line count catches logs with many short entries (frequent small commits)
- Byte size catches logs with few but verbose entries (detailed summaries)
- Combined check ensures logs stay manageable regardless of entry style
- Thresholds chosen based on typical IDE/editor performance with markdown files

**Implications:**
- Need to check both line count and file size in rotation logic
- Archive naming includes timestamp to avoid collisions
- Thresholds are hardcoded (configuration deferred to future work)

---

#### [D02] Archive naming pattern: implementation-log-YYYY-MM-DD-HHMMSS.md (DECIDED) {#d02-archive-naming}

**Decision:** Archived logs use the pattern `implementation-log-YYYY-MM-DD-HHMMSS.md` in the `.specks/archive/` directory.

**Rationale:**
- Timestamp ensures uniqueness and sortability
- Matches the style of existing `implementation-log-2026-02-pre.md` archive
- ISO-8601-ish format is unambiguous across locales

**Implications:**
- Archive directory must exist (create if missing)
- Multiple rotations on same day get unique filenames via seconds precision

---

#### [D03] Auto-rotation behavior: rotate and commit both files (DECIDED) {#d03-auto-rotate-behavior}

**Decision:** When committer-agent detects an oversized log, it automatically rotates the log and commits both the archived file and the fresh log file together.

**Rationale:**
- Keeps the git history clean with a single commit for rotation
- Avoids leaving stale large logs if rotation is deferred
- Auto-rotation is transparent to the user

**Implications:**
- Committer must stage both files when rotation occurs
- Commit message indicates rotation occurred
- Rotation failure is non-blocking (set `needs_reconcile`)

---

#### [D04] Doctor standard checks: minimal + log size + worktree + broken refs (DECIDED) {#d04-doctor-checks}

**Decision:** The `specks doctor` command runs these checks: (1) specks initialized, (2) log size within thresholds, (3) worktree consistency, (4) broken anchor references in specks.

**Rationale:**
- These are the most common issues that cause silent failures
- Checks can complete quickly (under 5 seconds)
- Provides actionable output for each issue

**Implications:**
- Each check has pass/warn/fail status
- JSON output includes all checks with details
- Exit code is non-zero if any check fails

---

#### [D05] Dual JSON validation: agents self-validate, orchestrator validates contract (DECIDED) {#d05-json-validation}

**Decision:** Agents must self-validate their JSON output before returning, AND orchestrators must validate the received JSON against the expected contract.

**Rationale:**
- Defense in depth: agents catch errors early, orchestrators catch agent bugs
- Self-validation provides better error messages (agent knows context)
- Orchestrator validation provides contract enforcement

**Implications:**
- Agent prompts include JSON schema requirements
- Orchestrators use serde_json to parse and validate fields
- Validation failures produce structured error responses

---

#### [D06] Worktree path pattern: must start with .specks-worktrees/ (DECIDED) {#d06-worktree-path}

**Decision:** Valid worktree paths must start with `.specks-worktrees/` and the directory must exist.

**Rationale:**
- Consistent location makes cleanup and discovery reliable
- Existence check catches stale session.json files
- Pattern validation catches manual path entry errors

**Implications:**
- Doctor check verifies all worktree paths match pattern
- Worktree create enforces the pattern
- Stale worktrees are flagged for cleanup

---

### 1.0.1 Specification {#specification}

#### 1.0.1.1 Log Rotate Command {#cmd-log-rotate}

```
specks log rotate [--force] [--json] [--quiet] [--verbose]
```

**Arguments:** None

**Flags:**
- `--force`: Rotate even if below thresholds
- `--json`: Output in JSON format
- `--quiet`: Suppress non-error output
- `--verbose`: Show detailed progress

**Behavior:**
1. Check if `.specks/specks-implementation-log.md` exists
2. Get line count and byte size
3. If either exceeds threshold (or --force), proceed with rotation
4. Create archive directory if missing
5. Move log to `.specks/archive/implementation-log-YYYY-MM-DD-HHMMSS.md`
6. Create fresh log with header template
7. Report success with archived file path

**Spec S01: Log Rotate Response Schema** {#s01-rotate-response}

```json
{
  "status": "ok",
  "rotated": true,
  "archived_path": ".specks/archive/implementation-log-2026-02-09-143022.md",
  "original_lines": 523,
  "original_bytes": 98432,
  "reason": "line_count_exceeded"
}
```

**Table T01: Rotation Reasons** {#t01-rotation-reasons}

| Reason | Description |
|--------|-------------|
| `line_count_exceeded` | Log exceeded 500 lines |
| `byte_size_exceeded` | Log exceeded 100KB |
| `forced` | User specified --force |
| `not_needed` | Below thresholds (status: ok, rotated: false) |

---

#### 1.0.1.2 Log Prepend Command {#cmd-log-prepend}

```
specks log prepend --step <step> --speck <speck> --summary <summary> [--bead <bead>] [--json]
```

**Arguments:**
- `--step`: Step anchor (e.g., `#step-0`)
- `--speck`: Speck file path
- `--summary`: One-line summary of completed work

**Optional:**
- `--bead`: Bead ID to record
- `--json`: Output in JSON format

**Behavior:**
1. Read current implementation log
2. Generate YAML frontmatter entry with timestamp
3. Prepend entry after the header section
4. Write updated log atomically

**Spec S02: Log Prepend Response Schema** {#s02-prepend-response}

```json
{
  "status": "ok",
  "entry_added": true,
  "step": "#step-0",
  "speck": ".specks/specks-13.md",
  "timestamp": "2026-02-09T14:30:00Z"
}
```

---

#### 1.0.1.3 Doctor Command {#cmd-doctor}

```
specks doctor [--json] [--quiet] [--verbose]
```

**Behavior:**
1. Run all health checks in sequence
2. Collect results with pass/warn/fail status
3. Report summary and details
4. Exit with code 0 if all pass, 1 if any warnings, 2 if any failures

**Spec S03: Doctor Response Schema** {#s03-doctor-response}

```json
{
  "status": "ok",
  "checks": [
    {
      "name": "initialized",
      "status": "pass",
      "message": "Specks is initialized"
    },
    {
      "name": "log_size",
      "status": "warn",
      "message": "Implementation log is 487 lines (approaching 500 limit)",
      "details": { "lines": 487, "bytes": 89234 }
    },
    {
      "name": "worktrees",
      "status": "pass",
      "message": "2 worktrees found, all paths valid"
    },
    {
      "name": "broken_refs",
      "status": "fail",
      "message": "3 broken anchor references found",
      "details": { "refs": ["#missing-anchor", "#old-step", "#typo"] }
    }
  ],
  "summary": {
    "passed": 2,
    "warnings": 1,
    "failures": 1
  }
}
```

**Table T02: Doctor Check Types** {#t02-doctor-checks}

| Check | Pass | Warn | Fail |
|-------|------|------|------|
| `initialized` | .specks/ exists with required files | - | Missing .specks/ or required files |
| `log_size` | Under 400 lines AND 80KB | 400-500 lines OR 80-100KB | Over 500 lines OR 100KB |
| `worktrees` | All paths valid and exist | - | Invalid paths or missing directories |
| `broken_refs` | No broken references | - | Any broken anchor references |

---

#### 1.0.1.4 Beads Close Auto-Rotation Hook {#beads-close-hook}

When `specks beads close` is called, after closing the bead:

1. Check implementation log size
2. If over threshold, call `specks log rotate` internally
3. Report rotation in output if it occurred

This ensures logs don't grow unboundedly during implementation sessions.

---

#### 1.0.1.5 Committer-Agent Size Check {#committer-size-check}

Add to committer-agent workflow:

1. Before prepending log entry, check current log size
2. If over threshold, rotate log first
3. Stage both archived file and fresh log
4. Include rotation in commit

**Output contract addition:**

```json
{
  "log_rotated": true,
  "archived_path": ".specks/archive/implementation-log-2026-02-09-143022.md"
}
```

---

#### 1.0.1.6 JSON Validation Enforcement {#json-validation}

**Agent self-validation (add to agent system prompts):**

```
Before returning your JSON response:
1. Verify all required fields are present
2. Verify field types match the contract
3. If validation fails, return an error response instead
```

**Orchestrator validation (in implementer skill):**

```rust
// Parse agent output
let response: AgentResponse = serde_json::from_str(&output)
    .map_err(|e| format!("Agent returned invalid JSON: {}", e))?;

// Validate required fields
if response.status.is_none() {
    return Err("Agent response missing 'status' field");
}
```

---

### 1.0.2 Symbol Inventory {#symbol-inventory}

#### 1.0.2.1 New files {#new-files}

| File | Purpose |
|------|---------|
| `crates/specks/src/commands/log.rs` | Log subcommand (rotate, prepend) |
| `crates/specks/src/commands/doctor.rs` | Doctor command implementation |

#### 1.0.2.2 Symbols to add / modify {#symbols}

| Symbol | Kind | Location | Notes |
|--------|------|----------|-------|
| `Log` | enum variant | `cli.rs` | New Commands variant |
| `LogCommands` | enum | `cli.rs` | Rotate, Prepend subcommands |
| `Doctor` | enum variant | `cli.rs` | New Commands variant |
| `run_log_rotate` | fn | `commands/log.rs` | Rotate implementation |
| `run_log_prepend` | fn | `commands/log.rs` | Prepend implementation |
| `run_doctor` | fn | `commands/doctor.rs` | Doctor implementation |
| `RotateData` | struct | `commands/log.rs` | JSON output for rotate |
| `PrependData` | struct | `commands/log.rs` | JSON output for prepend |
| `DoctorData` | struct | `commands/doctor.rs` | JSON output for doctor |
| `HealthCheck` | struct | `commands/doctor.rs` | Individual check result |
| `check_log_size` | fn | `commands/doctor.rs` | Log size check |
| `check_worktrees` | fn | `commands/doctor.rs` | Worktree consistency check |
| `check_broken_refs` | fn | `commands/doctor.rs` | Broken reference check |
| `LOG_LINE_THRESHOLD` | const | `commands/log.rs` | 500 lines |
| `LOG_BYTE_THRESHOLD` | const | `commands/log.rs` | 102400 bytes (100KB) |

---

### 1.0.3 Test Plan Concepts {#test-plan-concepts}

#### Test Categories {#test-categories}

| Category | Purpose | When to use |
|----------|---------|-------------|
| **Unit** | Test threshold logic, path patterns | Size checks, worktree validation |
| **Integration** | Test full commands with real files | End-to-end command flows |
| **Golden** | Verify JSON output format | Output schema compliance |

#### Test Scenarios {#test-scenarios}

**Unit tests:**
- `test_log_exceeds_line_threshold`: 501 lines triggers rotation
- `test_log_exceeds_byte_threshold`: 101KB triggers rotation
- `test_log_under_thresholds`: No rotation when below both
- `test_worktree_path_valid`: `.specks-worktrees/foo` is valid
- `test_worktree_path_invalid`: `../worktrees/foo` is invalid

**Integration tests:**
- `test_log_rotate_creates_archive`: Full rotation flow
- `test_log_prepend_adds_entry`: Entry appears at top
- `test_doctor_reports_issues`: Doctor catches known problems
- `test_beads_close_triggers_rotation`: Auto-rotation hook

---

### 1.0.4 Execution Steps {#execution-steps}

#### Step 0: Add log subcommand skeleton {#step-0}

**Commit:** `feat(cli): add specks log subcommand skeleton`

**References:** [D01] Rotation threshold, [D02] Archive naming, (#cmd-log-rotate, #cmd-log-prepend, #symbols)

**Artifacts:**
- `crates/specks/src/commands/log.rs` - New file with command structure
- `crates/specks/src/commands/mod.rs` - Export log module
- `crates/specks/src/cli.rs` - Add Log variant and LogCommands enum

**Tasks:**
- [ ] Create `log.rs` with `run_log_rotate` and `run_log_prepend` function signatures
- [ ] Add `Log` variant to `Commands` enum in `cli.rs`
- [ ] Add `LogCommands` enum with `Rotate` and `Prepend` variants
- [ ] Add command routing in `main.rs`
- [ ] Implement `--force` flag for rotate
- [ ] Add `RotateData` and `PrependData` structs for JSON output

**Tests:**
- [ ] Unit test: CLI parses `specks log rotate` command correctly
- [ ] Unit test: CLI parses `specks log prepend` with required args

**Checkpoint:**
- [ ] `cargo build` succeeds
- [ ] `cargo nextest run` passes
- [ ] `specks log --help` shows subcommands

**Rollback:**
- Revert commit, remove log.rs

**Commit after all checkpoints pass.**

---

#### Step 1: Implement log rotate {#step-1}

**Depends on:** #step-0

**Commit:** `feat(log): implement log rotation with threshold detection`

**References:** [D01] Rotation threshold, [D02] Archive naming, Table T01, Spec S01, (#cmd-log-rotate)

**Artifacts:**
- `run_log_rotate` function with full implementation
- `LOG_LINE_THRESHOLD` and `LOG_BYTE_THRESHOLD` constants

**Tasks:**
- [ ] Implement line count and byte size detection
- [ ] Implement threshold comparison logic
- [ ] Create archive directory if missing
- [ ] Generate archive filename with timestamp
- [ ] Move log to archive (atomic via rename)
- [ ] Create fresh log with header template
- [ ] Return `RotateData` with rotation details

**Tests:**
- [ ] Unit test: threshold detection for lines
- [ ] Unit test: threshold detection for bytes
- [ ] Unit test: archive filename generation
- [ ] Integration test: full rotation creates archive and fresh log

**Checkpoint:**
- [ ] `cargo nextest run` passes
- [ ] Manual test: `specks log rotate --force` creates archive

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

#### Step 2: Implement log prepend {#step-2}

**Depends on:** #step-0

**Commit:** `feat(log): implement log prepend for atomic entry insertion`

**References:** Spec S02, (#cmd-log-prepend)

**Artifacts:**
- `run_log_prepend` function with full implementation
- YAML frontmatter entry generation

**Tasks:**
- [ ] Parse command arguments (step, speck, summary, bead)
- [ ] Read current implementation log
- [ ] Generate YAML frontmatter entry with timestamp
- [ ] Find insertion point (after header, before first entry)
- [ ] Prepend entry atomically
- [ ] Return `PrependData` with success details

**Tests:**
- [ ] Unit test: YAML entry generation format
- [ ] Unit test: insertion point detection
- [ ] Integration test: prepend adds entry at correct position

**Checkpoint:**
- [ ] `cargo nextest run` passes
- [ ] Manual test: `specks log prepend` adds entry to log

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

#### Step 3: Add doctor command {#step-3}

**Depends on:** #step-1

**Commit:** `feat(cli): add specks doctor health check command`

**References:** [D04] Doctor checks, [D06] Worktree path, Table T02, Spec S03, (#cmd-doctor, #symbols)

**Artifacts:**
- `crates/specks/src/commands/doctor.rs` - New file
- `DoctorData`, `HealthCheck` structs
- Individual check functions

**Tasks:**
- [ ] Create `doctor.rs` with `run_doctor` function
- [ ] Add `Doctor` variant to `Commands` enum
- [ ] Implement `check_initialized` (verifies .specks/ exists)
- [ ] Implement `check_log_size` (line and byte thresholds with warn zone)
- [ ] Implement `check_worktrees` (path pattern and existence)
- [ ] Implement `check_broken_refs` (anchor validation in specks)
- [ ] Aggregate results with pass/warn/fail summary
- [ ] Set exit codes (0=pass, 1=warn, 2=fail)

**Tests:**
- [ ] Unit test: each check function independently
- [ ] Integration test: doctor with clean project reports all pass
- [ ] Integration test: doctor with issues reports failures

**Checkpoint:**
- [ ] `cargo nextest run` passes
- [ ] `specks doctor` runs and reports status
- [ ] `specks doctor --json` outputs valid JSON

**Rollback:**
- Revert commit, remove doctor.rs

**Commit after all checkpoints pass.**

---

#### Step 4: Add auto-rotation hook to beads close {#step-4}

**Depends on:** #step-1

**Commit:** `feat(beads): add auto-rotation hook to beads close`

**References:** [D03] Auto-rotate behavior, (#beads-close-hook)

**Artifacts:**
- Modified `run_beads_close` to check log size after close
- Rotation trigger integrated into close workflow

**Tasks:**
- [ ] After successful bead close, check log size
- [ ] If over threshold, invoke rotate logic
- [ ] Include rotation status in output JSON
- [ ] Document the hook in command help text

**Tests:**
- [ ] Integration test: beads close triggers rotation when over threshold
- [ ] Integration test: beads close does not rotate when under threshold

**Checkpoint:**
- [ ] `cargo nextest run` passes
- [ ] Beads close with large log triggers rotation

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

#### Step 5: Update committer-agent with size checks {#step-5}

**Depends on:** #step-1

**Commit:** `feat(agents): add log size checks to committer-agent`

**References:** [D03] Auto-rotate behavior, (#committer-size-check)

**Artifacts:**
- Updated `agents/committer-agent.md` with size check workflow
- New output fields: `log_rotated`, `archived_path`

**Tasks:**
- [ ] Add size check section to committer-agent workflow
- [ ] Document threshold values in agent prompt
- [ ] Add output contract fields for rotation reporting
- [ ] Update example outputs to show rotation case

**Tests:**
- [ ] Validate agent markdown structure
- [ ] Verify output contract includes new fields

**Checkpoint:**
- [ ] Agent file parses correctly
- [ ] Output contract documented

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

#### Step 6: Add JSON validation to agent contracts {#step-6}

**Depends on:** #step-0

**Commit:** `feat(agents): enforce JSON validation in agent contracts`

**References:** [D05] JSON validation, (#json-validation)

**Artifacts:**
- Updated agent system prompts with validation requirements
- Orchestrator validation patterns documented

**Tasks:**
- [ ] Add self-validation instructions to each agent's output section
- [ ] Document required fields and types in each contract
- [ ] Add validation example to implementer skill
- [ ] Update error handling guidance for malformed JSON

**Tests:**
- [ ] Review each agent for validation instructions
- [ ] Verify contract completeness

**Checkpoint:**
- [ ] All agent files include validation instructions
- [ ] Implementer skill documents validation pattern

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

#### Step 7: Add worktree path verification {#step-7}

**Depends on:** #step-3

**Commit:** `feat(worktree): add path verification patterns`

**References:** [D06] Worktree path, (#cmd-doctor)

**Artifacts:**
- Path validation function in worktree module
- Doctor check uses shared validation

**Tasks:**
- [ ] Implement `is_valid_worktree_path` function
- [ ] Verify path starts with `.specks-worktrees/`
- [ ] Verify directory exists
- [ ] Use in doctor check and worktree create
- [ ] Add helpful error messages for invalid paths

**Tests:**
- [ ] Unit test: valid path patterns
- [ ] Unit test: invalid path patterns
- [ ] Integration test: worktree create rejects invalid paths

**Checkpoint:**
- [ ] `cargo nextest run` passes
- [ ] Doctor reports invalid worktree paths

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

#### Step 8: Documentation and final polish {#step-8}

**Depends on:** #step-3, #step-4, #step-5, #step-6, #step-7

**Commit:** `docs: document log rotation, doctor, and validation features`

**References:** (#cmd-log-rotate, #cmd-log-prepend, #cmd-doctor)

**Artifacts:**
- Updated CLAUDE.md with new commands
- Help text polish for all new commands

**Tasks:**
- [ ] Add `specks log rotate` to CLAUDE.md CLI commands section
- [ ] Add `specks log prepend` to CLAUDE.md CLI commands section
- [ ] Add `specks doctor` to CLAUDE.md CLI commands section
- [ ] Review and polish --help text for all new commands
- [ ] Add troubleshooting section for common doctor findings

**Tests:**
- [ ] Help text includes all documented flags
- [ ] CLAUDE.md is accurate and complete

**Checkpoint:**
- [ ] `cargo nextest run` passes
- [ ] All new commands documented in CLAUDE.md

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

### 1.0.5 Deliverables and Checkpoints {#deliverables}

**Deliverable:** A robust agent execution environment with log management, health checks, and validation enforcement.

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

- [ ] `specks log rotate` archives logs over 500 lines or 100KB
- [ ] `specks log prepend` atomically adds log entries
- [ ] `specks doctor` reports log size, worktree, and broken ref issues
- [ ] `specks beads close` auto-rotates oversized logs
- [ ] Committer-agent documents size check workflow
- [ ] All agents include JSON self-validation instructions
- [ ] CLAUDE.md documents all new commands

**Acceptance tests:**
- [ ] Integration test: log rotation creates archive and fresh log
- [ ] Integration test: doctor detects known issues
- [ ] Integration test: beads close triggers rotation
- [ ] Manual test: end-to-end workflow with rotation

#### Roadmap / Follow-ons (Explicitly Not Required for Phase Close) {#roadmap}

- [ ] User-configurable rotation thresholds via config.toml
- [ ] Log compression for archived files
- [ ] Doctor auto-repair for fixable issues
- [ ] Validation schema definitions as JSON Schema files

| Checkpoint | Verification |
|------------|--------------|
| CLI parses correctly | `specks log --help`, `specks doctor --help` |
| Build succeeds | `cargo build` |
| Tests pass | `cargo nextest run` |
| Commands work | Manual test of rotate, prepend, doctor |

**Commit after all checkpoints pass.**
