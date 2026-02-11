# Beads Enrichment: Phase Proposals

> This document defines four phases for enriching beads usage in specks.
> Each phase is designed to be a standalone speck. Phases are ordered by
> dependency but each is independently valuable.

---

## Invariants (Apply to All Phases)

1. **All beads interaction goes through `BeadsCli` in `beads.rs`.** No direct `bd` CLI calls anywhere else. This is our abstraction boundary for a future pivot away from beads.
2. **The speck file is read-only during implementation.** It is the authoritative design document, created by the planner, approved by the user. Agents do not modify it during execution.
3. **Rock-solid conventions.** Every piece of state has exactly one canonical location. No agent should ever wonder where to find something.

---

## Phase A: Rich Sync

### Problem

Today, `specks beads sync` creates thin beads — each step bead gets a title and a one-line description that's just a pointer back to the speck file:

```
title: "Step 0: Initialize API client"
description: "Specks: .specks/specks-5.md#step-0\nCommit: feat(api): add client"
```

Every agent that needs step requirements must open the speck, find the right section, and extract tasks/tests/checkpoints. The bead is a tracking token, not a work item.

### Goal

Make each step bead a **self-contained work item** by populating Beads' content fields during sync. After sync, `bd show <step-id> --json` returns everything an agent needs to implement the step.

### Bead Field Mapping Convention

| Bead Field | Content | Source in Speck |
|-----------|---------|-----------------|
| `title` | `"Step N: <title>"` | Step heading (already done) |
| `description` | Full work specification: tasks, artifacts, commit template, rollback | Step's **Tasks:**, **Artifacts:**, **Commit:**, **Rollback:** sections |
| `acceptance_criteria` | Verification requirements: tests + checkpoints | Step's **Tests:** and **Checkpoint:** sections |
| `design` | Plan references: resolved design decisions and anchor references | Step's **References:** line, with decision text inlined |
| `notes` | Initially empty; reserved for agent runtime notes (Phase B) | Empty at sync time |
| `metadata` | Structured JSON with step metadata | `{ "speck_path": "...", "anchor": "#step-0", "commit_message": "..." }` |

For the **root bead** (molecule/epic):

| Bead Field | Content |
|-----------|---------|
| `title` | Phase title from speck |
| `description` | Speck purpose statement + strategy bullets |
| `design` | Summary of design decisions (IDs + titles + one-line decisions) |
| `acceptance_criteria` | Phase exit criteria from Deliverables section |

### What Changes in Code

**`crates/specks-core/src/beads.rs`** — Add new `BeadsCli` methods:
- `create_with_body_file(title, body_file_path, ...)` — uses `bd create --body-file=<path>` for large descriptions
- `edit_description(id, content)` — uses `bd update <id> --description <content>` or body-file
- `edit_design(id, content)` — uses `bd edit <id> --design` (or equivalent CLI)
- `edit_acceptance(id, content)` — uses `bd edit <id> --acceptance` (or equivalent CLI)
- `edit_notes(id, content)` — uses `bd edit <id> --notes`
- `update_metadata(id, json_value)` — uses `bd update <id> --metadata <json>`

**`crates/specks-core/src/beads.rs`** — Extend `Issue` and `IssueDetails` structs:
- Add optional fields: `design`, `acceptance_criteria`, `notes`, `metadata`
- These are returned by `bd show --json` but we currently ignore them

**`crates/specks/src/commands/beads/sync.rs`** — Enrich sync logic:
- `ensure_root_bead()`: Generate rich description from speck overview sections
- `ensure_step_bead()`: Generate description from step's tasks/artifacts/commit/rollback, acceptance_criteria from tests/checkpoints, design from resolved references
- Add a `generate_step_description(step, speck)` helper that renders step content as markdown
- Add a `generate_step_acceptance(step)` helper that renders tests + checkpoints
- Add a `resolve_step_references(step, speck)` helper that expands `[D01]` references to include decision text

**`crates/specks-core/src/types.rs`** — May need a helper on `Step`:
- `fn render_description(&self) -> String` — render tasks/artifacts/commit/rollback as markdown
- `fn render_acceptance_criteria(&self) -> String` — render tests + checkpoints as markdown

### Content Generation Detail

**Step description** (goes into bead `description` field):

