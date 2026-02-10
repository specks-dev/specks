## Phase 1.0: Fix Worktree Lifecycle Fragility {#phase-worktree-lifecycle}

**Purpose:** Move session and artifact storage outside worktrees to eliminate `--force` requirements in cleanup and make worktree operations idempotent and robust.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | specks team |
| Status | draft |
| Target branch | main |
| Tracking issue/PR | |
| Last updated | 2026-02-09 |
| Beads Root | |
| Beads Root | `specks-tyo` |

---

### Phase Overview {#phase-overview}

#### Context {#context}

The current worktree lifecycle has fragility issues:

1. **Session files inside worktrees**: `session.json` lives at `{worktree_path}/.specks/session.json`, making it untracked content that requires `git worktree remove --force` to clean up.

2. **Artifact files inside worktrees**: Step artifacts (`architect-output.json`, `coder-output.json`, etc.) accumulate as untracked files in `{worktree_path}/.specks/step-artifacts/`.

3. **Non-idempotent worktree creation**: If a worktree already exists for a speck, `specks worktree create` fails rather than reusing the existing worktree.

4. **Merge command worktree selection**: When multiple worktrees exist for the same speck, the merge command prefers the one with an open PR, but worktree creation doesn't offer reuse.

Moving session and artifact storage to `.specks-worktrees/.sessions/` and `.specks-worktrees/.artifacts/` solves these problems by keeping orchestration data outside the git-managed worktree.

#### Strategy {#strategy}

- Create centralized session and artifact directories outside worktrees
- Generate session IDs that link back to worktree paths
- Update session schema to include worktree_path as explicit field (already present, but now the file lives outside)
- Update all session read/write functions to use new locations
- Update worktree removal to clean orchestration files first, eliminating need for `--force`
- Add `--reuse-existing` flag for idempotent worktree creation
- Maintain backward compatibility by checking both old and new locations during migration period

#### Stakeholders / Primary Customers {#stakeholders}

1. Specks implementer skill users who need robust worktree lifecycle
2. Specks merge command users who need clean worktree removal

#### Success Criteria (Measurable) {#success-criteria}

- `git worktree remove` succeeds without `--force` after session storage migration (verify via test)
- `specks worktree create --reuse-existing` returns existing worktree when one exists (verify via integration test)
- `specks merge` cleanup step succeeds without warnings about untracked files (verify via test)
- Existing sessions continue to work during migration period (verify via backward compatibility test)

#### Scope {#scope}

1. Relocate session.json storage to `.specks-worktrees/.sessions/<session-id>.json`
2. Relocate step artifacts to `.specks-worktrees/.artifacts/<session-id>/step-N/`
3. Add `--reuse-existing` flag to `specks worktree create`
4. Update `worktree_remove()` to clean orchestration files before git removal
5. Update implementer skill to use new session locations
6. Update setup-agent to handle new storage layout

#### Non-goals (Explicitly out of scope) {#non-goals}

- Migrating existing sessions automatically (users can manually delete and recreate)
- Adding session persistence across worktree recreation (sessions are ephemeral)
- Changing bead storage location (beads remain in `.beads/`)

#### Dependencies / Prerequisites {#dependencies}

- Existing worktree module in `crates/specks-core/src/worktree.rs`
- Existing session module in `crates/specks-core/src/session.rs`

#### Constraints {#constraints}

- Must maintain backward compatibility for at least one release cycle
- Session files must be discoverable by worktree path lookup
- Session IDs must be unique and sortable by timestamp

#### Assumptions {#assumptions}

- Sessions are ephemeral orchestration data, not critical state
- Users accept that old-format sessions may not be automatically migrated
- The `.specks-worktrees/` directory is already gitignored

---

### 1.0.0 Design Decisions {#design-decisions}

#### [D01] Session Storage Location (DECIDED) {#d01-session-storage}

**Decision:** Store sessions at `.specks-worktrees/.sessions/<session-id>.json`

