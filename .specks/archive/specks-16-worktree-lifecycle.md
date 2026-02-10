# Specks Worktree Lifecycle: Architectural Analysis and Proposal

**Date:** 2026-02-09
**Author:** Code Architect
**Status:** PROPOSAL

---

## 1. Current State Analysis

### 1.1 What Exists

The specks worktree system consists of:

| Component | Location | Purpose |
|-----------|----------|---------|
| `worktree.rs` | `crates/specks-core/src/` | Create, list, cleanup worktrees |
| `session.rs` | `crates/specks-core/src/` | Session state management |
| `merge.rs` | `crates/specks/src/commands/` | Merge workflow orchestration |
| `worktree.rs` | `crates/specks/src/commands/` | CLI subcommands |

**Directory Structure:**
```
.specks-worktrees/
  .sessions/
    <session-id>.json         # External session storage (new)
  .artifacts/
    <session-id>/             # Step artifacts (logs, strategies)
  specks__<slug>-<timestamp>/ # Worktree directories
    .git                      # File (not directory) pointing to main .git
    .specks/
      session.json            # Legacy internal storage (old)
```

### 1.2 What Works

1. **Worktree creation**: `specks worktree create` successfully creates isolated worktrees
2. **Session management**: External session storage works (`.sessions/<id>.json`)
3. **PR creation**: Implementer creates PRs via `gh pr create`
4. **Squash merge detection**: `is_pr_merged()` correctly uses GitHub API
5. **Atomic writes**: Session files use temp+fsync+rename pattern

### 1.3 What is Broken

#### Critical Issue 1: Session Discovery Mismatch

**Problem:** `find_worktree_for_speck()` in `merge.rs` only looks for sessions at `{worktree}/.specks/session.json` (internal storage), but the implementer now saves sessions to `.specks-worktrees/.sessions/{session-id}.json` (external storage).

**Evidence:**
```
$ specks worktree list --json
{
  "worktrees": [
    {
      "worktree_path": ".../specks__13-20250209-152616",
      "status": "pending",     <-- Found via load_session with external fallback
      ...
    }
  ]
}
```

But `specks merge` calls `find_worktree_for_speck()` which:
1. Scans worktree directories
2. Looks for `{worktree}/.specks/session.json` ONLY
3. Ignores external storage completely

**Code in merge.rs lines 147-151:**
```rust
let session_path = worktree_path.join(".specks").join("session.json");
if !session_path.exists() {
    continue;  // SKIPS worktrees with external sessions!
}
```

#### Critical Issue 2: Cleanup Cannot Find Merged Worktrees

**Problem:** `specks worktree cleanup --merged` returns "No merged worktrees to remove" even though multiple worktrees exist for merged PRs.

**Evidence:**
```
$ specks worktree list
Branch: specks/13-20250209-152616  Status: Pending
Branch: specks/14-20250209-172637  Status: Pending

$ gh pr view specks/13-20250209-152734 --json state
{"state":"MERGED"}   # PR #5 was merged!

$ specks worktree cleanup --merged --dry-run
No merged worktrees to remove
```

**Root Cause:** Only 2 worktrees are being discovered:
- `specks__13-20250209-152616` - has internal session, never got a PR
- `specks__14-20250209-172637` - has internal session, never got a PR

The worktrees that DO have merged PRs:
- `specks__13-20250209-152734` - has internal session, PR #5 MERGED
- `specks__14-20250209-172747` - has internal session, PR #6 MERGED
- `specks__15-20250210-024623` - has external session, PR #7 MERGED

But `list_worktrees()` is finding sessions via `load_session()` which checks external first, then internal. The problem is somewhere else...

Let me trace deeper. The issue is that `is_pr_merged()` returns `false` for branches without PRs, but then falls back to `git merge-base --is-ancestor`:

```rust
Ok(false) => {
    // PR doesn't exist or is not merged - fall back to git merge-base
    git.is_ancestor(&session.branch_name, &session.base_branch)
}
```

The branches `specks/13-20250209-152616` and `specks/14-20250209-172637` were never merged (no PR created), so `is_ancestor` correctly returns false.

The REAL issue: Why aren't `specks/13-20250209-152734` and `specks/14-20250209-172747` being cleaned up?

Looking at the list output again: only 2 worktrees are listed, not 5. Where are the other 3?

