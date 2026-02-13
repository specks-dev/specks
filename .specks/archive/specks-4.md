## Phase 4.0: Fix Remote Merge History Divergence {#phase-fix-remote-merge}

**Purpose:** Prevent git history divergence when `specks merge` runs in remote mode by deferring infrastructure file commits until after the PR squash-merge lands, ensuring local and remote main always stay in sync.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | kocienda |
| Status | draft |
| Target branch | main |
| Tracking issue/PR | — |
| Last updated | 2025-02-11 |
| Beads Root | `specks-xuf` |

---

### Phase Overview {#phase-overview}

#### Context {#context}

The `specks merge` command in remote mode commits infrastructure files (`.specks/`, `.beads/`) to local main **before** calling `gh pr merge --squash`. This creates a local-only commit. Then the squash-merge creates a separate remote-only commit. Local and remote main have now diverged. A subsequent `git pull origin main` either fails or creates an unwanted merge commit. Additionally, there is no pre-merge check that local main is in sync with `origin/main`, so users with unpushed commits encounter unpredictable failures. Finally, the current code silently discards non-infrastructure dirty files on main, which is data loss if the user has uncommitted work.

The fix restructures the merge flow (both modes) to: fail fast if non-infrastructure dirty files exist, then (remote mode only) verify sync, save infrastructure files to a temp directory, discard only infrastructure dirty files so the working tree is clean, let the remote squash-merge happen, pull with fast-forward only, restore and commit infrastructure files, then auto-push to keep local and remote in sync. All pre-merge checks (dirty file check, sync check) run during dry-run as well, so the merge skill surfaces problems before asking for confirmation.

#### Strategy {#strategy}

- Fail fast in both modes if non-infrastructure dirty files exist on main — never silently discard user work
- Run all pre-merge checks (dirty file check, sync check) before the dry-run return so problems are surfaced during preview
- Add a fail-fast sync check comparing local HEAD against `origin/main` before any merge work begins (remote mode only)
- Save infrastructure files to a temp directory, then discard only the saved infrastructure files from the working tree so `git pull --ff-only` is not blocked
- On error after infra discard (merge failure, pull failure), restore infra files to working tree before returning — never lose uncommitted infrastructure modifications
- Bypass `prepare_main_for_merge()` entirely in remote mode — it is only used by local mode
- Use `git pull --ff-only` after PR merge to guarantee a fast-forward pull
- Restore infrastructure files from temp, commit them, and auto-push to origin so the next `specks merge` sync check passes
- Ensure temp directory cleanup happens even if the merge fails partway through (RAII Drop guard that restores files before deleting temp)
- Add the same non-infra dirty file check to local mode, before calling `prepare_main_for_merge()`
- Add tests for the dirty file check, sync check, save/restore pattern, error recovery, and cleanup guard

#### Stakeholders / Primary Customers {#stakeholders}

1. Specks users who use `specks merge` with a GitHub remote (remote mode)
2. Specks users who use `specks merge` locally (local mode)
3. Specks developers maintaining the merge command

#### Success Criteria (Measurable) {#success-criteria}

- After `specks merge` completes in remote mode, `git status` shows the working tree is clean and `git log --oneline origin/main..main` shows 0 commits (infra commit has been pushed)
- Running `specks merge` when local main has diverged from `origin/main` produces an error message mentioning `git push origin main`
- Running `specks merge` with non-infrastructure dirty files (either mode) produces an error listing the dirty files and suggesting `git stash` or `git commit`
- Running `specks merge --dry-run` with non-infra dirty files or a diverged main produces the same errors (checks run during dry-run)
- If `gh pr merge` or `git pull --ff-only` fails after infra files were discarded, the working tree is restored to its pre-merge state (infra files copied back from temp)
- All existing merge tests continue to pass
- New unit tests cover: dirty file check, sync check, infrastructure file save/restore, error recovery, temp directory cleanup

#### Scope {#scope}

1. Add non-infrastructure dirty file fail-fast check (both modes, including dry-run)
2. Add `check_main_sync()` helper for remote-mode pre-merge validation (including dry-run)
3. Add `save_infra_to_temp()`, `restore_infra_from_temp()`, `copy_infra_from_temp()`, and `TempDirGuard` helpers
4. Add targeted infrastructure file discard after saving to temp (remote mode only)
5. Add error recovery: restore infra files to working tree on merge/pull failure
6. Restructure the remote-mode path in `run_merge()` to bypass `prepare_main_for_merge()` and use the new helpers instead
7. Auto-push the infrastructure commit after restore
8. Add the non-infra dirty file check to local mode before `prepare_main_for_merge()`
9. Add unit tests for new helpers and integration tests for the updated flow

#### Non-goals (Explicitly out of scope) {#non-goals}

- Modifying `prepare_main_for_merge()` — it stays as-is, used only by local mode (the non-infra check gates before it is called)
- Changing the worktree creation or cleanup logic
- Modifying the `gh pr merge` invocation parameters (still `--squash`)
- Handling the case where `origin/main` is ahead of local (this is a normal pull scenario, not a divergence)

#### Dependencies / Prerequisites {#dependencies}

- `gh` CLI must be available for remote-mode operations (existing requirement)
- Git must support `pull --ff-only` (available in all modern git versions)

#### Constraints {#constraints}

- Must not silently discard non-infrastructure dirty files — this is data loss
- Must restore infrastructure files to working tree on error — never lose uncommitted modifications
- Must handle temp directory cleanup on all error paths
- The fix is contained entirely within `crates/specks/src/commands/merge.rs`
- Warnings are errors (`-D warnings` enforced by `.cargo/config.toml`)
- On repositories with branch protection that prevents direct pushes to main, the auto-push after infrastructure commit will fail (non-fatal warning). This leaves local main 1 commit ahead of origin, which will cause the sync check to fail on the next merge. The `--force` flag (roadmap item) would bypass the sync check for this scenario. No code changes are needed for this constraint — it is a known limitation documented here and in Assumptions.

#### Assumptions {#assumptions}

