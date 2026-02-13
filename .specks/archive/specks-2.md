## Phase 2.0: CLI Step-Commit and Step-Publish Commands {#phase-step-commands}

**Purpose:** Replace the committer-agent's 746 lines of LLM-orchestrated mechanical work with two deterministic Rust CLI commands (`specks step-commit` and `specks step-publish`), reducing the agent to a thin Bash wrapper while improving reliability, speed, and eliminating CWD issues.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | kocienda |
| Status | draft |
| Target branch | main |
| Tracking issue/PR | TBD |
| Last updated | 2026-02-10 |
| Beads Root | `specks-jlu` |

---

### Phase Overview {#phase-overview}

#### Context {#context}

The committer-agent is currently 746 lines of LLM instructions orchestrating an entirely mechanical, deterministic process. Every step it performs is rote: rotate log, prepend log entry, stage files, commit, get hash, close bead, update session JSON. No LLM judgment is involved. This causes CWD issues, path-guessing errors, unreliable ordering, slowness, and fragile session updates. All the building blocks already exist in Rust (`run_log_rotate`, `run_log_prepend`, `BeadsCli::close`, `save_session_atomic`) but they are called piecemeal through Bash commands by an LLM agent.

By consolidating these operations into two CLI commands, we get atomic execution sequencing, deterministic error handling, and a single point of failure recovery (needs_reconcile) instead of scattered partial-failure states.

#### Strategy {#strategy}

- Implement `specks step-commit` first as a single Rust command that performs the full commit sequence: validate inputs, log rotate, log prepend, stage archive, stage files, git commit, get hash, close bead, update session.
- Implement `specks step-publish` as a separate command for push + PR creation + session update.
- Both commands accept explicit paths (worktree, session file) rather than relying on CWD or find_project_root().
- Extract internal helper functions from `run_log_rotate` and `run_log_prepend` that return structured results (the existing functions return `Result<i32, String>` and print output to stdout, which is unsuitable for programmatic use). Reuse `save_session_atomic` as-is.
- Follow the established command module pattern (`crates/specks/src/commands/`) and output data structs in `output.rs`.
- Simplify the committer-agent from 746 lines to approximately 50 lines that call these CLI commands via Bash.
- Files are relative to the worktree root; session path is always passed explicitly.

#### Stakeholders / Primary Customers {#stakeholders}

1. The implementer skill and committer-agent (direct consumers of the new CLI commands)
2. Developers using specks for implementation workflows (benefit from faster, more reliable commits)

#### Success Criteria (Measurable) {#success-criteria}

- `specks step-commit` completes the full sequence (log rotate, prepend, stage, commit, bead close, session update) in a single invocation and returns valid JSON with `--json` flag
- `specks step-publish` pushes branch, creates PR, and updates session in a single invocation and returns valid JSON
- `cargo nextest run` passes with no warnings for all new and existing tests
- The committer-agent markdown is reduced to under 100 lines
- Partial failure (commit succeeds, bead close fails) produces exit code 0 with `needs_reconcile: true` in JSON output

#### Scope {#scope}

1. New Rust source file `crates/specks/src/commands/step_commit.rs` implementing the step-commit command
2. New Rust source file `crates/specks/src/commands/step_publish.rs` implementing the step-publish command
3. CLI argument definitions added to `cli.rs` with the `StepCommit` and `StepPublish` variants
4. Output data structs `StepCommitData` and `StepPublishData` added to `output.rs`
5. Module registration and re-exports in `commands/mod.rs`
6. Main dispatch wiring in `main.rs`
7. Simplified committer-agent.md
8. Updated CLAUDE.md and implementer skill documentation

#### Non-goals (Explicitly out of scope) {#non-goals}

- Changing the session JSON schema (reuse existing `Session` struct as-is)
- Adding new error codes to specks-core (reuse existing patterns)
- Modifying the implementer skill's orchestration logic beyond updating committer payloads
- Making `step-publish` work without GitHub CLI (`gh` is a hard dependency)
- Adding dry-run modes to either command
- Supporting `commit_policy: "manual"` (stage-only mode) — step-commit always commits; the orchestrator uses `auto` policy exclusively

#### Dependencies / Prerequisites {#dependencies}