```markdown
## Tasks
- [ ] Create `src/api/client.rs` with retry logic
- [ ] Add `reqwest` dependency to Cargo.toml

## Artifacts
- New file: `src/api/client.rs`
- Modified: `Cargo.toml`

## Commit Template
feat(api): add client with retry support

## Rollback
- Revert commit
- Remove `src/api/client.rs`
```

**Step acceptance criteria** (goes into bead `acceptance_criteria` field):

```markdown
## Tests
- [ ] Unit test: retry with exponential backoff
- [ ] Integration test: client connects to mock server

## Checkpoints
- [ ] `cargo test` passes
- [ ] `cargo clippy` clean
```

**Step design** (goes into bead `design` field):

```markdown
## References
- [D01] REST API client (DECIDED): Use reqwest with configurable retry
- [D03] Retry strategy (DECIDED): Exponential backoff with jitter, max 3 retries
- Anchors: #inputs-outputs, #error-scenarios
```

### Tests

- Unit test: `generate_step_description()` produces expected markdown from a Step struct
- Unit test: `generate_step_acceptance()` produces expected markdown
- Unit test: `resolve_step_references()` expands [D01] to include decision text
- Integration test: Full sync creates beads with populated description/acceptance_criteria/design fields
- Golden test: Sync of a known speck produces expected bead content

### Migration / Backward Compatibility

- Existing thin beads are not broken; they just have less content
- Rich sync can be run on an existing speck to enrich beads (idempotent — checks if bead exists, updates content if needed)
- New flag `--enrich` on `specks beads sync` to force content update on existing beads (default: only set content on newly created beads)

### Success Criteria

- `bd show <step-bead-id> --json` returns description, acceptance_criteria, and design fields with meaningful content
- An agent can implement a step using only `bd show` output, without reading the speck file
- Existing workflows continue to work unchanged

---

## Phase B: Agent-Bead Communication

### Problem

Currently, agents communicate through artifact files:
1. Architect writes `architect-output.json` to the artifact directory
2. Coder reads architect's artifact, implements, writes `coder-output.json`
3. Reviewer reads both artifacts, writes `reviewer-output.json`
4. Committer reads reviewer's artifact for the step summary

This creates a dependency chain on local filesystem paths. The orchestrator must pass artifact directory paths, agents must know file naming conventions, and the artifacts directory is an external location separate from both the worktree and the beads database.

### Goal

Replace inter-agent artifact files with bead field updates. Each agent reads from and writes to the step's bead, making the bead the single coordination point during implementation.

### Convention: Which Agent Writes Where

| Agent | Reads From Bead | Writes To Bead |
|-------|----------------|----------------|
| **Setup** | `bd ready --parent <root>` to get ready steps | Nothing (creates worktree + infrastructure) |
| **Architect** | `description` (tasks), `acceptance_criteria` (tests/checkpoints), `design` (references) | `design` field — appends strategy section below references |
| **Coder** | `description` (tasks), `design` (references + architect strategy) | `notes` field — writes implementation results |
| **Reviewer** | `description`, `acceptance_criteria`, `design`, `notes` (coder results) | `notes` field — appends review below coder results |
| **Committer** | `notes` (for step summary extraction) | `close_reason` on `bd close` — records commit hash + summary |

### Field Ownership Rules

1. **`description`** — Written once by sync (Phase A). Never modified by agents. This is the step's work specification.
2. **`acceptance_criteria`** — Written once by sync. Never modified by agents. This is what "done" looks like.
3. **`design`** — Written by sync with plan references. Architect **appends** strategy below a `---` separator. Format:

```markdown
## References
[D01] REST API client...
[D03] Retry strategy...

---

## Architect Strategy
**Approach:** Create client module with retry middleware...
**Expected files:** src/api/client.rs, src/api/retry.rs
**Test plan:** Unit tests for retry logic, integration test with mock server
**Risks:** None identified
```

4. **`notes`** — Empty after sync. Coder writes results first, reviewer appends below. Format:

```markdown
## Coder Results
**Success:** true
**Files created:** src/api/client.rs, src/api/retry.rs
**Files modified:** Cargo.toml
**Tests:** passed (12/12)
**Build:** clean

---

## Review
**Recommendation:** APPROVE
**Plan conformance:** All tasks completed, all checkpoints verified
**Issues:** None
```

5. **`close_reason`** — Set by committer on `bd close`: `"Committed: <hash> — <summary>"`

### What Changes in Code