**Actual Issue:** `list_worktrees()` is skipping some worktrees because `load_session()` is failing to find their sessions.

#### Critical Issue 3: Orphaned Worktrees with No Session

Worktrees created before session external storage was added have sessions only in internal storage. But some worktrees may have been created and failed before session was written at all.

**Current State of Worktrees:**

| Worktree | Internal Session | External Session | PR | Status |
|----------|------------------|------------------|-----|--------|
| `specks__13-20250209-152616` | YES | NO | NONE | Orphaned |
| `specks__13-20250209-152734` | YES | NO | #5 MERGED | Should cleanup |
| `specks__14-20250209-172637` | YES | NO | NONE | Orphaned |
| `specks__14-20250209-172747` | YES | NO | #6 MERGED | Should cleanup |
| `specks__15-20250210-024623` | NO | YES | #7 MERGED | Should cleanup |

But `list_worktrees()` only found 2 worktrees (the ones with internal sessions that are Pending status).

#### Critical Issue 4: Stale Branches Without Worktrees

```
$ git branch | grep specks/
  specks/11-20250209-025927
  specks/11-20250209-030003
  specks/12-20250209-135556
  specks/12-20250209-135638
  ...
```

Multiple branches exist from previous implementer runs that failed before creating worktrees, or whose worktrees were manually deleted.

---

## 2. User Stories

### Story 1: Happy Path - Implementation Succeeds

1. User runs `/specks:implementer .specks/specks-N.md`
2. Worktree created at `.specks-worktrees/specks__N-<timestamp>/`
3. Session saved externally at `.sessions/N-<timestamp>.json`
4. All steps complete, PR created
5. User reviews and merges PR on GitHub (squash merge)
6. User runs `specks merge .specks/specks-N.md`
7. Merge command: commits infrastructure, merges PR, pulls, cleans up worktree
8. Everything is clean

### Story 2: Implementation Fails Partway

1. User runs `/specks:implementer .specks/specks-N.md`
2. Worktree created with timestamp T1
3. Fails at step 3 (test failures, etc.)
4. User runs `/specks:implementer .specks/specks-N.md` again
5. **Current behavior:** New worktree created with timestamp T2
6. Second run succeeds, creates PR
7. User merges PR
8. **Problem:** T1 worktree is now orphaned (never got a PR)
9. User runs `specks worktree cleanup --merged`
10. T1 is not cleaned up because it has no PR

### Story 3: User Abandons a Speck

1. User starts implementing specks-N
2. Creates worktree, makes progress
3. Decides to abandon (requirements changed, etc.)
4. User wants to clean up the abandoned worktree
5. **Current:** No command exists for this
6. User must manually: `git worktree remove`, `git branch -D`, delete session

### Story 4: Cleanup After Multiple Attempts

1. User tried implementing specks-N three times
2. First two failed (worktrees T1, T2 orphaned)
3. Third succeeded (T3), PR merged
4. User wants to clean up ALL THREE worktrees
5. `specks worktree cleanup --merged` only cleans T3
6. T1 and T2 remain forever

---

## 3. Edge Case Inventory

| # | Scenario | Current Behavior | Desired Behavior |
|---|----------|------------------|------------------|
| E1 | Worktree with merged PR | Detected if session found | Clean up worktree + branch |
| E2 | Worktree with open PR | Detected if session found | Skip (work in progress) |
| E3 | Worktree with closed PR (not merged) | Detected if session found | Skip or warn user |
| E4 | Worktree with no PR ever | Not cleaned by --merged | New: `--orphaned` flag |
| E5 | Worktree with external session only | **BUG: Not found by merge** | Fix session discovery |
| E6 | Worktree with internal session only | Found, but may fail cleanup | Support both locations |
| E7 | Branch exists without worktree | Never cleaned | New: `--stale-branches` flag |
| E8 | Worktree dir exists without session | Silently skipped | Report as "corrupted" |
| E9 | Multiple worktrees for same speck | Last one preferred | Clear ownership rules |
| E10 | Session in NeedsReconcile state | Blocked from cleanup | Require reconcile first |
| E11 | Worktree with uncommitted changes | `git worktree remove` fails | Warn and skip, or --force |
| E12 | Worktree path not in .specks-worktrees | Ignored | Validate all paths |

