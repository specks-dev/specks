## Phase D.0: Status from Beads {#phase-status-from-beads}

**Purpose:** Rebuild `specks status` to pull real implementation state from beads, showing closed/ready/blocked steps with commit info and agent summaries, falling back to checkbox counting when beads are absent.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | kocienda |
| Status | draft |
| Target branch | main |
| Tracking issue/PR | -- |
| Last updated | 2026-02-12 |
| Beads Root | `specks-hfq` |

---

### Phase Overview {#phase-overview}

#### Context {#context}

Currently, `specks status` reads the speck file and counts checked/unchecked checkboxes to report progress. This only works if someone manually checks off items in the markdown. It does not reflect actual implementation state.

After Phases A-C, beads contain the complete state: which steps are done (closed beads), which are in progress (open + ready), which are blocked (open + waiting on deps), plus rich content about what was done (close_reason, notes). The status command needs to be rebuilt to use this real data when available.

#### Strategy {#strategy}

- Add a batch API `list_children_detailed` to `BeadsCli` that returns full `IssueDetails` (including `close_reason`) for all children of a parent bead in one call, avoiding N+1 subprocess calls.
- Auto-detect beads mode: use beads if `beads_root_id` exists in speck Plan Metadata, fall back to checkbox counting otherwise.
- Parse speck markdown to extract task/test/checkpoint counts for each step (for "ready" steps that show remaining work).
- Add `--full` flag that displays raw bead field content (description, acceptance_criteria, design, notes, close_reason) in sections.
- Redesign both text and JSON output formats to show complete/ready/blocked step states with commit hashes and summaries.
- Maintain backward compatibility: pre-beads specks continue to work with checkbox-based status.
- Add comprehensive tests including golden tests for known output.

#### Stakeholders / Primary Customers {#stakeholders}

1. Specks users checking implementation progress from the CLI
2. CI/CD dashboards consuming `--json` output for automated progress tracking
3. Agent orchestrators querying status to determine next steps

#### Success Criteria (Measurable) {#success-criteria}

- `specks status <speck>` shows real implementation state from beads when `beads_root_id` is present (verified by integration test with mock bead data)
- Complete steps display commit hash and summary extracted from `close_reason` (verified by unit test on `close_reason` parsing)
- Ready steps show remaining work counts (tasks/tests/checkpoints parsed from speck markdown, verified by unit test)
- Blocked steps show what they are waiting on (verified by unit test checking dependency resolution)
- `--full` flag renders raw bead field content (verified by golden test)
- JSON output conforms to the schema defined in Spec S01 (verified by golden test matching expected output)
- Fallback to checkbox counting works when no `beads_root_id` exists (verified by test with pre-beads speck)

#### Scope {#scope}

1. Add `list_children_detailed` batch API to `BeadsCli` in `specks-core`
2. Rewrite `specks status` command to use beads state when available
3. Add `--full` CLI flag for rich bead content view
4. Redesign text and JSON output formats
5. Update output types in `output.rs`
6. Comprehensive test coverage including golden tests

#### Non-goals (Explicitly out of scope) {#non-goals}

- Modifying `specks list` command (separate future work)
- Adding real-time polling or watch mode
- Changing the beads CLI (`bd`) itself -- we only add a wrapper method
- Substep-level bead status (substeps remain reflected in parent step bead)

#### Dependencies / Prerequisites {#dependencies}

- Beads CLI (`bd`) installed and supporting `bd children <id> --detailed --json` (or equivalent batch detailed query)
- Phases A-C complete: beads sync, rich content, close_reason written by committer-agent
- Existing `BeadsCli` struct with `children()`, `ready()`, and `show()` methods

#### Constraints {#constraints}

- Warnings are errors (`-D warnings` enforced via `.cargo/config.toml`)
- Must not break existing `specks status` behavior for specks without beads
- Minimize subprocess calls to `bd` (batch where possible)

#### Assumptions {#assumptions}

