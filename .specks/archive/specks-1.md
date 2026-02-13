## Phase 1.0: Local Merge Support for `specks merge` {#phase-local-merge}

**Purpose:** Enable `specks merge` to perform a local `git merge --squash` when no GitHub remote is configured, so ad-hoc local projects without GitHub remotes can use the full specks workflow end-to-end.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | kocienda |
| Status | draft |
| Target branch | main |
| Tracking issue/PR | N/A |
| Last updated | 2026-02-10 |
| Beads Root | `specks-w87` |

---

### Phase Overview {#phase-overview}

#### Context {#context}

The `specks merge` command currently requires a GitHub remote and uses `gh pr merge --squash` to complete the merge workflow. This means projects without a GitHub remote (local-only repos, air-gapped environments, early prototypes) cannot use `specks merge` at all -- they hit errors at the "check main sync" and "get PR for branch" steps. Since the implementer skill creates worktrees and branches regardless of remote status, the merge command is the missing piece that blocks the full local workflow.

#### Strategy {#strategy}

- Detect whether `origin` remote exists early in the merge flow using `git remote get-url origin`
- Branch the `run_merge` function into "remote" vs "local" paths after worktree discovery
- In local mode, skip all origin/main sync checks, PR lookups, and PR check validations
- Perform `git merge --squash` + `git commit` directly on main instead of `gh pr merge`
- Pre-validate that the branch has commits ahead of main to catch empty merges
- Reuse all existing infrastructure file handling and worktree cleanup unchanged
- Add three new fields to `MergeData` for local mode JSON output
- Update the merge skill to detect and display local merge results

#### Stakeholders / Primary Customers {#stakeholders}

1. Developers using specks on local-only repositories without GitHub remotes
2. The merge skill (consumer of `MergeData` JSON output)

#### Success Criteria (Measurable) {#success-criteria}

- `specks merge` succeeds on a repo with no `origin` remote and produces a squash-merged commit on main (`git log --oneline -1` shows the merged commit)
- `specks merge --dry-run` on a local repo returns JSON with `merge_mode: "local"` and `would_squash_merge` containing the branch name
- `specks merge` on a repo with `origin` remote continues to use the PR-based flow unchanged (no regression)
- Merge conflicts are caught, main is restored via `git reset --merge`, and a clear error is returned
- Empty merges (branch already in main) produce an error before attempting the merge

#### Scope {#scope}

1. `has_remote_origin()` detection helper in `merge.rs`
2. `squash_merge_branch()` helper for local `git merge --squash` + commit
3. Pre-merge empty-branch check via `git rev-list`
4. Branched flow in `run_merge()` for local vs remote mode
5. Three new fields on `MergeData` struct: `merge_mode`, `squash_commit`, `would_squash_merge`
6. Updated CLI help text for the `Merge` command
7. Updated merge skill (`skills/merge/SKILL.md`) to handle local mode output
8. Unit and integration tests for all new code paths

#### Non-goals (Explicitly out of scope) {#non-goals}

- No new CLI flags (local mode is auto-detected, not opted into)
- No changes to `specks-core` library crate
- No changes to the committer agent or implementer workflow
- No support for non-main base branches in local mode
- No remote fallback -- if `origin` exists but `gh` is unavailable, that is an error

#### Dependencies / Prerequisites {#dependencies}

- Existing `run_merge()` function in `crates/specks/src/commands/merge.rs`
- Existing `MergeData` struct with `skip_serializing_if` pattern
- Git available on PATH (already required)

#### Constraints {#constraints}

- Project enforces `-D warnings` via `.cargo/config.toml`; no dead code or unused imports
- `MergeData` new fields must use `skip_serializing_if` to keep JSON output clean
- Local merge conflict recovery must use `git reset --merge` (not `git merge --abort`, because `--squash` does not create `MERGE_HEAD`)

#### Assumptions {#assumptions}

- `git remote get-url origin` returning a non-zero exit code reliably indicates no remote named `origin`
- The local merge commit message should follow the same format as squash PR merges for consistency
- Local mode skips all origin/main sync checks (no push, no pull, no fetch)
- If `origin` remote exists but `gh` CLI is missing, the command errors rather than falling back to local mode

---

### 1.0.0 Design Decisions {#design-decisions}

#### [D01] Auto-detect local mode via `git remote get-url origin` (DECIDED) {#d01-detect-via-remote}

**Decision:** Use `git remote get-url origin` to determine merge mode. If the command fails (exit code non-zero), use local mode. If it succeeds, use remote (PR-based) mode.

**Rationale:**
- Simple, reliable detection with no configuration needed
- `git remote get-url` is universally available in git versions specks targets
- No ambiguity -- either the remote exists or it does not