**Rationale:**
- Keeps orchestration data outside git-managed worktree
- Eliminates need for `--force` on worktree removal
- Session ID provides unique identifier for lookup
- Location is already inside gitignored `.specks-worktrees/`

**Implications:**
- Session must store worktree_path explicitly (already does)
- Need index/lookup function to find session by worktree path
- Need cleanup function to delete session file when worktree is removed

#### [D02] Session ID Format (DECIDED) {#d02-session-id-format}

**Decision:** Session ID format is `<speck-slug>-<timestamp>` matching the worktree directory name convention

**Rationale:**
- Timestamp ensures uniqueness
- Speck slug provides human-readable identification
- Matches existing worktree directory naming convention
- Sortable chronologically

**Implications:**
- Session ID can be derived from worktree path: `specks__<slug>-<timestamp>` -> `<slug>-<timestamp>`
- Multiple sessions for same speck are distinguishable by timestamp

#### [D03] Artifact Storage Location (DECIDED) {#d03-artifact-storage}

**Decision:** Store artifacts at `.specks-worktrees/.artifacts/<session-id>/step-N/`

**Rationale:**
- Keeps step artifacts with session data
- Same cleanup lifecycle as session
- Organized by session and step for easy lookup

**Implications:**
- Create artifact directory structure when session is created
- Clean up entire artifact directory when session is removed

#### [D04] Cleanup Policy (DECIDED) {#d04-cleanup-policy}

**Decision:** Auto-delete session and artifacts when worktree is removed

**Rationale:**
- Sessions are ephemeral orchestration data
- No need to preserve after worktree is gone
- Simplifies cleanup logic

**Implications:**
- `worktree_remove()` must clean orchestration files first
- No separate garbage collection needed

#### [D05] Reuse Behavior (DECIDED) {#d05-reuse-behavior}

**Decision:** With `--reuse-existing` flag, prefer most recent worktree (highest timestamp in directory name)

**Rationale:**
- Most recent worktree is most likely to be the active implementation
- Consistent with merge command's "prefer open PR" heuristic
- Timestamp in directory name provides natural ordering

**Implications:**
- `create_worktree()` needs to scan existing worktrees
- Return existing session if match found and flag is set
- Default behavior (without flag) remains error on existing worktree

#### [D06] Backward Compatibility (DECIDED) {#d06-backward-compatibility}

**Decision:** Check both old and new session locations during a migration period

**Rationale:**
- Users may have in-progress sessions in old format
- Graceful degradation better than hard failure
- Can remove old-location check in future release

**Implications:**
- `load_session()` checks new location first, falls back to old
- `save_session()` always writes to new location
- Document migration in release notes

---

### 1.0.1 Specification {#specification}

#### 1.0.1.1 Session Storage Schema {#session-storage-schema}

