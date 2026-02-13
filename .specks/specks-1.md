## Phase 1.0: Merge Preflight Checks {#phase-merge-preflight}

**Purpose:** Add comprehensive preflight validation to `specks merge` so that dirty worktrees, missing speck files, incomplete beads, and mode-detection surprises are caught and reported clearly before any destructive merge operations begin.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | kocienda |
| Status | draft |
| Target branch | main |
| Tracking issue/PR | TBD |
| Last updated | 2026-02-12 |
| Beads Root | `specks-dg0` |

---

### Phase Overview {#phase-overview}

#### Context {#context}

The `specks merge` command currently moves directly from worktree discovery to merge execution with minimal validation. Several gaps have been identified through an audit: dirty files in the implementation worktree are silently destroyed during cleanup, speck file typos produce confusing "No worktree found" errors, incomplete bead/step status is not surfaced, and mode-detection fallbacks happen without explanation. These gaps undermine user trust and can cause data loss.

The existing merge flow already handles dirty files on the main worktree and partitions infrastructure vs non-infrastructure files. This phase extends that pattern to the implementation worktree and adds a structured preflight phase between worktree discovery and merge execution.

#### Strategy {#strategy}

- Add a `warnings` field to `MergeData` so all non-blocking findings can be reported in both JSON and text output
- Group all new checks into a single `run_preflight_checks()` function inserted between worktree discovery (Step 1) and the existing pre-dry-run checks (Step 2)
- P0 check (worktree dirty) blocks merge; P1/P2/P3 checks warn but do not block
- Expose worktree match count from `find_worktree_by_speck` so merge.rs can warn about multiples
- Update SKILL.md dry-run field table to document the new `warnings` field
- Follow existing patterns: `get_dirty_files()` for porcelain parsing, `run_cmd()` for command execution, `is_infrastructure_path()` for file classification

#### Stakeholders / Primary Customers {#stakeholders}

1. Developers using `/specks:merge` to complete the implementation workflow
2. The merge skill (SKILL.md) which parses MergeData JSON output

#### Success Criteria (Measurable) {#success-criteria}

- Running `specks merge` on a worktree with dirty implementation files blocks with a clear error listing the files (P0 verified by integration test)
- Running `specks merge` with a typo in the speck path produces "Speck file not found: .specks/specks-typo.md" instead of "No worktree found" (P1 verified by unit test)
- Running `specks merge --dry-run --json` includes a `warnings` array in the output when preflight checks find non-blocking issues (verified by serialization tests)
- `cargo nextest run` passes with zero warnings
- `specks validate .specks/specks-1.md` passes

#### Scope {#scope}

1. P0: Implementation worktree dirty-file check (blocker)
2. P1: Speck file existence validation after `normalize_speck_path()`
3. P1: Bead/step completion status warning
4. P2: Branch divergence preview in dry-run
5. P2: Infrastructure diff preview in dry-run
6. P2: Push failure messaging with explicit instructions
7. P3: gh CLI fallback explanation when silently switching to local mode
8. P3: Multiple worktree warning
9. P3: PR checks status in dry-run for remote mode
10. `warnings: Option<Vec<String>>` field on `MergeData`
11. SKILL.md documentation update

#### Non-goals (Explicitly out of scope) {#non-goals}

- Auto-fixing dirty worktree files (the check blocks; the user decides what to do)
- Changing the merge execution logic itself (squash merge, PR merge, conflict resolution)
- Adding new CLI flags or subcommands
- Changing the worktree creation or beads sync workflows

#### Dependencies / Prerequisites {#dependencies}

- Existing `merge.rs` command infrastructure (MergeData, run_cmd, get_dirty_files, etc.)
- Existing `find_worktree_by_speck` in worktree.rs
- Existing `specks beads status` command and BeadsCli infrastructure

#### Constraints {#constraints}

- `-D warnings` enforced: all new code must compile without warnings
- No `std::env::set_current_dir` in tests (thread-safety constraint)
- Existing MergeData JSON output must remain backwards-compatible (new fields are additive/optional)

#### Assumptions {#assumptions}