---

## 4. Proposed Solution

### 4.1 Fix Session Discovery in merge.rs

**Change:** Replace custom session discovery with `list_worktrees()` from specks-core.

```rust
// BEFORE (broken):
fn find_worktree_for_speck(...) {
    for entry in fs::read_dir(worktrees_dir) {
        let session_path = worktree_path.join(".specks").join("session.json");
        if !session_path.exists() { continue; }  // WRONG!
    }
}

// AFTER (correct):
fn find_worktree_for_speck(...) {
    let sessions = list_worktrees(repo_root)?;  // Uses load_session with fallback
    for session in sessions.filter(|s| s.speck_path == speck_path) {
        // ...
    }
}
```

### 4.2 Add Worktree Categories to Cleanup

**New flags for cleanup command:**

```bash
specks worktree cleanup --merged      # Clean worktrees with merged PRs
specks worktree cleanup --orphaned    # Clean worktrees with no PR
specks worktree cleanup --all         # Clean all finished worktrees
specks worktree cleanup --stale       # Clean branches without worktrees
```

**Logic for each category:**

| Flag | Selection Criteria | Actions |
|------|-------------------|---------|
| `--merged` | PR state = MERGED | Remove worktree, delete branch |
| `--orphaned` | No PR exists AND status != InProgress | Remove worktree, delete branch |
| `--closed` | PR state = CLOSED (not merged) | Remove worktree, delete branch |
| `--stale` | Branch exists, no worktree dir | Delete branch only |
| `--all` | Union of above | All cleanup actions |

### 4.3 Add Explicit Remove Command

For users who want to remove a specific worktree without checking PR status:

```bash
specks worktree remove <speck-or-branch>
specks worktree remove specks-14       # By speck name
specks worktree remove specks/14-...   # By branch name
specks worktree remove --force ...     # Force even with uncommitted changes
```

### 4.4 Add Doctor Subcommand for Diagnostics

```bash
specks worktree doctor

Output:
Worktree Health Report
======================

Found 5 worktrees:
  [MERGED]   specks__13-20250209-152734 - PR #5 merged 2h ago
  [MERGED]   specks__14-20250209-172747 - PR #6 merged 1h ago
  [MERGED]   specks__15-20250210-024623 - PR #7 merged 30m ago
  [ORPHANED] specks__13-20250209-152616 - No PR, status: Pending
  [ORPHANED] specks__14-20250209-172637 - No PR, status: Pending

Found 6 stale branches (no worktree):
  specks/11-20250209-025927
  specks/11-20250209-030003
  specks/12-20250209-135556
  specks/12-20250209-135638
  specks/14-20250209-172252
  specks/14-20250209-172346

Recommendations:
  Run: specks worktree cleanup --merged --dry-run
  Run: specks worktree cleanup --orphaned --dry-run
  Run: specks worktree cleanup --stale --dry-run
```

### 4.5 Session Storage Migration

**Problem:** Old worktrees have internal sessions, new ones have external.

**Solution:** Keep both fallback paths, but always write to external going forward.

```rust
// In load_session:
1. Try external: .sessions/{session-id}.json
2. Fallback to internal: {worktree}/.specks/session.json

// In save_session:
1. Always write to external: .sessions/{session-id}.json
2. (Optional) Delete internal if it exists after successful external write
```

---

## 5. State Diagram

```
                        +-----------+
                        | Not Found |
                        +-----+-----+
                              |
                    specks worktree create
                              |
                              v
          +---------+   +------------+
          | Pending |-->| InProgress |
          +---------+   +-----+------+
               ^              |
               |         (steps complete)
               |              |
               |              v
               |       +------------+     (bead close fails)    +--------------+
               |       | Completed  |-------------------------->| NeedsRecon   |
               |       +------+-----+                           +------+-------+
               |              |                                        |
               |         (PR created)                    specks session reconcile
               |              |                                        |
               |              v                                        v
               |       +-----------+                            +-----------+
               |       | PRCreated |--------------------------->| Completed |
               |       +-----+-----+                            +-----------+
               |             |
               |        (PR merged)
               |             |
               |             v
               |       +----------+
               +-------|  MERGED  |
                       +-----+----+
                             |
                    specks worktree cleanup --merged
                             |
                             v
                       +-----------+
                       |  REMOVED  |
                       +-----------+

Edge transitions:
  * Failed at any step --> Failed state
  * User abandons --> Orphaned (no PR)
  * specks worktree cleanup --orphaned removes Orphaned
```

