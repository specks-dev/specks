## Phase 1.0: Fix Worktree Lifecycle - Session Schema, Discovery, and Cleanup {#phase-worktree-lifecycle}

**Purpose:** Unify the incompatible session schemas, fix session discovery so `specks merge` and `specks worktree cleanup` work correctly, and add cleanup modes for orphaned worktrees and stale branches.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | specks team |
| Status | draft |
| Target branch | main |
| Tracking issue/PR | - |
| Last updated | 2026-02-10 |
| Beads Root | `specks-dqg` |

---

### Phase Overview {#phase-overview}

#### Context {#context}

The specks worktree system has three critical bugs that prevent normal cleanup workflows from functioning:

1. **Session schema incompatibility**: The `Session` struct in `session.rs` requires fields like `schema_version`, `speck_slug`, `current_step` (usize), and `total_steps`. But the implementer skill writes sessions with a different schema: `session_id`, `steps_completed`, `steps_remaining`, `current_step` (string anchor or null), `root_bead`, and `bead_mapping`. These are incompatible — `load_session()` silently fails to deserialize implementer-created sessions, making 3 of 5 existing worktrees invisible to `list_worktrees()` and all downstream commands.

2. **Session discovery mismatch in merge.rs**: The `find_worktree_for_speck()` function in `merge.rs` only looks for sessions at `{worktree}/.specks/session.json` (internal storage), but the implementer now saves sessions to `.specks-worktrees/.sessions/{session-id}.json` (external storage). This causes `specks merge` to fail with "No worktree found" even when a valid worktree exists.

3. **No cleanup path for orphaned worktrees or stale branches**: `specks worktree cleanup --merged` is the only cleanup mode. Worktrees from failed implementations (no PR ever created) accumulate forever. Branches from deleted worktrees persist indefinitely. Currently 7 stale branches and 2 orphaned worktrees exist in the repo.

#### Strategy {#strategy}

- **Step 0 (P0 prerequisite)**: Unify the Session struct to tolerate both the `specks worktree create` format and the implementer skill format, so `load_session()` can deserialize all existing sessions
- **Step 1 (P0)**: Fix `find_worktree_for_speck()` in `merge.rs` to use `list_worktrees()` from specks-core, which properly handles both session storage locations
- **Steps 2-3 (P1)**: Add `CleanupMode` enum, `--orphaned` flag, and `specks worktree remove` command — each step keeps both specks-core and CLI in sync to avoid compilation gaps
- **Steps 4-5 (P2)**: Add stale branch cleanup and extend `specks doctor` with worktree-specific diagnostics

#### Stakeholders / Primary Customers {#stakeholders}

1. Specks users who need to clean up after completing implementations
2. Specks maintainers who need reliable worktree lifecycle management

#### Success Criteria (Measurable) {#success-criteria}

- `specks worktree list` discovers all 5 existing worktrees regardless of session format (verified by running command)
- `specks merge .specks/specks-N.md` successfully finds worktrees with external sessions (verified by test)
- `specks worktree cleanup --merged` removes all worktrees with merged PRs (verified by test)
- `specks worktree cleanup --orphaned` removes worktrees without PRs that are not in progress (verified by test)
- `specks worktree cleanup --stale` removes `specks/*` branches without worktrees (verified by test)
- `specks doctor` reports stale branches and orphaned worktrees (verified by output inspection)

#### Scope {#scope}

1. Unify `Session` struct to accept both old and implementer session formats
2. Fix `find_worktree_for_speck()` in `merge.rs` to use `list_worktrees()`
3. Add `CleanupMode` enum with `--orphaned`, `--stale`, `--all` flags to cleanup command
4. Add `specks worktree remove` command for explicit removal
5. Extend `specks doctor` with worktree health checks (stale branches, orphaned worktrees, sessionless worktrees)

#### Non-goals (Explicitly out of scope) {#non-goals}

- Migrating existing internal sessions to external storage (both locations remain supported)
- Dropping support for the old session format (backward compatibility preserved)
- Custom branch patterns beyond `specks/*` (hardcoded)
- Automatic cleanup triggers (cleanup remains manual)
- `--closed` flag for PRs closed without merging (follow-on)

#### Dependencies / Prerequisites {#dependencies}

- `load_session()` in `session.rs` correctly handles external-first, internal-fallback storage paths
- `remove_worktree()` in `worktree.rs` handles both session storage locations and cleans up artifacts
- `is_pr_merged()` correctly uses GitHub API with `git merge-base` fallback
- `gh` CLI available for optimal PR status checks (graceful degradation when absent per D11)
- `tempfile` crate available for test fixtures (already in dev-dependencies)

#### Constraints {#constraints}

- Must maintain backward compatibility with old session format (`schema_version: "1"`)
- Must not break existing `specks merge` or `specks worktree cleanup` workflows
- Warnings are errors: all changes must compile cleanly with `-D warnings`
- NeedsReconcile sessions must never block cleanup (terminal state)

#### Assumptions {#assumptions}

- The architectural analysis in `.specks/specks-16-worktree-lifecycle.md` accurately describes the current state of worktrees, branches, and PRs
- `SessionStatus` enum values serialize identically in both formats (`snake_case`: `"in_progress"`, `"completed"`, etc.)
- `is_pr_merged()` correctly handles the `gh pr view` → `git merge-base` fallback chain

---

### Open Questions (MUST RESOLVE OR EXPLICITLY DEFER) {#open-questions}

#### [Q01] Should the implementer skill be updated to write canonical Session format? (DEFERRED) {#q01-implementer-session-format}

**Question:** Should we update the implementer skill (SKILL.md) to write sessions matching the Session struct, or should the Session struct remain flexible enough for both?

**Why it matters:** If both sides write the same format, there's one schema to maintain. If only the struct is flexible, the implementer format could drift further over time.

**Options:**
- Update the implementer skill to write the canonical format (higher effort, must update all agent contracts)
- Keep the struct flexible and document both formats (lower effort, implemented by this speck)
- Migrate to a new v2 schema that both sides adopt (highest effort, cleanest long-term)

