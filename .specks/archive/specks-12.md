## Phase 1.0: Add `specks merge` Subcommand {#phase-merge}

**Purpose:** Provide a single command to automate the post-implementation merge workflow: commit infrastructure changes, verify PR checks, merge via squash, and clean up worktrees.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | kocienda |
| Status | draft |
| Target branch | main |
| Tracking issue/PR | TBD |
| Last updated | 2026-02-09 |
| Beads Root | `specks-s5s` |

---

### Phase Overview {#phase-overview}

#### Context {#context}

After implementing a speck in a worktree and creating a PR, users must perform several manual steps to complete the merge: commit any infrastructure file changes in main (agents, skills, CLAUDE.md), push main, merge the PR via `gh pr merge --squash`, pull the squashed commit, and clean up the worktree. This manual process is error-prone and tedious.

The `specks merge` command automates this entire workflow, providing safety checks along the way to prevent common mistakes like merging with uncommitted non-infrastructure files or failing PR checks.

#### Strategy {#strategy}

- Add a new top-level `merge` subcommand to the CLI
- Implement speck-to-worktree and worktree-to-PR lookup via session.json
- Categorize uncommitted changes as infrastructure vs other files
- Provide `--dry-run` to preview operations and `--force` to override warnings
- Reuse existing worktree cleanup logic for the final cleanup step
- Verify PR checks pass before merging to prevent broken builds

#### Stakeholders / Primary Customers {#stakeholders}

1. Developers using specks to implement features
2. Automation scripts that need deterministic merge workflows

#### Success Criteria (Measurable) {#success-criteria}

- `specks merge <speck>` successfully merges a PR and cleans up worktree (manual test)
- `specks merge --dry-run <speck>` shows all planned operations without side effects
- `specks merge` aborts with clear error when non-infrastructure files are uncommitted
- `specks merge` aborts with clear error when PR checks are failing or pending
- `specks merge` aborts when main has unpushed commits

#### Scope {#scope}

1. New `specks merge` subcommand with `--dry-run` and `--force` flags
2. Infrastructure file detection using defined patterns
3. Pre-merge validation: uncommitted changes, unpushed commits, PR check status
4. Auto-commit infrastructure files with descriptive commit message
5. PR merge via `gh pr merge --squash`
6. Worktree cleanup after successful merge

#### Non-goals (Explicitly out of scope) {#non-goals}

- Support for merge strategies other than squash (rebase, merge commit)
- Interactive conflict resolution
- Automatic retry on transient failures
- Support for multiple worktrees per speck

#### Dependencies / Prerequisites {#dependencies}

- `gh` CLI must be installed and authenticated
- Git must be configured with push access to origin
- Worktree must exist for the specified speck with a valid session.json
- PR must exist for the worktree's branch

#### Constraints {#constraints}

- Must not modify files in the worktree directory
- Must use `gh pr merge --squash` to match documented workflow
- All git operations must be atomic or have clear rollback semantics

#### Assumptions {#assumptions}

- One worktree per speck at a time (no multi-worktree resolution needed)
- Infrastructure files are defined by explicit patterns (no user configuration)
- PR branch name matches the branch stored in session.json

---

### 1.0.0 Design Decisions {#design-decisions}

#### [D01] Infrastructure file patterns are hardcoded (DECIDED) {#d01-infra-patterns}

**Decision:** Infrastructure files are identified by these glob patterns: `agents/*.md`, `.claude/skills/**/`, `.specks/specks-skeleton.md`, `.specks/config.toml`, `.specks/specks-implementation-log.md`, `.beads/*`, `CLAUDE.md`.

**Rationale:**
- These are the files that commonly change during speck development but belong in main
- Hardcoding avoids configuration complexity
- Patterns can be extended in future versions if needed

**Implications:**
- Speck content files (`specks-N.md`) are NOT included - they should already be in the PR
- Users cannot customize patterns without code changes

---

#### [D02] Abort on unpushed commits to main (DECIDED) {#d02-unpushed-abort}

**Decision:** The command aborts with an error if main has unpushed commits to origin.

**Rationale:**
- Merging the PR would create a non-linear history if main is ahead of origin/main
- Forcing users to sync first prevents surprising merge conflicts
- This matches the "clean main" expectation of the workflow

**Implications:**
- Users must run `git push` before `specks merge` if they have local commits
- Clear error message explains the requirement