---

## 6. CLI Interface

### 6.1 Current Commands

| Command | Purpose | Status |
|---------|---------|--------|
| `specks worktree create <speck>` | Create worktree | Working |
| `specks worktree list` | List worktrees | Mostly working |
| `specks worktree cleanup --merged` | Clean merged | **BROKEN** |
| `specks merge <speck>` | Full merge workflow | **BROKEN** |

### 6.2 Proposed Commands

| Command | Purpose | Priority |
|---------|---------|----------|
| `specks worktree list` | Show all worktrees with status | P0 (fix) |
| `specks worktree cleanup --merged` | Clean merged | P0 (fix) |
| `specks worktree cleanup --orphaned` | Clean orphaned | P1 (new) |
| `specks worktree cleanup --stale` | Clean stale branches | P1 (new) |
| `specks worktree cleanup --all` | Clean everything | P2 (new) |
| `specks worktree remove <id>` | Remove specific | P1 (new) |
| `specks worktree doctor` | Diagnostic report | P2 (new) |
| `specks merge <speck>` | Full merge workflow | P0 (fix) |

---

## 7. Implementation Plan

### Phase 1: Critical Fixes (P0)

**Step 1: Fix session discovery in merge.rs**

- Modify `find_worktree_for_speck()` to use `list_worktrees()`
- Remove duplicate session loading logic
- Add test for external session discovery

**Step 2: Fix cleanup detection**

- Ensure `list_worktrees()` finds all worktrees regardless of session location
- Add logging/debug output for worktrees skipped and why
- Add test with mixed internal/external sessions

**Step 3: Test end-to-end merge workflow**

- Create worktree, implement, create PR, merge PR, run `specks merge`
- Verify cleanup works correctly

### Phase 2: Orphaned Worktree Handling (P1)

**Step 4: Add `--orphaned` flag to cleanup**

- Detect worktrees where `is_pr_merged()` returns Err(no PR)
- Only clean if session status is not InProgress
- Add `--dry-run` support

**Step 5: Add `specks worktree remove` command**

- Allow explicit removal by speck name or branch name
- Require confirmation or `--force`
- Clean session, artifacts, worktree, and branch

### Phase 3: Diagnostics and Polish (P2)

**Step 6: Add `specks worktree doctor` command**

- Enumerate all worktrees with health status
- Find stale branches
- Provide actionable recommendations

**Step 7: Add `--stale` flag for branch cleanup**

- Find branches matching `specks/*` pattern without worktrees
- Delete only the branch (no worktree to remove)

---

## 8. Specific Code Changes Required

### File: crates/specks/src/commands/merge.rs

**Function: `find_worktree_for_speck()`**

```rust
// REPLACE lines 109-193 with:
fn find_worktree_for_speck(
    root: Option<&Path>,
    speck_path: &str,
) -> Result<(PathBuf, Session), String> {
    let base = root
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    // Use list_worktrees which properly handles both storage locations
    let sessions = list_worktrees(&base)
        .map_err(|e| format!("Failed to list worktrees: {}", e))?;

    // Normalize speck path for comparison
    let normalized = normalize_speck_path(speck_path);

    // Filter matching sessions
    let matching: Vec<_> = sessions
        .into_iter()
        .filter(|s| normalize_speck_path(&s.speck_path) == normalized)
        .collect();

    if matching.is_empty() {
        return Err(format!("No worktree found for speck: {}", speck_path));
    }

    // Prefer worktree with open PR
    for session in &matching {
        if let Ok(pr_info) = get_pr_for_branch(&session.branch_name) {
            if pr_info.state == "OPEN" {
                let path = PathBuf::from(&session.worktree_path);
                return Ok((path, session.clone()));
            }
        }
    }

    // Fall back to most recent
    let session = matching.into_iter().last().unwrap();
    let path = PathBuf::from(&session.worktree_path);
    Ok((path, session))
}
```

### File: crates/specks-core/src/worktree.rs

**Function: `cleanup_worktrees()`**