- The sync check compares HEAD vs `origin/main` using `git rev-parse` on both refs
- Temp directory uses the system temp directory (e.g., `/tmp/specks-merge-XXXXX/`)
- Infrastructure files are those matching `.specks/` or `.beads/` prefixes (using existing `is_infrastructure_path()`)
- The squash commit from a PR may touch infrastructure files (e.g., `.specks/specks-implementation-log.md`), so infrastructure dirty files must be discarded from the working tree before pull
- On repositories with branch protection that prevents direct pushes to main, the auto-push will fail (non-fatal warning). This leaves local main 1 commit ahead of origin, which will cause the sync check to fail on the next merge. The `--force` flag (roadmap item) would bypass the sync check for this scenario.

---

### 4.0.0 Design Decisions {#design-decisions}

#### [D01] Fail-fast sync check before remote merge (DECIDED) {#d01-sync-check}

**Decision:** In remote mode, fetch `origin/main` and verify that local `HEAD` is at the same commit as `origin/main` before proceeding. If they differ, abort with an error suggesting `git push origin main`.

**Rationale:**
- Divergence between local and remote main is the root cause of the history mess
- Catching it early avoids partial-merge states that are harder to recover from
- The check is cheap (one fetch + one rev comparison)

**Implications:**
- Requires network access (fetch) at the start of remote-mode merge
- Users with unpushed local commits must push first
- The error message must be actionable ("run `git push origin main` first")

#### [D02] Temp directory for infrastructure files instead of pre-merge commit (DECIDED) {#d02-temp-directory}

**Decision:** Copy dirty infrastructure files to a temp directory (e.g., `/tmp/specks-merge-XXXXX/`) instead of committing them to main before the PR merge. Restore them after `git pull --ff-only`.

**Rationale:**
- Committing before `gh pr merge` creates a local-only commit that diverges from remote
- A temp directory sidesteps the divergence entirely — no local commit exists when the remote merge happens
- `git pull --ff-only` is guaranteed to succeed because we verified sync in D01 and the only dirty files (infrastructure) have been discarded per D03

**Implications:**
- Need save and restore helpers that preserve file paths relative to repo root
- Temp directory must be cleaned up on all code paths (success, error, panic)
- On error paths after infra files have been discarded, files must be copied back from temp to the working tree before cleanup — otherwise uncommitted modifications are lost
- If the machine crashes mid-merge, temp files are in `/tmp/` and can be manually recovered

#### [D03] Fail fast if non-infrastructure dirty files exist (DECIDED) {#d03-fail-fast-dirty}

**Decision:** Before any merge operations, check for non-infrastructure dirty files on main. If any exist, abort with an error listing the files and suggesting `git stash` or `git commit`. This check applies to both remote and local mode, including dry-run.

**Rationale:**
- The previous behavior silently discarded non-infrastructure dirty files, which is data loss
- Non-infrastructure dirty files on main could be unrelated user work that should not be destroyed
- In local mode, these files would likely conflict with the squash merge anyway — failing fast with a clear message is better than a confusing git merge conflict
- In remote mode, these files would block `git pull --ff-only` if the squash commit touches the same paths
- Running the check during dry-run ensures the merge skill surfaces problems before asking for confirmation

**Implications:**
- Users must commit or stash non-infrastructure changes before running `specks merge`
- Only infrastructure files (`.specks/`, `.beads/`) are tolerated as dirty on main
- The check is a simple conditional in `run_merge()` — no new helper function needed
- After this check passes, infrastructure files in remote mode are saved to temp then discarded from the working tree using targeted `git checkout`/`git clean` on just those paths

#### [D04] Use git pull --ff-only after remote merge (DECIDED) {#d04-ff-only-pull}

**Decision:** After `gh pr merge --squash` succeeds, run `git pull --ff-only origin main` instead of plain `git pull origin main`.

**Rationale:**
- Because we verified sync in D01 and discarded infrastructure dirty files from the working tree, fast-forward is guaranteed
- `--ff-only` fails loudly if the assumption is violated, preventing silent merge commits
- This makes the operation deterministic and safe

**Implications:**
- If `--ff-only` fails, something unexpected happened — treat as an error
- The infrastructure file restore happens only after a successful pull

#### [D05] Bypass prepare_main_for_merge in remote mode (DECIDED) {#d05-bypass-prepare}

**Decision:** In remote mode, bypass `prepare_main_for_merge()` entirely. The remote-mode flow handles dirty files directly: fail fast on non-infra, save infra to temp, discard infra files, merge, pull, restore. The function `prepare_main_for_merge()` remains unchanged and is used only by local mode.

**Rationale:**
- `prepare_main_for_merge()` re-fetches dirty files internally (redundant — remote mode already has the list)
- Its "discard non-infra, commit infra" behavior is wrong for remote mode — we refuse non-infra and discard only infra
- Adding a `remote_mode: bool` parameter would make the function do two completely different things
- Leaving it unchanged for local mode minimizes risk

**Implications:**
- `prepare_main_for_merge()` is not modified — zero risk to local mode
- Remote mode implements its own dirty file handling inline in `run_merge()`
- The non-infra dirty file check in local mode gates before `prepare_main_for_merge()` is called, so its internal "discard non-infra" code becomes effectively dead — but leaving it is defensive (in case the function is called from elsewhere)

#### [D06] MergeData uses existing error field for sync check failures (DECIDED) {#d06-sync-error-field}

**Decision:** When the sync check fails, `MergeData` returns with `status: "error"` and `error` containing the sync check message. No new fields are added to `MergeData`. The existing `error` field is sufficient since the sync check is a fail-fast that prevents any merge operations from starting.

**Rationale:**
- The sync check is a pre-condition failure, not a partial-merge state
- Adding a separate `sync_error` field would add schema complexity for a simple "abort early" case
- The `error` message is descriptive enough for both human and machine consumers

**Implications:**
- JSON consumers can detect sync errors by checking `error` field content for "diverged" or "git push"
- No breaking changes to the `MergeData` JSON schema

#### [D07] Auto-push infrastructure commit after restore (DECIDED) {#d07-auto-push}

**Decision:** After committing the restored infrastructure files, automatically push to origin with `git push origin main`. If the push fails, warn the user but do not fail the merge — the merge itself already succeeded.