**Plan to resolve:** Implement the flexible-struct approach now (lowest risk, unblocks cleanup). Revisit as a follow-on after worktree lifecycle is stable.

**Resolution:** DEFERRED — follow-on work after this phase.

---

### Risks and Mitigations {#risks}

| Risk | Impact | Likelihood | Mitigation | Trigger to revisit |
|------|--------|------------|------------|--------------------|
| Schema change breaks old session loading | high | low | Add tests for both formats; use `#[serde(default)]` | Any test_load_session failure |
| Aggressive cleanup deletes in-progress work | high | low | InProgress sessions always protected; `--dry-run` default messaging | User reports lost work |
| GitHub API rate limiting during batch cleanup | med | med | Fall back to `git merge-base`; process one worktree at a time | cleanup hangs or fails |
| Dirty worktree blocks batch cleanup | low | med | Skip and warn; continue to next worktree | cleanup aborts partway |
| `gh` CLI absent or unreachable | med | low | Degrade to `git branch -d` only; warn about reduced accuracy | Offline user can't clean up |

**Risk R01: Serde deserialization changes break existing session loading** {#r01-serde-break}

- **Risk:** Adding `#[serde(default)]` and optional fields could inadvertently change serialization output, breaking round-trips for old sessions.
- **Mitigation:** Golden test with both old-format and implementer-format session JSON; verify round-trip for old format produces semantically equivalent output (same field values, no unexpected keys). Do not require byte-identical output — serde field ordering is not guaranteed.
- **Residual risk:** Future schema additions must be `Option<T>` with `#[serde(default, skip_serializing_if = "Option::is_none")]` to avoid polluting old-format output.

**Risk R02: `git branch -d` fails on unmerged stale branches** {#r02-branch-delete}

- **Risk:** `git branch -d` (safe delete) will refuse to delete branches not fully merged into HEAD, even if the corresponding PR was squash-merged.
- **Mitigation:** Use `-d` first; if it fails, check if branch has a merged PR via `gh pr view`. Only escalate to `-D` if PR is confirmed merged or user passes `--force`.
- **Residual risk:** Branches without any PR and without ancestor relationship to main require explicit `--force`.

**Risk R03: `gh` CLI absent degrades cleanup accuracy** {#r03-gh-absent}

- **Risk:** Without `gh`, squash-merged PRs cannot be reliably detected. `git merge-base` only works for non-squash merges, so some merged worktrees/branches may not be cleaned.
- **Mitigation:** Use `git branch -d` (safe delete) which naturally handles regular merges. Warn user about reduced accuracy. `--force` flag provides escape hatch. Doctor reports "gh not found" as a warning.
- **Residual risk:** Users without `gh` must use `--force` or `specks worktree remove` for squash-merged worktrees.

---

### 1.0.0 Design Decisions {#design-decisions}

#### [D01] Use list_worktrees() for session discovery (DECIDED) {#d01-use-list-worktrees}

**Decision:** Replace custom session discovery logic in `merge.rs` with the `list_worktrees()` function from specks-core.

**Rationale:**
- `list_worktrees()` handles both external and internal session storage via `load_session()`
- Eliminates duplicate session loading logic that is prone to divergence
- Single source of truth for session discovery

**Implications:**
- `find_worktree_for_speck()` must be refactored to filter results from `list_worktrees()`
- Path normalization logic must be preserved for speck matching

#### [D02] Add CleanupMode enum for cleanup command (DECIDED) {#d02-cleanup-mode}

**Decision:** Introduce a `CleanupMode` enum to support different cleanup strategies.

**Rationale:**
- Multiple cleanup modes needed: merged, orphaned, stale, all
- Enum provides type safety and clear semantics
- Matches existing patterns in specks-core

**Implications:**
- `cleanup_worktrees()` signature changes to accept `CleanupMode`
- CLI command adds flags: `--merged`, `--orphaned`, `--stale`, `--all`
- Core and CLI must be updated atomically in the same step to avoid compilation gaps

#### [D03] NeedsReconcile cleanup follows PR state (DECIDED) {#d03-needs-reconcile-cleanup}

**Decision:** Sessions in NeedsReconcile state are cleaned up based on their PR state: `--merged` cleans them if their PR is merged; `--orphaned` cleans them if they have no PR. They are never protected from cleanup.

**Rationale:**
- NeedsReconcile means "implementation completed but bead tracking failed" — the worktree is no longer needed regardless
- NeedsReconcile sessions may have merged PRs (implementation succeeded) or no PR at all (failed before PR creation)
- Lumping all NeedsReconcile under `--orphaned` is a category error — some have merged PRs

**Implications:**
- Cleanup logic checks PR state for NeedsReconcile sessions same as any other status
- Only InProgress sessions are unconditionally protected from cleanup

#### [D04] Extend existing doctor command (DECIDED) {#d04-extend-doctor}

**Decision:** Add worktree-specific health checks to the existing `specks doctor` command rather than creating a separate `specks worktree doctor` command.

**Rationale:**
- Consistent with existing doctor architecture that aggregates multiple health checks
- Single command for overall project health
- Avoids command proliferation

**Implications:**
- Add new health check functions for stale branches, orphaned worktrees, and sessionless worktrees
- Doctor output includes worktree diagnostics in the same format

#### [D05] Hardcode specks/* branch pattern (DECIDED) {#d05-branch-pattern}

**Decision:** Only clean up branches matching the `specks/*` pattern for stale branch detection.

**Rationale:**
- This is the only pattern used by specks worktree creation
- Avoids accidental deletion of user branches
- Simplifies implementation without configuration overhead

**Implications:**
- `--stale` flag only targets `specks/*` branches
- No configuration option for custom patterns

#### [D06] Unify Session struct via serde flexibility and CurrentStep enum (DECIDED) {#d06-unify-session}

**Decision:** Make the `Session` struct tolerate both the old format (from `specks worktree create`) and the implementer format by using `#[serde(default)]`, `#[serde(alias)]`, optional fields, and a typed `CurrentStep` enum for the `current_step` field.

**Rationale:**
- The implementer skill writes sessions with `session_id`, `steps_completed`, `steps_remaining`, `bead_mapping`, `root_bead`, and `current_step` as a string anchor or null — none of which match the Session struct's required fields (`schema_version`, `speck_slug`, `current_step: usize`, `total_steps`)
- `load_session()` silently fails to deserialize implementer sessions, making `list_worktrees()` unable to find 3 of 5 existing worktrees
- Modifying the Session struct is lower risk and lower effort than changing the implementer skill and all agent contracts
- Fields absent in either format get sensible defaults; fields renamed between formats get aliases
- A typed enum (`Index(usize) | Anchor(String) | Done`) for `current_step` is more robust than a `serde_json::Value` bag — it enables pattern matching at use sites and makes reconciliation logic explicit

**Implications:**
- `schema_version`, `speck_slug`, `total_steps` become optional with defaults
- `current_step` becomes `CurrentStep` enum with custom serde: `null` → `Done`, integers → `Index(N)`, strings → `Anchor("#step-N")`
- `beads_root` gets `#[serde(alias = "root_bead")]`
- New optional fields added: `session_id`, `last_updated_at`, `steps_completed`, `steps_remaining`, `bead_mapping`, `step_summaries`
- Old-format serialization output must remain semantically unchanged: no unexpected keys, same field values (use `skip_serializing_if` on new optional fields; `CurrentStep::Index(N)` serializes as `N`)
- `session reconcile` must be updated to resolve beads via `CurrentStep::Anchor` → `bead_mapping` lookup, not just numeric indexing

#### [D07] Use safe branch deletion by default (DECIDED) {#d07-safe-branch-delete}

**Decision:** Use `git branch -d` (safe delete) by default, escalating to `git branch -D` (force delete) only when the branch has a confirmed merged PR or the user passes `--force`.

**Rationale:**
- `git branch -D` force-deletes even unmerged branches, risking data loss
- Squash-merged branches won't have an ancestor relationship with main, so `-d` may fail for them — but if we can confirm the PR is merged via `gh`, force delete is safe
- Explicit `--force` gives users an escape hatch for branches with no PR

**Implications:**
- Stale branch cleanup tries `-d` first, then checks PR state before `-D`
- `--force` flag on cleanup enables unconditional `-D` for stale branches
- Branches that can't be safely deleted are reported as warnings, not errors

#### [D08] Skip dirty worktrees during batch cleanup; --force never overrides InProgress (DECIDED) {#d08-dirty-worktree}

**Decision:** During batch cleanup (`--merged`, `--orphaned`, `--all`), skip worktrees with uncommitted changes and emit a warning. Do not abort the entire batch. The `--force` flag controls git-level force operations (force-delete branches, force-remove dirty worktrees) but **never** overrides the InProgress session guard.

**Rationale:**
- `git worktree remove` (without `--force`) fails if the worktree has uncommitted changes
- Aborting the entire batch for one dirty worktree is too aggressive
- Users can clean individual dirty worktrees with `specks worktree remove --force`
- InProgress means someone is actively working — no flag combination should silently destroy active work

**Implications:**
- Batch cleanup continues past dirty worktrees; `--force` overrides this to force-remove
- InProgress sessions are **never** removed by any batch cleanup combination including `--all --force`
- To remove an InProgress worktree, the user must explicitly: (1) change the session status, or (2) use `specks worktree remove --force <branch>` (single-target, deliberate)
- Summary output lists skipped worktrees with reason

#### [D09] Closed-but-unmerged PRs are protected from orphaned cleanup (DECIDED) {#d09-closed-pr-protected}

**Decision:** Worktrees with closed-but-unmerged PRs are NOT cleaned by `--orphaned` or `--merged`. They can only be removed via explicit `specks worktree remove` or `--all`.

**Rationale:**
- A closed PR may be intentionally on hold, under revision, or awaiting reopen
- Cleaning it silently is dangerous — the user may have uncommitted local work or plans to reopen
- `specks doctor` will report these as a recommendation, giving the user visibility

**Implications:**
- `--orphaned` skips worktrees where `gh pr view` returns state `CLOSED`
- `--merged` only acts on state `MERGED`
- `--all` includes closed PRs (since the user is explicitly requesting full cleanup)
- Doctor reports closed-PR worktrees with recommendation: "Run: specks worktree remove <branch>"

#### [D10] Ambiguous worktree remove fails fast with candidate list (DECIDED) {#d10-remove-ambiguity}

**Decision:** When `specks worktree remove <speck-path>` matches multiple worktrees, fail immediately and print all candidates with branch name, status, and timestamp. Do not auto-select.

**Rationale:**
- `worktree remove` is a destructive operation — auto-selecting the "best" match is surprising and dangerous
- The merge flow auto-selects (preferring open PR) because it's non-destructive, but remove has different UX requirements
- Listing candidates lets the user narrow by branch name or path in a follow-up command

**Implications:**
- `run_remove` checks candidate count before acting
- Error message lists all matching worktrees with actionable identifiers
- User re-runs with branch name or path to disambiguate

#### [D11] gh CLI absence is not a hard blocker (DECIDED) {#d11-gh-optional}

**Decision:** All cleanup operations degrade gracefully when `gh` CLI is unavailable. Use local-only heuristics (`git branch -d`, `git merge-base`) and warn about reduced accuracy.

**Rationale:**
- `gh` is needed for squash-merge detection (the only reliable way to know a PR was merged when commits are rewritten)
- But requiring network access for cleanup is too restrictive — users should be able to clean up offline
- `git branch -d` already does the right thing for regular (non-squash) merges

**Implications:**
- When `gh` is absent or fails: use `git branch -d` only (safe delete), warn that squash-merged branches may not be detected
- `--force` flag overrides to `-D` regardless of `gh` availability
- Stale branch cleanup summary notes which branches were skipped due to `gh` unavailability

---

### 1.0.1 Specification {#specification}

#### 1.0.1.1 Unified Session Schema {#unified-session-schema}

The `Session` struct must accept both formats. Common fields are required; format-specific fields are optional with defaults.

**Spec S01: Session Struct Field Compatibility** {#s01-session-fields}

| Field | Old Format | Implementer Format | Unified Struct |
|-------|-----------|-------------------|----------------|
| `schema_version` | `"1"` (required) | absent | `#[serde(default)]` → `""` |
| `speck_path` | present | present | required (no change) |
| `speck_slug` | present | absent | `#[serde(default)]` → `""` |
| `branch_name` | present | present | required (no change) |
| `base_branch` | present | present | required (no change) |
| `worktree_path` | present | present | required (no change) |
| `created_at` | present | present | required (no change) |
| `status` | `SessionStatus` | `SessionStatus` | required (no change) |
| `current_step` | `usize` | `String` or `null` | `CurrentStep` enum with custom serde (Spec S02) |
| `total_steps` | `usize` (required) | absent | `#[serde(default)]` → `0` |
| `beads_root` | `Option<String>` | absent (see `root_bead`) | `#[serde(default, alias = "root_bead")]` |
| `reused` | `bool` (default false) | absent | `#[serde(default)]` (no change) |
| `session_id` | absent | present | `Option<String>`, `#[serde(default)]`, `skip_serializing_if` |
| `last_updated_at` | absent | present | `Option<String>`, `#[serde(default)]`, `skip_serializing_if` |
| `steps_completed` | absent | `Vec<String>` | `Option<Vec<String>>`, `#[serde(default)]`, `skip_serializing_if` |
| `steps_remaining` | absent | `Vec<String>` | `Option<Vec<String>>`, `#[serde(default)]`, `skip_serializing_if` |
| `bead_mapping` | absent | `Map<String, String>` | `Option<HashMap<String, String>>`, `#[serde(default)]`, `skip_serializing_if` |
| `step_summaries` | absent | `Vec<StepSummary>` | `Option<Vec<serde_json::Value>>`, `#[serde(default)]`, `skip_serializing_if` |

**Key invariant:** Serializing an old-format Session and re-parsing it must produce semantically equivalent output — same field values, no unexpected keys added. New optional fields use `skip_serializing_if = "Option::is_none"`. `CurrentStep::Index(N)` serializes as `N` to match old format. Field ordering may vary (not byte-identical, but key-identical).

**Spec S02: CurrentStep Enum and Normalization Rules** {#s02-current-step}

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CurrentStep {
    /// Old format: numeric index (0-based)
    Index(usize),
    /// Implementer format: step anchor (e.g., "#step-3")
    Anchor(String),
    /// Implementer format when all steps complete: null
    Done,
}
```

**Serde behavior (custom deserializer/serializer):**

| JSON value | Deserializes to | Serializes as |
|-----------|----------------|---------------|
| `2` (integer) | `Index(2)` | `2` |
| `"#step-3"` (string) | `Anchor("#step-3")` | `"#step-3"` |
| `null` | `Done` | `null` |
| absent (missing key) | `Index(0)` (default) | `0` |

**Bead resolution rules (for `session reconcile`):**

| CurrentStep variant | Bead resolution |
|--------------------|----------------|
| `Index(N)` | `format!("{}.{}", beads_root, N + 1)` (legacy numeric mapping) |
| `Anchor(anchor)` | `bead_mapping[anchor]` if `bead_mapping` is present; error if missing |
| `Done` | No bead to resolve (all steps complete) |

**Step index extraction (for display/progress):**

| CurrentStep variant | Display |
|--------------------|---------|
| `Index(N)` | `step N/total_steps` |
| `Anchor("#step-N")` | Parse N from anchor; combine with `steps_remaining.len() + steps_completed.len()` for total |
| `Done` | `complete` |

#### 1.0.1.2 CleanupMode Enum {#cleanup-mode-enum}

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupMode {
    /// Clean worktrees with merged PRs
    Merged,
    /// Clean worktrees with no PR (not InProgress)
    Orphaned,
    /// Clean specks/* branches without worktrees
    Stale,
    /// All of the above
    All,
}
```

#### 1.0.1.3 Cleanup Selection Criteria {#cleanup-criteria}

**Table T01: Cleanup Mode Behavior (normative)** {#t01-cleanup-modes}

| Mode | Selection Criteria | Protected | Actions |
|------|-------------------|-----------|---------|
| `Merged` | PR state = MERGED (any session status) | InProgress | Remove worktree, delete session/artifacts, delete branch |
| `Orphaned` | No PR exists AND status not InProgress | InProgress | Remove worktree, delete session/artifacts, delete branch |
| `Stale` | Branch matches `specks/*`, no worktree exists | n/a | Delete branch (safe delete, then force if PR merged) |
| `All` | Merged + Orphaned + Stale + **Closed** (see below) | InProgress | All cleanup actions |

`All` is NOT a simple union of the three named modes. It includes one additional selection branch that is only reachable via `--all`:

| Implicit `All`-only selection | Criteria | Notes |
|-------------------------------|----------|-------|
| Closed | PR state = CLOSED (not merged), status not InProgress | Only fires under `--all`; never triggered by `--merged` or `--orphaned` alone (D09) |

This means `--all` = `Merged ∪ Orphaned ∪ Stale ∪ Closed`. The Closed branch is how D09-protected worktrees become cleanable without requiring explicit `specks worktree remove`.

**Semantics:**
- **InProgress is an absolute guard.** InProgress sessions are always protected from batch cleanup regardless of mode — including `--all` and `--force`. The `--force` flag affects git operations (force-delete branches, force-remove dirty worktrees) but **never** overrides InProgress protection
- NeedsReconcile sessions are cleaned by whichever mode matches their PR state (D03)
- Closed-but-unmerged PRs are protected from `--merged` and `--orphaned`; cleanable only by `--all` or explicit `specks worktree remove` (D09)
- Dirty worktrees are skipped with a warning during batch cleanup; `--force` overrides this to force-remove (D08)
- `--dry-run` is supported for all modes and shows what would be removed without acting

**PR-state-unknown policy** (when `gh` fails and `git merge-base` is inconclusive):
- `--merged` mode: **skip** the worktree (conservative — don't remove unless merge is confirmed)
- `--orphaned` mode: **skip** the worktree (can't confirm no PR exists)
- `--stale` mode: use `git branch -d` only (safe delete refuses unmerged branches)
- `--all` mode: same skip behavior unless `--force` is passed
- All skipped worktrees/branches are listed in summary output with reason "PR state unknown"
- `--force` overrides PR-state-unknown skips: removes worktrees and force-deletes branches regardless of PR state (D11). Does **not** override InProgress protection

#### 1.0.1.4 Session Discovery Flow {#session-discovery-flow}

```
find_worktree_for_speck(repo_root, speck_path):
  1. Call list_worktrees(repo_root)          # returns Vec<Session>
  2. Canonicalize both inputs:
     a. Resolve speck_path to canonical form: strip ./, resolve to absolute via repo_root, then
        strip repo_root prefix to get relative form (e.g., ".specks/specks-13.md")
     b. For each session.speck_path: if absolute, strip repo_root prefix; if relative, normalize
        ./ prefix — result is always relative to repo root
     Note: existing sessions have BOTH absolute paths ("/Users/.../specks-13.md") and relative
     paths (".specks/specks-13.md"). Both must resolve to the same canonical relative form.
  3. Filter sessions where canonicalized session.speck_path == canonicalized input
  4. If multiple matches:
     a. Prefer session with open PR           # via get_pr_for_branch()
     b. Fall back to most recent by created_at (parsed as timestamp, not string sort)
     c. Final tie-break: lexicographic sort by branch_name (deterministic)
  5. Return (worktree_path, session) or error
```

#### 1.0.1.5 Worktree Remove Behavior {#worktree-remove-behavior}

`specks worktree remove <target>` identifies a worktree by:
1. Speck path (e.g., `.specks/specks-14.md`) — matches via `speck_path` in `list_worktrees()`
2. Branch name (e.g., `specks/14-20250209-172637`) — exact match in `list_worktrees()`
3. Worktree path (e.g., `.specks-worktrees/specks__14-...`) — direct path lookup

**Ambiguity handling (D10):** If target is a speck path and multiple worktrees match, fail immediately with an error listing all candidates:
```
Error: Multiple worktrees found for .specks/specks-14.md

  specks/14-20250209-172637  Pending    2025-02-09T17:26:37Z
  specks/14-20250209-172747  Completed  2025-02-09T17:27:47Z

Use branch name or worktree path to disambiguate:
  specks worktree remove specks/14-20250209-172747
```

Branch name and worktree path lookups are always unambiguous (exact match).

Without `--force`: refuses to remove if worktree has uncommitted changes.
With `--force`: passes `--force` to `git worktree remove`.

Always cleans up: external session, artifacts directory, internal session (if present), worktree directory, branch.

---

### 1.0.2 Symbol Inventory {#symbol-inventory}

#### 1.0.2.1 New/Modified in specks-core {#symbols-core}

| Symbol | Kind | Location | Notes |
|--------|------|----------|-------|
| `CurrentStep` | enum | `session.rs` | New: `Index(usize)`, `Anchor(String)`, `Done` with custom serde |
| `Session` | struct | `session.rs` | Modified: `current_step` → `CurrentStep`, add optional implementer fields, serde attributes |
| `CleanupMode` | enum | `worktree.rs` | New: cleanup mode enum |
| `CleanupResult` | struct | `worktree.rs` | New: structured result from cleanup |
| `cleanup_worktrees` | fn | `worktree.rs` | Modified: accepts `CleanupMode`, returns `CleanupResult` |
| `cleanup_stale_branches` | fn | `worktree.rs` | New: stale branch detection and cleanup |
| `list_specks_branches` | fn | `worktree.rs` | New: list all `specks/*` branches |

#### 1.0.2.2 New/Modified in specks CLI {#symbols-cli}

| Symbol | Kind | Location | Notes |
|--------|------|----------|-------|
| `find_worktree_for_speck` | fn | `merge.rs` | Modified: use `list_worktrees()`, tie-break by `created_at` |
| `run_reconcile` | fn | `session.rs` | Modified: resolve bead via `CurrentStep::Anchor` → `bead_mapping` |
| `WorktreeCommands::Remove` | variant | `worktree.rs` | New: remove subcommand |
| `WorktreeCommands::Cleanup` | variant | `worktree.rs` | Modified: add `--orphaned`, `--stale`, `--all`, `--force` flags |
| `run_remove` | fn | `worktree.rs` | New: remove command impl |
| `check_stale_branches` | fn | `doctor.rs` | New: stale branch health check |
| `check_orphaned_worktrees` | fn | `doctor.rs` | New: orphaned worktree check |
| `check_sessionless_worktrees` | fn | `doctor.rs` | New: worktrees without parseable sessions |
| `check_worktrees` | fn | `doctor.rs` | Modified: narrow to `specks__*` dirs only (fix false positive on `.sessions`/`.artifacts`) |

---

### 1.0.3 Documentation Plan {#documentation-plan}

- [ ] Update `CLAUDE.md` worktree section with new cleanup flags (`--orphaned`, `--stale`, `--all`)
- [ ] Update `CLAUDE.md` troubleshooting section with sessionless worktree guidance
- [ ] Update CLI help text for `specks worktree cleanup` to explain all modes
- [ ] Add help text for new `specks worktree remove` command

---

### 1.0.4 Test Plan Concepts {#test-plan-concepts}

#### Test Categories {#test-categories}

| Category | Purpose | When to use |
|----------|---------|-------------|
| **Unit** | Test session deserialization, cleanup selection logic | Schema compatibility, mode filtering |
| **Integration** | Test end-to-end cleanup with real git worktrees | CLI commands, git operations |
| **Golden / Contract** | Compare session serialization against known-good JSON | Schema backward compatibility |

#### Test Infrastructure {#test-infrastructure}

Testing worktree operations requires git repos with real worktrees. Use `tempfile::TempDir` with `git init` to create throwaway repos:

1. **Session deserialization tests**: Create JSON strings matching old and implementer formats; verify `serde_json::from_str::<Session>()` succeeds for both
2. **Cleanup logic tests**: Mock `is_pr_merged()` responses to test cleanup selection without GitHub API calls
3. **CLI integration tests**: Use `assert_cmd` or `Command::new()` with temp git repos for end-to-end verification

**Fixture approach:** Embed session JSON literals as `const &str` in test modules. No fixture files needed — the schemas are small enough to inline.

---

### 1.0.5 Execution Steps {#execution-steps}

#### Step 0: Unify Session struct and CurrentStep enum to handle both formats {#step-0}

**Bead:** `specks-dqg.1`

**Commit:** `fix(session): unify Session struct with CurrentStep enum to accept both old and implementer formats`

**References:** [D06] Unify Session struct via serde flexibility and CurrentStep enum, Spec S01, Spec S02, (#unified-session-schema, #s02-current-step), Risk R01

**Artifacts:**
- Modified `crates/specks-core/src/session.rs`
- Modified `crates/specks/src/commands/session.rs` (reconcile bead resolution)

**Tasks:**
- [ ] Add `CurrentStep` enum with `Index(usize)`, `Anchor(String)`, `Done` variants
- [ ] Implement custom serde `Deserialize`/`Serialize` for `CurrentStep` per Spec S02 mapping table
- [ ] Change `Session.current_step` field type from `usize` to `CurrentStep`
- [ ] Add `#[serde(default)]` to `schema_version`, `speck_slug`, `total_steps`
- [ ] Add `#[serde(alias = "root_bead")]` to `beads_root`
- [ ] Add optional implementer fields: `session_id`, `last_updated_at`, `steps_completed`, `steps_remaining`, `bead_mapping`, `step_summaries`
- [ ] Add `#[serde(default, skip_serializing_if = "Option::is_none")]` to all new optional fields
- [ ] Update `session reconcile` bead resolution: `CurrentStep::Anchor(a)` → lookup `bead_mapping[a]`; `CurrentStep::Index(n)` → `beads_root.{n+1}` (legacy); `CurrentStep::Done` → no-op
- [ ] Update all code that reads `current_step` as `usize` to pattern-match on `CurrentStep` (including `worktree list` progress display)
- [ ] Verify old-format serialization round-trip produces semantically equivalent output (same values, no extra keys)

**Tests:**
- [ ] Golden test: `test_deserialize_old_format` — parse old-format JSON (with `schema_version`, `speck_slug`, `current_step: 2`, `total_steps: 5`)
- [ ] Golden test: `test_deserialize_implementer_format` — parse implementer JSON (with `session_id`, `steps_completed`, `current_step: "#step-0"`, `root_bead`, `bead_mapping`)
- [ ] Golden test: `test_deserialize_implementer_format_completed` — parse implementer JSON with `current_step: null` → `CurrentStep::Done`
- [ ] Contract test: `test_old_format_roundtrip_no_extra_keys` — serialize old-format Session, verify JSON output has same keys and values (semantic equality, not byte-identical)
- [ ] Unit test: `test_beads_root_alias` — `root_bead` in JSON maps to `beads_root` field
- [ ] Unit test: `test_current_step_index_serializes_as_number` — `CurrentStep::Index(3)` serializes as `3`
- [ ] Unit test: `test_current_step_anchor_serializes_as_string` — `CurrentStep::Anchor("#step-3")` serializes as `"#step-3"`
- [ ] Unit test: `test_current_step_done_serializes_as_null` — `CurrentStep::Done` serializes as `null`
- [ ] Unit test: `test_reconcile_with_anchor_current_step` — reconcile resolves bead via `bead_mapping["#step-3"]`
- [ ] Unit test: `test_reconcile_with_index_current_step` — reconcile resolves bead via `beads_root.{N+1}` (legacy)
- [ ] Unit test: `test_reconcile_with_done_current_step` — reconcile on `Done` reports no bead to close
- [ ] Unit test: `test_deserialize_missing_current_step` — JSON with no `current_step` key defaults to `CurrentStep::Index(0)` (serde default)

**Checkpoint:**
- [ ] `cargo nextest run -p specks-core session`
- [ ] `cargo nextest run -p specks session`
- [ ] `cargo build`

**Rollback:**
- Revert changes to `session.rs` (core) and `session.rs` (CLI)

**Commit after all checkpoints pass.**

---

#### Step 1: Fix session discovery in merge.rs {#step-1}

**Depends on:** #step-0

**Bead:** `specks-dqg.2`

**Commit:** `fix(merge): use list_worktrees for session discovery`

**References:** [D01] Use list_worktrees() for session discovery, (#session-discovery-flow)

**Artifacts:**
- Modified `crates/specks/src/commands/merge.rs`

**Tasks:**
- [ ] Replace custom directory scanning in `find_worktree_for_speck()` with `list_worktrees()` call
- [ ] Preserve speck path normalization logic for matching
- [ ] Preserve preference for worktree with open PR when multiple matches exist
- [ ] Fall back to most recent worktree by `created_at` (parsed timestamp, not string sort), then deterministic `branch_name` lexicographic tie-break
- [ ] Update imports to include `specks_core::list_worktrees`

**Tests:**
- [ ] Unit test: `test_find_worktree_matches_by_speck_path` — finds worktree when speck path matches
- [ ] Unit test: `test_find_worktree_normalizes_relative_path` — handles `./`, `.specks/`, bare filename variations
- [ ] Unit test: `test_find_worktree_matches_absolute_session_path` — session with absolute `speck_path` (e.g., `/Users/.../specks-13.md`) matches relative input
- [ ] Unit test: `test_find_worktree_prefers_open_pr` — prefers worktree with open PR when multiple match
- [ ] Unit test: `test_find_worktree_tiebreak_by_created_at` — most recent by parsed timestamp wins when no open PR
- [ ] Unit test: `test_find_worktree_no_match_returns_error` — returns clear error when no worktree matches speck path

**Checkpoint:**
- [ ] `cargo nextest run -p specks`
- [ ] `cargo build`

**Rollback:**
- Revert changes to merge.rs

**Commit after all checkpoints pass.**

---

#### Step 2: Add CleanupMode and orphaned cleanup with CLI flags {#step-2}

**Depends on:** #step-1

**Bead:** `specks-dqg.3`

**Commit:** `feat(worktree): add CleanupMode enum with --orphaned flag`

**References:** [D02] Add CleanupMode enum, [D03] NeedsReconcile cleanup follows PR state, [D08] Skip dirty worktrees, [D09] Closed PRs protected, [D11] gh optional, (#cleanup-mode-enum, #cleanup-criteria), Table T01

**Artifacts:**
- Modified `crates/specks-core/src/worktree.rs`
- Modified `crates/specks-core/src/lib.rs` (export `CleanupMode`, `CleanupResult`)
- Modified `crates/specks/src/commands/worktree.rs`

**Tasks:**
- [ ] Add `CleanupMode` enum with `Merged`, `Orphaned`, `Stale`, `All` variants
- [ ] Add `CleanupResult` struct with `merged_removed`, `orphaned_removed`, `stale_branches_removed`, `skipped` fields
- [ ] Update `cleanup_worktrees()` signature to accept `CleanupMode` and return `CleanupResult`
- [ ] Implement `Merged` mode (existing logic, refactored)
- [ ] Implement `Orphaned` mode: no PR exists AND status not InProgress
- [ ] InProgress sessions always protected regardless of mode
- [ ] NeedsReconcile sessions cleaned by whichever mode matches their PR state
- [ ] Skip dirty worktrees with warning during batch cleanup
- [ ] Export `CleanupMode` and `CleanupResult` from `lib.rs`
- [ ] Update CLI `Cleanup` subcommand: change `--merged` from required to optional, add `--orphaned` and `--all` flags
- [ ] Update `run_cleanup` to map flags to `CleanupMode` (default to `Merged` if no flag specified for backward compatibility)
- [ ] Update help text to explain cleanup modes

**Tests:**
- [ ] Unit test: `test_cleanup_merged_only` — Merged mode only removes worktrees with merged PRs
- [ ] Unit test: `test_cleanup_orphaned_no_pr` — Orphaned mode removes worktrees without PRs
- [ ] Unit test: `test_cleanup_orphaned_skips_in_progress` — Orphaned mode skips InProgress sessions
- [ ] Unit test: `test_cleanup_needs_reconcile_merged_pr` — NeedsReconcile with merged PR cleaned by Merged mode
- [ ] Unit test: `test_cleanup_needs_reconcile_no_pr` — NeedsReconcile with no PR cleaned by Orphaned mode
- [ ] Unit test: `test_cleanup_skips_closed_pr` — Worktree with closed-but-unmerged PR skipped by both `--merged` and `--orphaned` (D09)
- [ ] Unit test: `test_cleanup_skips_unknown_pr_state` — Worktree skipped when PR state cannot be determined (gh fails, merge-base inconclusive)
- [ ] Unit test: `test_cleanup_all_includes_closed_pr` — `All` mode removes worktree with closed-but-unmerged PR (D09 + T01 Closed branch)
- [ ] Unit test: `test_cleanup_all_protects_in_progress` — `All` mode (even with `--force`) never removes InProgress sessions (D08 absolute guard)
- [ ] Unit test: `test_cleanup_skips_dirty_worktree` — Dirty worktree skipped with warning in batch cleanup, batch continues to next (D08)
- [ ] Unit test: `test_cleanup_force_overrides_unknown_pr` — `--force` removes worktree when PR state cannot be determined (D11)

**Checkpoint:**
- [ ] `cargo nextest run`
- [ ] `cargo build`
- [ ] `specks worktree cleanup --orphaned --dry-run` (manual verification)

**Rollback:**
- Revert changes to worktree.rs (core), lib.rs, worktree.rs (CLI)

**Commit after all checkpoints pass.**

---

#### Step 3: Add specks worktree remove command {#step-3}

**Depends on:** #step-2

**Bead:** `specks-dqg.4`

**Commit:** `feat(cli): add worktree remove command for explicit removal`

**References:** [D10] Ambiguous worktree remove fails fast, (#worktree-remove-behavior, #symbols-cli)

**Artifacts:**
- Modified `crates/specks/src/commands/worktree.rs`

**Tasks:**
- [ ] Add `Remove` variant to `WorktreeCommands` enum
- [ ] Implement `run_remove` function
- [ ] Support identification by speck path, branch name, or worktree path
- [ ] Implement ambiguity handling: fail fast with candidate list when speck path matches multiple worktrees (D10)
- [ ] Add `--force` flag for worktrees with uncommitted changes
- [ ] Clean up session, artifacts, worktree directory, and branch
- [ ] Print confirmation of what was removed

**Tests:**
- [ ] Integration test: `test_remove_by_speck_path` — remove worktree identified by speck path
- [ ] Integration test: `test_remove_by_branch_name` — remove worktree identified by branch
- [ ] Integration test: `test_remove_refuses_dirty_without_force` — requires `--force` for dirty worktree
- [ ] Integration test: `test_remove_by_worktree_path` — remove worktree identified by filesystem path
- [ ] Integration test: `test_remove_ambiguous_fails_fast` — multiple worktrees for same speck prints candidates and errors

**Checkpoint:**
- [ ] `cargo nextest run -p specks`
- [ ] `cargo build`

**Rollback:**
- Revert changes to worktree.rs (CLI)

**Commit after all checkpoints pass.**

---

#### Step 4: Add stale branch detection and cleanup with CLI flags {#step-4}

**Depends on:** #step-3

**Bead:** `specks-dqg.5`

**Commit:** `feat(worktree): add stale branch detection and --stale flag`

**References:** [D05] Hardcode specks/* branch pattern, [D07] Safe branch deletion, (#cleanup-criteria), Table T01, Risk R02

**Artifacts:**
- Modified `crates/specks-core/src/worktree.rs`
- Modified `crates/specks/src/commands/worktree.rs`

**Tasks:**
- [ ] Add `list_specks_branches()` function to list all `specks/*` branches
- [ ] Add `cleanup_stale_branches()` function: finds branches without worktrees, deletes with `git branch -d` first
- [ ] If `-d` fails, check PR state via `gh pr view`: force delete with `-D` only if PR is confirmed merged
- [ ] Integrate stale cleanup into `cleanup_worktrees()` for `Stale` and `All` modes
- [ ] Add `--stale` flag to CLI `Cleanup` subcommand
- [ ] Add `--force` flag to CLI `Cleanup` subcommand (enables unconditional `-D` for stale branches)
- [ ] Update help text

**Tests:**
- [ ] Unit test: `test_list_specks_branches` — lists only `specks/*` branches
- [ ] Unit test: `test_cleanup_stale_removes_orphan_branch` — removes branch without worktree
- [ ] Unit test: `test_cleanup_stale_skips_branch_with_worktree` — skips branches that have worktrees
- [ ] Unit test: `test_cleanup_stale_safe_delete_fallback` — tries `-d` first, then `-D` only if PR merged
- [ ] Unit test: `test_cleanup_stale_gh_absent_safe_only` — without `gh`, only `git branch -d` is used; squash-merged branches skipped with warning (D11, R03)

**Checkpoint:**
- [ ] `cargo nextest run`
- [ ] `cargo build`
- [ ] `specks worktree cleanup --stale --dry-run` (manual verification)

**Rollback:**
- Revert changes to worktree.rs (core and CLI)

**Commit after all checkpoints pass.**

---

#### Step 5: Fix doctor false positive and extend with worktree diagnostics {#step-5}

**Depends on:** #step-4

**Bead:** `specks-dqg.6`

**Commit:** `feat(doctor): fix worktree path check, add stale branch and orphan diagnostics`

**References:** [D04] Extend existing doctor command, [D09] Closed-but-unmerged PRs protected, (#symbols-cli)

**Artifacts:**
- Modified `crates/specks/src/commands/doctor.rs`
- Modified `crates/specks-core/src/worktree.rs` (if `is_valid_worktree_path` lives here)

**Tasks:**
- [ ] Fix existing `check_worktrees()`: narrow directory scan to `specks__*` entries only, excluding `.sessions`, `.artifacts`, and other infrastructure dirs (fixes current false positive)
- [ ] Add `check_stale_branches()` health check: counts `specks/*` branches without worktrees
- [ ] Add `check_orphaned_worktrees()` health check: counts worktrees without PRs that aren't InProgress
- [ ] Add `check_sessionless_worktrees()` health check: cross-references `git worktree list` output with `list_worktrees()` to find worktrees without parseable sessions
- [ ] Report closed-but-unmerged PR worktrees as a recommendation (D09): "Consider: specks worktree remove <branch>"
- [ ] Integrate new checks into `run_doctor()`
- [ ] Output actionable recommendations (e.g., "Run: specks worktree cleanup --orphaned")

**Tests:**
- [ ] Unit test: `test_check_worktrees_ignores_sessions_artifacts` — `.sessions` and `.artifacts` dirs do NOT trigger invalid path warning
- [ ] Unit test: `test_check_stale_branches_none` — pass when no stale branches
- [ ] Unit test: `test_check_stale_branches_found` — warn when stale branches exist
- [ ] Unit test: `test_check_orphaned_worktrees_none` — pass when none orphaned
- [ ] Unit test: `test_check_orphaned_worktrees_found` — warn when orphans exist

**Checkpoint:**
- [ ] `cargo nextest run -p specks doctor`
- [ ] `cargo build`
- [ ] `specks doctor --json` (manual verification: no false positives for `.sessions`/`.artifacts`)

**Rollback:**
- Revert changes to doctor.rs and worktree.rs (if modified)

**Commit after all checkpoints pass.**

---

### 1.0.6 Deliverables and Checkpoints {#deliverables}

**Deliverable:** Reliable worktree lifecycle management with unified session schema, proper session discovery, and multiple cleanup modes.

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

- [ ] `specks worktree list` discovers all worktrees regardless of session format (old + implementer)
- [ ] `specks merge` correctly finds worktrees with external sessions (test passes)
- [ ] `specks worktree cleanup --merged` removes all worktrees with merged PRs (test passes)
- [ ] `specks worktree cleanup --orphaned` removes worktrees without PRs (test passes)
- [ ] `specks worktree cleanup --stale` removes `specks/*` branches without worktrees (test passes)
- [ ] `specks worktree remove <target>` removes specific worktrees (test passes)
- [ ] `specks doctor` reports stale branches, orphaned worktrees, and sessionless worktrees (output verified)
- [ ] All tests pass: `cargo nextest run`
- [ ] No compiler warnings: `cargo build`

**Acceptance tests:**
- [ ] Integration test: session deserialization works for both old-format and implementer-format JSON
- [ ] Integration test: full workflow from create -> implement -> merge -> cleanup
- [ ] Integration test: cleanup after failed implementation (orphaned worktree)
- [ ] Integration test: doctor reports issues and cleanup resolves them
- [ ] Integration test: `--all` mode removes closed-PR worktrees that `--merged` and `--orphaned` individually skip
- [ ] End-to-end test: implementer session (anchor/null `current_step`) → `specks worktree list` shows it → `specks merge` finds it → `specks worktree cleanup --merged` removes it → `specks doctor` reports clean

#### Roadmap / Follow-ons (Explicitly Not Required for Phase Close) {#roadmap}

- [ ] Update implementer skill to write canonical Session format (resolves Q01)
- [ ] Add `--closed` flag for PRs closed without merging
- [ ] Automatic cleanup after PR merge (webhook integration)
- [ ] Session migration tool (convert internal to external)
- [ ] Worktree reuse improvements (better conflict detection)

| Checkpoint | Verification |
|------------|--------------|
| Session schema unified | `cargo nextest run -p specks-core session` |
| Session discovery fixed | `cargo nextest run -p specks` |
| Cleanup modes work | `cargo nextest run` |
| Doctor extended | `cargo nextest run -p specks doctor` |
| Full test suite | `cargo nextest run` |

**Commit after all checkpoints pass.**