**Agent definitions** (`agents/*.md`):
- **architect-agent.md**: Read step content from `bd show` output passed by orchestrator. Write strategy via `bd edit <id> --design`. Remove artifact file writing.
- **coder-agent.md**: Read step content + architect strategy from `bd show`. Write results via `bd edit <id> --notes`. Remove artifact file reading/writing.
- **reviewer-agent.md**: Read all bead fields from `bd show`. Append review to notes. Remove artifact file reading/writing.
- **committer-agent.md**: Read notes for summary extraction. Close bead with reason. Remove session update (Phase C).

**Orchestrator skill** (`skills/implementer/SKILL.md`):
- Instead of passing artifact directory paths to agents, pass bead ID for the current step
- Simplify the per-step loop: no artifact path construction, no artifact existence checks
- Step data flows through the bead, not through the orchestrator

**`crates/specks-core/src/beads.rs`** — Ensure all write methods work with `current_dir` for worktree context:
- Methods that write to beads must accept an optional `working_dir` parameter so `bd` finds the `.beads/` directory when running in a worktree

**Artifact directory**:
- Move to inside the worktree: `<worktree>/.specks/artifacts/` instead of `<repo>/.specks-worktrees/.artifacts/<session-id>/`
- Purpose changes from "inter-agent communication" to "debug log" — agents write artifacts for debugging but never read each other's artifacts
- Artifacts are cleaned up automatically when the worktree is removed after merge

### Tests

- Integration test: Architect writes to design field, coder can read it back via `bd show`
- Integration test: Coder writes to notes, reviewer can read it back
- Test: Notes field uses append convention (coder section preserved when reviewer writes)
- Test: `bd close` with reason records commit info

### Migration

- Agents gain `bd` CLI access (architect-agent, coder-agent already have Bash)
- Reviewer-agent needs Bash added to its tools (currently Read, Grep, Glob, Write only) OR the orchestrator passes bead content as context
- Artifacts directory moves inside worktree — update `session.rs:artifacts_dir()` and setup agent

### Success Criteria

- No agent reads another agent's artifact file as part of the normal workflow
- All inter-agent data flows through bead fields
- `bd show <step-id>` after completion shows full history: specification, strategy, results, review
- Debugging artifacts still available inside worktree for post-mortem analysis

---

## Phase C: Eliminate Session File

### Problem