**Rationale:**
- Without auto-push, local main is 1 commit ahead of origin after every remote-mode merge
- The next `specks merge` would fail the D01 sync check because local has diverged
- Auto-pushing keeps local and remote in sync, making the sync check transparent to users
- Push failure is non-fatal because the merge has already completed — the user can push manually later

**Implications:**
- Requires network access at the end of the merge flow (in addition to the fetch at the start)
- If push fails (permissions, network, branch protection), the user sees a warning and can push manually
- After a successful push, `git log --oneline origin/main..main` shows 0 commits

#### [D08] Pre-merge checks run during dry-run (DECIDED) {#d08-checks-in-dry-run}

**Decision:** The dirty file check and sync check run before the dry-run return in `run_merge()`. If either check fails during dry-run, return `MergeData` with `status: "error"` — the same pattern used for the existing worktree-not-found error.

**Rationale:**
- The merge skill calls dry-run first, shows a preview, asks for confirmation, then calls the actual merge
- If checks only run on the actual merge, the user gets a clean dry-run preview, confirms, then hits an error — bad UX
- Dry-run should surface all pre-condition failures so the user can fix them before confirming
- The checks are read-only (fetch + rev-parse + status) so they are safe to run during dry-run

**Implications:**
- The dirty file check and sync check are placed between worktree/mode detection and the `if dry_run { return }` block
- Dry-run output already includes `dirty_files` — this is consistent with surfacing problems early
- No side effects beyond a `git fetch` (which updates remote tracking refs only)

#### [D09] Restore infra files to working tree on error (DECIDED) {#d09-restore-on-error}

**Decision:** If `gh pr merge` or `git pull --ff-only` fails after infrastructure files have been discarded from the working tree, copy the infra files back from the temp directory to the working tree before returning the error. This restores the user's pre-merge state.

**Rationale:**
- After save-to-temp + discard, the temp directory holds the ONLY copy of the user's uncommitted infrastructure modifications
- If TempDirGuard just deletes the temp dir on error, those modifications are lost — this is data loss
- Restoring files to the working tree is cheap (just file copies, no git operations) and leaves the user exactly where they started
- The user can then fix the issue (e.g., retry merge, push first) without losing their infrastructure changes

**Implications:**
- TempDirGuard's Drop impl must copy files back to the working tree before deleting the temp dir
- Need a `copy_infra_from_temp()` helper that copies files without staging or committing (unlike `restore_infra_from_temp()` which also commits)
- TempDirGuard needs to store `repo_root` and `infra_files` in addition to the temp path so it can perform the restore
- On the happy path, `restore_infra_from_temp()` handles the commit and cleanup, then `guard.defuse()` prevents the Drop from running

---

### Deep Dives {#deep-dives}

#### Affected Code Analysis {#affected-code}

##### Current Remote-Mode Flow (Broken) {#current-flow}

```
run_merge()
  |-- is_main_worktree()           // OK
  |-- find_worktree_by_speck()     // OK
  |-- detect mode (remote/local)   // OK
  |-- get_dirty_files()            // OK
  |-- if dry_run { return }        // dry-run exits without checking anything
  |-- prepare_main_for_merge()     // BUG: commits infra, silently discards non-infra
  |   |-- git checkout/clean non-infra  <- DATA LOSS: silently discards user work
  |   |-- git add .specks/ .beads/
  |   +-- git commit -m "chore: pre-merge sync"  <- creates local-only commit
  |-- gh pr merge --squash         // creates remote-only squash commit
  |-- git pull origin main         // DIVERGED: local has pre-merge commit, remote has squash
  +-- cleanup worktree
```

##### Fixed Remote-Mode Flow {#fixed-flow}

```
run_merge()
  |-- is_main_worktree()                          // unchanged (line ~452)
  |-- find_worktree_by_speck()                    // unchanged (line ~461)
  |-- detect mode (remote/local)                  // unchanged (line ~491)
  |                                               //
  |   --- PRE-DRY-RUN CHECKS (run in both dry-run and actual merge) ---
  |                                               //
  |-- check_main_sync()                           // NEW: fetch + compare HEAD vs origin/main
  |   +-- on failure: return MergeData { status: "error" } immediately
  |-- get_dirty_files() + partition               // get infra and non-infra lists
  |-- FAIL if non-infra dirty files exist         // NEW: return MergeData { status: "error" }
  |                                               //
  |   --- DRY-RUN RETURN (line ~512) ---
  |                                               //
  |-- if dry_run { return MergeData preview }     // dry-run exits AFTER checks pass
  |                                               //
  |   --- ACTUAL MERGE (only reached when dry_run=false) ---
  |                                               //
  |-- save_infra_to_temp()                        // NEW: copy infra files to /tmp/
  |-- TempDirGuard::new(temp, repo, infra_files)  // NEW: RAII guard for error recovery
  |-- discard infra files from working tree       // NEW: git checkout/clean on infra paths only
  |-- gh pr merge --squash                        // unchanged
  |   +-- on failure: guard Drop restores infra to working tree, return error
  |-- git pull --ff-only origin main              // CHANGED: --ff-only
  |   +-- on failure: guard Drop restores infra to working tree, return error
  |-- restore_infra_from_temp()                   // NEW: copy files back, git add, git commit
  |-- guard.defuse()                              // prevent Drop from re-restoring
  |-- git push origin main                        // NEW: auto-push (warn on failure)
  +-- cleanup worktree                            // unchanged
```

##### Fixed Local-Mode Flow {#fixed-local-flow}

```
run_merge()
  |-- is_main_worktree()                          // unchanged
  |-- find_worktree_by_speck()                    // unchanged
  |-- detect mode (remote/local)                  // unchanged
  |                                               //
  |   --- PRE-DRY-RUN CHECKS ---
  |                                               //
  |-- get_dirty_files() + partition               // get infra and non-infra lists
  |-- FAIL if non-infra dirty files exist         // NEW: return MergeData { status: "error" }
  |                                               //
  |   --- DRY-RUN RETURN ---
  |                                               //
  |-- if dry_run { return MergeData preview }     // dry-run exits AFTER checks pass
  |                                               //
  |   --- ACTUAL MERGE ---
  |                                               //
  |-- prepare_main_for_merge()                    // unchanged (only sees infra dirty files now)
  |-- squash_merge_branch()                       // unchanged
  +-- cleanup worktree                            // unchanged
```