- The `bd children <parent_id> --detailed --json` command (or equivalent) returns an array of `IssueDetails` objects with `close_reason` populated for closed beads
- The `close_reason` format set by committer-agent is: `Committed: <hash> -- <summary>`
- `bd ready <parent_id> --json` returns the set of open beads with all dependencies satisfied
- The `IssueDetails` struct already has `close_reason: Option<String>` available
- Fallback to checkbox counting is acceptable when no `beads_root_id` exists (pre-Phase A specks)

---

### D.0.0 Design Decisions {#design-decisions}

#### [D01] Auto-detect beads vs checkbox mode (DECIDED) {#d01-auto-detect-mode}

**Decision:** Status command checks for `beads_root_id` in speck Plan Metadata. If present, query beads for real state. If absent, fall back to current checkbox-counting behavior.

**Rationale:**
- Pre-beads specks must continue to work without modification
- No user configuration needed -- presence of `beads_root_id` is a reliable signal that beads sync has run

**Implications:**
- Status command must handle both code paths cleanly
- Error handling needed when beads CLI is unavailable but `beads_root_id` exists (graceful degradation with warning)

#### [D02] Batch detailed children API (DECIDED) {#d02-batch-children-api}

**Decision:** Add `list_children_detailed(parent_id)` method to `BeadsCli` that calls `bd children <id> --detailed --json` and returns `Vec<IssueDetails>` with `close_reason` in one subprocess call. If `--detailed` is not supported by the installed beads CLI, fall back to N x `bd show` calls.

**Rationale:**
- Current `children()` returns `Vec<Issue>` without `close_reason`
- Calling `bd show` per bead creates N subprocess calls
- A single `--detailed` flag on the existing `children` command returns full details efficiently
- Fallback ensures the feature works even with older beads CLI versions

**Implications:**
- Requires beads CLI to support `--detailed` flag on `children` command for optimal performance
- Falls back to `children()` + N x `show()` calls if `--detailed` returns an error (e.g., unrecognized flag)
- Fallback path is O(N) subprocess calls but still correct

#### [D03] Parse speck for remaining work counts (DECIDED) {#d03-parse-speck-counts}

**Decision:** For ready (in-progress) steps, count tasks/tests/checkpoints from the speck markdown, not from beads.

**Rationale:**
- The speck file is the source of truth for what work a step contains
- Beads store the step description but not a structured count of remaining items
- Speck parser already extracts tasks, tests, and checkpoints per step

**Implications:**
- Ready steps show `Tasks: N | Tests: M | Checkpoints: K` from speck data
- Counts are always "total" counts, not "remaining" (since beads close the whole step atomically)

#### [D04] Raw bead content for --full view (DECIDED) {#d04-full-view-raw}

**Decision:** The `--full` flag displays raw bead field content (description, acceptance_criteria, design, notes, close_reason) in labeled sections without reformatting.

**Rationale:**
- Bead fields contain structured markdown written by agents
- Reformatting would lose information and add complexity
- Raw display lets users see exactly what agents recorded

**Implications:**
- Output may be verbose for steps with rich content
- Each field renders under a labeled header (e.g., `Description:`, `Design:`, etc.)
- Fields that are `None` or empty are omitted

#### [D05] Redesigned output format (DECIDED) {#d05-output-format}

**Decision:** Text output shows phase title, overall status with completion fraction, and per-step entries with emoji indicators and contextual detail lines. JSON output follows the existing `JsonResponse` envelope with a redesigned `StatusData` payload.

**Rationale:**
- Current checkbox-based output does not communicate implementation state clearly
- Emoji indicators (checkmark, spinner, hourglass) provide quick visual scanning
- JSON schema must be machine-readable for CI/dashboard consumption

**Implications:**
- `StatusData` struct in `output.rs` gets new fields for bead-based status; count fields use `_step_count` suffix to avoid collision with existing `completed_steps: Option<Vec<StepInfo>>` field
- In beads mode, `bead_steps` is populated and legacy `steps` is empty; in fallback mode, `steps` is populated and `bead_steps` is omitted -- consumers check the `mode` field to know which array to read
- Backward-compatible: existing fields remain, new fields added with `Option` wrappers and `skip_serializing_if`
- Text output changes are a breaking visual change but not a breaking API change

