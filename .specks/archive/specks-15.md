## Phase 1.0: Merge and Worktree Robustness Improvements {#phase-merge-robustness}

**Purpose:** Fix critical architectural issues in `specks merge` and worktree cleanup to handle squash merges, session state corruption, and race conditions reliably.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | specks |
| Status | draft |
| Target branch | main |
| Tracking issue/PR | TBD |
| Last updated | 2026-02-09 |
| Beads Root | *(written by `specks beads sync`)* |
| Beads Root | `specks-15g` |

---

### Phase Overview {#phase-overview}

#### Context {#context}

An architectural review of `specks merge` and worktree functionality identified 12 issues across 3 severity levels. The critical issues involve broken squash merge detection, session state inconsistencies after partial failures, and race conditions during merge operations. These bugs can leave repositories in inconsistent states and cause the `specks worktree cleanup --merged` command to fail silently.

The current implementation uses `git merge-base --is-ancestor` to detect merged branches, which fundamentally does not work for squash merges (the dominant merge strategy). Additionally, session updates are not atomic, leading to corruption on interrupted writes, and there is no recovery mechanism for sessions left in `NeedsReconcile` state.

#### Strategy {#strategy}

- Fix critical issues first (squash detection, session atomicity, race conditions)
- Add recovery CLI for NeedsReconcile state
- Use GitHub API via `gh` CLI for PR merge status (replaces broken git-based detection)
- Implement temp-file + atomic rename pattern for session writes
- Validate working directory assumptions before merge operations
- Improve error context for shell command failures
- Deduplicate timestamp generation code

#### Stakeholders / Primary Customers {#stakeholders}

1. Developers using `specks merge` to complete PR workflows
2. CI/CD pipelines relying on `specks worktree cleanup --merged`

#### Success Criteria (Measurable) {#success-criteria}

- `specks worktree cleanup --merged` correctly identifies squash-merged PRs (test with squash merge)
- Session files are never corrupted on interrupted writes (verified by test)
- `specks session reconcile` can recover NeedsReconcile sessions
- All 12 identified issues are addressed with tests

#### Scope {#scope}

1. Critical: Fix squash merge detection using GitHub API
2. Critical: Add atomic session writes (temp + rename)
3. Critical: Add check_main_sync race condition mitigation
4. Major: Add `specks session reconcile` command
5. Major: Validate current directory is main worktree before merge
6. Major: Use `gh pr checks --json` instead of parsing tabs
7. Minor: Improve rollback error handling (log warnings)
8. Minor: Improve infrastructure pattern matching
9. Minor: Add command context to shell error messages
10. Minor: Deduplicate timestamp generation code
11. Minor: Validate gh CLI version for --json support

#### Non-goals (Explicitly out of scope) {#non-goals}

- Changing the session ID extraction convention (basename parsing is stable)
- Full rewrite of worktree management
- Adding new worktree features beyond fixing existing bugs

#### Dependencies / Prerequisites {#dependencies}

- `gh` CLI must be installed and authenticated (already required)
- GitHub API access for PR merge status queries

#### Constraints {#constraints}

- Must maintain backward compatibility with existing session.json files
- Cannot change public CLI output schemas without versioning

#### Assumptions {#assumptions}

- The `specks__<slug>-<timestamp>` naming convention will remain stable
- Users have `gh` CLI version 2.0+ installed (supports --json flags)
- Breaking changes to CLI output are acceptable if documented

---

### 1.0.0 Design Decisions {#design-decisions}

#### [D01] Use GitHub API for squash merge detection (DECIDED) {#d01-github-api-merge-detection}

**Decision:** Replace `git merge-base --is-ancestor` with `gh pr view --json mergedAt,mergeCommit,state` to detect merged PRs.

**Rationale:**
- `git merge-base --is-ancestor` checks if branch commits are ancestors of main, which is false after squash/rebase merges
- GitHub API directly reports whether a PR was merged, regardless of merge strategy
- The `gh` CLI is already a required dependency for PR creation and checks

**Implications:**
- Adds network dependency for cleanup operations (acceptable, already required for other operations)
- Must handle case where PR was never created or was deleted
- Fallback to current behavior if `gh` fails (log warning)

---

#### [D02] Atomic session writes via temp file + rename (DECIDED) {#d02-atomic-session-writes}

**Decision:** Write session.json to a temporary file first, then atomically rename to final location.

