## Phase C: Eliminate Session File {#phase-slug}

**Purpose:** Remove the session file entirely from the specks implementation workflow, deriving all needed state from git worktree metadata and beads. After this phase, no session files are created, read, or updated during worktree creation, step commits, publishing, or merge cleanup.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | kocienda |
| Status | draft |
| Target branch | main |
| Tracking issue/PR | TBD |
| Last updated | 2026-02-12 |
| Beads Root | `specks-t9v` |

---

### Phase Overview {#phase-overview}

#### Context {#context}

The session file (`.specks-worktrees/.sessions/<session-id>.json`) was introduced to track worktree metadata: path, branch name, base branch, speck path, root bead ID, step summaries, and timestamps. With the completion of Phase A (Rich Sync) and Phase B (Agent-Bead Communication), nearly everything stored in the session is now redundant. Git worktree metadata provides path and branch information. Beads track step completion state and close reasons. Git log provides commit history and step summaries.

The session file has become maintenance burden without adding value. It requires atomic write handling, backward-compatible schema evolution (v1 to v2), and session-cleanup logic threaded through merge, worktree cleanup, and doctor commands. Removing it simplifies the codebase, eliminates a class of "needs_reconcile" failures, and makes git + beads the single source of truth as intended.

#### Strategy {#strategy}

- Work consumer-first: remove session usage from each consumer (`step-commit`, `step-publish`, `session` subcommand, worktree commands) before gutting `session.rs` itself. Each step produces a compiling, green-test commit.
- Remove the `--session` parameter from `step-commit` and `step-publish` CLI commands entirely (clean break, no deprecation period)
- Remove the deprecated `session reconcile` CLI subcommand and its `SessionReconcileData` output type
- Rewrite `list_worktrees` and `find_existing_worktree` to use git-native discovery before removing session creation from worktree create
- Gut `session.rs` to a stub keeping only `now_iso8601()` and its helpers as the final step, after all consumers have been updated
- Modify `step_publish.rs` to build PR body from `git log --oneline <base>..HEAD` instead of session `step_summaries`
- Remove `session_file` and `artifacts_base` from the `CreateData` output and setup agent contract
- Update agent and skill definitions (including `coder-agent.md`) to remove all session references
- Add a `specks doctor` check that warns about orphaned `.sessions/` directories

#### Stakeholders / Primary Customers {#stakeholders}

1. Implementer skill and its sub-agents (committer, setup, coder)
2. Users running `specks worktree`, `specks step-commit`, and `specks step-publish` commands

#### Success Criteria (Measurable) {#success-criteria}

- No session file is created during `specks worktree create` (verify: check `.specks-worktrees/.sessions/` is not created)
- No session file is read during `step-commit`, `step-publish`, or `merge` (verify: grep for session loading in those modules returns zero hits)
- `specks step-commit` works without `--session` parameter (verify: run command, exits 0)
- `specks step-publish` generates PR body from git log (verify: PR body contains git commit messages)
- `specks doctor` warns about orphaned `.sessions/` directories (verify: create orphaned dir, run doctor, see warning)
- `cargo nextest run` passes with zero warnings (verify: full test suite)

#### Scope {#scope}

1. Remove `--session` parameter from `step-commit` and `step-publish` CLI commands
2. Remove session loading/saving from `step_commit.rs` and `step_publish.rs`
3. Build PR body from `git log --oneline <base>..HEAD` in `step_publish.rs`
4. Stop creating session files in `worktree.rs` create command
5. Remove `session_file` and `artifacts_base` from `CreateData` and setup agent output
6. Gut `session.rs` — keep `now_iso8601()`, remove `Session`/`StepSummary` structs and all persistence functions
7. Update `lib.rs` exports to remove session types and functions
8. Update `worktree.rs` list/cleanup to not depend on session files (use git-native discovery)
9. Update `merge.rs` to remove session cleanup calls
10. Update `committer-agent.md` to remove `--session` from CLI calls
11. Update `implementer-setup-agent.md` to remove session output fields
12. Update `skills/implementer/SKILL.md` to remove session references
13. Add `specks doctor` check for orphaned `.sessions/` directories
14. Update CLI tests to match new parameter signatures
15. Remove deprecated `session` CLI subcommand (`commands/session.rs`, `SessionCommands`, match arm in `main.rs`)
16. Update `agents/coder-agent.md` to remove `session_id` references
17. Remove `session_id_from_worktree`/`delete_session` calls from `run_worktree_remove`
18. Update `cleanup_worktrees_with_pr_checker` and `cleanup_stale_branches_with_pr_checker` to use `DiscoveredWorktree` instead of `Session`
19. Update `remove_worktree()` to remove `delete_session`/`session_id_from_worktree` calls
20. Remove `cleanup_orphaned_sessions()` from `worktree.rs` (replaced by doctor check)
21. Update doctor checks (`check_stale_branches`, `check_orphaned_worktrees`, `check_closed_pr_worktrees`) to use `DiscoveredWorktree` fields

#### Non-goals (Explicitly out of scope) {#non-goals}

- Automatically migrating or deleting existing `.sessions/` directories (users clean up manually, doctor warns)
- Changing the artifacts directory structure (already moved inside worktree in Phase B)
- Modifying `find_worktree_by_speck()` (already uses git-native discovery, no changes needed)
- Changing the bead close workflow (already works independently of sessions)

#### Dependencies / Prerequisites {#dependencies}

- Phase A (Rich Sync) must be complete: beads contain rich step content
- Phase B (Agent-Bead Communication) must be complete: agents communicate through bead fields, not artifact files
- `find_worktree_by_speck()` in `worktree.rs` must be functional for git-native discovery

#### Constraints {#constraints}

- `now_iso8601()` must be preserved — it is used by `worktree.rs` for timestamp generation
- Warnings are errors (`-D warnings`) — all dead code must be removed or annotated
- No breaking changes to `specks merge` (already uses git-native discovery)

#### Assumptions {#assumptions}

- Phases A and B are fully complete and tested
- `find_worktree_by_speck()` at `worktree.rs:698` is sufficient for worktree discovery
- Existing `.sessions/` and external `.artifacts/` directories can be orphaned; doctor will warn and users clean up manually
- Git log `--oneline` provides sufficient information for PR body generation
- `session.rs` module can be mostly gutted but kept as a stub with `now_iso8601()` to minimize file churn

---

### 3.0.0 Design Decisions {#design-decisions}