Add support for different cleanup modes:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupMode {
    Merged,      // Clean worktrees with merged PRs
    Orphaned,    // Clean worktrees with no PR
    Stale,       // Clean branches without worktrees
    All,         // All of the above
}

pub fn cleanup_worktrees(
    repo_root: &Path,
    mode: CleanupMode,
    dry_run: bool
) -> Result<CleanupResult, SpecksError> {
    // ...
}

pub struct CleanupResult {
    pub merged_removed: Vec<String>,
    pub orphaned_removed: Vec<String>,
    pub stale_branches_removed: Vec<String>,
}
```

### File: crates/specks/src/commands/worktree.rs

**Add WorktreeCommands::Remove variant:**

```rust
/// Remove a specific worktree
Remove {
    /// Worktree identifier (speck name, branch, or path)
    target: String,

    /// Force removal even with uncommitted changes
    #[arg(long)]
    force: bool,
}
```

---

## 9. Testing Strategy

### Unit Tests

1. `test_find_worktree_external_session` - Find worktree with external session only
2. `test_find_worktree_internal_session` - Find worktree with internal session only
3. `test_find_worktree_mixed_sessions` - External takes precedence
4. `test_cleanup_merged_pr` - Correctly detects and removes merged
5. `test_cleanup_orphaned` - Removes worktrees without PRs
6. `test_cleanup_skips_in_progress` - Does not remove active work
7. `test_stale_branch_detection` - Finds branches without worktrees

### Integration Tests

1. Full workflow: create -> implement -> PR -> merge -> cleanup
2. Failed implementation: create -> fail -> new attempt -> succeed -> cleanup both
3. Abandoned work: create -> abandon -> explicit remove

---

## 10. Summary

### Root Causes of Current Issues

1. **Session discovery mismatch**: `merge.rs` looks only in internal storage while implementer writes to external storage
2. **No orphaned worktree handling**: `--merged` flag requires a PR to exist
3. **No stale branch cleanup**: Branches from failed runs accumulate forever

### Key Changes Required

1. **Fix `find_worktree_for_speck()`** to use `list_worktrees()` which handles both storage locations
2. **Add `--orphaned` cleanup mode** for worktrees that never got PRs
3. **Add `--stale` cleanup mode** for branches without worktrees
4. **Add `specks worktree remove`** for explicit removal
5. **Add `specks worktree doctor`** for diagnostics

### Risk Assessment

| Risk | Mitigation |
|------|------------|
| Data loss from aggressive cleanup | Always default to `--dry-run`, require explicit flags |
| Breaking existing workflows | Maintain backward compatibility with internal session fallback |
| Cleanup during active work | Check session status, never clean InProgress |
| Network failures during PR check | Fall back to git merge-base, log warnings |

---

## Appendix: Current Repository State

### Worktrees (from filesystem)
```
.specks-worktrees/
  specks__13-20250209-152616/  (internal session, no PR, ORPHANED)
  specks__13-20250209-152734/  (internal session, PR #5 MERGED)
  specks__14-20250209-172637/  (internal session, no PR, ORPHANED)
  specks__14-20250209-172747/  (internal session, PR #6 MERGED)
  specks__15-20250210-024623/  (external session, PR #7 MERGED)
```

### Sessions (from .sessions/)
```
.specks-worktrees/.sessions/
  15-20250210-024623.json  (status: completed)
```

### Branches (from git)
```
  specks/11-20250209-025927  (STALE - no worktree)
  specks/11-20250209-030003  (STALE - no worktree)
  specks/12-20250209-135556  (STALE - no worktree)
  specks/12-20250209-135638  (STALE - no worktree)
+ specks/13-20250209-152616  (worktree exists)
+ specks/13-20250209-152734  (worktree exists)
+ specks/14-20250209-172637  (worktree exists)
+ specks/14-20250209-172747  (worktree exists)
  specks/14-20250209-181148  (STALE - no worktree)
+ specks/15-20250210-024623  (worktree exists)
```

### PRs (from GitHub)
```
#7  specks/15-20250210-024623  MERGED
#6  specks/14-20250209-172747  MERGED
#5  specks/13-20250209-152734  MERGED
#4  specks/12-20250209-135638  MERGED
#3  specks/11-20250209-030003  MERGED
#2  specks/10-20250209-013458  MERGED
#1  specks/9-20250208-230132   MERGED
```