**Implications:**
- If a user has `origin` configured but pointing to an unreachable server, the remote flow runs and will fail at a later step (PR lookup). This is correct behavior -- the user intended to use GitHub.
- The `has_remote_origin()` helper returns a plain `bool`, keeping the API simple.

#### [D02] Error when origin exists but `gh` CLI unavailable (DECIDED) {#d02-error-no-gh}

**Decision:** If `origin` remote exists but `gh` CLI is not found, return an error with a clear message rather than falling back to local mode.

**Rationale:**
- Having `origin` configured signals intent to use the remote workflow
- Silent fallback to local mode could cause confusing behavior (user expects PR merge, gets local merge)
- Explicit errors are safer than implicit fallbacks

**Implications:**
- The `gh` availability check (already in `get_pr_for_branch`) naturally produces this error in the remote path
- No additional code needed for this behavior -- it falls out of the existing flow

#### [D03] Dry-run returns branch name in `would_squash_merge` (DECIDED) {#d03-dry-run-branch-name}

**Decision:** In local dry-run mode, `would_squash_merge` contains just the branch name string (not a URL or composite object).

**Rationale:**
- Mirrors the simplicity of `would_merge_pr` (which contains a URL string)
- Branch name is the only meaningful identifier in local mode (there is no PR URL)
- The skill can display it directly without parsing

**Implications:**
- `would_squash_merge` is `Option<String>` on `MergeData`, skipped when `None`
- `would_merge_pr` remains `None` in local mode; `would_squash_merge` remains `None` in remote mode

#### [D04] Abort and reset on merge conflicts (DECIDED) {#d04-conflict-abort}

**Decision:** On merge conflict, run `git reset --merge` to restore a clean main branch state, then return an error suggesting the user rebase the branch.

**Rationale:**
- `git merge --squash` does not create `MERGE_HEAD`, so `git merge --abort` does not work
- `git reset --merge` reliably restores the working tree to pre-merge state
- Automatic conflict resolution is out of scope and would be fragile

**Implications:**
- The error message must mention `git reset --merge` was already performed and suggest `git rebase main` on the worktree branch
- The `MergeData.error` field carries the conflict information

#### [D05] Pre-check for empty merge via `git rev-list` (DECIDED) {#d05-empty-merge-check}

**Decision:** Before attempting the squash merge, check `git rev-list --count main..<branch>`. If the count is zero, return an error immediately rather than attempting a no-op merge.

**Rationale:**
- An empty `git merge --squash` followed by `git commit` would fail with "nothing to commit" which is a confusing error
- Pre-checking gives a clear, specific error: "Branch has no commits ahead of main"
- Matches the user's stated preference for "error if empty"

**Implications:**
- This check runs only in local mode (remote mode relies on PR state validation)
- The check uses the same `run_command_with_context` helper for consistent error formatting

#### [D06] Three new optional fields on MergeData (DECIDED) {#d06-mergedata-fields}

**Decision:** Add `merge_mode`, `squash_commit`, and `would_squash_merge` as `Option<String>` fields with `skip_serializing_if = "Option::is_none"` to `MergeData`.

**Rationale:**
- `merge_mode` ("remote" or "local") lets consumers branch their display logic
- `squash_commit` holds the commit hash from a successful local squash merge (analogous to PR URL in remote mode)
- `would_squash_merge` holds the branch name in dry-run local mode (analogous to `would_merge_pr`)
- All optional with skip-serializing to maintain backward compatibility -- existing JSON consumers see no new fields unless in local mode

**Implications:**
- Remote mode sets `merge_mode: Some("remote")` and the existing PR fields
- Local mode sets `merge_mode: Some("local")` and the new local fields
- The merge skill reads `merge_mode` to decide how to display results

---

### 1.0.1 Specification {#specification}

#### 1.0.1.1 Merge Mode Detection Flow {#detection-flow}