- Existing `run_log_rotate` and `run_log_prepend` functions accept `Option<&Path>` for root, but return `Result<i32, String>` (exit codes) — they need internal refactoring to expose structured results
- `BeadsCli::close()` is available, but the underlying `bd` CLI tool may depend on CWD to find `.beads/` — bead close must use `Command::current_dir(worktree_path)` to run from the worktree context
- `save_session_atomic()` is available in specks-core
- Session types (`Session`, `SessionStatus`, `CurrentStep`, `StepSummary`) are defined in specks-core

#### Constraints {#constraints}

- Warnings are errors (`-D warnings` enforced via `.cargo/config.toml`)
- Never use `std::env::set_current_dir` in any code (process-global, not thread-safe)
- Git operations must use `git -C {worktree}` consistently
- All new code follows existing command module patterns

#### Assumptions {#assumptions}

- `run_log_rotate` and `run_log_prepend` accept `Option<&Path>` for root directory but need internal refactoring to return structured results instead of printing to stdout
- Bead close requires running the `bd` CLI from the worktree directory (via `Command::current_dir`), not calling `BeadsCli::close()` directly from the main process
- `save_session_atomic()` handles atomic writes correctly via temp-file-then-rename pattern
- The implementer skill will be updated separately to pass new CLI flags (not in scope for this speck beyond documenting the committer-agent changes)

---

### 2.0.0 Design Decisions {#design-decisions}

#### [D01] Worktree-relative paths with explicit session flag (DECIDED) {#d01-path-handling}

**Decision:** All file paths in `--files` are relative to the worktree root. The `--session` flag takes an absolute path to the session JSON file. The `--worktree` flag takes an absolute path to the worktree directory.