**Spec S01: Session File Paths** {#s01-session-paths}

| Item | Path Pattern |
|------|--------------|
| Session directory | `.specks-worktrees/.sessions/` |
| Session file | `.specks-worktrees/.sessions/<session-id>.json` |
| Artifact directory | `.specks-worktrees/.artifacts/<session-id>/` |
| Step artifact directory | `.specks-worktrees/.artifacts/<session-id>/step-N/` |
| Architect output | `.specks-worktrees/.artifacts/<session-id>/step-N/architect-output.json` |
| Coder output | `.specks-worktrees/.artifacts/<session-id>/step-N/coder-output.json` |
| Reviewer output | `.specks-worktrees/.artifacts/<session-id>/step-N/reviewer-output.json` |
| Committer output | `.specks-worktrees/.artifacts/<session-id>/step-N/committer-output.json` |

**Spec S02: Session ID Derivation** {#s02-session-id}

Session ID is derived from worktree directory name:
```
Worktree: .specks-worktrees/specks__auth-20260208-143022/
Session ID: auth-20260208-143022
```

Formula: Strip `specks__` prefix from worktree directory basename.

**Spec S03: Session Lookup by Worktree** {#s03-session-lookup}

To find session for a worktree path:
1. Derive session ID from worktree directory name
2. Check `.specks-worktrees/.sessions/<session-id>.json`
3. If not found and backward-compat enabled, check `<worktree>/.specks/session.json`

#### 1.0.1.2 API Surface {#api-surface}

**Spec S04: Updated Session Functions** {#s04-session-functions}

```rust
/// Load session from external storage or worktree (backward compat)
pub fn load_session(worktree_path: &Path) -> Result<Session, SpecksError>;

/// Save session to external storage
pub fn save_session(session: &Session) -> Result<(), SpecksError>;

/// Delete session and artifacts
pub fn delete_session(session_id: &str, repo_root: &Path) -> Result<(), SpecksError>;

/// Derive session ID from worktree path
pub fn session_id_from_worktree(worktree_path: &Path) -> Option<String>;

/// Get sessions directory path
pub fn sessions_dir(repo_root: &Path) -> PathBuf;

/// Get artifacts directory for a session
pub fn artifacts_dir(repo_root: &Path, session_id: &str) -> PathBuf;
```

**Spec S05: Updated Worktree Functions** {#s05-worktree-functions}

```rust
/// Configuration for worktree creation
pub struct WorktreeConfig {
    pub speck_path: PathBuf,
    pub base_branch: String,
    pub repo_root: PathBuf,
    pub reuse_existing: bool,  // NEW: if true, return existing worktree
}

/// Create or reuse worktree
pub fn create_worktree(config: &WorktreeConfig) -> Result<Session, SpecksError>;

/// Remove worktree and clean orchestration files
pub fn remove_worktree(worktree_path: &Path, repo_root: &Path) -> Result<(), SpecksError>;
```

#### 1.0.1.3 CLI Changes {#cli-changes}

**Spec S06: worktree create Flag** {#s06-worktree-create-flag}

```bash
# Existing behavior: fail if worktree exists
specks worktree create .specks/specks-3.md

# New flag: reuse existing worktree if found
specks worktree create --reuse-existing .specks/specks-3.md
```

When `--reuse-existing` is set and a worktree exists for the speck:
- Return the existing worktree's session data
- Do not create a new worktree
- Output indicates reuse: `"reused": true`

---

### 1.0.2 Symbol Inventory {#symbol-inventory}

#### 1.0.2.1 New Functions {#new-functions}

| Symbol | Kind | Location | Notes |
|--------|------|----------|-------|
| `session_id_from_worktree` | fn | `session.rs` | Derive session ID from worktree path |
| `sessions_dir` | fn | `session.rs` | Get sessions directory path |
| `artifacts_dir` | fn | `session.rs` | Get artifacts directory for session |
| `delete_session` | fn | `session.rs` | Delete session file and artifacts |
| `remove_worktree` | fn | `worktree.rs` | Remove worktree with orchestration cleanup |

#### 1.0.2.2 Modified Types {#modified-types}

| Symbol | Kind | Location | Notes |
|--------|------|----------|-------|
| `WorktreeConfig` | struct | `worktree.rs` | Add `reuse_existing: bool` field |

---

### 1.0.3 Test Plan Concepts {#test-plan-concepts}

#### Test Categories {#test-categories}

| Category | Purpose | When to use |
|----------|---------|-------------|
| **Unit** | Test session path derivation, ID generation | Core logic |
| **Integration** | Test worktree create/remove with new storage | End-to-end |
| **Backward Compat** | Test old-location fallback | Migration period |

---

### 1.0.4 Execution Steps {#execution-steps}

#### Step 0: Add Session Path Helper Functions {#step-0}

**Bead:** `specks-tyo.1`

**Commit:** `feat(session): add helper functions for external session storage paths`

**References:** [D01] Session Storage Location, [D02] Session ID Format, Spec S01, Spec S02, (#session-storage-schema)

**Artifacts:**
- New functions in `crates/specks-core/src/session.rs`

**Tasks:**
- [ ] Add `session_id_from_worktree(worktree_path: &Path) -> Option<String>` that extracts session ID from worktree path
- [ ] Add `sessions_dir(repo_root: &Path) -> PathBuf` returning `.specks-worktrees/.sessions/`
- [ ] Add `artifacts_dir(repo_root: &Path, session_id: &str) -> PathBuf` returning `.specks-worktrees/.artifacts/<session-id>/`
- [ ] Add `session_file_path(repo_root: &Path, session_id: &str) -> PathBuf` returning full session file path

**Tests:**
- [ ] Unit test: `session_id_from_worktree` extracts correct ID from various path formats
- [ ] Unit test: `sessions_dir` returns correct path
- [ ] Unit test: `artifacts_dir` returns correct path for given session ID

**Checkpoint:**
- [ ] `cargo nextest run -p specks-core session_id`
- [ ] `cargo build --all`

**Rollback:**
- Revert commit, functions are additive

**Commit after all checkpoints pass.**

---

#### Step 1: Update Session Load/Save for External Storage {#step-1}

**Depends on:** #step-0

**Bead:** `specks-tyo.2`

**Commit:** `feat(session): support external session storage with backward compatibility`

**References:** [D01] Session Storage Location, [D06] Backward Compatibility, Spec S03, Spec S04, (#api-surface)

**Artifacts:**
- Modified `load_session()` and `save_session()` in `crates/specks-core/src/session.rs`

**Tasks:**
- [ ] Modify `load_session()` signature to take both `worktree_path` and optional `repo_root`
- [ ] Implement new load logic: check external location first, fall back to worktree location
- [ ] Modify `save_session()` to write to external location using `session.worktree_path` to derive session ID
- [ ] Add `save_session()` parameter for `repo_root` to locate external storage
- [ ] Ensure session directory is created if it doesn't exist

**Tests:**
- [ ] Unit test: `load_session` finds session at new external location
- [ ] Unit test: `load_session` falls back to old worktree location if external not found
- [ ] Unit test: `save_session` writes to external location
- [ ] Unit test: `save_session` creates sessions directory if needed

**Checkpoint:**
- [ ] `cargo nextest run -p specks-core load_session`
- [ ] `cargo nextest run -p specks-core save_session`
- [ ] `cargo build --all`

**Rollback:**
- Revert commit, restore original load/save functions

**Commit after all checkpoints pass.**

---

#### Step 2: Add delete_session and Orchestration Cleanup {#step-2}

**Depends on:** #step-1

**Bead:** `specks-tyo.3`

**Commit:** `feat(session): add delete_session with artifact cleanup`

**References:** [D03] Artifact Storage Location, [D04] Cleanup Policy, Spec S01, (#api-surface)

**Artifacts:**
- New `delete_session()` function in `crates/specks-core/src/session.rs`

**Tasks:**
- [ ] Implement `delete_session(session_id: &str, repo_root: &Path) -> Result<(), SpecksError>`
- [ ] Delete session file from `.specks-worktrees/.sessions/<session-id>.json`
- [ ] Delete entire artifacts directory `.specks-worktrees/.artifacts/<session-id>/`
- [ ] Handle case where files/directories don't exist (not an error)

**Tests:**
- [ ] Unit test: `delete_session` removes session file
- [ ] Unit test: `delete_session` removes artifacts directory recursively
- [ ] Unit test: `delete_session` succeeds even if files don't exist

**Checkpoint:**
- [ ] `cargo nextest run -p specks-core delete_session`
- [ ] `cargo build --all`

**Rollback:**
- Revert commit, function is additive

**Commit after all checkpoints pass.**

---

#### Step 3: Update worktree_remove to Clean Orchestration First {#step-3}

**Depends on:** #step-2

**Bead:** `specks-tyo.4`

**Commit:** `feat(worktree): clean orchestration files before git worktree remove`

**References:** [D04] Cleanup Policy, Spec S05, (#api-surface)

**Artifacts:**
- Modified `worktree_remove()` in `crates/specks-core/src/worktree.rs`
- Removal of `--force` flag from git worktree remove

**Tasks:**
- [ ] Add `remove_worktree(worktree_path: &Path, repo_root: &Path)` function
- [ ] Extract session ID from worktree path
- [ ] Call `delete_session()` to clean orchestration files
- [ ] Also clean legacy location: `rm -rf {worktree_path}/.specks/session.json` and `{worktree_path}/.specks/step-artifacts/`
- [ ] Update `GitCli::worktree_remove()` to NOT use `--force` flag
- [ ] Call new `remove_worktree()` instead of `worktree_remove()` in cleanup paths

**Tests:**
- [ ] Integration test: `remove_worktree` succeeds without `--force` when orchestration files are cleaned
- [ ] Integration test: Worktree with only session.json at old location is cleaned properly
- [ ] Integration test: Worktree with session at new location is cleaned properly

**Checkpoint:**
- [ ] `cargo nextest run -p specks-core worktree_remove`
- [ ] `cargo build --all`

**Rollback:**
- Revert commit, restore `--force` flag

**Commit after all checkpoints pass.**

---

#### Step 4: Add reuse_existing Flag to WorktreeConfig {#step-4}

**Depends on:** #step-3

**Bead:** `specks-tyo.5`

**Commit:** `feat(worktree): add reuse_existing flag for idempotent worktree creation`

**References:** [D05] Reuse Behavior, Spec S05, Spec S06, (#cli-changes)

**Artifacts:**
- Modified `WorktreeConfig` struct in `crates/specks-core/src/worktree.rs`
- Modified `create_worktree()` function

**Tasks:**
- [ ] Add `reuse_existing: bool` field to `WorktreeConfig` struct
- [ ] Modify `create_worktree()` to check for existing worktree when `reuse_existing` is true
- [ ] Use `list_worktrees()` to find matching speck, prefer most recent by timestamp
- [ ] Return existing session if found, with `reused: true` indicator
- [ ] Update Session struct to include `reused: bool` field for output reporting

**Tests:**
- [ ] Unit test: `create_worktree` with `reuse_existing: true` returns existing worktree
- [ ] Unit test: `create_worktree` with `reuse_existing: true` and no existing worktree creates new one
- [ ] Unit test: `create_worktree` with `reuse_existing: false` (default) fails if worktree exists

**Checkpoint:**
- [ ] `cargo nextest run -p specks-core create_worktree`
- [ ] `cargo build --all`

**Rollback:**
- Revert commit, struct change is backward compatible

**Commit after all checkpoints pass.**

---

#### Step 5: Update CLI worktree create Command {#step-5}

**Depends on:** #step-4

**Bead:** `specks-tyo.6`

**Commit:** `feat(cli): add --reuse-existing flag to worktree create command`

**References:** [D05] Reuse Behavior, Spec S06, (#cli-changes)

**Artifacts:**
- Modified `crates/specks/src/commands/worktree.rs`

**Tasks:**
- [ ] Add `--reuse-existing` flag to clap argument definition
- [ ] Pass flag value to `WorktreeConfig` when creating worktree
- [ ] Update JSON output to include `reused: true` when worktree was reused
- [ ] Update text output to indicate reuse: "Reusing existing worktree: ..."

**Tests:**
- [ ] Integration test: `specks worktree create --reuse-existing` with existing worktree succeeds
- [ ] Integration test: `specks worktree create --reuse-existing` without existing worktree creates new one
- [ ] Integration test: `specks worktree create` (without flag) with existing worktree fails

**Checkpoint:**
- [ ] `cargo nextest run worktree_create`
- [ ] `specks worktree create --help` shows `--reuse-existing` flag
- [ ] `cargo build --all`

**Rollback:**
- Revert commit, flag is additive

**Commit after all checkpoints pass.**

---

#### Step 6: Update Implementer Skill for New Session Locations {#step-6}

**Depends on:** #step-5

**Bead:** `specks-tyo.7`

**Commit:** `docs(skill): update implementer skill for external session storage`

**References:** [D01] Session Storage Location, [D03] Artifact Storage Location, Spec S01, (#session-storage-schema)

**Artifacts:**
- Modified `skills/implementer/SKILL.md`

**Tasks:**
- [ ] Update "Worktree Structure" section to show new artifact locations
- [ ] Update session.json path references from `{worktree_path}/.specks/session.json` to `.specks-worktrees/.sessions/<session-id>.json`
- [ ] Update step artifact paths from `{worktree_path}/.specks/step-artifacts/` to `.specks-worktrees/.artifacts/<session-id>/`
- [ ] Note that session ID is derived from worktree directory name
- [ ] Update error.json path to new artifacts location

**Tests:**
- [ ] Manual: Review skill documentation for correctness

**Checkpoint:**
- [ ] Read `skills/implementer/SKILL.md` and verify paths are updated

**Rollback:**
- Revert commit, documentation change

**Commit after all checkpoints pass.**

---

#### Step 7: Update Setup Agent for New Session Storage {#step-7}

**Depends on:** #step-6

**Bead:** `specks-tyo.8`

**Commit:** `docs(agent): update setup agent for external session storage`

**References:** [D01] Session Storage Location, [D05] Reuse Behavior, Spec S01, (#session-storage-schema)

**Artifacts:**
- Modified `agents/implementer-setup-agent.md`

**Tasks:**
- [ ] Update Phase 0 to note that session files are stored externally
- [ ] Add mention of `--reuse-existing` flag for idempotent worktree creation
- [ ] Update session path references in output examples
- [ ] Update artifact path references in behavior descriptions

**Tests:**
- [ ] Manual: Review agent documentation for correctness

**Checkpoint:**
- [ ] Read `agents/implementer-setup-agent.md` and verify paths are updated

**Rollback:**
- Revert commit, documentation change

**Commit after all checkpoints pass.**

---

#### Step 8: Update Merge Command for Clean Worktree Removal {#step-8}

**Depends on:** #step-3

**Bead:** `specks-tyo.9`

**Commit:** `refactor(merge): use remove_worktree for clean cleanup`

**References:** [D04] Cleanup Policy, Spec S05, (#api-surface)

**Artifacts:**
- Modified `crates/specks/src/commands/merge.rs`

**Tasks:**
- [ ] Import `remove_worktree` from specks-core
- [ ] Replace direct `git worktree remove --force` call with `remove_worktree()`
- [ ] Update error handling for new function signature
- [ ] Remove warning about untracked files since cleanup is handled

**Tests:**
- [ ] Integration test: `specks merge` cleanup succeeds without `--force` warnings
- [ ] Integration test: Verify session and artifacts are cleaned during merge

**Checkpoint:**
- [ ] `cargo nextest run merge`
- [ ] `cargo build --all`

**Rollback:**
- Revert commit, restore direct git call

**Commit after all checkpoints pass.**

---

### 1.0.5 Deliverables and Checkpoints {#deliverables}

**Deliverable:** Robust worktree lifecycle with external session storage, idempotent creation, and clean removal without `--force`.

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

- [ ] `git worktree remove` succeeds without `--force` after implementation (`cargo nextest run -p specks-core worktree_remove`)
- [ ] `specks worktree create --reuse-existing` reuses existing worktree (`cargo nextest run worktree_create`)
- [ ] `specks merge` cleanup step succeeds without warnings (`cargo nextest run merge`)
- [ ] Backward compatibility: old sessions can still be loaded (`cargo nextest run -p specks-core load_session`)
- [ ] All tests pass: `cargo nextest run`

**Acceptance tests:**
- [ ] Integration test: Full workflow with new session storage
- [ ] Integration test: Backward compatibility with old session location

#### Roadmap / Follow-ons (Explicitly Not Required for Phase Close) {#roadmap}

- [ ] Remove backward compatibility check after one release cycle
- [ ] Add garbage collection for orphaned sessions/artifacts
- [ ] Add `specks worktree status` command showing session details