**Spec S01: Remote Detection** {#s01-remote-detection}

```
has_remote_origin() -> bool:
  1. Run: git remote get-url origin
  2. If exit code == 0: return true (remote exists)
  3. If exit code != 0: return false (no remote)
```

The function does not validate the URL or test connectivity. It only checks whether the remote is configured.

#### 1.0.1.2 Local Merge Flow {#local-merge-flow}

**Spec S02: Local Merge Sequence** {#s02-local-merge-sequence}

The local merge path replaces steps 2-5 and modifies steps 9-11 of the existing `run_merge` flow:

| Existing Step | Remote Mode | Local Mode |
|---------------|-------------|------------|
| Step 0: Validate main worktree | Same | Same |
| Step 1: Find worktree | Same | Same |
| Step 1.5: Detect mode | N/A (new) | `has_remote_origin()` |
| Step 2: Check main sync | `git rev-list origin/main..main` | SKIP |
| Step 3: Get PR info | `gh pr view` | SKIP |
| Step 4: Validate PR state | `validate_pr_state()` | SKIP |
| Step 5: Check PR checks | `gh pr checks` | SKIP |
| Step 5.5: Empty merge check | N/A | `git rev-list --count main..<branch>` |
| Step 6: Categorize uncommitted | Same | Same |
| Step 7: Validate non-infra files | Same | Same |
| Dry-run return | `would_merge_pr` | `would_squash_merge` |
| Step 8: Commit infrastructure | Same | Same |
| Step 9: Push main | `git push origin main` | SKIP |
| Step 10: Merge | `gh pr merge --squash` | `git merge --squash` + `git commit` |
| Step 11: Pull main | `git pull origin main` | SKIP |
| Step 12: Cleanup worktree | Same | Same |

#### 1.0.1.3 Squash Merge Helper {#squash-merge-helper}

**Spec S03: squash_merge_branch Function** {#s03-squash-merge-branch}

```
squash_merge_branch(branch: &str, message: &str) -> Result<String, String>:
  1. Run: git merge --squash <branch>
  2. If exit code != 0:
     a. Run: git reset --merge
     b. Return Err("Merge conflict ... git reset --merge performed ... suggest rebase")
  3. Run: git commit -m <message>
  4. If exit code != 0: Return Err("Failed to commit squash merge: ...")
  5. Run: git rev-parse HEAD
  6. Return Ok(commit_hash)
```

The commit message follows the same format as infrastructure commits: `chore(<speck-name>): squash merge <branch>`.

#### 1.0.1.4 Error Scenarios {#error-scenarios}

**Table T01: Local Mode Error Cases** {#t01-local-errors}

| Scenario | Detection | Error Message | Recovery |
|----------|-----------|---------------|----------|
| Empty merge | `git rev-list --count main..<branch>` returns 0 | "Branch '<branch>' has no commits ahead of main. Nothing to merge." | Verify correct branch, or check if already merged |
| Merge conflict | `git merge --squash` fails | "Merge conflict merging '<branch>'. Working tree restored via git reset --merge. Rebase your branch: cd <worktree> && git rebase main" | Rebase branch in worktree, then retry |
| Commit failure | `git commit` fails after squash | "Failed to commit squash merge: <stderr>" | Investigate git state manually |
| Branch not found | `git merge --squash` fails with "not something we can merge" | Passed through from git stderr | Check branch name in session |

---

### 1.0.2 Symbol Inventory {#symbol-inventory}

#### 1.0.2.1 Modified Files {#modified-files}

| File | Changes |
|------|---------|
| `crates/specks/src/commands/merge.rs` | Add `has_remote_origin()`, `squash_merge_branch()`, modify `MergeData`, modify `run_merge()` |
| `crates/specks/src/cli.rs` | Update `long_about` text for Merge command |
| `skills/merge/SKILL.md` | Add local mode handling in dry-run preview, confirmation, and results |

#### 1.0.2.2 Symbols to Add / Modify {#symbols}

| Symbol | Kind | Location | Notes |
|--------|------|----------|-------|
| `has_remote_origin()` | fn | `commands/merge.rs` | Returns `bool`, checks `git remote get-url origin` |
| `squash_merge_branch()` | fn | `commands/merge.rs` | Takes `branch: &str, message: &str`, returns `Result<String, String>` (commit hash) |
| `MergeData.merge_mode` | field | `commands/merge.rs` | `Option<String>`, skip_serializing_if None |
| `MergeData.squash_commit` | field | `commands/merge.rs` | `Option<String>`, skip_serializing_if None |
| `MergeData.would_squash_merge` | field | `commands/merge.rs` | `Option<String>`, skip_serializing_if None |

---

### 1.0.3 Test Plan Concepts {#test-plan-concepts}

#### Test Categories {#test-categories}

| Category | Purpose | When to use |
|----------|---------|-------------|
| **Unit** | Test `has_remote_origin()`, `squash_merge_branch()`, `MergeData` serialization | Isolated function behavior |
| **Integration** | Test full local merge flow in temporary git repos | End-to-end merge paths |

#### Key Test Scenarios {#key-test-scenarios}

1. `has_remote_origin()` returns true when origin is configured
2. `has_remote_origin()` returns false when no origin exists
3. `MergeData` serialization with local mode fields (merge_mode, squash_commit)
4. `MergeData` serialization with local dry-run fields (would_squash_merge)
5. `MergeData` serialization omits local fields in remote mode (backward compat)
6. Empty merge detection (branch with no commits ahead)
7. Successful local squash merge in temp git repo
8. Merge conflict detection and `git reset --merge` recovery

---

### 1.0.4 Execution Steps {#execution-steps}

#### Step 0: Add `has_remote_origin()` helper and extend `MergeData` {#step-0}

**Bead:** `specks-w87.1`

**Commit:** `feat(merge): add has_remote_origin detection and MergeData local fields`

**References:** [D01] Auto-detect local mode via git remote get-url origin, [D06] Three new optional fields on MergeData, Spec S01, (#detection-flow, #symbols)

**Artifacts:**
- `has_remote_origin()` function in `commands/merge.rs`
- Three new fields on `MergeData`: `merge_mode`, `squash_commit`, `would_squash_merge`

**Tasks:**
- [ ] Add `has_remote_origin()` function that runs `git remote get-url origin` and returns `bool`
- [ ] Add `merge_mode: Option<String>` field to `MergeData` with `#[serde(skip_serializing_if = "Option::is_none")]`
- [ ] Add `squash_commit: Option<String>` field to `MergeData` with `#[serde(skip_serializing_if = "Option::is_none")]`
- [ ] Add `would_squash_merge: Option<String>` field to `MergeData` with `#[serde(skip_serializing_if = "Option::is_none")]`
- [ ] Initialize all three new fields to `None` in every existing `MergeData` construction site in `run_merge()`

**Tests:**
- [ ] Unit test: `has_remote_origin()` returns true in a temp repo with origin configured
- [ ] Unit test: `has_remote_origin()` returns false in a temp repo with no remotes
- [ ] Unit test: `MergeData` serialization with `merge_mode: Some("local")` includes the field
- [ ] Unit test: `MergeData` serialization with `merge_mode: None` omits the field
- [ ] Unit test: `MergeData` serialization with all three local fields populated
- [ ] Unit test: `MergeData` serialization backward compat -- remote mode omits local-only fields

**Checkpoint:**
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo nextest run` -- all existing tests pass, new tests pass

**Rollback:**
- Revert commit; no schema or data changes outside this file

**Commit after all checkpoints pass.**

---

#### Step 1: Add `squash_merge_branch()` helper with conflict recovery {#step-1}

**Depends on:** #step-0

**Bead:** `specks-w87.2`

**Commit:** `feat(merge): add squash_merge_branch helper with conflict recovery`

**References:** [D04] Abort and reset on merge conflicts, [D05] Pre-check for empty merge via git rev-list, Spec S03, Table T01, (#squash-merge-helper, #error-scenarios)

**Artifacts:**
- `squash_merge_branch()` function in `commands/merge.rs`

**Tasks:**
- [ ] Add `squash_merge_branch(branch: &str, message: &str) -> Result<String, String>` function
- [ ] Implement `git merge --squash <branch>` using `run_command_with_context`
- [ ] On merge failure, run `git reset --merge` and return error with recovery instructions
- [ ] On success, run `git commit -m <message>` and capture commit hash via `git rev-parse HEAD`
- [ ] Return the commit hash string on success

**Tests:**
- [ ] Integration test: successful squash merge in temp repo with diverged branch
- [ ] Integration test: merge conflict triggers `git reset --merge` and returns error
- [ ] Integration test: commit message appears in `git log` after successful merge

**Checkpoint:**
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo nextest run` -- all tests pass

**Rollback:**
- Revert commit; function is self-contained with no callers yet

**Commit after all checkpoints pass.**

---

#### Step 2: Wire local mode into `run_merge()` flow {#step-2}

**Depends on:** #step-1

**Bead:** `specks-w87.3`

**Commit:** `feat(merge): wire local merge mode into run_merge flow`

**References:** [D01] Auto-detect local mode, [D02] Error when origin exists but gh unavailable, [D03] Dry-run returns branch name, [D05] Pre-check for empty merge, [D06] Three new optional fields on MergeData, Spec S02, Table T01, (#local-merge-flow, #detection-flow, #error-scenarios)

**Artifacts:**
- Modified `run_merge()` function with branched local/remote flow
- Empty merge pre-check before squash merge attempt

**Tasks:**
- [ ] After Step 1 (find worktree), call `has_remote_origin()` to determine merge mode
- [ ] If local mode: skip Steps 2-5 (main sync check, PR lookup, PR validation, PR checks)
- [ ] If local mode: add empty merge check via `git rev-list --count main..<branch>`, error if zero
- [ ] If local mode dry-run: populate `merge_mode: Some("local")`, `would_squash_merge: Some(branch_name)`, return early
- [ ] If remote mode dry-run: populate `merge_mode: Some("remote")` alongside existing `would_merge_pr`
- [ ] If local mode: skip Step 9 (push) and Step 11 (pull)
- [ ] If local mode Step 10: call `squash_merge_branch()` instead of `gh pr merge --squash`
- [ ] If local mode: populate `merge_mode: Some("local")` and `squash_commit: Some(hash)` in success response
- [ ] If remote mode: populate `merge_mode: Some("remote")` in success response
- [ ] Remove `#[allow(dead_code)]` from `categorize_uncommitted` and `is_infrastructure_file` if they were only needed for this step (verify they are now called)

**Tests:**
- [ ] Integration test: full local merge workflow in temp repo (create branch, add commits, merge, verify squashed commit on main)
- [ ] Integration test: local dry-run returns correct JSON with `merge_mode` and `would_squash_merge`
- [ ] Integration test: empty branch in local mode returns error before merge attempt
- [ ] Unit test: verify `merge_mode: "remote"` appears in remote dry-run JSON

**Checkpoint:**
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo nextest run` -- all tests pass
- [ ] Manual test: `specks merge --dry-run` in a local repo shows local mode output

**Rollback:**
- Revert commit; Steps 0 and 1 remain valid (helpers exist but are unused by run_merge)

**Commit after all checkpoints pass.**

---

#### Step 3: Update CLI help text and merge skill {#step-3}

**Depends on:** #step-2

**Bead:** `specks-w87.4`

**Commit:** `docs(merge): update CLI help and merge skill for local mode`

**References:** [D03] Dry-run returns branch name, [D06] Three new optional fields on MergeData, (#specification, #local-merge-flow)

**Artifacts:**
- Updated `long_about` text for Merge command in `cli.rs`
- Updated `skills/merge/SKILL.md` with local mode handling

**Tasks:**
- [ ] Update `long_about` for the Merge command in `cli.rs` to document both remote and local modes
- [ ] In `skills/merge/SKILL.md`, update dry-run preview section to detect `merge_mode` and show appropriate text ("would squash-merge branch X" for local vs "would merge PR #N" for remote)
- [ ] In `skills/merge/SKILL.md`, update confirmation prompt to adapt text for local mode
- [ ] In `skills/merge/SKILL.md`, update results reporting to show `squash_commit` hash for local mode instead of PR URL
- [ ] In `skills/merge/SKILL.md`, add merge conflict as a documented error case with recovery path

**Tests:**
- [ ] Unit test: CLI parses `merge` command and help text renders without error
- [ ] Manual review: skill file covers both modes in each section

**Checkpoint:**
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo nextest run` -- all tests pass
- [ ] `specks merge --help` shows updated help text mentioning local mode

**Rollback:**
- Revert commit; only documentation/help text changes

**Commit after all checkpoints pass.**

---

### 1.0.5 Deliverables and Checkpoints {#deliverables}

**Deliverable:** `specks merge` supports local squash merge when no GitHub remote is configured, enabling the full specks workflow on local-only repositories.

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

- [ ] `specks merge <speck>` succeeds on a repo with no origin remote (integration test)
- [ ] `specks merge --dry-run <speck>` returns JSON with `merge_mode: "local"` and `would_squash_merge` (integration test)
- [ ] `specks merge <speck>` on a repo with origin remote continues to use PR flow (no regression; existing tests pass)
- [ ] Merge conflicts are caught and main is restored cleanly (integration test)
- [ ] Empty merges produce a clear error (integration test)
- [ ] `cargo build` produces zero warnings
- [ ] `cargo nextest run` passes all tests

**Acceptance tests:**
- [ ] Integration test: full local merge lifecycle (create worktree, add commits, merge, verify main)
- [ ] Integration test: local dry-run JSON output shape
- [ ] Integration test: conflict recovery restores clean state
- [ ] Integration test: empty merge error
- [ ] Unit test: `MergeData` serialization for all mode combinations

#### Roadmap / Follow-ons (Explicitly Not Required for Phase Close) {#roadmap}

- [ ] Support non-main base branches for local merge
- [ ] Add `--local` / `--remote` flags to force merge mode regardless of remote detection
- [ ] Integration with `specks doctor` to report local vs remote merge capability

| Checkpoint | Verification |
|------------|--------------|
| Local merge works | `cargo nextest run` integration tests pass |
| Remote merge unchanged | Existing merge tests continue to pass |
| No warnings | `cargo build` exits 0 |
| Help text updated | `specks merge --help` mentions local mode |

**Commit after all checkpoints pass.**