---

#### [D03] Verify PR checks before merge (DECIDED) {#d03-check-verification}

**Decision:** The command verifies that all PR checks have passed before attempting merge. It aborts if any checks are failing or pending.

**Rationale:**
- Merging with failing checks can break main
- Pending checks may flip to failing, causing a broken merge
- GitHub's branch protection rules may block the merge anyway

**Implications:**
- Uses `gh pr checks` to query check status
- Clear error message lists failing/pending checks

---

#### [D04] Commit message format for infrastructure changes (DECIDED) {#d04-commit-message}

**Decision:** Infrastructure changes are committed with message format: `chore(<speck-name>): infrastructure updates`.

**Rationale:**
- Follows conventional commit format used elsewhere in the project
- Includes speck name for traceability
- Generic enough to cover various infrastructure file types

**Implications:**
- Commit message is generated automatically, not user-provided

---

#### [D05] Speck files excluded from infrastructure auto-commit (DECIDED) {#d05-speck-exclusion}

**Decision:** Speck content files (`.specks/specks-*.md` except skeleton and implementation-log) are NOT auto-committed as infrastructure.

**Rationale:**
- The speck file itself should already be committed in the PR branch
- If a speck file is modified in main, it likely indicates a workflow issue
- Prevents accidental divergence between main and PR

**Implications:**
- If a speck file has uncommitted changes in main, it will be categorized as "other" and trigger a warning/abort

---

#### [D06] Worktree lookup uses session.json (DECIDED) {#d06-session-lookup}

**Decision:** The command finds the worktree for a speck by searching `.specks-worktrees/*/session.json` files and matching the `speck_path` field.

**Rationale:**
- Session files already contain all needed information (branch name, worktree path)
- No need for a separate index or registry
- Consistent with existing worktree list implementation

**Implications:**
- If session.json is missing or corrupt, the command fails with a clear error

---

#### [D07] Use gh pr view to find PR by branch (DECIDED) {#d07-pr-lookup}

**Decision:** The command uses `gh pr view <branch> --json number,url,state` to find the PR associated with the worktree's branch.

**Rationale:**
- Simple and reliable - gh CLI handles the mapping
- Returns PR state to detect if already merged
- Provides URL for output messages

**Implications:**
- Requires gh CLI to be installed and authenticated
- Fails if no PR exists for the branch

---

### 1.0.1 Specification {#specification}

#### 1.0.1.1 Command Syntax {#command-syntax}

```
specks merge <speck> [--dry-run] [--force] [--json] [--quiet] [--verbose]
```

**Arguments:**
- `<speck>`: Path to the speck file (e.g., `.specks/specks-12.md` or `specks-12.md`)

**Flags:**
- `--dry-run`: Show what would happen without executing
- `--force`: Proceed even with non-infrastructure uncommitted files (with warning)
- `--json`: Output in JSON format
- `--quiet`: Suppress non-error output
- `--verbose`: Show detailed progress

#### 1.0.1.2 Workflow Steps {#workflow-steps}

The command executes these steps in order:

1. **Find worktree**: Locate session.json for the specified speck
2. **Check main sync**: Verify main is in sync with origin/main (no unpushed commits)
3. **Find PR**: Look up PR for the worktree's branch using `gh pr view`
4. **Check PR state**: Verify PR is open (not already merged or closed)
5. **Check PR status**: Verify all PR checks have passed
6. **Check uncommitted**: Detect uncommitted changes in main
7. **Categorize changes**: Separate infrastructure files from other files
8. **Validate or warn**: Abort if other files exist (unless --force)
9. **Commit infrastructure**: Auto-commit infrastructure files if any
10. **Push main**: Push the infrastructure commit to origin
11. **Merge PR**: Run `gh pr merge --squash <branch>`
12. **Pull main**: Pull to get the squashed commit
13. **Cleanup worktree**: Remove worktree, prune, delete local branch
14. **Report success**: Print merged PR URL and cleanup summary

#### 1.0.1.3 Error Conditions {#error-conditions}