---

### D.0.1 Specification {#specification}

#### D.0.1.1 Inputs and Outputs {#inputs-outputs}

**Inputs:**
- Speck file path (positional argument)
- `--verbose` flag (existing, controls detail level)
- `--full` flag (new, shows raw bead content)
- `--json` flag (global, switches to JSON output)

**Outputs:**
- Text: Human-readable status with phase title, step states, and optional detail
- JSON: `JsonResponse<StatusData>` envelope with bead-enriched step information

**Key invariants:**
- If `beads_root_id` is present and beads CLI is available, status always queries beads
- If beads query fails, emit warning and fall back to checkbox mode
- Step order in output matches step order in speck (not bead creation order)

#### D.0.1.2 Terminology {#terminology}

- **Complete step**: Step whose bead is closed (status = "closed")
- **Ready step**: Step whose bead is open and all dependency beads are closed
- **Blocked step**: Step whose bead is open and at least one dependency bead is still open
- **Pending step**: Step with no bead ID assigned yet (beads not synced for this step)
- **Fallback mode**: Checkbox-counting behavior used when `beads_root_id` is absent

#### D.0.1.3 Close Reason Parsing {#close-reason-parsing}

The `close_reason` field set by committer-agent follows a known format:

```
Committed: <7-char-hash> -- <summary>
```

