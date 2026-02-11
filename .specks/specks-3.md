## Phase 3.0: Streamline Implementer Setup — Beads as Single Source of Truth {#phase-setup-streamline}

**Purpose:** Eliminate the dual state machine between the session file and beads by making beads the single source of truth for step state, slim the session to infrastructure-only fields, push all infrastructure work into `specks worktree create`, and reduce the implementer-setup-agent from an 8-phase / 636-line infrastructure worker to a ~150-line reasoning-only agent.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | kocienda |
| Status | draft |
| Target branch | main |
| Tracking issue/PR | TBD |
| Last updated | 2026-02-11 |
| Beads Root | `specks-tgg` |

---

### Phase Overview {#phase-overview}

#### Context {#context}

The implementer system has two competing state machines tracking step progress. The session file maintains `steps_completed`, `steps_remaining`, `current_step`, `bead_mapping`, and `status` (Pending/InProgress/Completed/Failed/NeedsReconcile). Meanwhile, beads natively tracks the same information: `bd ready --parent <root>` returns steps with no blocking deps that are open, `bd show <id>` returns whether a step is closed, and `bd close <id>` marks completion and unblocks dependents. The committer-agent updates both systems on every step commit, and when they disagree the session enters `NeedsReconcile` — a failure mode that exists only because there are two sources of truth.

Separately, the implementer-setup-agent performs 16-18 sequential tool calls across 8 phases to create a worktree, run `specks init`, sync beads, commit bead annotations, extract bead IDs, create session files, and create artifact directories. This takes approximately 2 minutes and is fragile: the Haiku model sometimes bypasses the CLI with raw git commands, creating duplicate worktrees and malformed session files. The root cause is that infrastructure work is split between the CLI and the agent.

This phase fixes both problems: beads becomes the single source of truth for step state (eliminating the dual state machine), and the CLI handles all infrastructure atomically (eliminating the fragile agent workflow).

#### Strategy {#strategy}

- Make beads the single source of truth for step state: `bd ready` replaces `steps_remaining`, `bd show` replaces `steps_completed`, root bead status replaces session status
- Slim the session file to infrastructure-only fields: remove `steps_completed`, `steps_remaining`, `current_step`, `bead_mapping`, `status`; keep `session_id`, `worktree_path`, `branch_name`, `root_bead_id`, `step_summaries`
- Enrich `specks worktree create` to perform everything atomically: check/reuse worktree, create branch + worktree, run `specks init`, sync beads, commit annotations, create slim session, create artifact directories, query `bd ready`, return complete JSON
- Make worktree reuse always-on and beads sync always-on (remove `--reuse-existing` and `--sync-beads` flags)
- Update the committer-agent's `step-commit` command to stop writing step-tracking fields
- Simplify the setup agent from 8 phases / 636 lines to 3 phases / ~150 lines

#### Stakeholders / Primary Customers {#stakeholders}

1. Implementer skill orchestrator (consumes setup agent output, drives step loop)
2. Committer agent / `specks step-commit` command (updates session after each step)
3. Downstream implementation agents (coder, reviewer — consume session and artifact data)
4. Users running `/specks:implementer` (experience faster, more reliable setup)

#### Success Criteria (Measurable) {#success-criteria}

- `specks worktree create` returns complete JSON including `ready_steps` from `bd ready` in a single invocation (verified by integration test)
- Session file does not contain `steps_completed`, `steps_remaining`, `current_step`, or `bead_mapping` fields (verified by reading session JSON)
- `SessionStatus` enum is fully removed (verified by compilation — no `SessionStatus` type in codebase)
- The setup agent performs 3 or fewer tool calls in the common case (verified by inspection)
- No `--reuse-existing` or `--sync-beads` flags exist (verified by `specks worktree create --help`)
- `implementer-setup-agent.md` is under 200 lines (verified by `wc -l`)
- All existing tests pass after changes (`cargo nextest run`)

#### Scope {#scope}

1. Make beads the single source of truth for step state (architectural change)
2. Slim the `Session` struct — remove step-tracking fields
3. Enrich `specks worktree create` with atomic init, beads sync, `bd ready` query, session, artifact dirs
4. Make worktree reuse always-on and beads sync always-on (remove flags)
5. Update `specks step-commit` to stop writing step-tracking fields to session
6. Simplify `implementer-setup-agent.md` to reasoning-only phases
7. Update existing tests and add new integration tests

#### Non-goals (Explicitly out of scope) {#non-goals}

- Changing the beads sync implementation itself (`specks beads sync`)
- Adding new `bd` commands or modifying beads itself
- Modifying downstream agents (architect, coder, reviewer) — only committer changes
- Changing the implementer SKILL.md orchestrator beyond removing `needs_reconcile` references
- Moving intent parsing or step resolution logic into the CLI (reasoning stays in the agent)

#### Dependencies / Prerequisites {#dependencies}

- Existing `create_worktree` function in `crates/specks-core/src/worktree.rs`
- Existing `Session` struct in `crates/specks-core/src/session.rs`
- Existing `sync_beads_in_worktree` and `commit_bead_annotations` functions in `crates/specks/src/commands/worktree.rs`
- Existing `specks init` command (idempotent)
- Beads CLI installed and functional (`bd ready`, `bd show`, `bd close`)
- Existing `specks step-commit` command in `crates/specks/src/commands/step_commit.rs`

#### Constraints {#constraints}

- Warnings are errors (`-D warnings` enforced via `.cargo/config.toml`)
- Never use `std::env::set_current_dir` in tests (process-global, not thread-safe)
- Must pass all existing tests plus new ones
- Agent model is Haiku — agent instructions must be simple and explicit
- Beads database is shared across worktrees — all agents see the same state

#### Assumptions {#assumptions}

- `bd ready --parent <root_bead_id> --json` returns all open steps with no blocking dependencies
- `bd show <id> --json` returns step details including `status: "closed"` for completed steps
- `bd close <id>` automatically unblocks dependent steps in the beads dependency graph
- `specks beads sync` creates the root epic, child beads per step, and dependency edges from `**Depends on:**` lines
- The speck file already has `**Bead:** bd-xxx` annotations on each step after sync (bead IDs are extractable by parsing the speck)
- The `specks init` command is safe to run idempotently inside a worktree
- Beads database persists across worktrees because it lives in `.git/` which is shared

---

### 3.0.0 Design Decisions {#design-decisions}

#### [D01] Worktree reuse is always-on, no flag needed (DECIDED) {#d01-always-reuse}

**Decision:** Remove the `--reuse-existing` flag from `specks worktree create`. The CLI always checks for an existing worktree first and reuses it if found, creating a new one only if none exists.