**Rationale:**
- Current `fs::write()` is not atomic; interrupted writes leave corrupted files
- POSIX rename is atomic on the same filesystem
- Temp file approach ensures either old or new content exists, never partial

**Implications:**
- Temp file must be in same directory as target (same filesystem)
- Error handling must clean up temp file on failure
- Pattern: write to `session.json.tmp`, rename to `session.json`

---

#### [D03] CLI command for NeedsReconcile recovery (DECIDED) {#d03-needs-reconcile-cli}

**Decision:** Add `specks session reconcile <session-id>` command to manually recover from NeedsReconcile state.

**Rationale:**
- NeedsReconcile occurs when commit succeeds but bead close fails
- Automatic recovery is risky without knowing which bead to close
- Manual CLI gives operator control with clear feedback

**Implications:**
- New subcommand under `specks session` namespace
- Must identify correct bead from session/artifacts
- Should support `--dry-run` flag for preview

---

#### [D04] Validate main worktree before merge operations (DECIDED) {#d04-validate-main-worktree}

**Decision:** Check that current directory is the main worktree (not a specks worktree or other detached checkout) before running merge.

**Rationale:**
- `specks merge` assumes it runs from main worktree to push changes
- Running from wrong directory causes confusing failures
- Early validation provides clear error message

**Implications:**
- Check `.git` is a directory (not a file, which indicates worktree)
- Verify HEAD is on expected branch (main/master)
- Provide actionable error message with `cd` suggestion

---

#### [D05] Use gh pr checks --json for robust parsing (DECIDED) {#d05-gh-checks-json}

**Decision:** Replace tab-separated parsing of `gh pr checks` output with `--json` format.

**Rationale:**
- Current tab-separated parsing is fragile (depends on column order)
- JSON output provides structured data with explicit field names
- Already using JSON for `gh pr view`, consistent approach

**Implications:**
- Query: `gh pr checks <branch> --json name,state,conclusion`
- Parse JSON array of check objects
- Handle empty array (no checks) as success

---

#### [D06] Race condition mitigation for check_main_sync (DECIDED) {#d06-race-condition-mitigation}

**Decision:** Re-verify main branch sync immediately before push operation, not just at workflow start.

**Rationale:**
- Time gap between initial check and actual push allows race conditions
- Another process could push commits between check and push
- Re-check minimizes window for race (still not eliminated, but practical)

**Implications:**
- Call `check_main_sync()` again right before `git push`
- Accept that tiny race window remains (unavoidable without locking)
- Document this limitation

---

#### [D07] Expose now_iso8601 as shared utility (DECIDED) {#d07-deduplicate-timestamp}

**Decision:** Expose `session::now_iso8601()` as public and use it from worktree.rs instead of duplicating.

**Rationale:**
- Same timestamp logic exists in both session.rs and worktree.rs
- DRY principle; single source of truth for timestamp format
- Already public in session.rs, just need to import in worktree.rs

**Implications:**
- Remove `generate_timestamp_utc()` from worktree.rs
- Import and use `now_iso8601()` from session module
- Update format conversion if needed (session uses ISO8601, worktree uses compact)

---

#### [D08] Log warnings instead of silently ignoring rollback errors (DECIDED) {#d08-rollback-warnings}

**Decision:** Log warning messages when rollback operations fail, but do not propagate as errors.

**Rationale:**
- Rollback failures during error handling should not mask the original error
- Silent failures make debugging difficult
- Warnings provide visibility without breaking error flow

**Implications:**
- Use `eprintln!` for warning output (consistent with existing pattern)
- Message format: "Warning: rollback failed: <operation>: <error>"
- Continue with other rollback operations after individual failure

---

#### [D09] Improve infrastructure pattern matching (DECIDED) {#d09-infrastructure-patterns}

**Decision:** Enhance `is_infrastructure_file()` to use explicit path matching with clear documentation.

**Rationale:**
- Current simple string matching works but is not extensible
- Edge cases like `.specks/specks-*.md` exclusion need explicit handling
- Glob-like patterns are overkill; explicit matching is clearer

**Implications:**
- Keep current implementation but add comprehensive test coverage
- Document pattern matching rules in code comments
- Add test cases for edge cases identified in review

---

#### [D10] Include command context in shell error messages (DECIDED) {#d10-command-context-errors}

**Decision:** Error messages for shell command failures should include the full command string and exit code.

**Rationale:**
- Current errors only include stderr, which may be empty or unclear
- Command string helps users reproduce issues
- Exit code provides additional debugging context