**Rationale:**
- Files in `--files` are naturally relative to the worktree (matching git's expectations for `git -C {worktree} add`)
- Session files live outside the worktree in `.specks-worktrees/.sessions/`, so an absolute path avoids ambiguity
- Explicit paths eliminate all CWD dependency

**Implications:**
- The committer-agent must pass absolute paths for `--worktree` and `--session`
- File staging uses `git -C {worktree} add {file}` where `{file}` is relative to worktree

#### [D02] Exit 0 with needs_reconcile for partial commit success (DECIDED) {#d02-partial-success}

**Decision:** When `git commit` succeeds but bead close fails, the command exits with code 0 and sets `needs_reconcile: true` in the JSON output. The session is updated with `NeedsReconcile` status.

**Rationale:**
- The commit is irrevocable once created; it is a successful operation
- The bead close failure is recoverable via `specks session reconcile`
- Exit code 0 signals to the orchestrator that the commit was created and work is not lost
- This matches the existing committer-agent behavior for this edge case

**Implications:**
- The orchestrator must check `needs_reconcile` in the JSON output even when exit code is 0
- Session update failure after bead close produces a warning, not an error (commit + bead close both succeeded)

#### [D03] No step-completion validation in step-publish (DECIDED) {#d03-publish-no-validation}

**Decision:** `specks step-publish` does not validate that all steps are completed before publishing. It trusts the orchestrator to only call publish when ready.

**Rationale:**
- The implementer skill already tracks step completion state
- Adding validation would create a coupling between CLI and orchestrator state management
- Keeping publish simple (push + PR + session update) makes it a reliable, composable primitive

**Implications:**
- The implementer skill is solely responsible for determining when to call publish
- If publish is called prematurely, the PR will be created with incomplete work (operator error)

#### [D04] Internal helpers with structured return types (DECIDED) {#d04-direct-reuse}

**Decision:** Extract internal helper functions from `run_log_rotate` and `run_log_prepend` that return structured result types instead of printing to stdout. `step-commit` calls these helpers directly. For bead close, use `Command::new(&bd_path).current_dir(&worktree_path)` since the `bd` CLI needs CWD set to find `.beads/`.

**Rationale:**
- The existing `run_log_rotate` and `run_log_prepend` return `Result<i32, String>` (exit codes) and communicate results via `println!()` to stdout — unusable for programmatic callers
- Separating logic from presentation is a standard refactor: extract the core logic into a helper that returns a result struct, then have the CLI function call the helper and format output
- `BeadsCli::close()` works, but the underlying `bd` tool searches for `.beads/` from CWD, so we must set CWD to the worktree when invoking it

**Implications:**
- New internal functions: `do_log_rotate(root) -> Result<RotateResult, String>` and `do_log_prepend(root, ...) -> Result<PrependResult, String>`
- `run_log_rotate` and `run_log_prepend` become thin wrappers that call the helpers and format output
- Bead close uses `Command::new(&bd_path).arg("close").arg(&bead_id).current_dir(&worktree_path)` instead of `BeadsCli::close()`

#### [D05] Session update via direct JSON manipulation (DECIDED) {#d05-session-update}

**Decision:** `step-commit` reads the session JSON from the explicit `--session` path, updates it in memory (move step from `steps_remaining` to `steps_completed`, append to `step_summaries` with commit hash and summary, advance `current_step`, update `last_updated_at`), and writes it atomically using a temp-file-then-rename pattern matching `save_session_atomic`.

**Rationale:**
- The session file path is known (passed explicitly via `--session`)
- Direct manipulation avoids the indirection of `save_session_atomic()` which derives the path from `worktree_path`
- Atomic writes prevent partial session corruption on interruption

**Implications:**
- `step_commit.rs` reads/writes the session JSON directly at `--session` path
- The session update logic duplicates some of the step-tracking logic that the committer-agent currently does, but in deterministic Rust code

#### [D06] Git operations via std::process::Command (DECIDED) {#d06-git-commands}

**Decision:** All git operations (`add`, `commit`, `rev-parse`, `push`) use `std::process::Command` with `.current_dir(worktree_path)` or `git -C {worktree_path}` arguments.

**Rationale:**
- No git library dependency needed for these simple operations
- `Command::new("git").arg("-C").arg(&worktree_path)` is explicit and never touches global CWD
- Matches the pattern used in existing worktree commands

**Implications:**
- Git must be available in PATH
- Error messages from git are captured via stderr and included in error output

#### [D07] PR body generated from step summaries (DECIDED) {#d07-pr-body}

**Decision:** `step-publish` generates the PR body from the `--step-summaries` flag values, formatting them as a markdown summary section.

**Rationale:**
- The committer-agent currently writes a PR body file; moving this to Rust is straightforward
- Step summaries are already available from the session's `steps_completed` data
- Using a flag rather than reading from session keeps the command composable

**Implications:**
- `--step-summaries` accepts multiple values (one per step)
- The generated body includes a `## Summary` section with bullet points and a `## Test plan` section

---

### 2.0.1 Specification {#specification}

#### 2.0.1.1 Inputs and Outputs {#inputs-outputs}

**Inputs for step-commit:**
- `--worktree` (required): Absolute path to the worktree directory
- `--step` (required): Step anchor (e.g., `#step-0`)
- `--speck` (required): Speck file path relative to repo root
- `--message` (required): Git commit message
- `--files` (required, multi-value): Files to stage, relative to worktree root (repeatable: `--files a.rs --files b.rs`, or space-separated: `--files a.rs b.rs`)
- `--bead` (required): Bead ID to close (e.g., `bd-abc123`)
- `--summary` (required): One-line summary for the implementation log entry
- `--session` (required): Absolute path to the session JSON file
- `--close-reason` (optional): Reason for closing the bead
- `--json` (global flag): Output in JSON format
- `--quiet` (global flag): Suppress non-error output

**Inputs for step-publish:**
- `--worktree` (required): Absolute path to the worktree directory
- `--branch` (required): Git branch name (e.g., `specks/auth-20260208-143022`)
- `--base` (required): Base branch to merge into (e.g., `main`)
- `--title` (required): PR title
- `--speck` (required): Speck file path relative to repo root
- `--step-summaries` (required, multi-value): Step completion summaries for PR body
- `--session` (required): Absolute path to the session JSON file
- `--repo` (optional): GitHub repo in `owner/repo` format (auto-derived from git remote if not provided)
- `--json` (global flag): Output in JSON format
- `--quiet` (global flag): Suppress non-error output

**Outputs:**
- JSON response conforming to `JsonResponse<StepCommitData>` or `JsonResponse<StepPublishData>` envelope
- Exit code 0 on success (including partial success with needs_reconcile)
- Exit code 1 on failure

**Key invariants:**
- Git operations always use `git -C {worktree}` or `Command::new("git").current_dir(worktree_path)`
- Session writes are atomic (temp file + rename)
- Log rotate is always attempted before log prepend
- Bead close happens only after successful commit

#### 2.0.1.2 Terminology {#terminology}

- **step-commit**: The atomic sequence of log-rotate, log-prepend, git-add, git-commit, bead-close, session-update for a single implementation step
- **step-publish**: The sequence of push-branch, create-PR, session-update after all steps are complete
- **needs_reconcile**: A partial success state where commit succeeded but bead close failed; recoverable via `specks session reconcile`

#### 2.0.1.9 Output Schemas {#output-schemas}

##### Command: `step-commit` {#cmd-step-commit}

**Spec S01: step-commit Response Schema** {#s01-step-commit-response}

**Success response:**

```json
{
  "schema_version": "1",
  "command": "step-commit",
  "status": "ok",
  "data": {
    "committed": true,
    "commit_hash": "abc1234def5678",
    "bead_closed": true,
    "bead_id": "bd-abc123",
    "log_updated": true,
    "log_rotated": false,
    "archived_path": null,
    "files_staged": ["src/api/client.rs", "src/api/config.rs", ".specks/specks-implementation-log.md"],
    "needs_reconcile": false,
    "warnings": []
  },
  "issues": []
}
```

**Table T01: step-commit Data Fields** {#t01-step-commit-fields}

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `committed` | boolean | yes | Whether the git commit was created |
| `commit_hash` | string or null | yes | Full git commit hash, null if not committed |
| `bead_closed` | boolean | yes | Whether the bead was closed successfully |
| `bead_id` | string or null | yes | Bead ID that was closed, null if not closed |
| `log_updated` | boolean | yes | Whether the implementation log was updated |
| `log_rotated` | boolean | yes | Whether log rotation occurred before prepend |
| `archived_path` | string or null | yes | Path to archived log file if rotation occurred |
| `files_staged` | string array | yes | List of files that were staged |
| `needs_reconcile` | boolean | yes | True if commit succeeded but bead close failed |
| `warnings` | string array | yes | Any non-fatal warnings encountered |

##### Command: `step-publish` {#cmd-step-publish}

**Spec S02: step-publish Response Schema** {#s02-step-publish-response}

**Success response:**

```json
{
  "schema_version": "1",
  "command": "step-publish",
  "status": "ok",
  "data": {
    "success": true,
    "pushed": true,
    "pr_created": true,
    "repo": "owner/repo",
    "pr_url": "https://github.com/owner/repo/pull/42",
    "pr_number": 42
  },
  "issues": []
}
```

**Table T02: step-publish Data Fields** {#t02-step-publish-fields}

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `success` | boolean | yes | Whether both push and PR creation succeeded |
| `pushed` | boolean | yes | Whether the branch was pushed to remote |
| `pr_created` | boolean | yes | Whether the PR was created |
| `repo` | string or null | yes | GitHub repo in `owner/repo` format |
| `pr_url` | string or null | yes | Full URL to the created PR |
| `pr_number` | integer or null | yes | PR number |

##### Exit Codes {#exit-codes}

| Code | Meaning |
|------|---------|
| 0 | Success (including partial success with needs_reconcile) |
| 1 | Failure (input validation, git operations, etc.) |

---

### 2.0.1.8 Internal Architecture {#internal-architecture}

- **Single source of truth**: Session JSON file at the explicit `--session` path
- **Execution pipeline for step-commit**:
  1. Validate inputs (worktree exists, session file exists, listed files exist in worktree)
  2. Call `do_log_rotate(worktree_path)` — internal helper returning `RotateResult { rotated, archived_path, ... }`
  3. Call `do_log_prepend(worktree_path, step, speck, summary, bead)` — internal helper returning `PrependResult { entry_added, ... }`
  4. If rotation occurred, stage `.specks/archive/` directory
  5. Stage listed files plus `.specks/specks-implementation-log.md` via `git -C {worktree} add`
  6. Commit via `git -C {worktree} commit -m "{message}"`
  7. Get hash via `git -C {worktree} rev-parse HEAD`
  8. Close bead via `Command::new(&bd_path).args(["close", &bead_id, "--reason", &reason]).current_dir(&worktree_path)` — must run from worktree so `bd` finds `.beads/`
  9. Read session JSON, move step to completed, append to `step_summaries`, advance `current_step`, update `last_updated_at`, write atomically
- **Execution pipeline for step-publish**:
  1. Check `gh auth status`
  2. Derive repo from `git -C {worktree} remote get-url origin` if not provided
  3. Generate PR body from step summaries, write to temp file in worktree
  4. Push via `git -C {worktree} push -u origin {branch}`
  5. Create PR via `gh pr create --repo {repo} --base {base} --head {branch} --title "{title}" --body-file {temp_file}` — uses `--body-file` to avoid shell escaping issues with markdown content
  6. Parse PR URL and number from output
  7. Read session JSON, set status to Completed, update `last_updated_at`, write atomically
- **Where code lives**:
  - `crates/specks/src/commands/step_commit.rs` — step-commit command implementation
  - `crates/specks/src/commands/step_publish.rs` — step-publish command implementation
  - `crates/specks/src/output.rs` — `StepCommitData` and `StepPublishData` structs
  - `crates/specks/src/cli.rs` — `StepCommit` and `StepPublish` enum variants
  - `crates/specks/src/commands/mod.rs` — module declarations and re-exports
  - `crates/specks/src/main.rs` — dispatch wiring

---

### 2.0.2 Symbol Inventory {#symbol-inventory}

#### 2.0.2.1 New files {#new-files}

| File | Purpose |
|------|---------|
| `crates/specks/src/commands/step_commit.rs` | Implementation of `specks step-commit` command |
| `crates/specks/src/commands/step_publish.rs` | Implementation of `specks step-publish` command |

#### 2.0.2.2 Symbols to add / modify {#symbols}

| Symbol | Kind | Location | Notes |
|--------|------|----------|-------|
| `StepCommit` | enum variant | `cli.rs` | Added to `Commands` enum with clap derive attributes |
| `StepPublish` | enum variant | `cli.rs` | Added to `Commands` enum with clap derive attributes |
| `StepCommitData` | struct | `output.rs` | JSON data payload for step-commit response |
| `StepPublishData` | struct | `output.rs` | JSON data payload for step-publish response |
| `run_step_commit` | fn | `step_commit.rs` | Main entry point for step-commit command |
| `run_step_publish` | fn | `step_publish.rs` | Main entry point for step-publish command |
| `do_log_rotate` | fn | `log.rs` | Internal helper returning `RotateResult` struct (extracted from `run_log_rotate`) |
| `do_log_prepend` | fn | `log.rs` | Internal helper returning `PrependResult` struct (extracted from `run_log_prepend`) |
| `RotateResult` | struct | `log.rs` | Structured return type: `{ rotated, archived_path, original_lines, original_bytes, reason }` |
| `PrependResult` | struct | `log.rs` | Structured return type: `{ entry_added, step, speck, timestamp }` |
| `step_commit` | mod | `commands/mod.rs` | Module declaration |
| `step_publish` | mod | `commands/mod.rs` | Module declaration |

---

### 2.0.3 Documentation Plan {#documentation-plan}

- [ ] Update CLAUDE.md Common Commands section to include `specks step-commit` and `specks step-publish`
- [ ] Update committer-agent.md to be a thin wrapper calling CLI commands
- [ ] Update implementer skill documentation if committer payloads change

---

### 2.0.4 Test Plan Concepts {#test-plan-concepts}

#### Test Categories {#test-categories}

| Category | Purpose | When to use |
|----------|---------|-------------|
| **Unit** | Test CLI argument parsing for new subcommands | Verify clap derive definitions, flag conflicts |
| **Unit** | Test output data struct serialization | Verify JSON round-trip for StepCommitData and StepPublishData |
| **Integration** | Test step-commit full flow in temp directory with git repo | End-to-end: create repo, init specks, create worktree, run step-commit, verify commit and session |
| **Integration** | Test step-commit partial failure (bead close fails) | Verify needs_reconcile output when bead close fails after commit |
| **Unit** | Test session update logic | Verify steps_remaining/steps_completed manipulation |

---

### 2.0.5 Execution Steps {#execution-steps}

#### Step 0: Add output data structs and CLI argument definitions {#step-0}

**Bead:** `specks-jlu.1`

**Commit:** `feat(cli): add step-commit and step-publish CLI definitions and output structs`

**References:** [D01] Worktree-relative paths with explicit session flag, [D02] Exit 0 with needs_reconcile for partial commit success, Spec S01, Spec S02, Table T01, Table T02, (#inputs-outputs, #output-schemas, #symbols)

**Artifacts:**
- `StepCommitData` and `StepPublishData` structs in `output.rs`
- `StepCommit` and `StepPublish` variants in `Commands` enum in `cli.rs`
- Module stubs in `commands/mod.rs`
- Dispatch stubs in `main.rs`

**Tasks:**
- [ ] Add `StepCommitData` struct to `output.rs` with fields: `committed`, `commit_hash`, `bead_closed`, `bead_id`, `log_updated`, `log_rotated`, `archived_path`, `files_staged`, `needs_reconcile`, `warnings`
- [ ] Add `StepPublishData` struct to `output.rs` with fields: `success`, `pushed`, `pr_created`, `repo`, `pr_url`, `pr_number`
- [ ] Add `StepCommit` variant to `Commands` enum in `cli.rs` with all flags: `--worktree`, `--step`, `--speck`, `--message`, `--files` (multi-value via clap `num_args(1..)`), `--bead`, `--summary`, `--session`, `--close-reason`
- [ ] Add `StepPublish` variant to `Commands` enum in `cli.rs` with all flags: `--worktree`, `--branch`, `--base`, `--title`, `--speck`, `--step-summaries`, `--session`, `--repo`
- [ ] Create stub `step_commit.rs` with `run_step_commit` function returning a placeholder error
- [ ] Create stub `step_publish.rs` with `run_step_publish` function returning a placeholder error
- [ ] Add `pub mod step_commit;` and `pub mod step_publish;` to `commands/mod.rs` with re-exports
- [ ] Wire up dispatch in `main.rs` for both `Commands::StepCommit` and `Commands::StepPublish`

**Tests:**
- [ ] Unit test: verify CLI parses `specks step-commit --worktree /path --step "#step-0" --speck ".specks/specks-1.md" --message "msg" --files a.rs b.rs --bead "bd-123" --summary "done" --session "/path/session.json"`
- [ ] Unit test: verify CLI parses `specks step-publish --worktree /path --branch "specks/auth-123" --base "main" --title "feat: auth" --speck ".specks/specks-1.md" --step-summaries "Step 0: done" --session "/path/session.json"`
- [ ] Unit test: verify `StepCommitData` serialization round-trip
- [ ] Unit test: verify `StepPublishData` serialization round-trip
- [ ] Unit test: verify `Cli::command().debug_assert()` still passes

**Checkpoint:**
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo nextest run` passes all tests

**Rollback:**
- Revert changes to `cli.rs`, `output.rs`, `commands/mod.rs`, `main.rs`; delete `step_commit.rs` and `step_publish.rs`

**Commit after all checkpoints pass.**

---

#### Step 1: Implement step-commit command {#step-1}

**Depends on:** #step-0

**Bead:** `specks-jlu.2`

**Commit:** `feat(cli): implement specks step-commit command`

**References:** [D01] Worktree-relative paths with explicit session flag, [D02] Exit 0 with needs_reconcile for partial commit success, [D04] Direct function reuse over subprocess calls, [D05] Session update via direct JSON manipulation, [D06] Git operations via std::process::Command, Spec S01, Table T01, (#internal-architecture, #inputs-outputs, #terminology)

**Artifacts:**
- Refactored `log.rs`: extracted `do_log_rotate` / `RotateResult` and `do_log_prepend` / `PrependResult` helpers; `run_log_rotate` and `run_log_prepend` become thin wrappers
- Full implementation of `run_step_commit` in `step_commit.rs`

**Tasks:**
- [ ] Extract `do_log_rotate(root: &Path) -> Result<RotateResult, String>` from `run_log_rotate` in `log.rs` — contains the core logic, returns structured `RotateResult { rotated, archived_path, original_lines, original_bytes, reason }`. Refactor `run_log_rotate` to call `do_log_rotate` and format output.
- [ ] Extract `do_log_prepend(root: &Path, step, speck, summary, bead) -> Result<PrependResult, String>` from `run_log_prepend` in `log.rs` — contains the core logic, returns structured `PrependResult { entry_added, step, speck, timestamp }`. Refactor `run_log_prepend` to call `do_log_prepend` and format output.
- [ ] Implement input validation: worktree exists, session file exists, all listed files exist in worktree
- [ ] Call `do_log_rotate(&worktree_path)` — use returned `RotateResult` to determine if archive staging is needed
- [ ] Call `do_log_prepend(&worktree_path, step, speck, summary, Some(bead))` — use returned `PrependResult` for log_updated status
- [ ] If rotation occurred, add `.specks/archive/` to staging list
- [ ] Stage files via `git -C {worktree} add {files} .specks/specks-implementation-log.md`
- [ ] Commit via `git -C {worktree} commit -m "{message}"`
- [ ] Get hash via `git -C {worktree} rev-parse HEAD`
- [ ] Close bead via `Command::new(&bd_path).args(["close", &bead_id, "--reason", &reason]).current_dir(&worktree_path)` — load Config from worktree to get `bd_path`, run from worktree directory so `bd` finds `.beads/`
- [ ] If bead close fails after commit: set `needs_reconcile: true`, update session with `NeedsReconcile` status, exit 0
- [ ] Read session JSON from `--session` path, move step from `steps_remaining` to `steps_completed`, append `StepSummary { step, commit_hash, summary }` to `step_summaries`, advance `current_step` to next remaining or `Done`, update `last_updated_at`, write atomically via temp-file-then-rename
- [ ] Handle session update failure after bead close as a warning (not an error)
- [ ] Return `JsonResponse<StepCommitData>` with all fields populated

**Tests:**
- [ ] Unit test: `do_log_rotate` returns `RotateResult` with correct fields (existing rotation tests still pass after refactor)
- [ ] Unit test: `do_log_prepend` returns `PrependResult` with correct fields (existing prepend tests still pass after refactor)
- [ ] Integration test: full step-commit flow in temp git repo with worktree, verify commit exists, session updated, log prepended, step_summaries populated
- [ ] Integration test: step-commit with log rotation (create oversized log, verify archive staging)
- [ ] Unit test: session update logic — move step from remaining to completed, append to step_summaries, advance current_step
- [ ] Unit test: error on missing worktree directory
- [ ] Unit test: error on missing session file
- [ ] Unit test: error on missing files in worktree

**Checkpoint:**
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo nextest run` passes all tests
- [ ] Manual test: run `specks step-commit --json` in a test worktree and verify JSON output matches Spec S01

**Rollback:**
- Revert `step_commit.rs` to stub implementation and revert `log.rs` helper extraction (dead helpers trigger `-D warnings` build failure)

**Commit after all checkpoints pass.**

---

#### Step 2: Implement step-publish command {#step-2}

**Depends on:** #step-0

**Bead:** `specks-jlu.3`

**Commit:** `feat(cli): implement specks step-publish command`

**References:** [D03] No step-completion validation in step-publish, [D06] Git operations via std::process::Command, [D07] PR body generated from step summaries, Spec S02, Table T02, (#internal-architecture, #inputs-outputs)

**Artifacts:**
- Full implementation of `run_step_publish` in `step_publish.rs`

**Tasks:**
- [ ] Check `gh auth status` and return error if not authenticated
- [ ] If `--repo` not provided, derive from `git -C {worktree} remote get-url origin` using regex to extract `owner/repo`
- [ ] Generate PR body markdown from `--step-summaries` values (summary section with bullets, test plan section, Claude Code attribution), write to temp file in worktree (e.g., `{worktree}/.specks/pr-body.md`)
- [ ] Push via `git -C {worktree} push -u origin {branch}` and capture output
- [ ] Create PR via `gh pr create --repo {repo} --base {base} --head {branch} --title "{title}" --body-file {worktree}/.specks/pr-body.md` — use `--body-file` to avoid shell escaping issues with markdown content containing quotes and newlines
- [ ] Parse PR URL and number from `gh pr create` output
- [ ] Read session JSON from `--session` path, set `status` to `Completed`, update `last_updated_at`, write atomically
- [ ] Handle push failure (return error, do not attempt PR creation)
- [ ] Handle PR creation failure (return partial success: `pushed: true`, `pr_created: false`)
- [ ] Return `JsonResponse<StepPublishData>` with all fields populated

**Tests:**
- [ ] Unit test: PR body generation from step summaries list
- [ ] Unit test: repo derivation from various git remote URL formats (SSH, HTTPS, with/without .git suffix)
- [ ] Unit test: error on gh auth failure (mock via env)
- [ ] Unit test: session update sets Completed status

**Checkpoint:**
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo nextest run` passes all tests
- [ ] Manual test: run `specks step-publish --json` in a test repo with a remote and verify JSON output matches Spec S02

**Rollback:**
- Revert `step_publish.rs` to stub implementation

**Commit after all checkpoints pass.**

---

#### Step 3: Simplify committer-agent and update documentation {#step-3}

**Depends on:** #step-1, #step-2

**Bead:** `specks-jlu.4`

**Commit:** `docs: simplify committer-agent to thin CLI wrapper and update docs`

**References:** [D01] Worktree-relative paths with explicit session flag, [D04] Direct function reuse over subprocess calls, Spec S01, Spec S02, (#documentation-plan, #strategy)

**Artifacts:**
- Simplified `agents/committer-agent.md` (reduced from 746 lines to under 100 lines)
- Updated `CLAUDE.md` with new commands in Common Commands section
- Updated `skills/implementer/SKILL.md` committer sections

**Tasks:**
- [ ] Rewrite `agents/committer-agent.md` as a thin Bash wrapper: commit mode extracts fields from its JSON input payload (`worktree_path`, `step_anchor`, `speck_path`, `proposed_message`, `files_to_stage`, `bead_id`, `close_reason`, `log_entry.summary`, `session_file`) and maps them to `specks step-commit --worktree ... --step ... --files file1 file2 ... --json` flags; publish mode maps (`worktree_path`, `branch_name`, `base_branch`, `speck_title`, `speck_path`, `step_summaries`, `session_file`) to `specks step-publish --worktree ... --branch ... --json` flags
- [ ] Define the committer-agent's output contract: for commit mode, pass through the step-commit JSON `data` object directly; for publish mode, pass through the step-publish JSON `data` object directly
- [ ] Remove all manual git operations, log management, bead operations, and session update instructions from committer-agent
- [ ] Update `CLAUDE.md` Common Commands section to document `specks step-commit` and `specks step-publish` with flag descriptions
- [ ] Update `skills/implementer/SKILL.md` to reference the simplified committer interface

**Tests:**
- [ ] Verify committer-agent.md is under 100 lines
- [ ] Verify CLAUDE.md includes both new commands

**Checkpoint:**
- [ ] `cargo build` succeeds with no warnings (ensures no code references were broken)
- [ ] `cargo nextest run` passes all tests

**Rollback:**
- Revert documentation files from git

**Commit after all checkpoints pass.**

---

### 2.0.6 Deliverables and Checkpoints {#deliverables}

**Deliverable:** Two new CLI commands (`specks step-commit` and `specks step-publish`) that atomically perform the committer-agent's mechanical work in deterministic Rust code, plus a simplified committer-agent that wraps these commands.

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

- [ ] `specks step-commit --json` produces valid JSON matching Spec S01 schema
- [ ] `specks step-publish --json` produces valid JSON matching Spec S02 schema
- [ ] `cargo nextest run` passes with no warnings
- [ ] `agents/committer-agent.md` is under 100 lines
- [ ] `CLAUDE.md` documents both new commands

**Acceptance tests:**
- [ ] Integration test: full step-commit round trip (init repo, create worktree, run step-commit, verify commit + session + log)
- [ ] Integration test: step-commit partial failure (bead close fails, verify needs_reconcile output)
- [ ] Unit test: all CLI argument combinations parse correctly
- [ ] Unit test: output data structs serialize/deserialize correctly

#### Roadmap / Follow-ons (Explicitly Not Required for Phase Close) {#roadmap}

- [ ] Add `--dry-run` flag to both commands for preview without side effects
- [ ] Add `specks session reconcile` support for step-commit partial failures
- [ ] Consider extracting shared git operation helpers into specks-core if other commands need them
- [ ] Add metrics/timing to step-commit for performance comparison with agent-based approach

| Checkpoint | Verification |
|------------|--------------|
| CLI definitions compile | `cargo build` with no warnings |
| step-commit works end-to-end | Integration test with real git repo |
| step-publish works end-to-end | Integration test with mock gh |
| committer-agent simplified | Line count under 100 |
| All tests pass | `cargo nextest run` |

**Commit after all checkpoints pass.**