**Rationale:**
- Eliminates the double-worktree bug where the agent forgets the flag or bypasses the CLI
- Makes the command truly idempotent — calling it twice with the same speck always succeeds
- Simplifies agent instructions (no flag to remember)

**Implications:**
- Remove `reuse_existing` field from `WorktreeConfig`
- `create_worktree` always calls `find_existing_worktree` and returns the result if found
- The `WorktreeAlreadyExists` error is no longer returned (reuse replaces it)
- Tests that assert `WorktreeAlreadyExists` error must be updated

---

#### [D02] CLI performs atomic init + beads sync + commit (DECIDED) {#d02-atomic-setup}

**Decision:** `specks worktree create` always runs `specks init` inside the worktree, syncs beads (creating the root epic, child beads per step, and dependency edges), and commits bead annotations as part of worktree creation. The `--sync-beads` flag is removed; this behavior is always-on.

**Rationale:**
- Every worktree needs init and beads sync — making them optional just creates failure modes
- Beads sync creates the dependency graph that `bd ready` later queries; without it, nothing works
- Atomic execution with rollback on failure prevents partial states
- Eliminates 5+ agent tool calls (init, beads sync, grep beads, git add, git commit)

**Implications:**
- Remove `--sync-beads` flag from CLI
- `sync_beads` parameter removed from `run_worktree_create` and `run_worktree_create_with_root`
- `sync_beads_in_worktree` and `commit_bead_annotations` move from conditional to always-executed
- Rollback must undo init, beads sync, and commit if any step fails
- Session creation moves out of `create_worktree` (specks-core) into the CLI layer (`run_worktree_create_with_root`). The core function returns worktree path and branch info; the CLI layer creates, populates, and saves the session after init, beads sync, and `bd ready` have all succeeded. This avoids double-saves and ensures the session is only written once with complete data (including `root_bead_id` and `session_id`)

---

#### [D03] Beads is single source of truth for step state (DECIDED) {#d03-beads-source-of-truth}

**Decision:** Beads is the single authority for step completion state. The session file no longer tracks which steps are completed, remaining, or current. All step state queries go through beads commands.

**Rationale:**
- The current system has two competing state machines (session file and beads) that must be kept in sync
- When they disagree, the session enters `NeedsReconcile` — a failure mode that only exists because of dual tracking
- Beads already provides everything needed: `bd ready --parent <root>` returns actionable steps, `bd show <id>` returns completion status, `bd close <id>` marks completion and unblocks dependents
- Single source of truth eliminates an entire category of bugs

**Implications:**
- Remove `steps_completed`, `steps_remaining`, `current_step`, and `bead_mapping` from `Session` struct
- Remove the entire `SessionStatus` enum — all variants (`Pending`, `InProgress`, `Completed`, `Failed`, `NeedsReconcile`) are replaced by querying root bead status via `bd show`
- The `specks session reconcile` command becomes obsolete
- The `update_session` function in `step_commit.rs` stops writing step-tracking fields
- `specks worktree list` progress display must query beads instead of reading session fields

---

#### [D04] Session file is infrastructure-only, no step tracking (DECIDED) {#d04-slim-session}

**Decision:** The session file tracks only implementation infrastructure that beads cannot track: worktree path, branch name, session ID, root bead ID, and step summaries (for PR description generation). Schema version bumped to "2".

**Rationale:**
- Session should not duplicate what beads provides natively
- `step_summaries` is kept because it stores commit hashes and human summaries for PR generation — data that beads does not track
- `root_bead_id` is kept because it is the key for all beads queries (`bd ready --parent <root>`)
- Infrastructure fields (paths, branch names) have no beads equivalent

**Implications:**
- `Session` struct fields removed: `steps_completed`, `steps_remaining`, `current_step`, `bead_mapping`, `status`
- `Session` struct fields kept: `schema_version`, `session_id`, `speck_path`, `speck_slug`, `worktree_path`, `branch_name`, `base_branch`, `root_bead_id` (renamed from `beads_root`), `created_at`, `last_updated_at`, `step_summaries`, `total_steps`, `reused`
- `schema_version` bumped from `"1"` to `"2"` to signal the new format
- Backward-compatible deserialization: old `"1"` sessions still parse (removed fields are `Option` / `default`)

---

#### [D05] CLI queries `bd ready` and returns `ready_steps` (DECIDED) {#d05-ready-steps}

**Decision:** `specks worktree create` queries `bd ready --parent <root_bead_id> --json` and returns `ready_steps` — the list of step anchors that have no blocking dependencies and are still open. This replaces the old `completed_steps` / `remaining_steps` fields.

**Rationale:**
- `bd ready` is the correct primitive: it returns exactly the steps that can be worked on next, accounting for the full dependency graph
- This is more powerful than the old approach which tracked completed/remaining as flat lists without dependency awareness
- The agent only needs to know "which steps are ready" — beads provides this in one call

**Implications:**
- Add `BeadsCli::ready(parent: Option<&str>) -> Result<Vec<Issue>, SpecksError>` method to `crates/specks-core/src/beads.rs`, following the existing pattern of `show()`, `children()`, and `close()`. The method calls `bd ready --json` (with optional `--parent <id>` flag) and parses the JSON array of `Issue` objects. The output contract is documented in `docs/beads-json-contract.md` and tested via `bd-fake` mock (`tests/bin/bd-fake:cmd_ready`)
- `CreateData` includes `ready_steps: Vec<String>` instead of `completed_steps` / `remaining_steps`
- The agent uses `ready_steps` for intent resolution (e.g., "next" means first element of `ready_steps`)
- `all_steps` is still returned (extracted from speck) for display and validation purposes
- `bead_mapping` is still returned in `CreateData` (extracted from synced speck) so agents can map step anchors to bead IDs for `bd close` calls — but it is NOT stored in the session

---

#### [D06] CLI creates all artifact directories upfront (DECIDED) {#d06-artifact-dirs}

**Decision:** `specks worktree create` creates the base artifact directory and per-step subdirectories for every step in the speck.

**Rationale:**
- Artifact directories are needed by downstream agents; creating them upfront eliminates agent responsibility
- Creating for all steps (not just resolved) avoids needing to re-run if user changes step selection
- Directory creation is cheap and idempotent

**Implications:**
- After session creation, CLI creates `{repo_root}/.specks-worktrees/.artifacts/{session_id}/` and `step-N/` for each step
- The `CreateData` JSON output includes `artifacts_base` path

---

#### [D07] Extended CreateData JSON with ready_steps (DECIDED) {#d07-extended-json}

**Decision:** The `CreateData` struct returned by `specks worktree create --json` is extended to include `session_id`, `session_file`, `artifacts_base`, `all_steps`, `ready_steps`, and `bead_mapping`.