- The `MergeData` struct will get a `warnings: Option<Vec<String>>` field with `skip_serializing_if = "Option::is_none"`
- All new checks are grouped in a preflight function called between worktree discovery and dry-run
- P0 worktree dirty check uses `git -C <worktree_path> status --porcelain`
- P1 speck file existence is checked immediately after `normalize_speck_path()`
- P1 bead status check reuses existing beads infrastructure (parse_speck, BeadsCli)
- P2 infrastructure diff reuses `is_infrastructure_path()` helper
- P3 gh fallback message goes into warnings (not blocking)
- P3 multiple worktree warning does not block merge
- Tests follow existing merge.rs test patterns (tempfile, init_git_repo, make_initial_commit)

---

### 1.0.0 Design Decisions {#design-decisions}

#### [D01] Preflight checks are a single function called before dry-run output (DECIDED) {#d01-preflight-function}

**Decision:** All preflight checks are grouped in a `run_preflight_checks()` function that returns a `PreflightResult` struct containing a list of warnings and an optional blocking error.

**Rationale:**
- Single insertion point in `run_merge()` keeps the diff minimal
- Testable in isolation without needing full merge infrastructure
- Clear separation between "should we proceed?" and "do the merge"

**Implications:**
- The function is called after worktree discovery (needs worktree path and branch) but before mode detection dirty-file checks
- Blocking errors short-circuit with `MergeData::error()`; warnings accumulate into `MergeData.warnings`

#### [D02] Warnings field is `Option<Vec<String>>` on MergeData (DECIDED) {#d02-warnings-field}

**Decision:** Add `warnings: Option<Vec<String>>` to `MergeData` with `skip_serializing_if = "Option::is_none"` for backwards compatibility.

**Rationale:**
- Simple, flat structure that is easy for SKILL.md to parse
- `Option` means the field is omitted entirely when there are no warnings, preserving existing JSON shape
- `Vec<String>` is sufficient; structured warning objects are over-engineering for this use case

**Implications:**
- Existing JSON consumers see no change when there are no warnings
- The merge skill needs a documentation update to mention the new field

#### [D03] P0 worktree dirty check inspects the implementation worktree only (DECIDED) {#d03-worktree-dirty-check}

**Decision:** Before merge, run `git -C <worktree_path> status --porcelain` on the implementation worktree. If any files are dirty, block merge with an error listing them.

**Rationale:**
- Dirty files in the implementation worktree would be silently destroyed during `git worktree remove`
- The main worktree dirty check already exists (Step 2b in current code)
- Checking only the implementation worktree avoids duplicating the main worktree check

**Implications:**
- Reuses the existing `get_dirty_files()` helper, passing the worktree path instead of repo_root
- The error message lists dirty files so the user knows what to commit or discard

#### [D04] Speck file existence check happens before worktree discovery (DECIDED) {#d04-speck-existence-check}

**Decision:** After `normalize_speck_path()`, check `repo_root.join(&speck_path).exists()`. If not found, return a clear "Speck file not found" error before attempting worktree discovery.

**Rationale:**
- A typo in the speck path currently falls through to `find_worktree_by_speck` which returns "No worktree found" — confusing because the problem is the file, not the worktree
- Checking existence first gives a precise, actionable error

**Implications:**
- This check is inserted between `normalize_speck_path()` and `find_worktree_by_speck()` in `run_merge()`
- Not part of the preflight function since it happens earlier in the flow

#### [D05] Bead completion check warns but does not block (DECIDED) {#d05-bead-completion-warning}

**Decision:** Check bead status and report "N of M steps incomplete. Run `specks beads status <speck>` to review." as a warning.

**Rationale:**
- Users may intentionally merge partial work (e.g., deferring steps to a follow-up)
- Blocking on incomplete beads would break legitimate workflows
- A warning surfaces the information without imposing a policy

**Implications:**
- Reuses `parse_speck()` to get step count and `BeadsCli` to check bead status
- If beads are not configured or not available, skip the check silently (no warning about missing beads tooling)

#### [D06] find_worktree_by_speck returns match count alongside the selected worktree (DECIDED) {#d06-worktree-match-count}

**Decision:** Modify `find_worktree_by_speck` to return a `WorktreeDiscovery` struct containing the selected worktree and total match count, instead of just `Option<DiscoveredWorktree>`.

**Rationale:**
- The current function silently picks the most recent match when multiples exist
- Surfacing the count lets merge.rs emit a warning listing alternatives
- Returning a struct is backwards-compatible if callers destructure the selected worktree

**Implications:**
- The return type changes from `Result<Option<DiscoveredWorktree>, SpecksError>` to `Result<WorktreeDiscovery, SpecksError>`
- `WorktreeDiscovery` contains `selected: Option<DiscoveredWorktree>`, `all_matches: Vec<DiscoveredWorktree>`, and `match_count: usize`
- All existing callers of `find_worktree_by_speck` must be updated