#### [D01] Remove --session parameter entirely (DECIDED) {#d01-remove-session-param}

**Decision:** Remove the `--session` parameter from both `step-commit` and `step-publish` CLI commands with no deprecation period.

**Rationale:**
- These commands are only called by the committer agent, which we control
- A deprecation period (accept-but-ignore) adds code complexity for zero benefit since no external users depend on the parameter
- Clean break is simpler to implement and test

**Implications:**
- CLI argument parsing in `cli.rs` must be updated
- All existing CLI tests with `--session` must be rewritten
- Committer agent definition must be updated simultaneously

#### [D02] Gut session.rs to now_iso8601() stub (DECIDED) {#d02-gut-session-module}

**Decision:** Keep `session.rs` as a minimal module containing only `now_iso8601()` and its helper functions (`is_leap_year`, `days_in_year`, `year_to_days`). Remove all types, structs, and persistence functions. Gut the module as the final execution step after all consumers have been updated, so every intermediate commit compiles.

**Rationale:**
- `now_iso8601()` is used by `worktree.rs` for timestamp generation via `format_compact_timestamp`
- Moving `now_iso8601()` to a different module would cause unnecessary churn across multiple files
- Keeping the module avoids renaming the `pub mod session` declaration in `lib.rs`
- Gutting last (not first) ensures each step produces a compiling commit -- consumers are updated first, then unused symbols are removed

**Implications:**
- `Session`, `StepSummary` structs are deleted
- `load_session`, `save_session`, `save_session_atomic`, `delete_session` are deleted
- `sessions_dir`, `session_file_path`, `session_id_from_worktree`, `artifacts_dir` are deleted
- `lib.rs` re-exports narrow to just `now_iso8601`
- `commands/session.rs` (deprecated `session reconcile` subcommand) is deleted entirely in an earlier step

#### [D03] Build PR body from git log (DECIDED) {#d03-pr-body-from-git-log}

**Decision:** Generate PR body in `step_publish.rs` by running `git log --oneline <base>..HEAD` in the worktree, replacing the `--step-summaries` parameter.

**Rationale:**
- Git log is the authoritative source of commit messages
- Eliminates the need for the committer agent to collect and pass step summaries
- Each step commit already has a descriptive conventional commit message

**Implications:**
- `--step-summaries` parameter is removed from `step-publish`
- `generate_pr_body()` is rewritten to call git log
- PR body format changes from agent-provided summaries to git commit messages

#### [D04] Replace list_worktrees with git-native discovery (DECIDED) {#d04-git-native-list}

**Decision:** Replace the current `list_worktrees()` function (which scans for session files) with a git-native implementation using `git worktree list --porcelain`, building on the pattern already established by `find_worktree_by_speck()`.

**Rationale:**
- Current `list_worktrees()` depends on `load_session()` which is being removed
- `find_worktree_by_speck()` already demonstrates the git-native pattern at line 698 of `worktree.rs`
- Git worktree list is authoritative — session files can be stale or corrupt

**Implications:**
- `list_worktrees()` return type changes from `Vec<Session>` to `Vec<DiscoveredWorktree>` -- this is a type cascade affecting ALL callers: `commands/worktree.rs` (3 call sites), `doctor.rs` (4 call sites: `check_stale_branches`, `check_orphaned_worktrees`, `check_sessionless_worktrees`, `check_closed_pr_worktrees`), and `cleanup_worktrees_with_pr_checker` + `cleanup_stale_branches_with_pr_checker` in `worktree.rs`
- `worktree list` command output changes (no longer shows session metadata like `total_steps` or `step_summaries`)
- `find_existing_worktree()` (used by create) must also be rewritten to not depend on sessions
- Doctor `check_sessionless_worktrees` is replaced by `check_orphaned_sessions`
- `remove_worktree()` and `cleanup_orphaned_sessions()` in `worktree.rs` call `delete_session`/`session_id_from_worktree` -- must be updated before session.rs is gutted

#### [D06] Extend DiscoveredWorktree with base_branch field (DECIDED) {#d06-discovered-worktree-base-branch}

**Decision:** Add a `base_branch: String` field to `DiscoveredWorktree`, defaulting to `"main"` when not determinable from git metadata. This field is required by `cleanup_worktrees_with_pr_checker()` for `is_ancestor` checks.

**Rationale:**
- `cleanup_worktrees_with_pr_checker()` at `worktree.rs:1119` calls `git.is_ancestor(&session.branch_name, &session.base_branch)` -- this requires knowing the base branch
- Session stored `base_branch` explicitly, but git-native discovery has no direct equivalent
- The base branch is almost always `main` in this project; defaulting is safe and avoids requiring branch metadata outside git
- If the project later needs multi-base support, this field can be populated from a config or convention

**Implications:**
- `DiscoveredWorktree` grows to 4 fields: `path`, `branch`, `speck_slug`, `base_branch`
- `cleanup_worktrees_with_pr_checker()` and `cleanup_stale_branches_with_pr_checker()` parameter types change from `&[Session]` to `&[DiscoveredWorktree]`
- Default value of `"main"` matches the single-base-branch convention used throughout this project

#### [D05] Add doctor check for orphaned .sessions/ directories (DECIDED) {#d05-orphaned-sessions-doctor}

**Decision:** Replace the current `check_sessionless_worktrees` doctor check with a new `check_orphaned_sessions` check that warns when `.specks-worktrees/.sessions/` directory exists.

**Rationale:**
- After session elimination, any `.sessions/` directory is orphaned legacy data
- Users should be informed so they can clean up manually
- The old "sessionless worktrees" check becomes meaningless without sessions

**Implications:**
- `check_sessionless_worktrees()` is replaced by `check_orphaned_sessions()`
- Doctor output changes: new check name, new message format

---

### Open Questions {#open-questions}

#### [Q01] Slug derivation from branch name is lossy (DECIDED) {#q01-slug-derivation-lossy}

**Question:** The slug extraction from branch format `specks/<slug>-<timestamp>` is ambiguous if the slug itself contains hyphens followed by digits resembling a timestamp. For example, slug `auth` produces branch `specks/auth-20260208-143022`, but could slug `auth-v2` produce `specks/auth-v2-20260208-143022` and be misparsed?

**Why it matters:** If `speck_path` is derived from slug as `.specks/specks-<slug>.md`, a misparsed slug produces a wrong path, causing worktree-to-speck matching to fail silently.