| Condition | Exit Code | Error Message |
|-----------|-----------|---------------|
| Speck file not found | 1 | `Speck file not found: <path>` |
| Worktree not found | 2 | `No worktree found for speck: <path>` |
| Main has unpushed commits | 3 | `Main branch has unpushed commits. Run 'git push' first.` |
| PR not found | 4 | `No PR found for branch: <branch>` |
| PR already merged | 5 | `PR already merged: <url>` |
| PR closed without merge | 6 | `PR is closed without merge: <url>` |
| PR checks failing | 7 | `PR checks failing: <list>` |
| PR checks pending | 8 | `PR checks pending: <list>` |
| Non-infrastructure uncommitted (no --force) | 9 | `Uncommitted non-infrastructure files: <list>` |
| gh CLI not found | 10 | `gh CLI not found. Install from https://cli.github.com/` |
| Git operation failed | 11 | `Git operation failed: <details>` |
| Merge failed | 12 | `PR merge failed: <details>` |

#### 1.0.1.4 Infrastructure File Patterns {#infra-patterns}

**Table T01: Infrastructure File Patterns** {#t01-infra-patterns}

| Pattern | Description |
|---------|-------------|
| `agents/*.md` | Agent definition files |
| `.claude/skills/**/` | Skill directories and contents |
| `.specks/specks-skeleton.md` | Speck template |
| `.specks/config.toml` | Specks configuration |
| `.specks/specks-implementation-log.md` | Implementation log |
| `.beads/*` | Beads tracking files |
| `CLAUDE.md` | Project instructions |

**Excluded from infrastructure (treated as "other"):**
- `.specks/specks-*.md` (except skeleton and implementation-log)

#### 1.0.1.5 JSON Output Schema {#json-schema}

**Spec S01: Merge Command Response Schema** {#s01-merge-response}

**Success response:**

```json
{
  "status": "ok",
  "pr_url": "https://github.com/owner/repo/pull/123",
  "pr_number": 123,
  "branch_name": "specks/feature-20260209-120000",
  "infrastructure_committed": true,
  "infrastructure_files": ["CLAUDE.md", "agents/coder-agent.md"],
  "worktree_cleaned": true,
  "dry_run": false
}
```

**Dry-run response:**

```json
{
  "status": "ok",
  "dry_run": true,
  "would_commit": ["CLAUDE.md", "agents/coder-agent.md"],
  "would_merge_pr": "https://github.com/owner/repo/pull/123",
  "would_cleanup_worktree": ".specks-worktrees/specks__feature-20260209-120000"
}
```

**Error response:**

```json
{
  "status": "error",
  "error": "E003",
  "message": "Main branch has unpushed commits. Run 'git push' first.",
  "details": {
    "unpushed_count": 2
  }
}
```

---

### 1.0.2 Symbol Inventory {#symbol-inventory}

#### 1.0.2.1 New files {#new-files}

| File | Purpose |
|------|---------|
| `crates/specks/src/commands/merge.rs` | Merge command implementation |

#### 1.0.2.2 Symbols to add / modify {#symbols}

| Symbol | Kind | Location | Notes |
|--------|------|----------|-------|
| `Merge` | enum variant | `crates/specks/src/cli.rs` | New Commands variant |
| `MergeCommands` | struct (not needed) | - | Single command, no subcommands |
| `run_merge` | fn | `crates/specks/src/commands/merge.rs` | Main entry point |
| `MergeData` | struct | `crates/specks/src/commands/merge.rs` | JSON output structure |
| `find_worktree_for_speck` | fn | `crates/specks/src/commands/merge.rs` | Session lookup |
| `categorize_uncommitted` | fn | `crates/specks/src/commands/merge.rs` | Infrastructure vs other |
| `check_pr_status` | fn | `crates/specks/src/commands/merge.rs` | PR check verification |
| `InfrastructurePattern` | const | `crates/specks/src/commands/merge.rs` | Hardcoded patterns |

#### 1.0.2.3 CLI module exports {#cli-exports}

Add to `crates/specks/src/commands/mod.rs`:
- `pub mod merge;`
- `pub use merge::run_merge;`

---

### 1.0.3 Test Plan Concepts {#test-plan-concepts}

#### Test Categories {#test-categories}

| Category | Purpose | When to use |
|----------|---------|-------------|
| **Unit** | Test pattern matching, categorization logic | Infrastructure detection |
| **Integration** | Test full workflow with git/gh mocks | End-to-end scenarios |
| **Golden** | Verify JSON output format | Output schema compliance |

#### Test Scenarios {#test-scenarios}

**Unit tests:**
- `test_is_infrastructure_file`: Verify each pattern matches correctly
- `test_categorize_uncommitted`: Verify correct categorization
- `test_parse_gh_pr_view_output`: Verify JSON parsing from gh CLI

**Integration tests (require git repo):**
- `test_merge_dry_run`: Full workflow in dry-run mode
- `test_merge_aborts_on_unpushed`: Verify abort when main ahead
- `test_merge_aborts_on_other_files`: Verify abort with non-infra files
- `test_merge_force_with_other_files`: Verify --force allows proceed

**Manual tests:**
- End-to-end merge with real PR (documented in implementation log)

---

### 1.0.4 Execution Steps {#execution-steps}

#### Step 0: Add merge command skeleton {#step-0}

**Bead:** `specks-s5s.1`

**Commit:** `feat(cli): add specks merge command skeleton`

**References:** [D01] Infrastructure file patterns, [D06] Worktree lookup, (#command-syntax, #symbols)

**Artifacts:**
- `crates/specks/src/commands/merge.rs` - New file with command structure
- `crates/specks/src/commands/mod.rs` - Export merge module
- `crates/specks/src/cli.rs` - Add Merge variant to Commands enum

**Tasks:**
- [ ] Create `merge.rs` with `run_merge` function signature
- [ ] Add `Merge` variant to `Commands` enum in `cli.rs`
- [ ] Add command routing in `main.rs`
- [ ] Implement `--dry-run` and `--force` flag parsing
- [ ] Add `MergeData` struct for JSON output

**Tests:**
- [ ] Unit test: CLI parses `specks merge` command correctly
- [ ] Unit test: `--dry-run` and `--force` flags parsed

**Checkpoint:**
- [ ] `cargo build` succeeds
- [ ] `cargo nextest run` passes
- [ ] `specks merge --help` shows usage

**Rollback:**
- Revert commit, remove merge.rs

**Commit after all checkpoints pass.**

---

#### Step 1: Implement worktree and PR lookup {#step-1}

**Depends on:** #step-0

**Bead:** `specks-s5s.2`

**Commit:** `feat(merge): implement worktree and PR lookup`

**References:** [D06] Worktree lookup, [D07] PR lookup, (#workflow-steps, #error-conditions)

**Artifacts:**
- `find_worktree_for_speck` function
- `get_pr_for_branch` function using gh CLI

**Tasks:**
- [ ] Implement `find_worktree_for_speck` to search session.json files
- [ ] Implement `get_pr_for_branch` using `gh pr view <branch> --json`
- [ ] Handle PR not found, already merged, and closed states
- [ ] Return structured data with PR URL, number, and state

**Tests:**
- [ ] Unit test: find_worktree_for_speck with valid session
- [ ] Unit test: find_worktree_for_speck with no matching session
- [ ] Unit test: parse gh pr view JSON output
- [ ] Unit test: handle various PR states (open, merged, closed)

**Checkpoint:**
- [ ] `cargo nextest run` passes
- [ ] Manual test: command finds worktree and reports PR info

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

#### Step 2: Implement uncommitted file categorization {#step-2}

**Depends on:** #step-0

**Bead:** `specks-s5s.3`

**Commit:** `feat(merge): implement infrastructure file categorization`

**References:** [D01] Infrastructure file patterns, [D05] Speck exclusion, Table T01, (#infra-patterns)

**Artifacts:**
- `is_infrastructure_file` function
- `categorize_uncommitted` function
- `INFRASTRUCTURE_PATTERNS` constant

**Tasks:**
- [ ] Define infrastructure patterns as glob-compatible patterns
- [ ] Implement `is_infrastructure_file` matching function
- [ ] Implement `categorize_uncommitted` using `git status --porcelain`
- [ ] Return two lists: infrastructure files and other files
- [ ] Ensure speck files (except skeleton/log) are categorized as "other"

**Tests:**
- [ ] Unit test: each infrastructure pattern matches correctly
- [ ] Unit test: speck content files are NOT infrastructure
- [ ] Unit test: skeleton and implementation-log ARE infrastructure
- [ ] Integration test: categorize_uncommitted with mixed files

**Checkpoint:**
- [ ] `cargo nextest run` passes

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

#### Step 3: Implement pre-merge validations {#step-3}

**Depends on:** #step-1, #step-2

**Bead:** `specks-s5s.4`

**Commit:** `feat(merge): implement pre-merge validations`

**References:** [D02] Unpushed abort, [D03] Check verification, (#error-conditions)

**Artifacts:**
- `check_main_sync` function
- `check_pr_checks` function
- Validation orchestration in run_merge

**Tasks:**
- [ ] Implement `check_main_sync` using `git rev-list origin/main..main`
- [ ] Implement `check_pr_checks` using `gh pr checks <branch>`
- [ ] Parse check output to detect failing/pending checks
- [ ] Wire validations into main run_merge flow
- [ ] Implement --force flag to skip non-infrastructure file check

**Tests:**
- [ ] Unit test: parse git rev-list output
- [ ] Unit test: parse gh pr checks output
- [ ] Integration test: abort on unpushed commits
- [ ] Integration test: abort on failing checks
- [ ] Integration test: abort on pending checks
- [ ] Integration test: --force proceeds with other files (with warning)

**Checkpoint:**
- [ ] `cargo nextest run` passes
- [ ] Manual test: command aborts correctly on validation failures

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

#### Step 4: Implement merge workflow {#step-4}

**Depends on:** #step-3

**Bead:** `specks-s5s.5`

**Commit:** `feat(merge): implement full merge workflow`

**References:** [D04] Commit message format, (#workflow-steps, #json-schema)

**Artifacts:**
- Infrastructure commit logic
- PR merge via gh CLI
- Worktree cleanup integration
- Complete JSON output

**Tasks:**
- [ ] Implement infrastructure file staging and commit
- [ ] Implement `git push` for main
- [ ] Implement `gh pr merge --squash <branch>`
- [ ] Implement `git pull` after merge
- [ ] Integrate existing cleanup_worktrees logic
- [ ] Implement complete JSON output format
- [ ] Implement dry-run mode showing all planned operations

**Tests:**
- [ ] Integration test: dry-run shows correct operations
- [ ] Integration test: JSON output matches schema
- [ ] Manual test: full workflow with real PR

**Checkpoint:**
- [ ] `cargo nextest run` passes
- [ ] `specks merge --dry-run .specks/specks-N.md` shows operations
- [ ] Manual end-to-end test documented

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

#### Step 5: Add documentation and final polish {#step-5}

**Depends on:** #step-4

**Bead:** `specks-s5s.6`

**Commit:** `docs(merge): update CLAUDE.md with merge command`

**References:** (#command-syntax, #workflow-steps)

**Artifacts:**
- Updated CLAUDE.md with merge command documentation
- Help text polish

**Tasks:**
- [ ] Add merge command to CLAUDE.md CLI commands section
- [ ] Add merge workflow to worktree documentation section
- [ ] Review and polish --help text
- [ ] Verify all error messages are clear and actionable

**Tests:**
- [ ] Unit test: help text includes all documented flags
- [ ] Review: CLAUDE.md is accurate and complete

**Checkpoint:**
- [ ] `cargo nextest run` passes
- [ ] `specks merge --help` matches documentation

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

### 1.0.5 Deliverables and Checkpoints {#deliverables}

**Deliverable:** A `specks merge` command that automates the post-implementation PR merge workflow with safety checks.

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

- [ ] `specks merge <speck>` successfully merges PR and cleans up worktree (manual test)
- [ ] `specks merge --dry-run <speck>` shows all planned operations without side effects
- [ ] All validation errors produce clear, actionable messages
- [ ] JSON output matches documented schema
- [ ] CLAUDE.md documents the command and its flags

**Acceptance tests:**
- [ ] Integration test: full dry-run workflow
- [ ] Integration test: abort on each validation failure type
- [ ] Manual test: end-to-end merge with real PR

#### Roadmap / Follow-ons (Explicitly Not Required for Phase Close) {#roadmap}

- [ ] User-configurable infrastructure patterns via config.toml
- [ ] Support for merge strategies other than squash
- [ ] Retry logic for transient network failures
- [ ] Multi-worktree resolution if multiple exist for same speck

| Checkpoint | Verification |
|------------|--------------|
| CLI parses correctly | `specks merge --help` |
| Build succeeds | `cargo build` |
| Tests pass | `cargo nextest run` |
| End-to-end works | Manual test with real PR |

**Commit after all checkpoints pass.**