#### [D07] Branch divergence uses merge-base and rev-list count (DECIDED) {#d07-branch-divergence}

**Decision:** In dry-run mode, compute divergence via `git merge-base main <branch>` and `git rev-list --count <merge-base>..<branch>`, then include commit count and `git diff --stat` summary in warnings.

**Rationale:**
- Gives the user a quick overview of what the merge will include
- Uses standard git primitives that work in both local and remote mode

**Implications:**
- Only computed in dry-run mode to avoid slowing down actual merges
- Divergence info goes into dry-run text output and warnings array

#### [D08] gh CLI fallback message is a warning, not an error (DECIDED) {#d08-gh-fallback-warning}

**Decision:** When `has_remote_origin` is true but `gh pr view` fails (gh not installed or not authenticated), add a warning: "Remote detected but gh CLI unavailable -- falling back to local mode" instead of silently switching.

**Rationale:**
- Silent mode fallback is confusing; users may not realize they are doing a local merge when they expected a remote PR merge
- A warning surfaces the situation without blocking

**Implications:**
- The warning is generated during mode detection (Step 1a/1b area of current code)
- Does not change the fallback behavior, only adds visibility

---

### 1.0.1 Specification {#specification}

#### 1.0.1.1 Inputs and Outputs {#inputs-outputs}

**Inputs:**
- Speck path (string, normalized by `normalize_speck_path()`)
- `--dry-run` flag (boolean)
- `--json` flag (boolean)
- `--quiet` flag (boolean)

**Outputs:**
- `MergeData` JSON (extended with `warnings` field)
- Text output to stdout/stderr

**Key invariants:**
- A P0 preflight failure always blocks the merge and returns `status: "error"`
- Warnings never block the merge; they appear in both dry-run and actual merge output
- The `warnings` field is omitted from JSON when empty (backwards-compatible)

#### 1.0.1.2 Terminology {#terminology}

- **Preflight check**: A validation that runs after worktree discovery but before merge execution
- **Blocking check**: A preflight check that produces an error and stops the merge (P0)
- **Warning check**: A preflight check that produces a warning but allows the merge to continue (P1-P3)

#### 1.0.1.3 Preflight Check Catalog {#preflight-catalog}