**Implications:**
- Create helper function for running commands with context
- Format: "Command '<cmd>' failed with exit code <N>: <stderr>"
- Include command in SpecksError variant where appropriate

---

### 1.0.1 Specification {#specification}

#### 1.0.1.1 Inputs and Outputs (Data Model) {#inputs-outputs}

**Inputs:**
- Existing session.json files (schema version "1")
- PR branch names for merge status queries
- Worktree paths for cleanup operations

**Outputs:**
- Updated session.json files (atomically written)
- CLI output for reconcile command (JSON and text formats)
- Warning messages on stderr for non-fatal issues

**Key invariants:**
- Session files are never in a partially-written state
- Session status transitions are valid: Pending -> InProgress -> {Completed, Failed, NeedsReconcile}
- Cleanup never removes worktrees for PRs that are not merged

#### 1.0.1.2 Terminology and Naming {#terminology}

- **Squash merge**: GitHub merge strategy that combines branch commits into single commit
- **Atomic write**: File operation that either fully succeeds or fully fails, no partial state
- **NeedsReconcile**: Session state indicating commit succeeded but bead close failed

#### 1.0.1.3 Semantics (Normative Rules) {#semantics}

- **Merge detection order**: GitHub API first, fallback to git merge-base with warning
- **Atomic write sequence**: write tmp -> fsync -> rename -> delete tmp on error
- **Reconcile eligibility**: Only sessions in NeedsReconcile state can be reconciled

#### 1.0.1.4 Public API Surface {#public-api}

**Rust (specks-core):**
```rust
// New in session.rs
pub fn save_session_atomic(session: &Session, repo_root: &Path) -> Result<(), SpecksError>;

// New in worktree.rs
pub fn is_pr_merged(branch: &str) -> Result<bool, SpecksError>;

// Deduplicated timestamp
pub fn now_iso8601() -> String;  // Already public, used from worktree.rs
```

**CLI (specks):**
```bash
# New command
specks session reconcile <session-id> [--dry-run]

# Existing commands with improved behavior
specks merge <speck>       # Now validates main worktree
specks worktree cleanup    # Now uses GitHub API for squash detection
```

---

### 1.0.2 Symbol Inventory {#symbol-inventory}

#### 1.0.2.1 New files (if any) {#new-files}

| File | Purpose |
|------|---------|
| `crates/specks/src/commands/session.rs` | Session subcommand including reconcile |

#### 1.0.2.2 Symbols to add / modify {#symbols}

| Symbol | Kind | Location | Notes |
|--------|------|----------|-------|
| `save_session_atomic` | fn | `session.rs` | Atomic session write |
| `is_pr_merged` | fn | `worktree.rs` or `merge.rs` | GitHub API merge check |
| `run_command_with_context` | fn | `merge.rs` or new `util.rs` | Command execution with error context |
| `is_main_worktree` | fn | `merge.rs` | Validate current directory |
| `SessionSubcommand::Reconcile` | enum variant | `cli.rs` | CLI argument structure |
| `run_session` | fn | `commands/session.rs` | Session command handler |

---

### 1.0.3 Test Plan Concepts {#test-plan-concepts}

#### Test Categories {#test-categories}

| Category | Purpose | When to use |
|----------|---------|-------------|
| **Unit** | Test atomic write, merge detection logic | Core functions in isolation |
| **Integration** | Test full merge workflow with mocked gh | End-to-end operations |
| **Golden** | Verify JSON output format for reconcile | API stability |

---

### 1.0.4 Execution Steps {#execution-steps}

#### Step 0: Add atomic session write function {#step-0}

**Bead:** `specks-15g.1`

**Commit:** `feat(session): add atomic session file writes`