The session file (`<repo>/.specks-worktrees/.sessions/<session-id>.json`) tracks:
- Worktree path, branch name, base branch (derivable from git)
- Speck path, speck slug (derivable from branch naming convention)
- Root bead ID (already in speck's Plan Metadata)
- Step summaries with commit hashes (derivable from git log + bead close_reason)
- Timestamps (derivable from git log)

After Phases A and B, nearly everything in the session is redundant with data that lives in git or beads.

### Goal

Remove the session file entirely. Derive all needed information from git worktree state and beads.

### Convention: Deriving State from Git + Beads

**Finding the worktree for a speck:**
```
1. Parse speck slug from filename: specks-auth.md → "auth"
2. Run: git worktree list --porcelain
3. Match branch: specks/<slug>-* (e.g., specks/auth-20260208-143022)
4. Return worktree path
```
Note: `merge.rs` already does this (line 4: "Uses git-native worktree discovery, not session files").

**Finding the branch name:**
```
1. From worktree path, run: git -C <worktree> branch --show-current
```

**Finding the base branch:**
```
Always "main" (or configurable in config.toml).
```

**Finding the root bead ID:**
```
1. Parse the speck file from the worktree: <worktree>/.specks/<speck-file>
2. Read Beads Root from Plan Metadata table
```

**Finding step summaries for PR body:**
```
1. Get root bead ID from speck
2. bd children <root-id> --json → get all step beads
3. Filter for closed beads
4. Each closed bead's close_reason has: "Committed: <hash> — <summary>"
5. Or: git log --oneline on the branch gives commit messages directly
```

**Finding commit hashes:**
```
git log --oneline <base>..HEAD in the worktree
```

### What Changes in Code

**Remove:**
- `crates/specks-core/src/session.rs` — Delete or gut the module. Keep `now_iso8601()` if used elsewhere.
- `Session`, `StepSummary` structs — Remove
- `save_session()`, `save_session_atomic()`, `load_session()` — Remove
- `sessions_dir()`, `session_file_path()`, `artifacts_dir()` — Remove
- `.specks-worktrees/.sessions/` directory — No longer created
- `.specks-worktrees/.artifacts/` directory — No longer created (artifacts move to worktree)

**Modify `step_commit.rs`:**
- Remove session loading, session update, session path parameter
- Close bead with rich close_reason: `"Committed: <hash> — <summary>"`
- The bead close IS the step completion record
- Implementation log update remains (it's in the worktree, committed with each step)

**Modify `step_publish.rs`:**
- Remove session loading and session update
- Build PR body from git log on the branch (`git log --oneline <base>..HEAD`)
- Or from bead children's close_reasons
- Remove `--session` parameter

**Modify `worktree.rs` (create command):**
- Stop creating session file
- Stop creating external artifacts directory
- Create in-worktree artifacts directory: `<worktree>/.specks/artifacts/`
- Return output JSON without `session_file` and `artifacts_base` fields
- Add `speck_slug` to output for the orchestrator

**Modify `merge.rs`:**
- Already uses git-native discovery — minimal changes
- Remove session cleanup from merge flow
- Remove `delete_session()` calls

**Modify `worktree.rs` (cleanup command):**
- Remove session cleanup
- Remove artifacts cleanup (they're inside the worktree, deleted with it)

**Modify agent definitions:**
- Remove all references to session file paths
- Remove `--session` parameter from `specks step-commit` and `specks step-publish` calls
- Setup agent: No longer creates/returns session path

**Update CLI argument parsing (`cli.rs`):**
- Remove `--session` from `step-commit` and `step-publish` subcommands
- Add deprecation warning if `--session` is passed (accept but ignore)

### Worktree Discovery Helper

Add a new function to `worktree.rs` or a new `discovery.rs` module:

```rust
/// Find the active worktree for a speck by slug matching.
/// Returns (worktree_path, branch_name) or None.
pub fn find_worktree_for_speck(
    repo_root: &Path,
    speck_slug: &str,
) -> Result<Option<(PathBuf, String)>, SpecksError>
```

This becomes the single entry point for "given a speck, find its worktree." Used by merge, status, and any future commands.

### Tests

- Test: `find_worktree_for_speck()` correctly matches slug to worktree
- Test: `step_commit` works without session file
- Test: `step_publish` builds PR body from git log
- Test: Worktree create no longer creates session file or external artifacts dir
- Test: Merge cleanup works without session cleanup
- Integration test: Full implementation cycle without session file

### Migration

- Existing sessions are ignored (not read, not updated)
- Old `.sessions/` and `.artifacts/` directories can be cleaned up manually or by a migration command
- Add a `specks doctor` check that warns about orphaned session/artifact directories

### Success Criteria

- No session file is created during `specks worktree create`
- No session file is read during `step-commit`, `step-publish`, or `merge`
- PR body is generated from git log or bead close_reasons
- `specks doctor` reports clean state
- All existing tests pass (with session-related tests removed/updated)

---

## Phase D: Status from Beads

### Problem

Currently, `specks status` reads the speck file and counts checked/unchecked checkboxes. This only works if someone manually checks off items in the markdown. It doesn't reflect actual implementation state.

After Phases A-C, beads contain the complete state: which steps are done (closed beads), which are in progress (open + ready), which are blocked (open + waiting on deps), plus the rich content of what was done (close_reason, notes).

### Goal

Rebuild `specks status` to pull from beads, showing real implementation state with closed beads, commit info, and agent summaries.

### Output Format

**Text output:**
```
## Phase 1.0: User Authentication

Status: active | 3/5 steps complete

Step 0: Initialize API client ✅ closed
  Committed: abc123d — feat(api): add client with retry support

Step 1: Add auth middleware ✅ closed
  Committed: def456a — feat(auth): add JWT middleware

Step 2: Add login endpoint 🔄 ready
  Tasks: 3 | Tests: 2 | Checkpoints: 2

Step 3: Add registration endpoint ⏳ blocked
  Blocked by: Step 2

Step 4: End-to-end tests ⏳ blocked
  Blocked by: Step 2, Step 3
```

**JSON output** (`--json`):
```json
{
  "status": "ok",
  "command": "status",
  "data": {
    "speck": ".specks/specks-5.md",
    "phase_title": "User Authentication",
    "total_steps": 5,
    "completed_steps": 2,
    "ready_steps": 1,
    "blocked_steps": 2,
    "steps": [
      {
        "anchor": "#step-0",
        "title": "Initialize API client",
        "bead_id": "bd-abc123",
        "status": "complete",
        "commit_hash": "abc123d",
        "commit_summary": "feat(api): add client with retry support"
      },
      {
        "anchor": "#step-2",
        "title": "Add login endpoint",
        "bead_id": "bd-ghi789",
        "status": "ready",
        "task_count": 3,
        "test_count": 2,
        "checkpoint_count": 2
      }
    ]
  }
}
```

### What Changes in Code

**`crates/specks/src/commands/status.rs`** — Major rewrite:
- Parse speck to get step list and root bead ID
- Query beads: `bd children <root-id> --json` to get all step beads with status
- For closed beads: parse `close_reason` to extract commit hash and summary
- For open beads: check `bd ready` to distinguish ready vs. blocked
- For ready beads: show task/test/checkpoint counts from `acceptance_criteria` or from parsed speck
- Render text or JSON output

**`crates/specks-core/src/beads.rs`** — Add/extend:
- Extend `IssueDetails` to include `close_reason`, `closed_at` fields
- Add `list_children_detailed(parent_id)` that returns full details including close_reason

**New: `specks status --beads`** (or make it the default):
- Pull status from beads instead of checkbox counting
- Show real implementation progress
- If beads are not synced (no root bead ID), fall back to checkbox counting

### Additional Feature: Reconstructed View

**`specks status --full`** — Show a rich view that reconstructs step details from beads:

```
## Step 0: Initialize API client ✅

### Specification (from bead description)
Tasks:
- [x] Create `src/api/client.rs` with retry logic
- [x] Add `reqwest` dependency

### Verification (from bead acceptance_criteria)
Tests:
- [x] Unit test: retry with exponential backoff
- [x] Integration test: client connects to mock server
Checkpoints:
- [x] `cargo test` passes
- [x] `cargo clippy` clean

### Implementation (from bead notes)
Files created: src/api/client.rs, src/api/retry.rs
Tests: 12/12 passed
Review: APPROVED

### Committed
abc123d — feat(api): add client with retry support
```

This is the "build a speck view back from beads" feature — showing checked-off checklists and implementation details from the bead's accumulated state.

### Tests

- Test: Status with no beads synced falls back to checkbox counting
- Test: Status with beads shows correct complete/ready/blocked counts
- Test: Close_reason parsing extracts commit hash and summary
- Test: JSON output matches schema
- Test: `--full` view renders bead content correctly
- Golden test: Status output for a known speck matches expected

### Success Criteria

- `specks status <speck>` shows real implementation state from beads
- Complete steps show commit hash and summary
- Ready steps show remaining work (task/test counts)
- Blocked steps show what they're waiting on
- `--full` flag shows rich view with checked-off checklists
- JSON output is machine-readable for CI/dashboards

---

## Phase Dependencies

```
Phase A (Rich Sync)
  ↓
Phase B (Agent-Bead Communication)     ← depends on A (needs rich bead content)
  ↓
Phase C (Eliminate Session)            ← depends on B (agents no longer need session)
  ↓
Phase D (Status from Beads)            ← depends on A (needs rich content), soft-dep on C
```

Phase D technically only needs Phase A (rich bead content to display). But it's most valuable after Phase C, when beads are the single source of truth and there's no conflicting session state.

---

## Scope Summary

| Phase | Speck Title | Core Change | Files Affected |
|-------|-------------|-------------|----------------|
| A | Rich Beads Sync | Populate bead description/acceptance/design during sync | beads.rs, sync.rs, types.rs |
| B | Agent-Bead Communication | Agents read/write bead fields instead of artifact files | All agent .md files, implementer SKILL.md, beads.rs |
| C | Eliminate Session File | Derive all state from git + beads conventions | session.rs, step_commit.rs, step_publish.rs, worktree.rs, merge.rs, cli.rs, all agents |
| D | Status from Beads | Show real implementation state from bead data | status.rs, beads.rs |

---

## Future Consideration: Beyond Beads

As noted in discussion, we may eventually outgrow beads and want our own database. The `BeadsCli` abstraction in `beads.rs` is our boundary. If we pivot:

1. Replace `BeadsCli` with a native implementation (SQLite, custom store, etc.)
2. Keep the same method signatures
3. Everything upstream (sync, agents, status) continues to work

The richer we make beads usage now, the clearer the interface contract becomes for a future replacement. We're effectively defining our ideal work-item API through usage.