**Spec S02: Close Reason Parser** {#s02-close-reason-parser}

Parsing rules:
- If `close_reason` starts with `Committed:`, extract hash and summary
- Hash: first whitespace-delimited token after `Committed:`
- Summary: everything after ` -- ` separator
- If format does not match, display the raw `close_reason` text as-is

**Parsed fields:**
- `commit_hash: Option<String>` -- the short commit hash (e.g., `abc123d`)
- `commit_summary: Option<String>` -- the commit message summary

#### D.0.1.4 Output Schemas {#output-schemas}

##### Command: `specks status` {#cmd-status}

**Spec S01: Status Response Schema** {#s01-status-response}

**Success response (beads mode):**

```json
{
  "schema_version": "1",
  "command": "status",
  "status": "ok",
  "data": {
    "speck": ".specks/specks-5.md",
    "name": "specks-5",
    "phase_title": "User Authentication",
    "status": "active",
    "mode": "beads",
    "total_step_count": 5,
    "completed_step_count": 2,
    "ready_step_count": 1,
    "blocked_step_count": 2,
    "progress": { "done": 2, "total": 5 },
    "bead_steps": [
      {
        "anchor": "#step-0",
        "title": "Initialize API client",
        "number": "0",
        "bead_status": "complete",
        "bead_id": "bd-abc1",
        "commit_hash": "abc123d",
        "commit_summary": "feat(api): add client with retry support",
        "close_reason": "Committed: abc123d -- feat(api): add client with retry support"
      },
      {
        "anchor": "#step-1",
        "title": "Add auth middleware",
        "bead_status": "complete",
        "number": "1",
        "bead_id": "bd-def2",
        "commit_hash": "def456a",
        "commit_summary": "feat(auth): add JWT middleware",
        "close_reason": "Committed: def456a -- feat(auth): add JWT middleware"
      },
      {
        "anchor": "#step-2",
        "title": "Add login endpoint",
        "number": "2",
        "bead_status": "ready",
        "bead_id": "bd-ghi3",
        "task_count": 3,
        "test_count": 2,
        "checkpoint_count": 2
      },
      {
        "anchor": "#step-3",
        "title": "Add registration endpoint",
        "number": "3",
        "bead_status": "blocked",
        "bead_id": "bd-jkl4",
        "blocked_by": ["#step-2"]
      },
      {
        "anchor": "#step-4",
        "title": "End-to-end tests",
        "number": "4",
        "bead_status": "blocked",
        "bead_id": "bd-mno5",
        "blocked_by": ["#step-2", "#step-3"]
      }
    ]
  },
  "issues": []
}
```

**Table T01: Status Step Fields** {#t01-status-step-fields}

| Field | Type | Present When | Description |
|-------|------|-------------|-------------|
| `anchor` | string | always | Step anchor with `#` prefix |
| `title` | string | always | Step title from speck |
| `number` | string | always | Step number (e.g., "0", "2.1") |
| `bead_status` | string | beads mode | One of: "complete", "ready", "blocked", "pending" |
| `bead_id` | string | beads mode + bead exists | Bead ID for this step |
| `commit_hash` | string | complete steps | Short commit hash from close_reason |
| `commit_summary` | string | complete steps | Commit summary from close_reason |
| `close_reason` | string | complete steps | Raw close_reason from bead |
| `task_count` | integer | ready steps | Number of tasks in step |
| `test_count` | integer | ready steps | Number of tests in step |
| `checkpoint_count` | integer | ready steps | Number of checkpoints in step |
| `blocked_by` | string[] | blocked steps | Anchors of incomplete dependency steps |
| `done` | integer | fallback mode | Completed checkbox count |
| `total` | integer | fallback mode | Total checkbox count |

**Mode-specific field population:**

In beads mode (`mode: "beads"`), the `bead_steps` array is populated with `BeadStepStatus` objects and the legacy `steps` array is left empty. In fallback mode (`mode: "checkbox"`), the legacy `steps` array is populated with `StepStatus` objects (existing behavior) and `bead_steps` is omitted (null). This avoids ambiguity about which array consumers should read -- check `mode` first, then read the corresponding array.

**Success response (fallback mode):**

Identical to current `StatusData` output -- fields `mode: "checkbox"` added, bead-specific fields (`bead_steps`, `*_step_count`) omitted.

**Text output (beads mode):**

```
## Phase D.0: User Authentication

Status: active | 2/5 steps complete

Step 0: Initialize API client                 [checkmark] closed
  Committed: abc123d -- feat(api): add client with retry support

Step 1: Add auth middleware                   [checkmark] closed
  Committed: def456a -- feat(auth): add JWT middleware

Step 2: Add login endpoint                    [spinner] ready
  Tasks: 3 | Tests: 2 | Checkpoints: 2

Step 3: Add registration endpoint             [hourglass] blocked
  Blocked by: Step 2

Step 4: End-to-end tests                      [hourglass] blocked
  Blocked by: Step 2, Step 3
```

**Text output (--full, for a complete step):**

```
Step 0: Initialize API client                 [checkmark] closed
  Committed: abc123d -- feat(api): add client with retry support
  --- Description ---
  <raw bead description content>
  --- Design ---
  <raw bead design content>
  --- Acceptance Criteria ---
  <raw bead acceptance_criteria content>
  --- Notes ---
  <raw bead notes content>
```

---

### D.0.2 Symbol Inventory {#symbol-inventory}

#### D.0.2.1 New files {#new-files}

| File | Purpose |
|------|---------|
| `tests/fixtures/golden/status_beads.json` | Golden test expected output for beads-mode status |
| `tests/fixtures/golden/status_fallback.json` | Golden test expected output for fallback-mode status |

#### D.0.2.2 Symbols to add / modify {#symbols}

| Symbol | Kind | Location | Notes |
|--------|------|----------|-------|
| `BeadsCli::list_children_detailed` | fn | `crates/specks-core/src/beads.rs` | New method: `bd children <id> --detailed --json` returning `Vec<IssueDetails>` |
| `parse_close_reason` | fn | `crates/specks-core/src/beads.rs` | Parse `Committed: <hash> -- <summary>` from close_reason string |
| `CloseReasonParsed` | struct | `crates/specks-core/src/beads.rs` | Holds `commit_hash: Option<String>`, `commit_summary: Option<String>`, `raw: String` |
| `BeadStepStatus` | struct | `crates/specks/src/output.rs` | New output struct for bead-enriched step status in JSON |
| `StatusData` | struct (modify) | `crates/specks/src/output.rs` | Add `mode`, `speck`, `phase_title`, `total_step_count`, `completed_step_count`, `ready_step_count`, `blocked_step_count`, `bead_steps` fields |
| `run_status` | fn (modify) | `crates/specks/src/commands/status.rs` | Rewrite to branch on beads presence, query beads, build enriched output |
| `build_beads_status_data` | fn | `crates/specks/src/commands/status.rs` | New: build StatusData from beads queries |
| `build_checkbox_status_data` | fn | `crates/specks/src/commands/status.rs` | Renamed from `build_status_data`: checkbox-based fallback |
| `output_beads_text` | fn | `crates/specks/src/commands/status.rs` | New: render beads-mode text output |
| `Commands::Status` | enum variant (modify) | `crates/specks/src/cli.rs` | Add `full: bool` field |

---

### D.0.3 Test Plan Concepts {#test-plan-concepts}

#### Test Categories {#test-categories}

| Category | Purpose | When to use |
|----------|---------|-------------|
| **Unit** | Test `parse_close_reason`, step classification logic | Pure functions with no I/O |
| **Integration** | Test `list_children_detailed` with mock bd output | Subprocess interaction |
| **Golden / Contract** | Compare JSON output against known-good snapshots | Schema stability |

---

### D.0.5 Execution Steps {#execution-steps}

#### Step 0: Add batch detailed children API and close_reason parser {#step-0}

**Bead:** `specks-hfq.1`

**Commit:** `feat(beads): add list_children_detailed and close_reason parser`

**References:** [D02] Batch detailed children API, [D03] Parse speck for remaining work counts, Spec S02, (#d02-batch-children-api, #close-reason-parsing, #symbols)

**Artifacts:**
- New method `BeadsCli::list_children_detailed()` in `crates/specks-core/src/beads.rs`
- New function `parse_close_reason()` in `crates/specks-core/src/beads.rs`
- New struct `CloseReasonParsed` in `crates/specks-core/src/beads.rs`
- Re-export `CloseReasonParsed` and `parse_close_reason` from `crates/specks-core/src/lib.rs`

**Tasks:**
- [ ] Add `CloseReasonParsed` struct with `commit_hash: Option<String>`, `commit_summary: Option<String>`, `raw: String`
- [ ] Implement `parse_close_reason(close_reason: &str) -> CloseReasonParsed` that extracts hash and summary from `Committed: <hash> -- <summary>` format, falling back to raw text
- [ ] Add `list_children_detailed` method to `BeadsCli` that calls `bd children <id> --detailed --json` and returns `Result<Vec<IssueDetails>, SpecksError>`
- [ ] Implement fallback path in `list_children_detailed`: if `--detailed` flag causes a non-zero exit (unrecognized flag), fall back to calling `children()` for the list of `Issue` objects, then call `show()` for each ID to get `IssueDetails`, and return the combined `Vec<IssueDetails>`
- [ ] Add re-exports in `lib.rs` for `CloseReasonParsed` and `parse_close_reason`

**Tests:**
- [ ] Unit test: `parse_close_reason` with valid format `Committed: abc123d -- feat(api): add client` extracts hash and summary
- [ ] Unit test: `parse_close_reason` with non-standard format returns raw text with None fields
- [ ] Unit test: `parse_close_reason` with empty string returns empty raw with None fields
- [ ] Unit test: `parse_close_reason` with `Committed:` prefix but no ` -- ` separator extracts hash only

**Checkpoint:**
- [ ] `cargo build -p specks-core` compiles with no warnings
- [ ] `cargo nextest run -p specks-core -- close_reason` passes all new tests

**Rollback:**
- Revert commit; no external dependencies changed.

**Commit after all checkpoints pass.**

---

#### Step 1: Add --full flag and redesign output types {#step-1}

**Depends on:** #step-0

**Bead:** `specks-hfq.2`

**Commit:** `feat(status): add --full flag and redesign output types for bead-enriched status`

**References:** [D04] Raw bead content for --full view, [D05] Redesigned output format, Spec S01, Table T01, (#d04-full-view-raw, #d05-output-format, #output-schemas, #symbols, #terminology)

**Artifacts:**
- New `BeadStepStatus` struct in `crates/specks/src/output.rs`
- Modified `StatusData` struct with new fields in `crates/specks/src/output.rs`
- Modified `Commands::Status` variant with `full` field in `crates/specks/src/cli.rs`

**Tasks:**
- [ ] Add `BeadStepStatus` struct to `output.rs` with all fields from Table T01: `anchor`, `title`, `number`, `bead_status`, `bead_id`, `commit_hash`, `commit_summary`, `close_reason`, `task_count`, `test_count`, `checkpoint_count`, `blocked_by` (all bead-specific fields are `Option` with `skip_serializing_if`)
- [ ] Add new fields to `StatusData`: `mode: Option<String>`, `speck: Option<String>`, `phase_title: Option<String>`, `total_step_count: Option<usize>`, `completed_step_count: Option<usize>`, `ready_step_count: Option<usize>`, `blocked_step_count: Option<usize>`, `bead_steps: Option<Vec<BeadStepStatus>>` (note: count field names use `_step_count` suffix to avoid collision with existing `completed_steps: Option<Vec<StepInfo>>`)
- [ ] Add `--full` flag to `Commands::Status` variant in `cli.rs`
- [ ] `StatusData` does not derive `Default`. Update all 5 existing construction sites in `status.rs` to set new fields to `None`: error handler at line ~36 (not-initialized), error handler at line ~73 (file-not-found), error handler at line ~110 (read-error), error handler at line ~147 (parse-error), and the success path in `build_status_data` at line ~328

**Tests:**
- [ ] Unit test: `BeadStepStatus` serializes correctly with all fields populated
- [ ] Unit test: `BeadStepStatus` serializes correctly with optional fields omitted (skip_serializing_if)
- [ ] Unit test: CLI parses `specks status specks-1.md --full` correctly

**Checkpoint:**
- [ ] `cargo build` compiles with no warnings (both crates)
- [ ] `cargo nextest run` passes all existing tests plus new output type tests

**Rollback:**
- Revert commit; output types revert to previous shape.

**Commit after all checkpoints pass.**

---

#### Step 2: Rewrite status command with beads integration {#step-2}

**Depends on:** #step-0, #step-1

**Bead:** `specks-hfq.3`

**Commit:** `feat(status): rewrite status command with beads-based implementation state`

**References:** [D01] Auto-detect beads vs checkbox mode, [D02] Batch detailed children API, [D03] Parse speck for remaining work counts, [D05] Redesigned output format, Spec S01, Spec S02, Table T01, (#d01-auto-detect-mode, #inputs-outputs, #terminology, #close-reason-parsing, #output-schemas)

**Artifacts:**
- Rewritten `run_status` function in `crates/specks/src/commands/status.rs`
- New `build_beads_status_data` function
- Renamed `build_status_data` to `build_checkbox_status_data`
- New `output_beads_text` function for text rendering
- Modified `output_text` to handle `--full` flag

**Tasks:**
- [ ] Rename existing `build_status_data` to `build_checkbox_status_data` (preserves fallback path)
- [ ] Build a reverse mapping from `bead_id -> step anchor` using the speck's step list (each `Step` has `bead_id: Option<String>` and `anchor: String`); this mapping is needed to populate the `blocked_by` field with step anchors rather than bead IDs
- [ ] Add `build_beads_status_data` function that: queries `list_children_detailed` for all child beads, queries `ready` for ready set, classifies each step as complete/ready/blocked/pending, parses `close_reason` for complete steps, counts tasks/tests/checkpoints from speck for ready steps, and uses the bead_id-to-anchor mapping to resolve `blocked_by` to step anchors
- [ ] Update `run_status` signature to accept `full: bool` parameter
- [ ] Add auto-detection logic in `run_status`: check `speck.metadata.beads_root_id`, if present and beads CLI available, use beads path; otherwise fallback
- [ ] Implement `output_beads_text` for standard beads-mode text output with emoji indicators and contextual detail
- [ ] Add `--full` rendering path that shows raw bead field content for each step under labeled headers
- [ ] Handle graceful degradation: if beads CLI fails, emit warning and fall back to checkbox mode
- [ ] Wire `full` flag from CLI through `run_status` to output functions
- [ ] Update `main.rs` dispatch to pass `full` flag to `run_status`

**Tests:**
- [ ] Integration test: status with no `beads_root_id` falls back to checkbox counting (uses existing speck fixture)
- [ ] Integration test: status with `beads_root_id` and mock bead data shows correct complete/ready/blocked counts
- [ ] Unit test: `build_beads_status_data` correctly classifies steps given known children and ready sets
- [ ] Unit test: blocked step correctly identifies which dependencies are incomplete
- [ ] Unit test: `output_beads_text` produces expected text for a known step configuration

**Checkpoint:**
- [ ] `cargo build` compiles with no warnings
- [ ] `cargo nextest run` passes all tests
- [ ] `specks status .specks/specks-10.md` runs without error (fallback mode, since this speck has no beads)

**Rollback:**
- Revert commit; status command returns to checkbox-only behavior.

**Commit after all checkpoints pass.**

---

#### Step 3: Golden tests and JSON schema validation {#step-3}

**Depends on:** #step-2

**Bead:** `specks-hfq.4`

**Commit:** `test(status): add golden tests for beads and fallback status output`

**References:** [D01] Auto-detect beads vs checkbox mode, [D05] Redesigned output format, Spec S01, (#output-schemas, #t01-status-step-fields)

**Artifacts:**
- New golden test fixture `tests/fixtures/golden/status_beads.json`
- New golden test fixture `tests/fixtures/golden/status_fallback.json`
- New test module in status command or separate test file

**Tasks:**
- [ ] Create golden fixture `status_beads.json` with expected JSON output for a speck with beads (complete/ready/blocked steps)
- [ ] Create golden fixture `status_fallback.json` with expected JSON output for a speck without beads
- [ ] Add golden test that generates status JSON for a known speck and compares against `status_beads.json`
- [ ] Add golden test that generates status JSON for a pre-beads speck and compares against `status_fallback.json`
- [ ] Add test verifying `--full` text output includes raw bead content sections

**Tests:**
- [ ] Golden test: beads-mode JSON output matches `status_beads.json` schema and values
- [ ] Golden test: fallback-mode JSON output matches `status_fallback.json` schema and values
- [ ] Integration test: `--full` flag output includes `--- Description ---` headers for complete steps with bead content

**Checkpoint:**
- [ ] `cargo nextest run` passes all tests including golden comparisons
- [ ] `specks validate .specks/specks-10.md` passes (no broken references)

**Rollback:**
- Revert commit; test files removed, no production code changed.

**Commit after all checkpoints pass.**

---

### D.0.6 Deliverables and Checkpoints {#deliverables}

**Deliverable:** `specks status` shows real implementation state from beads when available, with commit info for completed steps, remaining work for ready steps, and dependency info for blocked steps, while preserving fallback behavior for pre-beads specks.

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

- [ ] `specks status <speck>` with beads shows complete/ready/blocked states (manual test with a beads-synced speck)
- [ ] `specks status <speck>` without beads shows checkbox-based progress (automated test)
- [ ] `specks status <speck> --json` output conforms to Spec S01 schema (golden test)
- [ ] `specks status <speck> --full` shows raw bead content (golden test)
- [ ] All existing status-related tests continue to pass (regression)
- [ ] `cargo nextest run` passes with no warnings

**Acceptance tests:**
- [ ] Golden test: `status_beads.json` matches expected output
- [ ] Golden test: `status_fallback.json` matches expected output
- [ ] Unit test: `parse_close_reason` handles all known formats
- [ ] Integration test: end-to-end status with mock beads data

#### Roadmap / Follow-ons (Explicitly Not Required for Phase Close) {#roadmap}

- [ ] Extend `specks list` to show bead-based status summaries
- [ ] Add `specks status --watch` for real-time updates
- [ ] Dashboard web view consuming JSON output
- [ ] Substep-level bead status tracking

| Checkpoint | Verification |
|------------|--------------|
| Beads mode works | `specks status` with beads-synced speck shows correct states |
| Fallback mode works | `specks status` without beads shows checkbox progress |
| JSON schema stable | Golden test comparison passes |
| All tests pass | `cargo nextest run` exits 0 |

**Commit after all checkpoints pass.**