**Rationale:**
- The agent needs this data to do its reasoning work
- Returning it from the CLI eliminates the agent's need to re-read and re-parse the speck or query beads
- `bead_mapping` is computed from the speck file (not stored in session) so agents can call `bd close`

**Implications:**
- `CreateData` struct gains new fields; `rollback_performed` field removed
- JSON output is richer but backward-compatible (new fields are additive)
- Agent reads all infrastructure state from one CLI call

---

#### [D08] Agent reduced to 3 phases: state determination, intent parsing, step resolution (DECIDED) {#d08-agent-simplify}

**Decision:** The implementer-setup-agent is reduced from 8 phases to 3: (1) call CLI and determine state from output, (2) parse user intent, (3) resolve steps and validate. All infrastructure phases are removed.

**Rationale:**
- Infrastructure operations are now handled by the CLI
- Only reasoning work remains: interpreting user intent and validating step selection
- Fewer phases = fewer failure modes and faster execution

**Implications:**
- Agent instructions drop from ~636 lines to ~150 lines
- Phases 0a/0b (worktree creation/reuse), 1 (init + beads sync + commit), 2 (speck validation), 4b (bead extraction), 7 (session + artifacts) are all removed
- Agent output contract remains compatible — the orchestrator needs minimal changes
- The agent's first (and possibly only) tool call is `specks worktree create --json`
- The agent uses `ready_steps` from CLI output instead of computing completed/remaining itself

---

#### [D09] Remove NeedsReconcile session status (DECIDED) {#d09-remove-reconcile}

**Decision:** The entire `SessionStatus` enum is removed, including `NeedsReconcile`. The `specks session reconcile` command is deprecated and returns an error message directing users to beads.

**Rationale:**
- `NeedsReconcile` exists only because the system has two state machines that can disagree
- With beads as single source of truth, there is no second system to disagree with
- `bd close` either succeeds or fails — if it fails, the step is still open in beads and can be retried
- The committer can simply retry `bd close` on the next run

**Implications:**
- Remove the entire `SessionStatus` enum (per [D03])
- In `step_commit.rs`, if `bd close` fails, log a warning but do not modify the session — the step remains open in beads and will appear in `bd ready` again on next query
- Deprecate `specks session reconcile` — return error message "Session reconcile is no longer needed (beads is source of truth for step state)"
- Tests that construct `NeedsReconcile` sessions must be updated to use root bead open/closed checks
- Cleanup logic in `cleanup_worktrees_with_pr_checker` replaces `InProgress` guard with root bead open check

---

### Deep Dives {#deep-dives}

#### Beads Query Mapping {#beads-query-mapping}

This section maps every removed session field to its beads replacement.