##### Key Functions to Modify {#functions-to-modify}

**Table T01: Functions and Changes** {#t01-functions}

| Function | File | Change |
|----------|------|--------|
| `run_merge()` | `merge.rs:442` | Add checks before dry-run return; add non-infra dirty file check (both modes); restructure remote path: sync check, save/discard-infra/merge/pull/restore/push with error recovery; bypass `prepare_main_for_merge` in remote mode |
| `check_main_sync()` | `merge.rs` (new) | Fetch origin/main, compare HEAD, error if diverged |
| `save_infra_to_temp()` | `merge.rs` (new) | Copy dirty infra files to temp dir, return temp path |
| `restore_infra_from_temp()` | `merge.rs` (new) | Copy files back from temp, stage, commit (happy path) |
| `copy_infra_from_temp()` | `merge.rs` (new) | Copy files back from temp without staging/committing (error recovery) |
| `TempDirGuard` | `merge.rs` (new) | RAII guard: restores infra files to working tree then removes temp dir on Drop; defuse() disables |

---

### 4.0.1 Specification {#specification}

#### 4.0.1.1 Inputs and Outputs {#inputs-outputs}

**Inputs:**
- A speck path identifying the implementation to merge (existing, unchanged)
- `--dry-run`, `--force`, `--json`, `--quiet` flags (existing, unchanged)

**Outputs:**
- `MergeData` JSON struct (existing schema, no new fields per [D06])
- Side effects: git commits, git push, worktree cleanup (existing, reordered for remote mode)

**Key invariants:**
- Non-infrastructure dirty files are never silently discarded — merge aborts with an error listing them
- After remote-mode merge completes successfully, local main is in sync with `origin/main` (0 commits ahead)
- If the auto-push fails, local main is at most 1 commit ahead (the infrastructure sync commit) and the user is warned
- On error after infra files have been discarded, the working tree is restored to its pre-merge state (infra files copied back from temp)
- Local mode behavior is identical to before this change except for the new non-infra dirty file check

#### 4.0.1.2 Terminology {#terminology}

- **Infrastructure files**: Files under `.specks/` or `.beads/` directories, as identified by the existing `is_infrastructure_path()` function
- **Non-infrastructure files**: All other files — user code, configs, docs, etc.
- **Sync check**: Verification that local `HEAD` matches `origin/main` after a fresh fetch
- **Temp save/restore**: Pattern of copying infrastructure files to a temporary directory, performing the merge, then restoring them
- **Targeted discard**: Discarding only specific files (infrastructure) from the working tree using `git checkout`/`git clean` on those paths, as opposed to discarding everything
- **Error recovery restore**: Copying infra files back from temp to the working tree (without committing) so the user's pre-merge state is preserved on failure

#### 4.0.1.3 Normative Rules (Semantics) {#semantics}

**Pre-merge checks (both modes, including dry-run, normative):**

These checks run BEFORE the dry-run return at line ~512. In dry-run mode, if either check fails, return `MergeData` with `status: "error"` — the same pattern used for the existing worktree-not-found error. This ensures the merge skill surfaces problems during the preview step, before asking the user for confirmation.

1. Validate main worktree and branch (existing)
2. Discover worktree and detect mode (existing)
3. (Remote mode only) **Fetch and sync check**: `git fetch origin main`, then compare `git rev-parse HEAD` vs `git rev-parse origin/main`. If they differ, return `MergeData { status: "error", error: "..." }` immediately (same in both dry-run and actual merge).
4. **Dirty file check**: Call `get_dirty_files()` and partition into infra/non-infra. If non-infra list is non-empty, return `MergeData { status: "error", error: "..." }` immediately (same in both dry-run and actual merge).
5. If `dry_run`, return `MergeData` with preview info. All pre-condition checks have passed. Only infrastructure dirty files (if any) are reported in `dirty_files`.

**Remote-mode merge ordering (normative, after dry-run return):**
6. **Save and discard infra**: Copy infra files to temp directory, wrap in `TempDirGuard`. Discard only infra files from working tree (`git checkout -- <infra-paths>` + `git clean -f -- <infra-paths>`).
7. **Merge PR**: `gh pr merge --squash <branch>` (existing). On failure: TempDirGuard restores infra files to working tree, cleans up temp dir, returns error.
8. **Pull**: `git pull --ff-only origin main`. On failure: TempDirGuard restores infra files to working tree, cleans up temp dir, returns error.
9. **Restore and commit**: Copy infra files back from temp, `git add`, `git commit -m "chore: post-merge infrastructure sync"`. Defuse guard. Remove temp directory.
10. **Push**: `git push origin main`. If push fails, warn but do not fail the merge.
11. **Cleanup**: Remove worktree and branch (existing)

**Local-mode merge ordering (updated, after dry-run return):**
6. `prepare_main_for_merge()` — unchanged; now only ever sees infrastructure dirty files
7. `squash_merge_branch()` — unchanged
8. **Cleanup**: Remove worktree and branch (existing)

**Error semantics:**

Working tree restoration invariant: after save-to-temp + discard, the temp directory holds the ONLY copy of the user's uncommitted infrastructure modifications. On any error path after infra discard, infrastructure files MUST be copied back from temp to the working tree BEFORE letting TempDirGuard clean up the temp directory. This ensures the user's working tree returns to its pre-merge state on failure — no data loss.

- Non-infra dirty files: `MergeData { status: "error", error: "Working tree has uncommitted non-infrastructure files:\n  CLAUDE.md\n  src/main.rs\nCommit or stash these files before running specks merge.", ... }`
- Sync check failure: `MergeData { status: "error", error: "Local main has diverged from origin/main. Push your local commits with 'git push origin main' before retrying.", ... }`
- `gh pr merge` failure (after infra discard): `TempDirGuard::drop()` fires, calling `copy_infra_from_temp()` to restore infra files to the working tree, then removes temp dir. Return `MergeData { status: "error", error: "Failed to merge PR: <details>. Working tree has been restored to pre-merge state.", ... }`
- `git pull --ff-only` failure (after infra discard): `TempDirGuard::drop()` fires, calling `copy_infra_from_temp()` to restore infra files to the working tree, then removes temp dir. Return `MergeData { status: "error", error: "Fast-forward pull failed after PR merge. Working tree has been restored to pre-merge state. Local main may need manual recovery.", ... }`
- Push failure: not an error — merge succeeded. Print warning to stderr: "Warning: could not push infrastructure commit to origin. Run 'git push origin main' manually."
- Temp directory is always cleaned up (including panic via Drop guard). On error paths, infra files are restored to working tree before temp is deleted.

#### 4.0.1.4 Public API Surface {#public-api}

No public API changes. All new functions are private (`fn`, not `pub fn`) within `crates/specks/src/commands/merge.rs`. The `MergeData` struct and `run_merge()` public signature are unchanged.

**Spec S01: check_main_sync()** {#s01-check-main-sync}

```rust
/// Verify local main is in sync with origin/main.
/// Fetches origin/main first to ensure comparison is current.
/// Returns Ok(()) if in sync, Err with actionable message if diverged.
fn check_main_sync(repo_root: &Path) -> Result<(), String>
```

**Preconditions:**
- Repository has a remote named `origin`
- Current branch is `main` or `master`

**Algorithm:**
1. Run `git fetch origin main` — if fetch fails, return error describing fetch failure
2. Run `git rev-parse HEAD` to get local HEAD hash
3. Run `git rev-parse origin/main` to get remote HEAD hash
4. If hashes are equal, return `Ok(())`
5. If hashes differ, return `Err("Local main has diverged from origin/main. Push your local commits with 'git push origin main' before retrying.")`

**Postconditions:**
- No side effects on the repository (fetch updates remote tracking refs only)

**Spec S02: save_infra_to_temp()** {#s02-save-infra-to-temp}

```rust
/// Save dirty infrastructure files to a temp directory.
/// Returns the temp directory path. Caller is responsible for cleanup
/// (or use TempDirGuard for RAII cleanup).
fn save_infra_to_temp(repo_root: &Path, infra_files: &[String]) -> Result<PathBuf, String>
```

**Preconditions:**
- All paths in `infra_files` are relative to `repo_root`
- Files exist at `repo_root.join(path)` for each path

**Algorithm:**
1. Create temp directory: `std::env::temp_dir().join(format!("specks-merge-{}", random_suffix))`
2. Create the directory with `std::fs::create_dir_all`
3. For each file in `infra_files`:
   a. Compute source: `repo_root.join(file)`
   b. Compute dest: `temp_dir.join(file)`
   c. Create parent directories: `std::fs::create_dir_all(dest.parent())`
   d. Copy: `std::fs::copy(source, dest)`
4. Return `Ok(temp_dir)`

**Postconditions:**
- Temp directory contains copies of all infrastructure files with preserved relative paths
- Original files in repo are untouched

**Spec S03: restore_infra_from_temp()** {#s03-restore-infra-from-temp}

```rust
/// Restore infrastructure files from temp directory, stage, and commit.
/// Removes the temp directory on success. Used on the happy path.
fn restore_infra_from_temp(
    repo_root: &Path,
    temp_dir: &Path,
    infra_files: &[String],
    quiet: bool,
) -> Result<(), String>
```

**Preconditions:**
- `temp_dir` contains the files saved by `save_infra_to_temp()`
- Repository is in a clean state (post-pull)

**Algorithm:**
1. For each file in `infra_files`:
   a. Compute source: `temp_dir.join(file)`
   b. Compute dest: `repo_root.join(file)`
   c. Create parent directories if needed
   d. Copy: `std::fs::copy(source, dest)`
2. Run `git add` on all restored file paths
3. Run `git commit -m "chore: post-merge infrastructure sync"`
4. If commit succeeds, remove temp directory with `std::fs::remove_dir_all(temp_dir)`
5. If commit fails with "nothing to commit", remove temp directory and return `Ok(())` (files may have been identical to what the PR brought in)

**Postconditions:**
- Infrastructure files are committed to main
- Temp directory is removed
- Repository has at most 1 new commit

**Spec S04: copy_infra_from_temp()** {#s04-copy-infra-from-temp}

```rust
/// Copy infrastructure files from temp directory back to the working tree
/// WITHOUT staging or committing. Used for error recovery to restore the
/// user's pre-merge working tree state.
fn copy_infra_from_temp(
    repo_root: &Path,
    temp_dir: &Path,
    infra_files: &[String],
) -> Result<(), String>
```

**Preconditions:**
- `temp_dir` contains the files saved by `save_infra_to_temp()`

**Algorithm:**
1. For each file in `infra_files`:
   a. Compute source: `temp_dir.join(file)`
   b. Compute dest: `repo_root.join(file)`
   c. Create parent directories if needed
   d. Copy: `std::fs::copy(source, dest)`

**Postconditions:**
- Infrastructure files are back in the working tree with their original content
- Files are NOT staged or committed — they appear as dirty, same as before the merge attempt
- Temp directory is untouched (caller handles cleanup)

**Spec S05: TempDirGuard** {#s05-temp-dir-guard}

```rust
/// RAII guard that restores infra files to working tree, then removes the
/// temp directory on Drop. Call defuse() after successful restore+commit
/// to prevent the Drop from running.
struct TempDirGuard {
    temp_path: Option<PathBuf>,
    repo_root: PathBuf,
    infra_files: Vec<String>,
}

impl TempDirGuard {
    fn new(temp_path: PathBuf, repo_root: PathBuf, infra_files: Vec<String>) -> Self {
        Self { temp_path: Some(temp_path), repo_root, infra_files }
    }
    fn defuse(&mut self) { self.temp_path = None; }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        if let Some(ref temp_path) = self.temp_path {
            // Best-effort restore: copy infra files back to working tree
            let _ = copy_infra_from_temp(&self.repo_root, temp_path, &self.infra_files);
            // Then clean up temp dir
            let _ = std::fs::remove_dir_all(temp_path);
        }
    }
}
```

**Behavior:**
- On creation, stores the temp directory path, repo root, and infrastructure file list
- On drop (including unwinding from errors), copies infra files back to working tree via `copy_infra_from_temp()`, then removes the temp directory
- `defuse()` clears the temp path so Drop is a no-op — call after `restore_infra_from_temp()` succeeds (which handles its own cleanup and commit)
- The restore in Drop is best-effort (`let _ = ...`) — if it fails, at least the temp directory still exists for manual recovery

---

### 4.0.2 Definitive Symbol Inventory {#symbol-inventory}

#### 4.0.2.1 New files (if any) {#new-files}

No new files. All changes are within the existing `crates/specks/src/commands/merge.rs`.

#### 4.0.2.2 Symbols to add / modify {#symbols}

| Symbol | Kind | Location | Notes |
|--------|------|----------|-------|
| `check_main_sync` | fn | `merge.rs` | New. Fetch origin/main and compare HEAD hashes. See Spec S01. |
| `save_infra_to_temp` | fn | `merge.rs` | New. Copy dirty infra files to temp dir. See Spec S02. |
| `restore_infra_from_temp` | fn | `merge.rs` | New. Copy files back from temp, stage, commit (happy path). See Spec S03. |
| `copy_infra_from_temp` | fn | `merge.rs` | New. Copy files back from temp without staging/committing (error recovery). See Spec S04. |
| `TempDirGuard` | struct | `merge.rs` | New. RAII guard: restores infra files then removes temp dir on Drop; defuse() disables. See Spec S05. |
| `run_merge` | fn (modify) | `merge.rs:442` | Add checks before dry-run return; add non-infra dirty file check (both modes); restructure remote path with error recovery. |

---

### 4.0.3 Documentation Plan {#documentation-plan}

- [ ] Update troubleshooting section in `CLAUDE.md` to document the new sync check error and its resolution (`git push origin main`)
- [ ] Update troubleshooting section in `CLAUDE.md` to document the new dirty file check error and its resolution (`git stash` or `git commit`)
- [ ] Add note about branch protection and auto-push failure to troubleshooting
- [ ] No CLI flag changes, so `--help` text is unchanged
- [ ] No public API changes, so no library doc updates needed

---

### 4.0.4 Test Plan Concepts {#test-plan-concepts}

#### Test Categories {#test-categories}

| Category | Purpose | When to use |
|----------|---------|-------------|
| **Unit** | Test individual helper functions in isolation (sync check, save/restore, copy, guard) | Each new function gets dedicated unit tests |
| **Integration** | Test the full remote-mode merge flow end-to-end with a simulated origin | Step 2 integration tests verifying no history divergence, fail-fast behavior, and error recovery |
| **Regression** | Verify local-mode behavior is unchanged (except for the new dirty file check) | Existing tests continue to pass without modification |

#### Test Strategy {#test-strategy}

All tests use `tempfile::TempDir` for isolated git repos. Remote-mode tests create a bare repo as the "origin" and clone it to simulate a real remote setup. No tests require network access or a real GitHub remote.

**Dirty file check tests:** Create a repo with non-infrastructure dirty files, verify merge aborts with an error listing the files. Create a repo with only infrastructure dirty files, verify merge proceeds.

**Sync check tests:** Create a repo with a local bare origin, add a local-only commit to create divergence, verify the check catches it.

**Save/restore tests:** Create dirty infrastructure files in a repo, save them, verify temp directory contents, restore them, verify round-trip fidelity.

**Error recovery tests:** Save infra to temp, discard from working tree, simulate failure, verify TempDirGuard restores infra files to working tree with original content. Verify temp dir is also cleaned up.

**Guard tests:** Create a TempDirGuard, drop it without defusing, verify infra files are restored and temp dir is removed. Create another, defuse it, verify temp dir still exists and no restore happens.

**Integration test (remote-mode flow):** Set up a repo with bare origin, create a worktree branch, simulate the merge flow, verify `git rev-list --count origin/main..main` returns 0.

**Regression test (local-mode):** Existing `squash_merge_branch` and `prepare_main_for_merge` tests continue to pass — `prepare_main_for_merge()` is unchanged.

---

### 4.0.5 Execution Steps {#execution-steps}

#### Step 0: Add check_main_sync helper and unit tests {#step-0}

**Bead:** `specks-xuf.1`

**Commit:** `feat(merge): add check_main_sync helper for remote-mode pre-merge validation`

**References:** [D01] Fail-fast sync check, Spec S01, (#s01-check-main-sync, #current-flow, #fixed-flow, #semantics)

**Artifacts:**
- New function `check_main_sync()` in `crates/specks/src/commands/merge.rs`
- New unit tests for sync check

**Tasks:**
- [ ] Implement `check_main_sync(repo_root: &Path) -> Result<(), String>` per Spec S01: fetch origin/main, compare `rev-parse HEAD` vs `rev-parse origin/main`, return error if diverged
- [ ] Handle the case where `origin/main` does not exist (fetch fails) — return a descriptive error
- [ ] Error message must include the suggestion: "Push your local commits with 'git push origin main' before retrying"

**Tests:**
- [ ] Unit test: `test_check_main_sync_in_sync` — create repo with bare origin, verify sync passes when HEADs match
- [ ] Unit test: `test_check_main_sync_diverged` — add a local commit not pushed to origin, verify sync fails with error containing "git push origin main"
- [ ] Unit test: `test_check_main_sync_no_origin` — repo without remote, verify descriptive error about missing origin

**Checkpoint:**
- [ ] `cargo build 2>&1 | grep -c "warning" | grep "^0$"` (zero warnings)
- [ ] `cargo nextest run -p specks --filter-expr 'test(check_main_sync)'` passes

**Rollback:**
- Revert the commit; no other changes depend on this yet

**Commit after all checkpoints pass.**

---

#### Step 1: Add save/restore/copy helpers, TempDirGuard, and unit tests {#step-1}

**Depends on:** #step-0

**Bead:** `specks-xuf.2`

**Commit:** `feat(merge): add infrastructure temp save/restore, copy, and TempDirGuard helpers`

**References:** [D02] Temp directory, [D09] Restore infra on error, Spec S02, Spec S03, Spec S04, Spec S05, (#s02-save-infra-to-temp, #s03-restore-infra-from-temp, #s04-copy-infra-from-temp, #s05-temp-dir-guard, #fixed-flow, #semantics)

**Artifacts:**
- New function `save_infra_to_temp()` in `merge.rs`
- New function `restore_infra_from_temp()` in `merge.rs`
- New function `copy_infra_from_temp()` in `merge.rs`
- New struct `TempDirGuard` with `Drop` impl (restores then cleans up) and `defuse()` method in `merge.rs`
- Unit tests for all helpers and guard

**Tasks:**
- [ ] Implement `save_infra_to_temp(repo_root, infra_files)` per Spec S02 — creates temp dir with random suffix, copies files preserving relative paths, returns temp dir path
- [ ] Implement `restore_infra_from_temp(repo_root, temp_dir, infra_files, quiet)` per Spec S03 — copies files back, stages, commits with message "chore: post-merge infrastructure sync", cleans up temp dir. Used on the happy path.
- [ ] Implement `copy_infra_from_temp(repo_root, temp_dir, infra_files)` per Spec S04 — copies files back without staging or committing. Used for error recovery.
- [ ] Implement `TempDirGuard` per Spec S05 — struct storing `Option<PathBuf>` (temp path), `PathBuf` (repo root), `Vec<String>` (infra files). Drop calls `copy_infra_from_temp()` then `remove_dir_all`. `defuse()` sets temp path to `None`.
- [ ] Ensure directory structure is created for nested infrastructure files (e.g., `.specks/archive/`)

**Tests:**
- [ ] Unit test: `test_save_infra_to_temp_copies_files` — create repo with dirty `.specks/` and `.beads/` files, save to temp, verify temp dir contains files with correct relative paths and matching content
- [ ] Unit test: `test_restore_infra_from_temp_restores_and_commits` — create repo, save infra files, restore them, verify `git log` shows the infrastructure sync commit and file contents match
- [ ] Unit test: `test_copy_infra_from_temp_no_commit` — save infra files, copy them back via `copy_infra_from_temp`, verify files are in working tree but NOT staged or committed
- [ ] Unit test: `test_save_restore_round_trip` — save then restore, verify `git diff HEAD~1` shows only the expected infrastructure files
- [ ] Unit test: `test_save_infra_nested_dirs` — verify files in `.specks/archive/` are saved and restored with correct directory structure
- [ ] Unit test: `test_temp_dir_guard_restores_and_cleans_on_drop` — save infra to temp, discard from working tree, create `TempDirGuard`, drop it without defusing, verify infra files are back in working tree with original content AND temp dir is removed
- [ ] Unit test: `test_temp_dir_guard_defuse` — create `TempDirGuard`, call `defuse()`, drop it, verify no restore happens and temp dir still exists

**Checkpoint:**
- [ ] `cargo build 2>&1 | grep -c "warning" | grep "^0$"` (zero warnings)
- [ ] `cargo nextest run -p specks --filter-expr 'test(infra_to_temp) | test(infra_from_temp) | test(save_restore) | test(temp_dir_guard)'` passes

**Rollback:**
- Revert the commit; Step 0 helper remains intact

**Commit after all checkpoints pass.**

---

#### Step 2: Restructure merge flow with pre-dry-run checks, error recovery, and remote-mode overhaul {#step-2}

**Depends on:** #step-0, #step-1

**Bead:** `specks-xuf.3`

**Commit:** `fix(merge): fail fast on dirty non-infra files; prevent history divergence in remote mode`

**References:** [D01] Fail-fast sync check, [D02] Temp directory, [D03] Fail fast on non-infra dirty files, [D04] FF-only pull, [D05] Bypass prepare_main_for_merge, [D06] MergeData sync error, [D07] Auto-push infrastructure commit, [D08] Checks run during dry-run, [D09] Restore infra on error, Table T01, Spec S01, Spec S02, Spec S03, Spec S04, Spec S05, (#fixed-flow, #fixed-local-flow, #functions-to-modify, #t01-functions, #semantics)

**Artifacts:**
- Modified `run_merge()` — pre-dry-run checks, non-infra dirty file check (both modes), restructured remote-mode path with error recovery in `merge.rs`
- `prepare_main_for_merge()` unchanged (only called by local mode, now only sees infra dirty files)

**Tasks:**
- [ ] Insert pre-merge checks between mode detection (line ~507) and the dry-run return (line ~512). Position: AFTER `effective_mode` is determined, BEFORE `if dry_run { ... return }`. Both checks run in dry-run AND actual merge mode.
- [ ] First check (remote mode only): call `check_main_sync(&repo_root)`. On failure, return `MergeData { status: "error", error: sync_error_msg }` per [D06] and [D08]. In dry-run mode, this is the same `MergeData` error return used for worktree-not-found.
- [ ] Second check (both modes): call `get_dirty_files()`, partition into infra/non-infra. If non-infra list is non-empty, return `MergeData { status: "error", error: dirty_file_msg }` per [D03] and [D08]. Same error return pattern for both dry-run and actual merge.
- [ ] The dry-run return now happens AFTER both checks pass. The `dirty_files` field in dry-run `MergeData` reports only infrastructure dirty files (non-infra are blocked at the gate)
- [ ] After the dry-run return (actual merge path), if there are infra files in remote mode, call `save_infra_to_temp(&repo_root, &infra_files)` and wrap the result in `TempDirGuard::new(temp_dir, repo_root, infra_files)`
- [ ] Discard only infrastructure dirty files from the working tree: run `git checkout -- <infra-path1> <infra-path2> ...` for tracked files and `git clean -f -- <infra-path1> <infra-path2> ...` for untracked infra files
- [ ] After `gh pr merge --squash`, if it fails: let the `?` or explicit error return cause the function to exit — TempDirGuard is still in scope, so its `Drop` impl fires, calling `copy_infra_from_temp()` to copy infra files back to the working tree, then `remove_dir_all` to clean up temp. The error message MUST include "Working tree has been restored to pre-merge state." per [D09]. This is critical: without the restore, the temp dir (the ONLY copy of uncommitted infra modifications) would be deleted, causing data loss.
- [ ] After `gh pr merge --squash` succeeds, replace `git pull origin main` with `git pull --ff-only origin main`. On failure: same guard-based restore mechanism — TempDirGuard's Drop copies infra files back and cleans up temp. Return error with "Working tree has been restored to pre-merge state." per [D09]
- [ ] After successful pull, call `restore_infra_from_temp(&repo_root, &temp_dir, &infra_files, quiet)`, then call `guard.defuse()` to prevent the Drop from running
- [ ] After successful restore and commit, run `git push origin main` per [D07]. If push fails, print warning to stderr but do not return error
- [ ] In the local-mode path, the non-infra dirty file check already happened (before dry-run return). Call `prepare_main_for_merge(&repo_root, quiet)` unchanged per [D05]
- [ ] On all error paths after `save_infra_to_temp()`, ensure `TempDirGuard` is in scope so its `Drop` impl restores infra files and cleans up the temp directory

**Tests:**
- [ ] Unit test: `test_merge_rejects_non_infra_dirty_files` — create repo with dirty `src/main.rs`, verify `run_merge()` (or the check logic) returns error listing the file and containing "Commit or stash"
- [ ] Unit test: `test_merge_allows_infra_only_dirty_files` — create repo with only `.specks/` dirty files, verify the dirty file check passes (no error)
- [ ] Unit test: `test_dry_run_surfaces_dirty_file_error` — create repo with dirty non-infra file, call merge with `dry_run=true`, verify error is returned (not a clean preview)
- [ ] Unit test: `test_dry_run_surfaces_sync_check_error` — create repo with bare origin, add local-only commit, call merge with `dry_run=true` in remote mode, verify sync check error is returned
- [ ] Integration test: `test_remote_merge_sync_check_blocks_diverged` — set up repo with bare origin, add local-only commit, verify `run_merge()` returns error containing "git push origin main"
- [ ] Integration test: `test_remote_merge_flow_no_divergence` — set up repo with bare origin and worktree branch, simulate the full remote-mode flow (mock `gh pr merge` by merging locally on the bare repo), verify `git rev-list --count origin/main..main` is 0
- [ ] Integration test: `test_infra_restored_on_merge_failure` — create repo with dirty infra files, save to temp, discard from working tree (confirming files are gone), simulate `gh pr merge` failure causing TempDirGuard to drop, verify: (a) infra files are back in working tree with their original pre-merge content, (b) files are NOT staged (they appear as dirty, same as before merge attempt), (c) temp directory has been cleaned up
- [ ] Integration test: `test_infra_restored_on_pull_failure` — same setup as above but simulate `git pull --ff-only` failure (e.g., by creating a non-fast-forwardable state on the bare origin after merge). Verify same restore guarantees: infra files back in working tree, not staged, temp cleaned up
- [ ] Unit test: `test_targeted_infra_discard` — create repo with dirty infra files and clean non-infra files, discard only infra files, verify non-infra files are untouched and infra files are reverted

**Checkpoint:**
- [ ] `cargo build 2>&1 | grep -c "warning" | grep "^0$"` (zero warnings)
- [ ] `cargo nextest run -p specks` — all tests pass (existing + new)

**Rollback:**
- Revert the commit. Steps 0 and 1 helpers would need `#[allow(dead_code)]` annotations to compile under `-D warnings`, or revert those commits too.

**Commit after all checkpoints pass.**

---

### 4.0.6 Deliverables and Checkpoints {#deliverables}

**Deliverable:** The `specks merge` command refuses non-infrastructure dirty files (both modes), restores the working tree on error, and no longer causes git history divergence in remote mode.

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

- [ ] `specks merge` with non-infrastructure dirty files (either mode): produces an error listing the files and suggesting `git stash` or `git commit`
- [ ] `specks merge --dry-run` with non-infra dirty files or diverged main: produces the same errors (checks run during dry-run)
- [ ] `specks merge` in remote mode: after merge, `git log --oneline origin/main..main` shows 0 commits (infra commit pushed)
- [ ] `specks merge` in remote mode with diverged local main: fails with actionable error message
- [ ] `specks merge` in remote mode with merge/pull failure after infra discard: working tree is restored to pre-merge state
- [ ] `specks merge` in local mode: behavior is identical to before this change except for the new non-infra dirty file check
- [ ] All existing tests pass (`cargo nextest run`)
- [ ] New tests pass for dirty file check, sync check, save/restore, error recovery, guard, and full flow integration
- [ ] Zero warnings in `cargo build`

**Acceptance tests:**
- [ ] Unit test: non-infra dirty files rejected with actionable message (`test_merge_rejects_non_infra_dirty_files`)
- [ ] Unit test: dry-run surfaces errors (`test_dry_run_surfaces_dirty_file_error`)
- [ ] Integration test: full remote-mode merge flow produces clean git state (`test_remote_merge_flow_no_divergence`)
- [ ] Integration test: infra files restored on merge failure (`test_infra_restored_on_merge_failure`)
- [ ] Unit test: sync check rejects diverged state (`test_check_main_sync_diverged`)
- [ ] Unit test: save/restore round-trip preserves file contents (`test_save_restore_round_trip`)
- [ ] Unit test: guard restores and cleans up on drop (`test_temp_dir_guard_restores_and_cleans_on_drop`)

#### Roadmap / Follow-ons (Explicitly Not Required for Phase Close) {#roadmap}

- [ ] Handle `origin/main` being ahead of local (auto-pull before merge)
- [ ] Add `--force` flag to skip sync check and dirty file check for advanced users (also addresses branch protection scenario)
- [ ] Consider using `git stash` as an alternative to temp directory for infrastructure files

| Checkpoint | Verification |
|------------|--------------|
| Zero warnings | `cargo build 2>&1 \| grep warning \| wc -l` returns 0 |
| All tests pass | `cargo nextest run` exits 0 |
| Error recovery works | `test_infra_restored_on_merge_failure` and `test_temp_dir_guard_restores_and_cleans_on_drop` pass |
| Dirty files rejected | `test_merge_rejects_non_infra_dirty_files` passes |
| Dry-run checks work | `test_dry_run_surfaces_dirty_file_error` and `test_dry_run_surfaces_sync_check_error` pass |

**Commit after all checkpoints pass.**