**References:** [D02] Atomic session writes via temp file + rename, (#semantics)

**Artifacts:**
- Modified `crates/specks-core/src/session.rs` with `save_session_atomic` function
- Updated `save_session` to use atomic write internally

**Tasks:**
- [ ] Implement `save_session_atomic()` in session.rs
- [ ] Write to `{session-path}.tmp` first
- [ ] Call fsync on file before rename
- [ ] Rename temp file to final path
- [ ] Clean up temp file on any error
- [ ] Update `save_session()` to call `save_session_atomic()`

**Tests:**
- [ ] Unit test: atomic write succeeds and file is valid JSON
- [ ] Unit test: temp file is cleaned up on write error
- [ ] Unit test: original file unchanged on rename failure (simulated)

**Checkpoint:**
- [ ] `cargo nextest run -p specks-core session`
- [ ] `cargo build -p specks-core`

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

#### Step 1: Add GitHub API merge detection {#step-1}

**Depends on:** #step-0

**Bead:** `specks-15g.2`

**Commit:** `feat(worktree): use GitHub API for squash merge detection`

**References:** [D01] Use GitHub API for squash merge detection, [D05] Use gh pr checks --json, (#public-api)

**Artifacts:**
- New `is_pr_merged()` function in worktree.rs or merge.rs
- Updated `cleanup_worktrees()` to use new detection

**Tasks:**
- [ ] Implement `is_pr_merged(branch: &str) -> Result<bool, SpecksError>`
- [ ] Query `gh pr view <branch> --json state,mergedAt`
- [ ] Return true if state is "MERGED"
- [ ] Handle "no PR found" as false (not merged)
- [ ] Fallback to git merge-base with warning on gh failure
- [ ] Update `cleanup_worktrees()` to use `is_pr_merged()` instead of `is_ancestor()`

**Tests:**
- [ ] Unit test: parse MERGED state correctly
- [ ] Unit test: parse OPEN state correctly
- [ ] Unit test: handle no PR found case
- [ ] Integration test: cleanup identifies squash-merged PR (requires mock or skip in CI)

**Checkpoint:**
- [ ] `cargo nextest run -p specks-core worktree`
- [ ] `cargo build`

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

#### Step 2: Add main worktree validation {#step-2}

**Depends on:** #step-1

**Bead:** `specks-15g.3`

**Commit:** `feat(merge): validate running from main worktree`

**References:** [D04] Validate main worktree before merge operations, (#inputs-outputs)

**Artifacts:**
- New `is_main_worktree()` function in merge.rs
- Updated `run_merge()` to validate early

**Tasks:**
- [ ] Implement `is_main_worktree() -> Result<bool, String>`
- [ ] Check if `.git` is a directory (file means worktree)
- [ ] Verify current branch matches expected (main/master)
- [ ] Add validation at start of `run_merge()`
- [ ] Provide clear error message with suggested `cd` command

**Tests:**
- [ ] Unit test: detect main worktree (directory .git)
- [ ] Unit test: detect specks worktree (file .git)
- [ ] Integration test: merge fails with clear message from worktree

**Checkpoint:**
- [ ] `cargo nextest run -p specks merge`
- [ ] `cargo build`

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

#### Step 3: Add race condition mitigation {#step-3}

**Depends on:** #step-2

**Bead:** `specks-15g.4`

**Commit:** `fix(merge): re-verify sync before push to reduce race window`

**References:** [D06] Race condition mitigation for check_main_sync, (#semantics)

**Artifacts:**
- Modified `run_merge()` with additional sync check

**Tasks:**
- [ ] Add second `check_main_sync()` call immediately before `git push`
- [ ] Add code comment documenting the race window limitation
- [ ] Ensure error message is clear if sync fails at this point

**Tests:**
- [ ] Unit test: verify double-check pattern in code flow
- [ ] Integration test: second check catches intervening push (hard to test, may skip)

**Checkpoint:**
- [ ] `cargo nextest run -p specks merge`
- [ ] `cargo build`

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

#### Step 4: Add session reconcile command {#step-4}

**Depends on:** #step-3

**Bead:** `specks-15g.5`

**Commit:** `feat(cli): add specks session reconcile command`

**References:** [D03] CLI command for NeedsReconcile recovery, (#public-api, #symbols)

**Artifacts:**
- New `crates/specks/src/commands/session.rs` file
- Updated `cli.rs` with Session subcommand
- Updated `main.rs` to dispatch to session command

**Tasks:**
- [ ] Add `SessionSubcommand` enum with `Reconcile` variant in cli.rs
- [ ] Create `commands/session.rs` with `run_session()` function
- [ ] Implement reconcile logic: find session, verify NeedsReconcile status, close pending bead
- [ ] Support `--dry-run` flag to preview actions
- [ ] Output JSON when `--json` flag is set

**Tests:**
- [ ] Unit test: reconcile changes status from NeedsReconcile to Completed
- [ ] Unit test: reconcile fails for non-NeedsReconcile session
- [ ] Unit test: dry-run does not modify session
- [ ] Golden test: JSON output format

**Checkpoint:**
- [ ] `cargo nextest run -p specks session`
- [ ] `specks session reconcile --help` works
- [ ] `cargo build`

**Rollback:**
- Revert commit, delete session.rs

**Commit after all checkpoints pass.**

---

#### Step 5: Improve error context and minor fixes {#step-5}

**Depends on:** #step-4

**Bead:** `specks-15g.6`

**Commit:** `fix(merge): improve error messages and rollback handling`

**References:** [D08] Log warnings instead of silently ignoring rollback errors, [D10] Include command context in shell error messages, (#terminology)

**Artifacts:**
- Modified `merge.rs` with improved error handling
- Modified rollback code with warning output

**Tasks:**
- [ ] Create helper function `run_command_with_context()` that includes command string and exit code in errors
- [ ] Update git/gh command calls to use helper
- [ ] Add `eprintln!` warnings for rollback failures instead of silent `let _ =`
- [ ] Improve error messages with actionable suggestions

**Tests:**
- [ ] Unit test: error message includes command string
- [ ] Unit test: error message includes exit code
- [ ] Unit test: rollback warning is printed (capture stderr)

**Checkpoint:**
- [ ] `cargo nextest run -p specks`
- [ ] `cargo build`

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

#### Step 6: Use gh pr checks --json format {#step-6}

**Depends on:** #step-5

**Bead:** `specks-15g.7`

**Commit:** `fix(merge): use JSON output for gh pr checks parsing`

**References:** [D05] Use gh pr checks --json for robust parsing, (#public-api)

**Artifacts:**
- Modified `check_pr_checks()` function in merge.rs

**Tasks:**
- [ ] Change command to `gh pr checks <branch> --json name,state,conclusion`
- [ ] Parse JSON array response
- [ ] Update pending/failing check detection logic
- [ ] Handle empty array as success (no checks configured)

**Tests:**
- [ ] Unit test: parse successful checks JSON
- [ ] Unit test: parse failing checks JSON
- [ ] Unit test: parse pending checks JSON
- [ ] Unit test: empty array means success

**Checkpoint:**
- [ ] `cargo nextest run -p specks merge`
- [ ] `cargo build`

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

#### Step 7: Deduplicate timestamp code {#step-7}

**Depends on:** #step-6

**Bead:** `specks-15g.8`

**Commit:** `refactor(core): deduplicate timestamp generation`

**References:** [D07] Expose now_iso8601 as shared utility, (#symbols)

**Artifacts:**
- Modified `worktree.rs` to use `session::now_iso8601()`
- Removed duplicate `generate_timestamp_utc()` function

**Tasks:**
- [ ] Import `now_iso8601` from session module in worktree.rs
- [ ] Create format conversion if needed (ISO8601 to compact YYYYMMDD-HHMMSS)
- [ ] Remove `generate_timestamp_utc()` and helper functions from worktree.rs
- [ ] Update `generate_branch_name()` to use new shared function

**Tests:**
- [ ] Unit test: branch name timestamp format unchanged
- [ ] Existing tests pass without modification

**Checkpoint:**
- [ ] `cargo nextest run -p specks-core`
- [ ] `cargo build`

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

### 1.0.5 Deliverables and Checkpoints {#deliverables}

**Deliverable:** Robust merge and worktree cleanup that handles squash merges, atomic session writes, and provides recovery for failed sessions.

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

- [ ] `specks worktree cleanup --merged` works with squash-merged PRs (verified via test)
- [ ] Session files never partially written (atomic write test)
- [ ] `specks session reconcile` command exists and works
- [ ] All 12 issues from architectural review addressed
- [ ] All tests pass: `cargo nextest run`

**Acceptance tests:**
- [ ] Integration test: cleanup identifies squash-merged PR
- [ ] Unit test: atomic write does not corrupt on simulated failure
- [ ] Integration test: reconcile recovers NeedsReconcile session
- [ ] Unit test: merge validates main worktree

#### Roadmap / Follow-ons (Explicitly Not Required for Phase Close) {#roadmap}

- [ ] Add retry logic for transient gh API failures
- [ ] Consider file locking for multi-process session access
- [ ] Add `specks session list` command for visibility

| Checkpoint | Verification |
|------------|--------------|
| All tests pass | `cargo nextest run` |
| No warnings | `cargo build 2>&1 \| grep -c warning` returns 0 |
| Merge works | Manual test of `specks merge` with squash PR |

**Commit after all checkpoints pass.**