**Table T03: Beads Query Mapping** {#t03-beads-mapping}

| Old Session Field | Replaced By | Beads Command | Notes |
|---|---|---|---|
| `steps_completed` | Step beads with status "closed" | `bd show <id> --json` | Check `status` field |
| `steps_remaining` | Open steps with unmet deps | `bd ready --parent <root> --json` | Returns only actionable steps |
| `current_step` | First ready step | `bd ready --parent <root> --json` | First element of result |
| `status` (Pending) | Root bead open, no children closed | `bd show <root> --json` | All children open |
| `status` (InProgress) | Root bead open, some children closed | `bd show <root> --json` | Mixed children states |
| `status` (Completed) | Root bead closeable (all children closed) | `bd show <root> --json` | All children closed |
| `status` (Failed) | Not applicable | N/A | Failures are transient; retry via `bd ready` |
| `status` (NeedsReconcile) | Eliminated | N/A | Single source of truth prevents this state |
| `bead_mapping` | Speck file `**Bead:** bd-xxx` annotations | Parse speck file | Written by `specks beads sync`, read by CLI |

#### Committer Agent Session Update Flow {#committer-update-flow}

**Current flow (dual state machine):**
1. `specks step-commit` stages files, commits
2. Calls `bd close <bead_id>` to mark step done in beads
3. If close succeeds: calls `update_session` to move step from `steps_remaining` to `steps_completed`, advance `current_step`, update timestamp
4. If close fails: sets `session.status = NeedsReconcile`, saves session

**New flow (beads-only):**
1. `specks step-commit` stages files, commits
2. Calls `bd close <bead_id>` to mark step done in beads
3. If close succeeds: appends to `session.step_summaries`, updates `session.last_updated_at`, saves session
4. If close fails: logs warning, does NOT modify session — step remains open in beads, will appear in `bd ready` on next query

The key change: step tracking updates are removed from `update_session`. Only `step_summaries` and `last_updated_at` are written. No `NeedsReconcile` path exists.

---

### 3.0.1 Specification {#specification}

#### 3.0.1.1 CLI Changes: `specks worktree create` {#cli-changes}

**Current behavior:**
1. Validate speck exists and has steps
2. Check for existing worktree (only if `--reuse-existing`)
3. Create branch + worktree
4. Save minimal session (with step-tracking fields)
5. Optionally sync beads (if `--sync-beads`)
6. Return basic JSON

**New behavior:**
1. Validate speck exists and has steps
2. Always check for existing worktree, reuse if found
3. Create branch + worktree (if new)
4. Run `specks init` inside worktree
5. Sync beads and commit annotations (creates root epic, child beads, dep edges)
6. Parse speck to extract step anchors and bead mapping
7. Query `bd ready --parent <root_bead_id> --json` for ready steps
8. Create slim session (infrastructure-only, schema version "2")
9. Create artifact directories (base + per-step)
10. Return enriched JSON with `ready_steps`, `bead_mapping`, `all_steps`, `session_id`, etc.

**Rollback on failure:**
- If init fails after worktree creation: remove worktree + delete branch
- If beads sync fails: remove worktree + delete branch (beads created by sync are left in place — harmless if orphaned, next sync will reuse them)
- If commit fails: remove worktree + delete branch
- If `bd ready` query fails: remove worktree + delete branch
- If session save fails: remove worktree + delete branch
- If artifact dir creation fails: remove worktree + delete branch + delete session

**Note on beads during rollback:** Beads created by `specks beads sync` are not deleted on rollback. They are harmless if orphaned — a subsequent sync will reuse existing beads, and orphaned beads can be cleaned up manually via `bd` commands if desired. Adding `bd delete` to rollback would introduce unnecessary complexity.

**On reuse:** Re-parse speck for bead mapping, re-query `bd ready`, update session `last_updated_at`, return enriched data with `reused: true`.

#### 3.0.1.2 Enriched JSON Output Schema {#enriched-json}

**Spec S01: worktree create Response Schema** {#s01-create-response}

```json
{
  "worktree_path": "/abs/path/to/.specks-worktrees/specks__auth-20260208-143022",
  "branch_name": "specks/auth-20260208-143022",
  "base_branch": "main",
  "speck_path": ".specks/specks-auth.md",
  "total_steps": 3,
  "reused": false,
  "root_bead_id": "bd-root123",
  "bead_mapping": {
    "#step-0": "bd-abc123",
    "#step-1": "bd-def456",
    "#step-2": "bd-ghi789"
  },
  "all_steps": ["#step-0", "#step-1", "#step-2"],
  "ready_steps": ["#step-0"],
  "session_id": "auth-20260208-143022",
  "session_file": "/abs/path/.specks-worktrees/.sessions/auth-20260208-143022.json",
  "artifacts_base": "/abs/path/.specks-worktrees/.artifacts/auth-20260208-143022"
}
```

**Table T01: CreateData Fields** {#t01-create-fields}

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `worktree_path` | string | yes | Absolute path to worktree directory |
| `branch_name` | string | yes | Git branch name (e.g., `specks/auth-20260208-143022`) |
| `base_branch` | string | yes | Branch worktree was created from |
| `speck_path` | string | yes | Relative path to speck file |
| `total_steps` | integer | yes | Number of execution steps in speck |
| `reused` | boolean | no | True if existing worktree was reused (omitted when false) |
| `root_bead_id` | string | yes | Root bead ID from beads sync |
| `bead_mapping` | object | yes | Map of step anchors to bead IDs (extracted from speck, not stored in session) |
| `all_steps` | array | yes | All step anchors in order |
| `ready_steps` | array | yes | Step anchors ready to work on (from `bd ready`) |
| `session_id` | string | yes | Session identifier (derived from worktree dir) |
| `session_file` | string | yes | Absolute path to session JSON file |
| `artifacts_base` | string | yes | Absolute path to artifacts directory |

#### 3.0.1.3 Slim Session Schema {#slim-session}

**Spec S02: Slim Session Schema (v2)** {#s02-slim-session}

```json
{
  "schema_version": "2",
  "session_id": "auth-20260208-143022",
  "speck_path": ".specks/specks-auth.md",
  "speck_slug": "auth",
  "worktree_path": "/abs/path/.specks-worktrees/specks__auth-20260208-143022",
  "branch_name": "specks/auth-20260208-143022",
  "base_branch": "main",
  "root_bead_id": "bd-root123",
  "total_steps": 3,
  "created_at": "2026-02-08T14:30:22.000Z",
  "last_updated_at": "2026-02-08T15:45:00.000Z",
  "step_summaries": [
    {"step": "#step-0", "commit_hash": "abc1234", "summary": "Set up project scaffolding"}
  ]
}
```

**Table T02: Session Fields Removed** {#t02-session-removed}

| Removed Field | Type | Reason | Beads Replacement |
|---|---|---|---|
| `steps_completed` | `Vec<String>` | Duplicates beads closed status | `bd show <id> --json` with status "closed" |
| `steps_remaining` | `Vec<String>` | Duplicates `bd ready` output | `bd ready --parent <root> --json` |
| `current_step` | `CurrentStep` | Duplicates first `bd ready` result | First element of `bd ready` output |
| `bead_mapping` | `HashMap<String, String>` | Duplicates speck file annotations | Parse `**Bead:** bd-xxx` from speck |
| `status` | `SessionStatus` | Duplicates root bead status | `bd show <root> --json` |

**Table T04: Session Fields Kept** {#t04-session-kept}

| Field | Type | Why Kept |
|---|---|---|
| `schema_version` | string | Forward compatibility; "2" signals slim format |
| `session_id` | string | Identity; derived from worktree dir name |
| `speck_path` | string | Infrastructure; no beads equivalent |
| `speck_slug` | string | Infrastructure; used for branch naming |
| `worktree_path` | string | Infrastructure; filesystem path |
| `branch_name` | string | Infrastructure; git branch name |
| `base_branch` | string | Infrastructure; merge target |
| `root_bead_id` | string | Key for all beads queries (renamed from `beads_root`) |
| `total_steps` | integer | Display; avoids re-parsing speck |
| `created_at` | string | Audit trail |
| `last_updated_at` | string | Audit trail; updated on each step commit |
| `step_summaries` | array | PR generation; commit hashes and summaries have no beads equivalent |
| `reused` | boolean | Display; indicates worktree was reused |

---

### 3.0.2 Symbol Inventory {#symbol-inventory}

#### 3.0.2.1 Modified files {#modified-files}

| File | Purpose |
|------|---------|
| `crates/specks-core/src/worktree.rs` | Remove `reuse_existing` from `WorktreeConfig`, make reuse always-on |
| `crates/specks-core/src/session.rs` | Remove `steps_completed`, `steps_remaining`, `current_step`, `bead_mapping`, `status` from `Session`; remove `SessionStatus` enum, `CurrentStep` enum; rename `beads_root` to `root_bead_id`; bump schema to "2" |
| `crates/specks/src/commands/worktree.rs` | Remove `--reuse-existing` and `--sync-beads` flags; add init + beads sync + `bd ready` + session + artifacts; extend `CreateData` with `ready_steps`, `session_id`, `session_file`, `artifacts_base`, `all_steps` |
| `crates/specks/src/commands/step_commit.rs` | Remove step-tracking updates from `update_session`; remove `NeedsReconcile` path |
| `crates/specks/src/commands/session.rs` | Deprecate or remove `specks session reconcile` command |
| `agents/implementer-setup-agent.md` | Rewrite from 8 phases / 636 lines to 3 phases / ~150 lines |
| `crates/specks/src/commands/step_publish.rs` | Remove `SessionStatus::Completed` write after PR creation; remove `SessionStatus` import |
| `crates/specks/src/commands/doctor.rs` | Replace `SessionStatus::InProgress` guard in orphaned worktree check with root bead open check; remove `SessionStatus` import |
| `crates/specks/src/output.rs` | Rename `needs_reconcile` to `bead_close_failed` in `StepCommitData`; deprecate `SessionReconcileData` struct |
| `crates/specks-core/src/beads.rs` | Add `BeadsCli::ready()` method for `bd ready --parent <id> --json` |
| `crates/specks-core/src/lib.rs` | Remove `SessionStatus` from re-exports |
| `crates/specks/src/cli.rs` | Update `StepCommit` help text: `needs_reconcile` → `bead_close_failed` |
| `skills/implementer/SKILL.md` | Remove `needs_reconcile` handling; replace with `bead_close_failed` warn-and-continue |
| `agents/committer-agent.md` | Minor update: clarify agent no longer updates step-tracking session fields |

#### 3.0.2.2 Symbols to modify {#symbols-modify}

| Symbol | Kind | Location | Change |
|--------|------|----------|--------|
| `WorktreeConfig` | struct | `specks-core/src/worktree.rs` | Remove `reuse_existing` field |
| `create_worktree` | fn | `specks-core/src/worktree.rs` | Always call `find_existing_worktree`; never return `WorktreeAlreadyExists`; stop creating/saving session (return worktree path + branch info instead) |
| `Session` | struct | `specks-core/src/session.rs` | Remove `steps_completed`, `steps_remaining`, `current_step`, `bead_mapping`, `status`; rename `beads_root` to `root_bead_id`; add `session_id` as required |
| `SessionStatus` | enum | `specks-core/src/session.rs` | Remove entire enum (no longer needed) |
| `CurrentStep` | enum | `specks-core/src/session.rs` | Remove entire enum (no longer needed) |
| `WorktreeCommands::Create` | enum variant | `commands/worktree.rs` | Remove `sync_beads` and `reuse_existing` args |
| `CreateData` | struct | `commands/worktree.rs` | Add `session_id`, `session_file`, `artifacts_base`, `all_steps`, `ready_steps`; remove `rollback_performed` |
| `run_worktree_create` | fn | `commands/worktree.rs` | Remove `sync_beads` and `reuse_existing` params |
| `run_worktree_create_with_root` | fn | `commands/worktree.rs` | Remove `sync_beads`, `reuse_existing` params; add init, beads sync, `bd ready`, session, artifacts |
| `update_session` | fn | `commands/step_commit.rs` | Remove step-tracking field updates; keep only `step_summaries` and `last_updated_at` |
| `run_step_publish` | fn | `commands/step_publish.rs` | Remove `session_data.status = SessionStatus::Completed`; keep only `last_updated_at` update |
| `check_orphaned_worktrees` | fn | `commands/doctor.rs` | Replace `session.status == SessionStatus::InProgress` guard with root bead open check |
| `StepCommitData` | struct | `output.rs` | Rename `needs_reconcile: bool` to `bead_close_failed: bool` |
| `SessionReconcileData` | struct | `output.rs` | Deprecate struct (reconcile command is deprecated) |
| `SessionStatus` re-export | use | `specks-core/src/lib.rs` | Remove `SessionStatus` from `pub use session::{ ... }` |
| `BeadsCli` | struct | `specks-core/src/beads.rs` | Add `ready(parent: Option<&str>) -> Result<Vec<Issue>, SpecksError>` method |

---

### 3.0.3 Test Plan Concepts {#test-plan-concepts}

#### Test Categories {#test-categories}

| Category | Purpose | When to use |
|----------|---------|-------------|
| **Unit** | Test `Session` struct serialization with removed fields, `CreateData` with new fields | Core data structures |
| **Integration** | Test full `specks worktree create` flow with `bd ready`, slim session, artifacts | End-to-end CLI behavior |
| **Regression** | Verify existing worktree/cleanup/list tests pass after Session struct changes | Backward compatibility |
| **Contract** | Verify session v2 schema is backward-compatible with v1 deserialization | Schema migration |

---

### 3.0.4 Execution Steps {#execution-steps}

#### Step 0: Remove reuse_existing flag, make reuse always-on {#step-0}

**Bead:** `specks-tgg.1`

**Commit:** `refactor(worktree): make worktree reuse always-on, remove --reuse-existing flag`

**References:** [D01] Worktree reuse is always-on, (#d01-always-reuse, #cli-changes, #symbols-modify)

**Artifacts:**
- Modified `crates/specks-core/src/worktree.rs`: `WorktreeConfig` without `reuse_existing`, `create_worktree` always reuses
- Modified `crates/specks/src/commands/worktree.rs`: `WorktreeCommands::Create` without `reuse_existing` arg
- Updated tests

**Tasks:**
- [ ] Remove `reuse_existing` field from `WorktreeConfig` struct
- [ ] Update `create_worktree` to always call `find_existing_worktree` and return result if found (no `WorktreeAlreadyExists` error path when existing worktree matches)
- [ ] Remove `reuse_existing` arg from `WorktreeCommands::Create` enum
- [ ] Remove `reuse_existing` param from `run_worktree_create` and `run_worktree_create_with_root`
- [ ] Update all call sites that pass `reuse_existing`
- [ ] Update CLI help text (remove `--reuse-existing` documentation from `long_about`)

**Tests:**
- [ ] Unit test: `test_create_data_serialization` still passes
- [ ] Integration test: Creating worktree twice for same speck succeeds (returns reused session)
- [ ] Integration test: First creation for new speck still works
- [ ] Update `test_worktree_create_without_reuse_existing_flag_fails` — now expects success with `reused: true`
- [ ] Update `test_worktree_create_with_reuse_existing_flag` — remove flag from config, verify same behavior

**Checkpoint:**
- [ ] `cargo nextest run`
- [ ] `cargo build` (no warnings)

**Rollback:**
- Revert commit; `reuse_existing` field and flag are restored

**Commit after all checkpoints pass.**

---

#### Step 1: Remove --sync-beads flag, make beads sync always-on {#step-1}

**Depends on:** #step-0

**Bead:** `specks-tgg.2`

**Commit:** `refactor(worktree): make beads sync always-on, remove --sync-beads flag`

**References:** [D02] CLI performs atomic init + beads sync + commit, (#d02-atomic-setup, #cli-changes, #symbols-modify)

**Artifacts:**
- Modified `crates/specks/src/commands/worktree.rs`: `sync_beads` arg removed, sync logic always runs
- Updated CLI help text

**Tasks:**
- [ ] Remove `sync_beads` arg from `WorktreeCommands::Create` enum
- [ ] Remove `sync_beads` param from `run_worktree_create` and `run_worktree_create_with_root`
- [ ] Move `sync_beads_in_worktree` and `commit_bead_annotations` calls from conditional to always-executed in `run_worktree_create_with_root`
- [ ] Update CLI help text (remove `--sync-beads` documentation)
- [ ] Update `main.rs` or `cli.rs` if they pass the `sync_beads` param

**Tests:**
- [ ] Integration test: `specks worktree create` always syncs beads (verify bead annotations exist in worktree speck file)
- [ ] Integration test: Rollback works when beads sync fails
- [ ] Update `test_worktree_create_help_includes_sync_beads` test to verify flag is gone

**Checkpoint:**
- [ ] `cargo nextest run`
- [ ] `cargo build` (no warnings)

**Rollback:**
- Revert commit; `--sync-beads` flag is restored

**Commit after all checkpoints pass.**

---

#### Step 2: Add specks init inside worktree creation {#step-2}

**Depends on:** #step-1

**Bead:** `specks-tgg.3`

**Commit:** `feat(worktree): run specks init automatically inside worktree during creation`

**References:** [D02] CLI performs atomic init + beads sync + commit, (#d02-atomic-setup, #cli-changes)

**Artifacts:**
- Modified `crates/specks/src/commands/worktree.rs`: init logic added before beads sync
- Init output committed along with bead annotations

**Tasks:**
- [ ] Add `specks init` execution inside the worktree after worktree creation but before beads sync in `run_worktree_create_with_root`
- [ ] Use `Command::new(std::env::current_exe())` with `.current_dir(worktree_path)` to run init
- [ ] Add rollback: if init fails, remove worktree + delete branch
- [ ] Update `commit_bead_annotations` at `commands/worktree.rs:212` to also run `git add .specks/` before the commit — currently it only stages the speck file (`git add <speck_path>`), but init creates `.specks/specks-implementation-log.md`, `.specks/config.toml`, and `.specks/specks-skeleton.md` which must also be committed

**Tests:**
- [ ] Integration test: After `specks worktree create`, verify `.specks/specks-implementation-log.md` exists inside worktree
- [ ] Integration test: After `specks worktree create`, verify `.specks/config.toml` exists inside worktree
- [ ] Integration test: Rollback when init fails (mock failure)

**Checkpoint:**
- [ ] `cargo nextest run`
- [ ] `cargo build` (no warnings)

**Rollback:**
- Revert commit; init is no longer auto-run

**Commit after all checkpoints pass.**

---

#### Step 3: Slim the Session struct — remove step-tracking fields {#step-3}

**Depends on:** #step-2

**Bead:** `specks-tgg.4`

**Commit:** `refactor(session): remove step-tracking fields, beads is single source of truth`

**References:** [D03] Beads is single source of truth, [D04] Session file is infrastructure-only, [D09] Remove NeedsReconcile, Spec S02, Table T02, Table T03, Table T04, (#d03-beads-source-of-truth, #d04-slim-session, #d09-remove-reconcile, #s02-slim-session, #t02-session-removed, #t03-beads-mapping, #t04-session-kept, #slim-session, #beads-query-mapping)

**Artifacts:**
- Modified `crates/specks-core/src/session.rs`: `Session` struct slimmed, `SessionStatus` and `CurrentStep` enums removed
- Modified `crates/specks/src/commands/step_commit.rs`: `update_session` simplified
- Modified `crates/specks/src/commands/session.rs`: reconcile command deprecated
- Modified `crates/specks/src/commands/worktree.rs`: list display updated (no more session-based progress)
- Modified `crates/specks/src/commands/step_publish.rs`: `SessionStatus` import and `Completed` write removed
- Modified `crates/specks/src/commands/doctor.rs`: `SessionStatus` import removed, orphaned worktree check uses root bead
- Modified `crates/specks/src/output.rs`: `needs_reconcile` renamed to `bead_close_failed`, `SessionReconcileData` deprecated
- Modified `crates/specks-core/src/lib.rs`: `SessionStatus` removed from re-exports
- Updated all test constructors across the codebase

**Tasks:**

*Session struct changes (session.rs):*
- [ ] Remove `SessionStatus` enum entirely from `session.rs` — full removal, no marker
- [ ] Remove `CurrentStep` enum entirely from `session.rs`
- [ ] Remove fields from `Session` struct: `steps_completed`, `steps_remaining`, `current_step`, `bead_mapping`, `status`
- [ ] Rename `beads_root` to `root_bead_id` in `Session` struct (with `#[serde(alias = "beads_root")]` for backward compat)
- [ ] Make `session_id` a required field (not `Option`) in `Session`
- [ ] Bump `schema_version` default to `"2"`
- [ ] Update `save_session` / `save_session_atomic` for new struct shape
- [ ] Update `load_session` to handle both v1 (with removed fields, ignored via `#[serde(default)]`) and v2 (slim) formats

*SessionStatus removal — call site replacements:*
- [ ] `worktree.rs` `cleanup_worktrees_with_pr_checker` (line ~1158): Replace `session.status == SessionStatus::InProgress` guard with root bead open check — query `bd show <root_bead_id> --json` and protect worktree if root bead is open (meaning implementation is active). If `root_bead_id` is `None` or beads query fails, treat as active (safe default: do not clean up)
- [ ] `commands/worktree.rs` `run_worktree_list` (line ~559): Replace `session.status` display with a simple label derived from session presence — show "active" for all listed worktrees (beads tracks the real state; `specks worktree list` is for infrastructure visibility, not progress tracking). **Note:** This involves deleting a ~30-line `CurrentStep` match block (lines ~527-557) that formats progress as "Step 2/5 (#step-2)" — replace the entire block with a simple "active" label. This is a temporary regression in display richness; restoring progress display via beads queries is a follow-on (see #roadmap)
- [ ] `commands/worktree.rs` `run_worktree_remove` (line ~782): Replace `format!("{:?}", session.status)` in ambiguous-match error display — show `session.created_at` instead (status was informational here, creation time is more useful for disambiguation)
- [ ] `commands/session.rs` `run_session_reconcile` (line ~88): Deprecate entire function — return error message "Session reconcile is no longer needed (beads is source of truth for step state)"
- [ ] `commands/session.rs` tests: Remove all `SessionStatus::NeedsReconcile` and `SessionStatus::Completed` assertions — replace with tests that verify the deprecation message
- [ ] `worktree.rs` `create_worktree` (lines ~656-686): Remove session creation and `save_session` call entirely — `create_worktree` should return worktree path and branch info (not a `Session`). Session creation moves to the CLI layer in Step 4, where it is written once with complete data after init, beads sync, and `bd ready` succeed. Update the return type and all callers accordingly
- [ ] `commands/step_commit.rs` (line ~150): Remove `session_data.status = SessionStatus::NeedsReconcile` path — if `bd close` fails, log warning but do not modify session
- [ ] `commands/step_commit.rs` `update_session` (line ~267-294): Remove `steps_remaining.retain()`, `steps_completed.push()`, `current_step` advancement — keep only `step_summaries` append and `last_updated_at` update
- [ ] `commands/step_publish.rs` (line 6): Remove `SessionStatus` from import
- [ ] `commands/step_publish.rs` (line 144): Remove `session_data.status = SessionStatus::Completed` — publish only needs `last_updated_at` update (session is infrastructure-only, completion state lives in beads)
- [ ] `commands/doctor.rs` (line 7): Remove `use specks_core::SessionStatus` import
- [ ] `commands/doctor.rs` `check_orphaned_worktrees` (line 447): Replace `session.status == SessionStatus::InProgress` guard with root bead open check — query `bd show <root_bead_id> --json` and skip worktree if root bead is open. Same pattern as `cleanup_worktrees_with_pr_checker`. If `root_bead_id` is `None` or query fails, treat as active (safe default)
- [ ] `output.rs` `StepCommitData` (line 370): Rename `needs_reconcile: bool` to `bead_close_failed: bool` — the concept of reconciliation is gone; this field now simply indicates whether `bd close` failed
- [ ] `output.rs` `SessionReconcileData` (lines 332-345): Deprecate entire struct — add `#[deprecated]` or remove if no external consumers. The reconcile command returns an error message, so this struct is unused
- [ ] `output.rs` tests (lines 410, 425, 440, 447): Update `needs_reconcile` references to `bead_close_failed`
- [ ] `lib.rs` (line 42): Remove `SessionStatus` from `pub use session::{ ... }` re-exports — the enum no longer exists
- [ ] `commands/step_commit.rs` (line 134, 175): Update `StepCommitData` construction sites to use `bead_close_failed` instead of `needs_reconcile`
- [ ] `cli.rs` (line 174): Update `StepCommit` `long_about` help text — change `"exits 0 with needs_reconcile=true"` to `"exits 0 with bead_close_failed=true"`
- [ ] `commands/worktree.rs` (line 47): Update `List` `long_about` help text — remove the status list `"(pending/in_progress/completed/failed/needs_reconcile)"` since session no longer has a status field; replace with `"(active)"`

*Test constructor updates:*
- [ ] Update all `Session` constructors in tests across `worktree.rs` (~15 instances), `session.rs` (~20 instances), `step_commit.rs`, `commands/worktree.rs` — remove `status`, `current_step`, `steps_completed`, `steps_remaining`, `bead_mapping` fields
- [ ] Update `NeedsReconcile` cleanup tests in `worktree.rs` (lines ~2649, ~2727, ~2763, ~2821) — replace with tests that check root bead open/closed status for cleanup protection

**Tests:**
- [ ] Unit test: `Session` v2 serialization omits removed fields
- [ ] Unit test: `Session` v1 JSON still deserializes (backward compat — extra fields ignored)
- [ ] Unit test: `update_session` only modifies `step_summaries` and `last_updated_at`
- [ ] Integration test: `step-commit` no longer writes `steps_completed` to session file
- [ ] Regression test: All existing worktree/cleanup/list tests pass with updated Session constructors

**Checkpoint:**
- [ ] `cargo nextest run`
- [ ] `cargo build` (no warnings)
- [ ] Verify session JSON written by tests does not contain `steps_completed`, `steps_remaining`, `current_step`, or `bead_mapping`

**Rollback:**
- Revert commit; old Session struct with step-tracking fields is restored

**Commit after all checkpoints pass.**

---

#### Step 4: Add `bd ready` query and extend CreateData with ready_steps, session, artifacts {#step-4}

**Depends on:** #step-3

**Bead:** `specks-tgg.5`

**Commit:** `feat(worktree): extend CreateData with ready_steps, session, artifacts from bd ready`

**References:** [D05] CLI queries bd ready, [D06] CLI creates artifact directories, [D07] Extended CreateData JSON, Spec S01, Table T01, (#d05-ready-steps, #d06-artifact-dirs, #d07-extended-json, #s01-create-response, #t01-create-fields, #enriched-json)

**Artifacts:**
- Modified `crates/specks-core/src/beads.rs`: `BeadsCli::ready()` method added
- Modified `crates/specks/src/commands/worktree.rs`: `CreateData` with new fields, `bd ready` query, session created and saved by CLI, artifact dirs created

**Tasks:**
- [ ] Add `BeadsCli::ready(parent: Option<&str>) -> Result<Vec<Issue>, SpecksError>` method to `crates/specks-core/src/beads.rs` — follows the same pattern as `show()` and `children()`: call `bd ready --json` (with optional `--parent <id>`), parse the JSON array of `Issue` objects. Contract documented in `docs/beads-json-contract.md`; test mock exists in `tests/bin/bd-fake:cmd_ready`
- [ ] Add fields to `CreateData`: `session_id: Option<String>`, `session_file: Option<String>`, `artifacts_base: Option<String>`, `all_steps: Vec<String>`, `ready_steps: Vec<String>`
- [ ] Remove `rollback_performed` field from `CreateData`
- [ ] After beads sync + commit, parse speck to extract step anchors into `all_steps` and `bead_mapping`
- [ ] Call `BeadsCli::ready(Some(&root_bead_id))` and map the returned `Issue` titles/IDs to step anchors to populate `ready_steps`. Handle edge cases: empty result (all steps have unmet deps or all are closed) returns empty `ready_steps`; `bd` not installed returns error (beads is required, not optional)
- [ ] Create slim session in CLI layer (session creation was removed from `create_worktree` in Step 3): populate `session_id`, `root_bead_id`, `total_steps`, `speck_slug`, `schema_version: "2"`, `last_updated_at`, and save with `save_session`. This ensures the session is written once with complete data after all infrastructure steps succeed
- [ ] Save session with `save_session`
- [ ] Create artifact directories: `{repo_root}/.specks-worktrees/.artifacts/{session_id}/` and `step-N/` for each step
- [ ] Populate `CreateData` with all new fields for JSON output
- [ ] On reuse: re-parse speck for bead mapping, re-query `bd ready`, update session `last_updated_at`, return enriched data

**Tests:**
- [ ] Unit test: `BeadsCli::ready()` parses `bd-fake` output correctly (uses existing `test_mock_bd_ready_returns_unblocked_issues` pattern from `beads_integration_tests.rs`)
- [ ] Unit test: `CreateData` serialization includes `ready_steps`, `session_id`, `session_file`, `artifacts_base`, `all_steps`
- [ ] Integration test: `specks worktree create --json` returns all fields from Spec S01
- [ ] Integration test: Session file exists at `session_file` path and contains `root_bead_id` but NOT `steps_completed`
- [ ] Integration test: Artifact directories exist for all steps
- [ ] Integration test: Reused worktree returns updated `ready_steps` from fresh `bd ready` query

**Checkpoint:**
- [ ] `cargo nextest run`
- [ ] `cargo build` (no warnings)
- [ ] Manual: `specks worktree create .specks/specks-test.md --json` returns complete JSON

**Rollback:**
- Revert commit; `CreateData` returns to pre-extension shape

**Commit after all checkpoints pass.**

---

#### Step 5: Update committer-agent markdown instructions {#step-5}

**Depends on:** #step-3

**Bead:** `specks-tgg.6`

**Commit:** `docs(agents): update agent and skill instructions to reflect beads as source of truth`

**References:** [D03] Beads is single source of truth, [D09] Remove NeedsReconcile, (#d03-beads-source-of-truth, #d09-remove-reconcile, #committer-update-flow, #t02-session-removed)

> **Ownership note:** All Rust code changes to `step_commit.rs` are handled in Step 3. This step is documentation-only: updating the committer-agent markdown instructions to match the new session contract.

**Artifacts:**
- Modified `agents/committer-agent.md`: updated instructions to reflect session no longer tracks steps
- Modified `skills/implementer/SKILL.md`: `needs_reconcile` handling removed

**Tasks:**
- [ ] Update `agents/committer-agent.md` to remove references to `steps_completed`, `steps_remaining`, `current_step` session updates
- [ ] Update committer agent output contract if it references session fields that no longer exist
- [ ] Clarify in agent instructions that `--session` flag still works but session is infrastructure-only (no step tracking)
- [ ] `skills/implementer/SKILL.md` (line 144): Remove the `needs_reconcile` handling block — there is no reconcile state; if `bead_close_failed` is true, the step remains open in beads and will appear in `bd ready` on the next run
- [ ] `skills/implementer/SKILL.md` (line 487): Remove the `If needs_reconcile == true: output the needs_reconcile warning message and HALT` logic — replace with: if `bead_close_failed` is true, log a warning and continue (the step is still open in beads, the committer can retry `bd close` on the next invocation)

**Tests:**
- [ ] Manual: Verify committer agent instructions do not reference `steps_completed`, `steps_remaining`, or `current_step`
- [ ] Manual: Verify SKILL.md does not reference `needs_reconcile`

**Checkpoint:**
- [ ] Committer agent instructions do not reference removed session fields
- [ ] SKILL.md does not reference `needs_reconcile`
- [ ] `cargo nextest run` (no Rust changes in this step, but verify nothing regressed)

**Rollback:**
- Revert commit; committer agent instructions restored

**Commit after all checkpoints pass.**

---

#### Step 6: Simplify implementer-setup-agent instructions {#step-6}

**Depends on:** #step-4, #step-5

**Bead:** `specks-tgg.7`

**Commit:** `refactor(agent): simplify implementer-setup-agent to reasoning-only, 3 phases`

**References:** [D08] Agent reduced to 3 phases, Spec S01, (#d08-agent-simplify, #s01-create-response, #enriched-json)

**Artifacts:**
- Rewritten `agents/implementer-setup-agent.md`: ~150 lines, 3 phases

**Tasks:**
- [ ] Rewrite Phase 1: Call `specks worktree create <speck_path> --json`, parse JSON output to populate `worktree_path`, `branch_name`, `base_branch`, `beads`, `session`, `state` fields; use `ready_steps` from CLI output instead of computing completed/remaining
- [ ] Rewrite Phase 2: Parse user intent (same logic as current Phase 4, no change to intent parsing rules)
- [ ] Rewrite Phase 3: Resolve steps using `ready_steps` from CLI; validate that requested steps are in `ready_steps` (dependency check is implicit — `bd ready` already filters by deps)
- [ ] Remove Phases 0a/0b (worktree creation/reuse — CLI handles)
- [ ] Remove Phase 1 (init + beads sync + commit — CLI handles)
- [ ] Remove Phase 2 (speck existence validation — CLI handles)
- [ ] Remove Phase 4b (bead ID extraction — CLI returns `bead_mapping`)
- [ ] Remove Phase 7 (session file + artifact dir creation — CLI handles)
- [ ] Preserve output contract (same JSON shape, but `state.remaining_steps` comes from `ready_steps`)
- [ ] Preserve clarification templates (step selection, dependency, already completed)
- [ ] Update examples to show new 3-phase flow with `ready_steps`

**Tests:**
- [ ] Manual: Run `/specks:implementer .specks/specks-test.md` and verify setup completes with 3 or fewer tool calls
- [ ] Verify agent output JSON matches expected contract

**Checkpoint:**
- [ ] Agent file is under 200 lines
- [ ] Output contract is compatible with orchestrator expectations
- [ ] `cargo nextest run` (all Rust tests still pass)

**Rollback:**
- Revert commit; old 8-phase agent is restored

**Commit after all checkpoints pass.**

---

### 3.0.5 Deliverables and Checkpoints {#deliverables}

**Deliverable:** Beads as single source of truth for step state, a slim infrastructure-only session file, a single `specks worktree create` CLI call that atomically handles all infrastructure and queries `bd ready`, and a simplified setup agent that only does reasoning work.

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

- [ ] `specks worktree create <speck> --json` returns complete JSON with `ready_steps` from `bd ready` (verified by integration test)
- [ ] Session file does not contain `steps_completed`, `steps_remaining`, `current_step`, or `bead_mapping` (verified by reading session JSON)
- [ ] `SessionStatus` enum does not exist in codebase (verified by compilation — full removal)
- [ ] No `--reuse-existing` flag exists (verified by `specks worktree create --help`)
- [ ] No `--sync-beads` flag exists (verified by `specks worktree create --help`)
- [ ] `specks init` runs automatically inside worktree (verified by integration test)
- [ ] Artifact directories created for all steps (verified by integration test)
- [ ] `implementer-setup-agent.md` is under 200 lines (verified by `wc -l`)
- [ ] Committer agent no longer writes step-tracking fields to session (verified by inspection)
- [ ] All existing tests pass (`cargo nextest run`)
- [ ] No warnings (`cargo build`)

**Acceptance tests:**
- [ ] Integration test: full worktree create flow returns enriched JSON with `ready_steps`
- [ ] Integration test: reuse scenario returns fresh `ready_steps` from `bd ready`
- [ ] Integration test: rollback on failure cleans up completely
- [ ] Integration test: `step-commit` writes slim session (no step-tracking fields)
- [ ] Contract test: v1 session JSON still deserializes into v2 `Session` struct

#### Roadmap / Follow-ons (Explicitly Not Required for Phase Close) {#roadmap}

- [ ] Remove `specks session reconcile` command entirely (currently deprecated)
- [ ] Add `specks worktree create --steps "2-4"` to pre-filter steps in CLI
- [ ] Update `specks worktree list` to query beads for progress display
- [ ] Benchmark setup time reduction (target: 2 minutes down to 15 seconds)
- [ ] Remove `specks session reconcile` subcommand entirely from CLI (currently deprecated with error message)

| Checkpoint | Verification |
|------------|--------------|
| CLI returns `ready_steps` | `specks worktree create .specks/specks-test.md --json` includes `ready_steps` from `bd ready` |
| Session is slim | Session JSON does not contain `steps_completed`, `steps_remaining`, `current_step` |
| No dual state machine | `NeedsReconcile` does not appear in compiled binary |
| Reuse is always-on | `specks worktree create --help` does not mention `--reuse-existing` |
| Agent is reasoning-only | `wc -l agents/implementer-setup-agent.md` < 200 |
| Tests pass | `cargo nextest run` exits 0 |

**Commit after all checkpoints pass.**