**Table T01: Preflight Checks** {#t01-preflight-checks}

| ID | Priority | Check | Blocks? | Output |
|----|----------|-------|---------|--------|
| PF-01 | P0 | Implementation worktree dirty files | Yes | Error listing dirty files |
| PF-02 | P1 | Speck file existence | Yes | "Speck file not found: <path>" |
| PF-03 | P1 | Bead/step completion status | No | "N of M steps incomplete" warning |
| PF-04 | P2 | Branch divergence preview | No | Commit count + diff stat in warnings |
| PF-05 | P2 | Infrastructure diff preview | No | Note about .specks/.beads/ differences |
| PF-06 | P2 | Push failure messaging | No | Explicit "run git push origin main" instruction |
| PF-07 | P3 | gh CLI fallback explanation | No | "Remote detected but gh CLI unavailable" warning |
| PF-08 | P3 | Multiple worktree warning | No | Lists all found worktrees, notes which was selected |
| PF-09 | P3 | PR checks status | No | "CI checks failing" warning in dry-run |

#### 1.0.1.4 Error and Warning Model {#errors-warnings}

**Error fields (blocking checks):**
- `status`: always "error"
- `error`: descriptive message including the specific files or paths involved
- `warnings`: may still contain non-blocking warnings accumulated before the blocker

**Warning fields (non-blocking checks):**
- `warnings`: array of human-readable strings, one per finding

#### 1.0.1.5 MergeData Schema Changes {#merge-data-schema}

**Spec S01: MergeData warnings field** {#s01-merge-data-warnings}

```json
{
  "status": "ok",
  "merge_mode": "local",
  "branch_name": "specks/1-20260210-120000",
  "worktree_path": ".specks-worktrees/specks__1-20260210-120000",
  "warnings": [
    "2 of 5 steps incomplete. Run 'specks beads status .specks/specks-1.md' to review.",
    "Branch has 7 commits ahead of main (14 files changed, +312/-45)"
  ],
  "dry_run": true,
  "message": "Would squash-merge branch 'specks/1-20260210-120000' into main and clean up worktree"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `warnings` | `Option<Vec<String>>` | no | Non-blocking preflight findings. Omitted when empty. |

#### 1.0.1.6 WorktreeDiscovery Return Type {#worktree-discovery-type}

**Spec S02: WorktreeDiscovery struct** {#s02-worktree-discovery}

```rust
pub struct WorktreeDiscovery {
    pub selected: Option<DiscoveredWorktree>,
    pub all_matches: Vec<DiscoveredWorktree>,
    pub match_count: usize,
}
```

| Field | Type | Description |
|-------|------|-------------|
| `selected` | `Option<DiscoveredWorktree>` | The most recent matching worktree (same as current behavior) |
| `all_matches` | `Vec<DiscoveredWorktree>` | All matching worktrees found |
| `match_count` | `usize` | Count of matches (convenience for `all_matches.len()`) |

---

### 1.0.2 Symbol Inventory {#symbol-inventory}

#### 1.0.2.1 New files {#new-files}

| File | Purpose |
|------|---------|
| (none) | All changes are in existing files |

#### 1.0.2.2 Symbols to add / modify {#symbols}

**Table T02: Symbol Changes** {#t02-symbol-changes}

| Symbol | Kind | Location | Notes |
|--------|------|----------|-------|
| `PreflightResult` | struct | `merge.rs` | Contains `warnings: Vec<String>` and `blocking_error: Option<String>` |
| `run_preflight_checks()` | fn | `merge.rs` | Runs all P0-P3 checks, returns `PreflightResult` |
| `check_worktree_dirty()` | fn | `merge.rs` | P0: checks implementation worktree for dirty files |
| `check_bead_completion()` | fn | `merge.rs` | P1: checks bead/step completion status |
| `check_branch_divergence()` | fn | `merge.rs` | P2: computes merge-base, commit count, diff stat |
| `check_infra_diff()` | fn | `merge.rs` | P2: detects .specks/.beads/ differences between branches |
| `check_pr_checks()` | fn | `merge.rs` | P3: runs `gh pr checks` and reports failures |
| `WorktreeDiscovery` | struct | `worktree.rs` | New return type for `find_worktree_by_speck` |
| `find_worktree_by_speck` | fn (modified) | `worktree.rs` | Returns `WorktreeDiscovery` instead of `Option<DiscoveredWorktree>` |
| `warnings` | field | `MergeData` in `merge.rs` | `Option<Vec<String>>` with skip_serializing_if |

---

### 1.0.3 Test Plan Concepts {#test-plan-concepts}

#### Test Categories {#test-categories}

| Category | Purpose | When to use |
|----------|---------|-------------|
| **Unit** | Test individual preflight check functions in isolation | Each PF check, PreflightResult construction, MergeData serialization |
| **Integration** | Test full merge flow with preflight checks | Dirty worktree blocking, speck existence validation |
| **Golden / Contract** | Compare MergeData JSON output shape | Warnings field presence/absence |

---

### 1.0.4 Execution Steps {#execution-steps}

#### Step 0: Add warnings field to MergeData and update serialization {#step-0}

**Bead:** `specks-dg0.1`

**Commit:** `feat(merge): add warnings field to MergeData struct`

**References:** [D02] Warnings field is Option Vec String on MergeData, Spec S01, (#merge-data-schema, #inputs-outputs)

**Artifacts:**
- Modified `MergeData` struct in `merge.rs` with new `warnings` field
- Updated `MergeData::error()` helper to initialize warnings to None
- Updated all `MergeData` construction sites in `run_merge()` to include `warnings: None`
- Serialization tests for the new field

**Tasks:**
- [ ] Add `warnings: Option<Vec<String>>` field to `MergeData` with `#[serde(skip_serializing_if = "Option::is_none")]`
- [ ] Update `MergeData::error()` to set `warnings: None`
- [ ] Update every `MergeData { ... }` construction in `run_merge()` to include the `warnings` field
- [ ] Update dry-run text output to print warnings when present
- [ ] Update existing tests that construct `MergeData` to include the new field

**Tests:**
- [ ] Unit test: MergeData with no warnings omits `warnings` from JSON
- [ ] Unit test: MergeData with warnings includes `warnings` array in JSON
- [ ] Unit test: MergeData::error() includes `warnings: None`

**Checkpoint:**
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo nextest run -p specks --filter-expr 'test(merge)'` passes

**Rollback:**
- Revert the commit; no schema migration needed

**Commit after all checkpoints pass.**

---

#### Step 1: Add speck file existence check (P1) {#step-1}

**Depends on:** #step-0

**Bead:** `specks-dg0.2`

**Commit:** `feat(merge): validate speck file exists before worktree discovery`

**References:** [D04] Speck file existence check happens before worktree discovery, Table T01 PF-02, (#preflight-catalog, #errors-warnings)

**Artifacts:**
- Modified `run_merge()` in `merge.rs` to check speck file existence after `normalize_speck_path()`
- New error path returning "Speck file not found: <path>"

**Tasks:**
- [ ] After `normalize_speck_path()` call (line ~698), add `repo_root.join(&speck_path).exists()` check
- [ ] On failure, return `MergeData::error()` with message "Speck file not found: <display path>"
- [ ] Ensure both JSON and non-JSON output paths handle this error

**Tests:**
- [ ] Unit test: normalize_speck_path + existence check with non-existent file produces correct error message

**Checkpoint:**
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo nextest run -p specks --filter-expr 'test(merge)'` passes

**Rollback:**
- Revert the commit

**Commit after all checkpoints pass.**

---

#### Step 2: Modify find_worktree_by_speck to return WorktreeDiscovery {#step-2}

**Depends on:** #step-0

**Bead:** `specks-dg0.3`

**Commit:** `refactor(worktree): return WorktreeDiscovery with match count from find_worktree_by_speck`

**References:** [D06] find_worktree_by_speck returns match count, Spec S02, Table T02, (#worktree-discovery-type, #symbol-inventory)

**Artifacts:**
- New `WorktreeDiscovery` struct in `worktree.rs`
- Modified `find_worktree_by_speck` return type
- Updated all callers of `find_worktree_by_speck` (merge.rs and any others)

**Tasks:**
- [ ] Define `WorktreeDiscovery` struct in `worktree.rs` with `selected`, `all_matches`, and `match_count` fields
- [ ] Modify `find_worktree_by_speck` to build `all_matches` vec, select the most recent, and return `WorktreeDiscovery`
- [ ] Export `WorktreeDiscovery` from `specks_core` lib.rs
- [ ] Update `merge.rs` caller to destructure `WorktreeDiscovery` and extract the selected worktree
- [ ] Find and update any other callers of `find_worktree_by_speck` across the codebase

**Tests:**
- [ ] Unit test: `find_worktree_by_speck` with no matches returns `selected: None, match_count: 0`
- [ ] Unit test: `find_worktree_by_speck` with one match returns `selected: Some(...), match_count: 1`

**Checkpoint:**
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo nextest run` passes (all crates, since return type changed)

**Rollback:**
- Revert the commit; restore original return type

**Commit after all checkpoints pass.**

---

#### Step 3: Add P0 implementation worktree dirty-file check {#step-3}

**Depends on:** #step-0, #step-2

**Bead:** `specks-dg0.4`

**Commit:** `feat(merge): block merge when implementation worktree has dirty files (P0)`

**References:** [D03] P0 worktree dirty check inspects implementation worktree only, [D01] Preflight checks are a single function, Table T01 PF-01, (#preflight-catalog, #d03-worktree-dirty-check)

**Artifacts:**
- New `check_worktree_dirty()` function in `merge.rs`
- New `PreflightResult` struct in `merge.rs`
- New `run_preflight_checks()` function in `merge.rs` (initially containing only P0 check)
- Modified `run_merge()` to call `run_preflight_checks()` and handle blocking errors

**Tasks:**
- [ ] Define `PreflightResult` struct with `warnings: Vec<String>` and `blocking_error: Option<String>`
- [ ] Implement `check_worktree_dirty(wt_path: &Path) -> Result<Option<String>, String>` that calls `get_dirty_files(wt_path)` and returns a blocking error message if dirty
- [ ] Implement `run_preflight_checks()` that calls `check_worktree_dirty()` and returns `PreflightResult`
- [ ] In `run_merge()`, call `run_preflight_checks()` after worktree discovery, before mode-detection
- [ ] If `blocking_error` is Some, return `MergeData::error()` with the blocking message and any accumulated warnings
- [ ] If warnings are non-empty, store them in `MergeData.warnings` for all subsequent output paths

**Tests:**
- [ ] Integration test: create a git repo + worktree, add a dirty file to the worktree, verify `check_worktree_dirty()` returns a blocking error listing the file
- [ ] Integration test: create a clean worktree, verify `check_worktree_dirty()` returns None
- [ ] Unit test: `PreflightResult` with blocking error and warnings both populated

**Checkpoint:**
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo nextest run -p specks --filter-expr 'test(merge)'` passes

**Rollback:**
- Revert the commit

**Commit after all checkpoints pass.**

---

#### Step 4: Add P1 bead completion warning and P3 warnings {#step-4}

**Depends on:** #step-3

**Bead:** `specks-dg0.5`

**Commit:** `feat(merge): add bead completion, gh fallback, and multiple worktree warnings`

**References:** [D05] Bead completion check warns but does not block, [D06] find_worktree_by_speck returns match count, [D08] gh CLI fallback message is a warning, Table T01 PF-03 PF-07 PF-08, (#preflight-catalog, #d05-bead-completion-warning, #d06-worktree-match-count, #d08-gh-fallback-warning)

**Artifacts:**
- New `check_bead_completion()` function in `merge.rs`
- Multiple worktree warning logic using `WorktreeDiscovery.match_count`
- gh CLI fallback warning in mode detection
- All warnings flow into `PreflightResult.warnings`

**Tasks:**
- [ ] Implement `check_bead_completion(repo_root: &Path, speck_path: &Path) -> Option<String>` that parses the speck, checks bead status, and returns a warning like "N of M steps incomplete" (returns None if all complete or beads unavailable)
- [ ] Add bead completion check to `run_preflight_checks()`
- [ ] In `run_merge()`, after extracting `WorktreeDiscovery`, if `match_count > 1`, add a warning listing all matched worktrees and noting which was selected
- [ ] In mode detection (Step 1a/1b), when `has_origin` is true but `get_pr_for_branch` fails for reasons other than "no PR found", add a warning: "Remote detected but gh CLI unavailable -- falling back to local mode"
- [ ] Pass accumulated warnings through to `MergeData` in all output paths

**Tests:**
- [ ] Unit test: `check_bead_completion()` with all steps complete returns None
- [ ] Unit test: `check_bead_completion()` with incomplete steps returns warning string
- [ ] Unit test: `check_bead_completion()` with no beads configured returns None (no warning)
- [ ] Unit test: MergeData serialization with multiple warnings

**Checkpoint:**
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo nextest run -p specks --filter-expr 'test(merge)'` passes

**Rollback:**
- Revert the commit

**Commit after all checkpoints pass.**

---

#### Step 5: Add P2 branch divergence and infrastructure diff previews {#step-5}

**Depends on:** #step-3

**Bead:** `specks-dg0.6`

**Commit:** `feat(merge): add branch divergence and infrastructure diff previews for dry-run`

**References:** [D07] Branch divergence uses merge-base and rev-list count, Table T01 PF-04 PF-05, (#preflight-catalog, #d07-branch-divergence)

**Artifacts:**
- New `check_branch_divergence()` function in `merge.rs`
- New `check_infra_diff()` function in `merge.rs`
- Both checks added to `run_preflight_checks()` (only in dry-run mode)

**Tasks:**
- [ ] Implement `check_branch_divergence(repo_root: &Path, branch: &str) -> Option<String>` that runs `git merge-base main <branch>`, `git rev-list --count <merge-base>..<branch>`, and `git diff --stat <merge-base>..<branch>`, then formats a summary warning
- [ ] Implement `check_infra_diff(repo_root: &Path, branch: &str) -> Option<String>` that runs `git diff --name-only main..<branch>` and filters for `is_infrastructure_path()`, returning a warning noting which infra files differ and that conflicts will be auto-resolved
- [ ] Add both checks to `run_preflight_checks()`, gated by a `dry_run: bool` parameter
- [ ] Ensure warnings appear in both JSON `warnings` array and text output

**Tests:**
- [ ] Integration test: create a repo with a branch that has diverged from main, verify `check_branch_divergence()` returns a summary with correct commit count
- [ ] Integration test: create a branch with .specks/ file changes, verify `check_infra_diff()` returns a warning mentioning the files
- [ ] Unit test: `check_branch_divergence()` when merge-base fails returns None gracefully

**Checkpoint:**
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo nextest run -p specks --filter-expr 'test(merge)'` passes

**Rollback:**
- Revert the commit

**Commit after all checkpoints pass.**

---

#### Step 6: Add P2 push failure messaging and P3 PR checks {#step-6}

**Depends on:** #step-5

**Bead:** `specks-dg0.7`

**Commit:** `feat(merge): improve push failure messaging and add PR checks warning`

**References:** [D02] Warnings field, Table T01 PF-06 PF-09, (#preflight-catalog, #errors-warnings)

**Artifacts:**
- Modified push failure handling in remote mode to include explicit instruction
- New `check_pr_checks()` function in `merge.rs`
- PR checks warning in dry-run for remote mode

**Tasks:**
- [ ] In the remote mode push failure path (line ~999-1003), change the warning message to: "Failed to push infrastructure sync to origin. Run `git push origin main` to sync."
- [ ] In the post-merge push path (line ~1114-1118), add similar explicit messaging
- [ ] Implement `check_pr_checks(branch: &str) -> Option<String>` that runs `gh pr checks <branch>` and, if any checks fail, returns a warning listing the failed checks
- [ ] Add `check_pr_checks()` to `run_preflight_checks()`, gated by `dry_run && effective_mode == "remote"`

**Tests:**
- [ ] Unit test: `check_pr_checks()` with all checks passing returns None
- [ ] Unit test: `check_pr_checks()` when gh is unavailable returns None gracefully

**Checkpoint:**
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo nextest run -p specks --filter-expr 'test(merge)'` passes

**Rollback:**
- Revert the commit

**Commit after all checkpoints pass.**

---

#### Step 7: Update SKILL.md documentation {#step-7}

**Depends on:** #step-0

**Bead:** `specks-dg0.8`

**Commit:** `docs(merge): document warnings field in SKILL.md dry-run field table`

**References:** [D02] Warnings field, Spec S01, (#merge-data-schema)

**Artifacts:**
- Updated `skills/merge/SKILL.md` with `warnings` field in field tables
- Added guidance for interpreting preflight warnings

**Tasks:**
- [ ] Add `warnings` row to the dry-run JSON field table in SKILL.md section 1
- [ ] Add `warnings` row to the merge result JSON field table in SKILL.md section 3
- [ ] Add a note in section 2 (confirmation) about surfacing warnings to the user before asking for confirmation
- [ ] Add common warnings to the Error Handling section

**Tests:**
- [ ] Manual review: SKILL.md renders correctly

**Checkpoint:**
- [ ] `specks validate .specks/specks-1.md` passes

**Rollback:**
- Revert the commit

**Commit after all checkpoints pass.**

---

### 1.0.5 Deliverables and Checkpoints {#deliverables}

**Deliverable:** The `specks merge` command validates implementation worktree state, speck file existence, bead completion, branch divergence, and mode-detection edge cases before executing a merge, surfacing all findings via a `warnings` field in MergeData JSON output.

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

- [ ] `specks merge` on a dirty implementation worktree blocks with error listing files (`cargo nextest run` integration test)
- [ ] `specks merge` with non-existent speck path returns "Speck file not found" error
- [ ] `specks merge --dry-run --json` includes `warnings` array when preflight checks find issues
- [ ] `warnings` field is omitted from JSON when no warnings exist (backwards-compatible)
- [ ] All existing merge tests continue to pass
- [ ] `cargo build` passes with zero warnings
- [ ] `cargo nextest run` passes all tests across all crates
- [ ] `specks validate .specks/specks-1.md` passes
- [ ] SKILL.md documents the new `warnings` field

**Acceptance tests:**
- [ ] Integration test: dirty worktree blocks merge
- [ ] Integration test: clean worktree allows merge to proceed
- [ ] Unit test: MergeData JSON with and without warnings
- [ ] Unit test: speck file not found error
- [ ] Unit test: bead completion warning generation

#### Roadmap / Follow-ons (Explicitly Not Required for Phase Close) {#roadmap}

- [ ] Add `--skip-preflight` flag to bypass all preflight checks
- [ ] Add structured warning objects (code, severity, suggestion) instead of plain strings
- [ ] Integrate preflight checks with `specks doctor` for a unified health-check experience

| Checkpoint | Verification |
|------------|--------------|
| All P0-P3 checks implemented | `cargo nextest run -p specks --filter-expr 'test(merge)'` |
| No regressions | `cargo nextest run` (full suite) |
| Documentation updated | `skills/merge/SKILL.md` has `warnings` field |
| Speck valid | `specks validate .specks/specks-1.md` |

**Commit after all checkpoints pass.**