**Options (if known):**
- Parse from the right: timestamp is always `YYYYMMDD-HHMMSS` (exactly 15 chars with hyphen), so strip that fixed suffix
- Use `derive_speck_slug()` which already handles this parsing in `worktree.rs`

**Resolution:** DECIDED -- Use `derive_speck_slug()` pattern. The timestamp format is fixed at 15 characters (`YYYYMMDD-HHMMSS`). Parse from the right: strip `specks/` prefix, then strip the last 16 characters (hyphen + timestamp). The `generate_branch_name()` function at `worktree.rs` always produces this exact format, so the parse is reliable. Document this invariant in the `DiscoveredWorktree` struct doc comment.

---

### Risks and Mitigations {#risks}

| Risk | Impact | Likelihood | Mitigation | Trigger to revisit |
|------|--------|------------|------------|--------------------|
| Type cascade from `list_worktrees()` return type change | high | high | Update ALL callers in same step (Step 3) | Compile failure in any step |
| Slug derivation mismatch | med | low | Reuse existing `derive_speck_slug()` parsing | Worktree-speck matching fails |
| `base_branch` default ("main") incorrect | low | low | Matches project convention; configurable later | Multi-base-branch workflow adopted |

**Risk R01: Type cascade from list_worktrees() return type change** {#r01-type-cascade}

- **Risk:** Changing `list_worktrees()` from `Vec<Session>` to `Vec<DiscoveredWorktree>` breaks every caller. If any caller is missed, the step will not compile.
- **Mitigation:** Step 3 explicitly enumerates every caller: `worktree.rs` (4 internal sites), `commands/worktree.rs` (3 sites), `doctor.rs` (4 sites). Checkpoint grep verifies zero `Session` references remain.
- **Residual risk:** Future callers added between planning and execution would also need updating.

---

### 3.0.1 Symbol Inventory {#symbol-inventory}

#### 3.0.1.1 Symbols to remove {#symbols-remove}

**Table T01: Symbols to Remove** {#t01-symbols-remove}

| Symbol | Kind | Location | Notes |
|--------|------|----------|-------|
| `Session` | struct | `session.rs` | Main session struct |
| `StepSummary` | struct | `session.rs` | Step summary sub-struct |
| `default_schema_version()` | fn | `session.rs` | Session default helper |
| `load_session()` | fn | `session.rs` | Session loader |
| `save_session()` | fn | `session.rs` | Session saver (delegates to atomic) |
| `save_session_atomic()` | fn | `session.rs` | Atomic session writer |
| `delete_session()` | fn | `session.rs` | Session file deleter |
| `session_id_from_worktree()` | fn | `session.rs` | Path-to-ID extractor |
| `sessions_dir()` | fn | `session.rs` | Sessions directory path |
| `session_file_path()` | fn | `session.rs` | Session file path |
| `artifacts_dir()` | fn | `session.rs` | Artifacts directory path |
| `load_session_file()` | fn | `step_commit.rs` | Local session loader |
| `update_session()` | fn | `step_commit.rs` | Session update after commit |
| `load_session_file()` | fn | `step_publish.rs` | Local session loader |
| `session_id` | field | `CreateData` | Worktree create output |
| `session_file` | field | `CreateData` | Worktree create output |
| `artifacts_base` | field | `CreateData` | Worktree create output |
| `check_sessionless_worktrees()` | fn | `doctor.rs` | Replaced by orphaned check |
| `cleanup_orphaned_sessions()` | fn | `worktree.rs` | Replaced by doctor orphaned check; calls `delete_session`/`sessions_dir` |

#### 3.0.1.2 Symbols to add {#symbols-add}

**Table T02: Symbols to Add** {#t02-symbols-add}

| Symbol | Kind | Location | Notes |
|--------|------|----------|-------|
| `check_orphaned_sessions()` | fn | `doctor.rs` | Warn about `.sessions/` dirs |
| `slug_from_branch()` | fn | `worktree.rs` | Extract speck slug from branch name by stripping `specks/` prefix and timestamp suffix |

#### 3.0.1.3 Symbols to modify {#symbols-modify}

**Table T03: Symbols to Modify** {#t03-symbols-modify}

| Symbol | Kind | Location | Change |
|--------|------|----------|--------|
| `run_step_commit()` | fn | `step_commit.rs` | Remove `session` param, remove session load/save |
| `run_step_publish()` | fn | `step_publish.rs` | Remove `session` param, remove `step_summaries`, add git log PR body |
| `generate_pr_body()` | fn | `step_publish.rs` | Rewrite to use git log output |
| `StepCommit` | enum variant | `cli.rs` | Remove `--session` field |
| `StepPublish` | enum variant | `cli.rs` | Remove `--session` and `--step-summaries` fields |
| `Commands` | enum | `cli.rs` | Remove `Session(SessionCommands)` variant |
| `DiscoveredWorktree` | struct | `worktree.rs` | Add `Serialize` derive, add `speck_slug`, `base_branch` fields |
| `list_worktrees()` | fn | `worktree.rs` | Rewrite to use git-native discovery, return `Vec<DiscoveredWorktree>` |
| `find_existing_worktree()` | fn | `worktree.rs` | Rewrite to use git-native discovery |
| `cleanup_worktrees_with_pr_checker()` | fn | `worktree.rs` | Change `sessions: &[Session]` to `&[DiscoveredWorktree]`, adapt field accesses |
| `cleanup_stale_branches_with_pr_checker()` | fn | `worktree.rs` | Change `sessions: &[Session]` to `&[DiscoveredWorktree]`, adapt field accesses |
| `remove_worktree()` | fn | `worktree.rs` | Remove `delete_session`/`session_id_from_worktree` calls |
| `check_stale_branches()` | fn | `doctor.rs` | Change `s.branch_name` to `s.branch` for `DiscoveredWorktree` field |
| `check_orphaned_worktrees()` | fn | `doctor.rs` | Change `session.branch_name` to `wt.branch` for `DiscoveredWorktree` field |
| `check_closed_pr_worktrees()` | fn | `doctor.rs` | Change `session.branch_name` to `wt.branch` for `DiscoveredWorktree` field |
| `run_worktree_create()` | fn | `commands/worktree.rs` | Remove session creation and output fields |
| `run_worktree_remove()` | fn | `commands/worktree.rs` | Remove `session_id_from_worktree`/`delete_session` calls |
| `CreateData` | struct | `commands/worktree.rs` | Remove `session_id`, `session_file`, `artifacts_base` |
| `ListData` | struct | `commands/worktree.rs` | Change `worktrees` from `Vec<Session>` to new listing type |
| `lib.rs` exports | pub use | `lib.rs` | Narrow session re-exports to `now_iso8601` only |

#### 3.0.1.4 Files to remove {#files-remove}

**List L01: Files to Remove** {#l01-files-remove}

- `crates/specks/src/commands/session.rs` — Deprecated `session reconcile` command; entire file removed

---

### 3.0.2 Execution Steps {#execution-steps}

#### Step 0: Remove --session from step-commit and update step_commit.rs {#step-0}

**Bead:** `specks-t9v.1`

**Commit:** `refactor: remove --session from step-commit, eliminate session dependency`

**References:** [D01] Remove --session parameter entirely, Table T01, Table T03, (#d01-remove-session-param, #symbols-remove, #symbols-modify)

**Artifacts:**
- Modified: `crates/specks/src/cli.rs` (remove `--session` from `StepCommit`)
- Modified: `crates/specks/src/commands/step_commit.rs` (remove session logic)
- Modified: `crates/specks/src/main.rs` (update `StepCommit` match arm to not pass session)

**Tasks:**
- [ ] Remove `session: String` field from `StepCommit` variant in `cli.rs`
- [ ] Remove `session: String` parameter from `run_step_commit()` function signature
- [ ] Remove `session_path` validation (the `!session_path.exists()` check)
- [ ] Remove `load_session_file()` helper function
- [ ] Remove `update_session()` helper function
- [ ] Remove session loading call and session update call
- [ ] Remove `use specks_core::session::{Session, StepSummary, now_iso8601, save_session_atomic}` import — remove all session imports from this file
- [ ] Update the `StepCommit` match arm in `main.rs` to not pass `session`
- [ ] Update the module doc comment at `step_commit.rs` line 2 (`//! Atomically performs log rotation, prepend, git commit, bead close, and session update.`) to remove the "and session update" phrase -- this doc comment contains the word "session" and will be caught by the checkpoint grep
- [ ] Update the `close_bead_in_worktree` doc comment at `step_commit.rs` line ~217 -- if it contains the word "session" it will be caught by the checkpoint grep. Check and remove any session references in this helper's doc comment.
- [ ] Update the `StepCommit` variant doc comment in `cli.rs` (lines ~174-178): remove "and session update" from the short doc (`/// Atomically performs log rotation, prepend, git commit, bead close, and session update.`) and remove "6. Update session" from the `long_about` attribute
- [ ] Update CLI tests `test_step_commit_command` and `test_step_commit_with_close_reason` to not include `--session`

**Tests:**
- [ ] CLI test: `StepCommit` parses without `--session` argument
- [ ] CLI test: existing tests updated to match new signature

**Checkpoint:**
- [ ] `cargo build` succeeds
- [ ] `cargo nextest run` -- all step_commit and CLI tests pass
- [ ] `grep -c "session" crates/specks/src/commands/step_commit.rs` returns 0

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

#### Step 1: Remove --session and --step-summaries from step-publish, add git log PR body {#step-1}

**Depends on:** #step-0

**Bead:** `specks-t9v.2`

**Commit:** `refactor: remove --session/--step-summaries from step-publish, generate PR body from git log`

**References:** [D01] Remove --session parameter entirely, [D03] Build PR body from git log, Table T01, Table T03, (#d01-remove-session-param, #d03-pr-body-from-git-log, #symbols-modify)

**Artifacts:**
- Modified: `crates/specks/src/cli.rs` (remove `--session` and `--step-summaries` from `StepPublish`)
- Modified: `crates/specks/src/commands/step_publish.rs` (rewrite PR body generation)
- Modified: `crates/specks/src/main.rs` (update `StepPublish` match arm)

**Tasks:**
- [ ] Remove `session: String` field from `StepPublish` variant in `cli.rs`
- [ ] Remove `step_summaries: Vec<String>` field from `StepPublish` variant in `cli.rs`
- [ ] Remove `session: String` and `step_summaries: Vec<String>` from `run_step_publish()` signature
- [ ] Remove session validation, loading, and saving from `run_step_publish()`
- [ ] Remove `load_session_file()` helper function
- [ ] Rewrite `generate_pr_body()` to accept `worktree_path: &Path` and `base: &str` parameters and run `git log --oneline <base>..HEAD` to get commit messages
- [ ] Update `generate_pr_body()` call site to pass worktree path and base branch
- [ ] Remove all session imports from this file
- [ ] Update the `StepPublish` variant doc comment in `cli.rs` (lines ~218-222): remove "and updates session status" from the short doc and remove "6. Update session to Completed" from the `long_about` attribute. Also update "3. Generate PR body from step summaries" to "3. Generate PR body from git log" in the `long_about`.
- [ ] Update the `StepPublish` match arm in `main.rs`
- [ ] Update CLI tests `test_step_publish_command` and `test_step_publish_with_repo` to not include `--session` or `--step-summaries`
- [ ] Update `test_generate_pr_body` test to match new function signature (takes worktree path and base branch, not string vec). **Coder note:** the new function runs `git log`, so the test must set up a temp git repo with real commits (init repo, make commits, then call `generate_pr_body`). The existing test at `step_publish.rs:330` just constructs `Vec<String>` -- that approach no longer works.

**Tests:**
- [ ] Unit test: `generate_pr_body()` with mocked git log output produces expected markdown
- [ ] Unit test: `parse_pr_info()` still works (unchanged)
- [ ] CLI test: `StepPublish` parses without `--session` and `--step-summaries`

**Checkpoint:**
- [ ] `cargo build` succeeds
- [ ] `cargo nextest run` -- all step_publish tests pass
- [ ] `grep -c "session\|step_summaries" crates/specks/src/commands/step_publish.rs` returns 0

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

#### Step 2: Remove deprecated session CLI subcommand {#step-2}

**Depends on:** #step-0

**Bead:** `specks-t9v.3`

**Commit:** `refactor: remove deprecated session CLI subcommand and SessionReconcileData`

**References:** [D02] Gut session.rs to now_iso8601() stub, Table T01, List L01, (#d02-gut-session-module, #symbols-remove, #files-remove)

**Artifacts:**
- Removed: `crates/specks/src/commands/session.rs` (entire file deleted)
- Modified: `crates/specks/src/commands/mod.rs` (remove `SessionCommands` and `run_session` re-exports)
- Modified: `crates/specks/src/cli.rs` (remove `SessionCommands` import and `Session(SessionCommands)` variant from `Commands` enum)
- Modified: `crates/specks/src/main.rs` (remove `Commands::Session` match arm)
- Modified: `crates/specks/src/output.rs` (remove deprecated `SessionReconcileData` struct)

**Tasks:**
- [ ] Delete `crates/specks/src/commands/session.rs` entirely
- [ ] In `commands/mod.rs`: remove `pub mod session;` line and `pub use session::{SessionCommands, run_session};` re-export
- [ ] In `cli.rs`: remove `SessionCommands` from `use crate::commands::{BeadsCommands, LogCommands, SessionCommands, WorktreeCommands}`
- [ ] In `cli.rs`: remove the `Session(SessionCommands)` variant from the `Commands` enum (including its `#[command(long_about = ...)]` attribute)
- [ ] In `main.rs`: remove the `Some(Commands::Session(session_cmd))` match arm
- [ ] In `output.rs`: remove the deprecated `SessionReconcileData` struct and its `#[deprecated]` annotation
- [ ] Fix any remaining compilation errors from removed symbols

**Tests:**
- [ ] CLI test: `specks session` is no longer a recognized subcommand
- [ ] Build: `cargo build` succeeds

**Checkpoint:**
- [ ] `cargo build` succeeds
- [ ] `cargo nextest run` -- all tests pass
- [ ] `grep -rn "SessionCommands\|SessionReconcileData\|run_session" crates/specks/src/` returns 0
- [ ] `crates/specks/src/commands/session.rs` does not exist

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

#### Step 3: Rewrite list_worktrees, all callers, and cleanup functions to use git-native discovery {#step-3}

**Depends on:** #step-0

**Bead:** `specks-t9v.4`

**Commit:** `refactor: replace session-based worktree listing with git-native discovery`

**References:** [D04] Replace list_worktrees with git-native discovery, [D06] Extend DiscoveredWorktree with base_branch field, [Q01] Slug derivation from branch name, Risk R01, Table T01, Table T02, Table T03, (#d04-git-native-list, #d06-discovered-worktree-base-branch, #q01-slug-derivation-lossy, #r01-type-cascade, #symbols-remove, #symbols-add, #symbols-modify)

**Artifacts:**
- Modified: `crates/specks-core/src/worktree.rs` (rewrite `list_worktrees`, `find_existing_worktree`, update `create_worktree`, update `cleanup_worktrees_with_pr_checker`, update `cleanup_stale_branches_with_pr_checker`, update `remove_worktree`, remove `cleanup_orphaned_sessions`)
- Modified: `crates/specks/src/commands/worktree.rs` (update `ListData`, list command output, update `run_worktree_remove` type annotations)
- Modified: `crates/specks/src/commands/doctor.rs` (update `check_stale_branches`, `check_orphaned_worktrees`, `check_closed_pr_worktrees` to use `DiscoveredWorktree` fields)

**Tasks:**

*DiscoveredWorktree struct changes:*
- [ ] Add `Serialize` to the `#[derive(...)]` on `DiscoveredWorktree` (currently only `Debug, Clone`). This is required because `ListData` has `#[derive(Serialize)]` and will contain `Vec<DiscoveredWorktree>` after this step -- without `Serialize` on `DiscoveredWorktree`, the build will fail. Add `use serde::Serialize;` import if not already present in scope.
- [ ] Add a `speck_slug: String` field to `DiscoveredWorktree` in `crates/specks-core/src/worktree.rs`. Populate it by parsing the branch name: given format `specks/<slug>-<timestamp>`, the timestamp is always exactly 15 characters (`YYYYMMDD-HHMMSS`), so strip the `specks/` prefix and the last 16 characters (hyphen + timestamp) to get the slug. Add a `slug_from_branch()` helper function for this. Document the parsing invariant in the struct doc comment. (See [Q01] resolution.)
- [ ] Add a `base_branch: String` field to `DiscoveredWorktree`, defaulting to `"main"`. Git-native discovery does not provide the base branch, so this uses the project convention. (See [D06].)

*Core list/find rewrites:*
- [ ] Rewrite `list_worktrees()` to use `git worktree list --porcelain` pattern from `find_worktree_by_speck()`, returning `Vec<DiscoveredWorktree>` instead of `Vec<Session>`. Populate `speck_slug` via `slug_from_branch()` and `base_branch` as `"main"`.
- [ ] Rewrite `find_existing_worktree()` to use git-native discovery instead of loading sessions -- match on `speck_slug` derived from branch name
- [ ] Update `create_worktree()` in `crates/specks-core/src/worktree.rs` (line ~613) to destructure `DiscoveredWorktree` fields instead of `Session` fields. Specifically: replace `existing_session.worktree_path` with `existing.path`, `existing_session.branch_name` with `existing.branch`, and `existing_session.speck_slug` with `existing.speck_slug`.

*Cleanup function updates (same step to maintain compilation):*
- [ ] Update `cleanup_worktrees_with_pr_checker()` in `worktree.rs` (line ~1091): change parameter from implicit `list_worktrees()` returning `Vec<Session>` (line ~1098) to `Vec<DiscoveredWorktree>`. Adapt all field accesses: `session.branch_name` (lines 1110, 1157) becomes `wt.branch`, `session.base_branch` (line 1119) becomes `wt.base_branch`, `session.worktree_path` (line 1160) becomes `wt.path.to_string_lossy()` (or `wt.path.display()`).
- [ ] Update `cleanup_stale_branches_with_pr_checker()` in `worktree.rs` (line ~862): change `sessions: &[Session]` parameter to `worktrees: &[DiscoveredWorktree]`. Adapt `s.branch_name` (line 873) to `wt.branch`. Also remove the inline `crate::session::session_id_from_worktree()` and `crate::session::delete_session()` calls at lines 934-936 in the stale-branch-with-worktree removal path -- these calls clean up session files for force-removed worktrees, but sessions no longer exist. Remove the entire `if let Some(session_id)` block (lines 934-937) and keep only the worktree force-remove and branch delete logic.
- [ ] Update `remove_worktree()` in `worktree.rs` (line ~997): remove the `use crate::session::{delete_session, session_id_from_worktree}` import (line 998) and the `session_id_from_worktree()`/`delete_session()` call block (lines 1000-1004). Keep the legacy internal session file cleanup (lines 1006-1010) and artifacts cleanup.
- [ ] Remove `cleanup_orphaned_sessions()` function in `worktree.rs` (line ~1038) entirely -- it calls `delete_session()` and `sessions_dir()` from session.rs. The doctor orphaned check (Step 6) replaces this functionality. Also remove the `cleanup_orphaned_sessions(repo_root, dry_run)` call at line ~1203 in `cleanup_worktrees_with_pr_checker()`.

*Doctor callers (must update in same step as list_worktrees return type change):*
- [ ] Update `check_stale_branches()` in `doctor.rs` (line ~368): `list_worktrees()` now returns `Vec<DiscoveredWorktree>`. Change `s.branch_name.clone()` (line 382) to `s.branch.clone()`.
- [ ] Update `check_orphaned_worktrees()` in `doctor.rs` (line ~431): change `session.branch_name` (lines 446, 448) to `wt.branch`. Update loop variable name from `session` to `wt`.
- [ ] Update `check_closed_pr_worktrees()` in `doctor.rs` (line ~584): change `session.branch_name` (lines 599, 601) to `wt.branch`. Update loop variable name from `session` to `wt`.
- [ ] In `check_sessionless_worktrees()` in `doctor.rs` (line ~531): this function also calls `list_worktrees()` and accesses `s.worktree_path` (line 545). Change to `s.path.to_string_lossy().to_string()` (or `s.path.display().to_string()`). Note: this function is replaced entirely in Step 6, but must compile in this step.

*Commands/worktree.rs callers:*
- [ ] Update `run_worktree_remove()` in `commands/worktree.rs` (line ~991) to use `Vec<&DiscoveredWorktree>` instead of `Vec<&Session>` for `matching_sessions` (line ~1001). Adapt field accesses as follows:
  - `session.speck_path` (line ~1013, used for matching): derive at call site as `.specks/specks-<speck_slug>.md` using the `speck_slug` field on `DiscoveredWorktree`
  - `session.created_at` (line ~1036, used in disambiguation display): simplify disambiguation display to show `branch` and `path` instead (both are directly available on `DiscoveredWorktree`; `created_at` is not worth deriving for a disambiguation message)
  - `session.branch_name` (line ~1041): replace with `wt.branch`
  - `session.worktree_path` (line ~1101, in force-remove path): replace with `wt.path.display()` or `wt.path.to_string_lossy()`
- [ ] In `run_worktree_remove()`: remove the direct `session_id_from_worktree()`/`delete_session()` calls at lines 1117-1118 in the force-remove branch -- these are separate from the `remove_worktree()` call at line 1122 (which is updated in core). Both paths must be cleaned up in this step to compile with the new `DiscoveredWorktree` type.
- [ ] Update `ListData` struct to use `Vec<DiscoveredWorktree>` instead of `Vec<Session>` -- the struct now has `path`, `branch`, `speck_slug`, `base_branch`
- [ ] Update `run_worktree_list` to display new listing format (branch, path, speck_slug -- no session metadata like `total_steps` or `step_summaries`)

*Import and doc comment cleanup:*
- [ ] Remove `use crate::session::{Session, load_session, now_iso8601}` import from `crates/specks-core/src/worktree.rs` -- keep only `now_iso8601` import
- [ ] Remove `use crate::session::save_session` from `worktree.rs` test module (line ~1216)
- [ ] Update doc comment on `create_worktree()` at worktree.rs line ~575: change "Session creation is now handled by CLI layer" to remove the word "Session" (e.g., "Worktree metadata is returned as an infrastructure tuple").
- [ ] Update doc comment on `create_worktree()` return at worktree.rs line ~642: change "Return infrastructure tuple (CLI layer creates Session)" to remove the word "Session" (e.g., "Return infrastructure tuple").
- [ ] Update comment in `cleanup_worktrees_with_pr_checker()` at worktree.rs line ~1108: change "Note: Session status removed in v2. Protection now relies on PR state and user confirmation." to remove the word "Session" (e.g., "Note: Protection relies on PR state and user confirmation.").

*Test updates:*
- [ ] Rewrite integration tests that construct `Session` objects for `list_worktrees` and `find_existing_worktree`. There are approximately 35 `Session`/`save_session`/`load_session` references across the test module (lines 1213-3400+). Key test clusters requiring substantial rewriting: lines 1399-1420, 1444-1465, 1497-1518, 1571-1592, 1654-1677, 1901-1917 in `commands/worktree.rs`. All tests that construct `Session` objects or call `save_session()` must be rewritten to use git-native worktree setup (e.g., `git worktree add` directly). Expect the full scope to be significantly larger than 7 tests.
- [ ] Update `create_worktree_with_session()` test helper (line 1654) -- replace with a git-native helper that creates worktrees via `git worktree add` directly

**Tests:**
- [ ] Integration test: `list_worktrees()` returns worktrees found by git with correct `speck_slug` and `base_branch` fields
- [ ] Integration test: `find_existing_worktree()` matches by speck slug derived from branch name
- [ ] Unit test: `slug_from_branch("specks/auth-20260208-143022")` returns `"auth"`
- [ ] Unit test: `slug_from_branch("specks/auth-v2-20260208-143022")` returns `"auth-v2"` (multi-hyphen slug)
- [ ] All rewritten worktree tests pass
- [ ] All doctor tests pass (field access changes compile correctly)

**Checkpoint:**
- [ ] `cargo build` succeeds (critical: verifies ALL callers compile with new return type)
- [ ] `cargo nextest run` -- all worktree AND doctor tests pass
- [ ] `grep -c "Session\|load_session\|save_session\|delete_session\|session_id_from_worktree" crates/specks-core/src/worktree.rs` returns 0 (covers code references AND doc comments -- doc comments mentioning "Session" are explicitly updated in this step's tasks)
- [ ] `grep -c "\.branch_name\|\.worktree_path\|\.speck_path" crates/specks/src/commands/doctor.rs` returns 0 (dot-prefixed to match field accesses, avoiding false positives from function names like `is_valid_worktree_path`)

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

#### Step 4: Remove session creation from worktree create command {#step-4}

**Depends on:** #step-1, #step-2, #step-3

**Bead:** `specks-t9v.5`

**Commit:** `refactor: stop creating session files in worktree create, remove remaining session imports`

**References:** [D01] Remove --session parameter entirely, [D02] Gut session.rs, Table T01, Table T03, (#d01-remove-session-param, #d02-gut-session-module, #symbols-remove, #symbols-modify)

**Artifacts:**
- Modified: `crates/specks/src/commands/worktree.rs` (remove session creation, output fields, remaining session imports)

**Tasks:**
- [ ] Remove `session_id`, `session_file`, `artifacts_base` fields from `CreateData` struct
- [ ] Remove session creation block (the `Session { ... }` construction and `save_session()` call in `run_worktree_create`)
- [ ] Remove `session_file` path computation
- [ ] Remove `session_id`, `session_file`, `artifacts_base` from all `CreateData` construction sites (including the 3+ error-path instantiations)
- [ ] Remove the `existing_session` reuse check that calls `load_session()` -- reuse detection is now handled by `find_existing_worktree()` in core
- [ ] Remove `session_id` derivation from branch name -- keep `speck_slug` which is still needed
- [ ] Remove `use specks_core::session::{Session, delete_session, session_id_from_worktree, save_session, ...}` imports from `commands/worktree.rs` -- all session usage from this file should now be gone (note: `run_worktree_remove` session cleanup was already removed in Step 3 when adapting field accesses for the type change)
- [ ] Keep artifacts directory creation inside worktree (`.specks/artifacts/`) -- this is useful independent of sessions
- [ ] Rewrite worktree create integration tests that assert on `session_id`, `session_file`, or `artifacts_base` fields in `CreateData` output -- these fields no longer exist

**Tests:**
- [ ] Integration test: `run_worktree_create` succeeds without creating `.sessions/` directory
- [ ] JSON output test: `CreateData` does not contain `session_id`, `session_file`, or `artifacts_base`

**Checkpoint:**
- [ ] `cargo build` succeeds
- [ ] `cargo nextest run` -- all worktree create tests pass
- [ ] No `.sessions/` directory is created during worktree creation
- [ ] `grep -c "delete_session\|session_id_from_worktree\|save_session\|load_session\|Session" crates/specks/src/commands/worktree.rs` returns 0

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

#### Step 5: Update agent and skill definitions {#step-5}

**Depends on:** #step-1, #step-2, #step-4

**Bead:** `specks-t9v.6`

**Commit:** `docs: remove session references from agent and skill definitions`

**References:** [D01] Remove --session parameter entirely, (#d01-remove-session-param, #strategy, #scope)

**Artifacts:**
- Modified: `agents/committer-agent.md` (remove `--session` from CLI calls)
- Modified: `agents/implementer-setup-agent.md` (remove `session_file` and `artifacts_base` from output)
- Modified: `agents/coder-agent.md` (remove `session_id` from input contract and prompt JSON)
- Modified: `skills/implementer/SKILL.md` (remove all session references)

**Tasks:**
- [ ] In `committer-agent.md`: remove `--session "{session_file}"` from `specks step-commit` command template
- [ ] In `committer-agent.md`: remove `--session "{session_file}"` from `specks step-publish` command template
- [ ] In `committer-agent.md`: remove `--step-summaries` from `specks step-publish` command template (PR body now from git log)
- [ ] In `committer-agent.md`: remove `session_file` from input contract
- [ ] In `committer-agent.md`: update description text that mentions session updates
- [ ] In `implementer-setup-agent.md`: remove `session` object with `session_id`, `session_file`, `artifacts_base` from output contract example
- [ ] In `implementer-setup-agent.md`: remove `session_file`, `artifacts_base` from "Parse the JSON response for" instruction
- [ ] In `implementer-setup-agent.md`: update description to not mention "implementation session"
- [ ] In `coder-agent.md`: remove `session_id` from initial spawn JSON example (line 56)
- [ ] In `coder-agent.md`: remove `session_id` row from input contract table (line 66)
- [ ] In `skills/implementer/SKILL.md`: remove `Session: {session_id}` from status message format
- [ ] In `skills/implementer/SKILL.md`: remove `session.session_id`, `session.session_file` from "Store in memory" instruction
- [ ] In `skills/implementer/SKILL.md`: remove `session_id` from coder prompt JSON
- [ ] In `skills/implementer/SKILL.md`: remove `session_file` from committer prompt JSON (commit and publish modes)
- [ ] In `skills/implementer/SKILL.md`: update "session end message" references
- [ ] In `skills/implementer/SKILL.md`: update agent persistence table (committer row mentions "session file format")
- [ ] In `skills/implementer/SKILL.md`: update description of `specks step-commit` and `specks step-publish` to not mention session update
- [ ] In `skills/implementer/SKILL.md`: update the workflow diagram at line ~209 from `create PR with step_summaries` to `create PR (body from git log)` -- this diagram line describes the publish mode and must reflect that PR body is now generated from git log, not step_summaries
- [ ] In `skills/implementer/SKILL.md`: remove `step_summaries = []` initialization (line ~269) -- this collection variable is dead logic after `step-publish` no longer accepts `--step-summaries`
- [ ] In `skills/implementer/SKILL.md`: remove the per-step `step_summaries` collection logic (line ~497, "Extract commit summary and add to step_summaries") -- summaries are now derived from git log
- [ ] In `skills/implementer/SKILL.md`: remove `step_summaries` from the publish prompt JSON (line ~518) -- the `--step-summaries` CLI parameter no longer exists

**Tests:**
- [ ] Manual: grep for "session_file" across all agent/skill files returns zero hits
- [ ] Manual: grep for "--session" across all agent/skill files returns zero hits
- [ ] Manual: grep for "session_id" across all agent/skill files returns zero hits (excluding legitimate uses like "session" in general prose)

**Checkpoint:**
- [ ] `grep -r "session_file\|--session\|session_id" agents/ skills/` returns zero hits (excluding any legitimate non-session uses of the word "session")
- [ ] `grep -r "step_summaries" skills/implementer/SKILL.md` returns zero hits

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

#### Step 6: Replace sessionless worktrees doctor check with orphaned sessions check {#step-6}

**Depends on:** #step-3, #step-4

**Bead:** `specks-t9v.7`

**Commit:** `feat: replace sessionless_worktrees doctor check with orphaned_sessions check`

**References:** [D05] Add doctor check for orphaned .sessions/ directories, Table T02, (#d05-orphaned-sessions-doctor, #symbols-add)

**Artifacts:**
- Modified: `crates/specks/src/commands/doctor.rs` (replace check function)

**Tasks:**
- [ ] Remove `check_sessionless_worktrees()` function
- [ ] Add `check_orphaned_sessions()` function that checks if `.specks-worktrees/.sessions/` directory exists
- [ ] If `.sessions/` exists and contains files: warn with message listing the orphaned session files
- [ ] If `.sessions/` exists but is empty: warn with suggestion to remove the empty directory
- [ ] If `.sessions/` does not exist: pass
- [ ] Replace `check_sessionless_worktrees()` call in `run_doctor()` with `check_orphaned_sessions()`

**Tests:**
- [ ] Unit test: `check_orphaned_sessions()` returns "pass" when no `.sessions/` directory exists
- [ ] Unit test: `check_orphaned_sessions()` returns "warn" when `.sessions/` directory exists with files

**Checkpoint:**
- [ ] `cargo build` succeeds
- [ ] `cargo nextest run` -- doctor tests pass
- [ ] `specks doctor` runs without error

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

#### Step 7: Gut session.rs to now_iso8601 stub and final cleanup {#step-7}

**Depends on:** #step-0, #step-1, #step-2, #step-3, #step-4, #step-5, #step-6

**Bead:** `specks-t9v.8`

**Commit:** `refactor: gut session.rs to now_iso8601 stub, remove all remaining session symbols`

**References:** [D02] Gut session.rs to now_iso8601() stub, Table T01, (#d02-gut-session-module, #symbols-remove, #scope, #success-criteria)

**Artifacts:**
- Modified: `crates/specks-core/src/session.rs` (gutted to stub: only `now_iso8601` and helpers remain)
- Modified: `crates/specks-core/src/lib.rs` (narrow re-exports to `pub use session::now_iso8601` only)
- Modified: `CLAUDE.md` (update documentation references)

**Tasks:**
- [ ] In `session.rs`: remove `Session` struct, `StepSummary` struct, `Default` impl, `default_schema_version()`
- [ ] In `session.rs`: remove `load_session()`, `save_session()`, `save_session_atomic()`, `delete_session()`
- [ ] In `session.rs`: remove `session_id_from_worktree()`, `sessions_dir()`, `session_file_path()`, `artifacts_dir()`
- [ ] In `session.rs`: remove all `#[cfg(test)]` module tests
- [ ] In `session.rs`: remove unused imports (`serde`, `fs`, `Path`) -- keep only `std::time` imports used by `now_iso8601`
- [ ] In `session.rs`: keep `now_iso8601()`, `is_leap_year()`, `days_in_year()`, `year_to_days()` functions
- [ ] In `lib.rs`: change re-exports from `pub use session::{Session, StepSummary, artifacts_dir, load_session, now_iso8601, save_session, session_file_path, session_id_from_worktree, sessions_dir}` to `pub use session::now_iso8601`
- [ ] Update `CLAUDE.md` to remove references to `--session` in step-commit/step-publish documentation and remove `session reconcile` from the commands listing
- [ ] Run `cargo build` and fix any remaining compilation errors from dangling session references
- [ ] Run `cargo clippy --all-targets -- -D warnings` and fix all warnings
- [ ] Grep entire codebase for remaining `Session` (capital-S) references in Rust code -- should be zero outside comments

**Tests:**
- [ ] Unit test: `now_iso8601()` still returns valid ISO 8601 format string
- [ ] Full test suite: `cargo nextest run` passes
- [ ] Lint: `cargo clippy --all-targets -- -D warnings` passes

**Checkpoint:**
- [ ] `cargo nextest run` -- all tests pass
- [ ] `cargo clippy --all-targets -- -D warnings` -- zero warnings
- [ ] `session.rs` contains only `now_iso8601` and helper functions (< 100 lines)
- [ ] `grep -c "Session\|StepSummary\|load_session\|save_session\|delete_session" crates/specks-core/src/session.rs` returns 0
- [ ] `grep -rn "use.*session::" crates/` returns only `now_iso8601` imports
- [ ] `grep -rn "Session\b" crates/specks-core/src/ crates/specks/src/` returns zero hits in non-comment lines

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

### 3.0.3 Deliverables and Checkpoints {#deliverables}

**Deliverable:** Session file infrastructure is fully removed from the specks codebase. All state that was previously stored in session files is now derived from git worktree metadata and beads. The CLI, agents, and orchestrator skill operate without session files.

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

- [ ] No session file is created during `specks worktree create` (verify: run create, check no `.sessions/` directory)
- [ ] `specks step-commit` works without `--session` parameter (verify: run command)
- [ ] `specks step-publish` generates PR body from git log without `--session` or `--step-summaries` (verify: inspect PR body)
- [ ] `specks doctor` warns about orphaned `.sessions/` directories (verify: create orphaned dir, run doctor)
- [ ] `cargo nextest run` passes with zero failures (verify: run full test suite)
- [ ] `cargo clippy --all-targets -- -D warnings` passes (verify: run clippy)
- [ ] No Rust source file imports `Session` or `StepSummary` types (verify: grep)
- [ ] Agent and skill definitions contain no `session_file`, `--session`, or `session_id` references (verify: grep)
- [ ] `specks session` is no longer a recognized CLI subcommand (verify: `specks session --help` errors)
- [ ] `crates/specks/src/commands/session.rs` does not exist (verify: file check)

**Acceptance tests:**
- [ ] Integration test: full worktree create + step-commit cycle without session files
- [ ] Integration test: step-publish generates correct PR body from git log
- [ ] Unit test: `now_iso8601()` still works correctly in stubbed `session.rs`

#### Roadmap / Follow-ons (Explicitly Not Required for Phase Close) {#roadmap}

- [ ] Provide a `specks cleanup-legacy` command to auto-delete orphaned `.sessions/` and external `.artifacts/` directories
- [ ] Move `now_iso8601()` to a dedicated `time.rs` or `util.rs` module for better organization
- [ ] Phase D: Status from Beads (depends on Phase A rich content, benefits from Phase C removing conflicting state)

| Checkpoint | Verification |
|------------|--------------|
| Session types removed | `grep -c "pub struct Session" crates/` returns 0 |
| CLI parameters removed | `specks step-commit --help` shows no `--session` |
| Agent definitions clean | `grep -r "session_file" agents/ skills/` returns 0 |
| Full test suite passes | `cargo nextest run` exits 0 |
| No warnings | `cargo clippy --all-targets -- -D warnings` exits 0 |

**Commit after all checkpoints pass.**