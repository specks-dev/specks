## Phase 1.0: Git Worktree Integration for Specks Workflow {#phase-worktree}

**Purpose:** Enable isolated implementation environments via git worktrees, with automatic PR creation and simplified session management, eliminating the need for complex session directories and enabling true branch-per-speck workflows.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | kocienda |
| Status | draft |
| Target branch | main |
| Tracking issue/PR | TBD |
| Last updated | 2026-02-08 |
| Beads Root | *(optional; written by `specks beads sync`)* |

---

### Phase Overview {#phase-overview}

#### Context {#context}

The current specks implementation workflow uses `.specks/runs/<session-id>/` directories to track session state, with complex metadata files (`metadata.json`), step-level output directories (`step-N/*.json`), and manual commit policies. This complexity creates friction and makes it difficult to understand what work is in progress.

Git worktrees provide a native solution: each speck implementation gets its own branch and worktree, changes are isolated automatically, and the PR review process provides a natural checkpoint before merging to main. This approach leverages git's built-in branch isolation rather than reinventing session management.

#### Strategy {#strategy}

- **Worktrees as sessions**: Replace `.specks/runs/<session-id>/` with `.specks-worktrees/<worktree_dir_name>/` (filesystem-safe name derived from branch; see [D08])
- **One commit per step**: Each execution step becomes exactly one git commit, matching bead granularity
- **Path prefixing over persistent cd**: All agents receive `worktree_path` and use absolute paths; they MUST NOT rely on shell working-directory persistence between commands. If a tool does not support `-C`/path arguments, it is acceptable to use `cd {worktree_path} && <cmd>` **within a single command invocation only**.
- **Always-auto commit**: Eliminate manual/auto commit policy; every step completion triggers a commit
- **PR as approval gate**: Implementation is complete when PR is merged; this replaces the Done/Abort prompts
- **Minimal session state**: Replace complex metadata with `session.json` containing only essential state
- **Phased rollout**: MVP in Phase 1, concurrent sessions and recovery in Phase 2, dashboard in Phase 3

#### Stakeholders / Primary Customers {#stakeholders}

1. Developers using specks to implement features
2. Teams reviewing implementation PRs

#### Success Criteria (Measurable) {#success-criteria}

- `specks implementer .specks/specks-N.md` creates a worktree, executes all steps with commits, creates a PR (verify: PR exists on GitHub)
- Each execution step produces exactly one commit (verify: in the worktree, `git log` shows **N step commits for N steps**, plus an optional preflight “sync beads” commit that is not counted as a step)
- Implementation log is committed with each step and appears in PR (verify: log present in PR diff)
- `specks worktree cleanup --merged` removes worktrees for merged PRs (verify: directory removed)
- No agent relies on persistent working-directory changes (verify: no standalone `cd` usage; `cd {worktree_path} && ...` is allowed only as a single-command prefix when a tool lacks `-C`)

#### Scope {#scope}

1. CLI commands: `specks worktree create`, `specks worktree list`, `specks worktree cleanup`
2. Modified implementer skill: worktree creation, path prefixing, PR creation
3. Modified committer agent: git operations with `-C` flag, bead closing
4. Session model: `session.json` replaces `metadata.json`
5. Implementation log: lives in worktree, committed with each step
6. Publish: push branch and create PR at end of successful run

#### Non-goals (Explicitly out of scope) {#non-goals}

- Phase 2: Concurrent session detection and conflict handling
- Phase 2: Session resume after failure
- Phase 3: Multi-speck dashboard
- Phase 3: Automatic worktree cleanup on merge detection
- Nested worktrees or worktree-within-worktree scenarios

#### Dependencies / Prerequisites {#dependencies}

- Git 2.15+ (worktree support)
- GitHub CLI (`gh`) 2.0+ for PR creation (required for `--body-file` and `--repo` flags)
- Existing beads integration (for bead sync and close)
- Rust 1.70+ (for build, if building from source)

#### Constraints {#constraints}

- Must work with existing speck format (no speck schema changes)
- Must maintain backward compatibility with existing beads workflow
- Path prefixing must work on macOS, Linux, and Windows (path separators)

#### Assumptions {#assumptions}

- Branch naming: `specks/<speck-slug>-<timestamp>` (e.g., `specks/auth-20260208-143022`)
- Worktree directory uses a **filesystem-safe name**, derived from branch name:
  - `worktree_dir_name = branch_name` with `/` replaced by `__`
  - Example: branch `specks/auth-20260208-143022` → dir `.specks-worktrees/specks__auth-20260208-143022/`
- One commit per step means bead granularity equals commit granularity (1:1 mapping)
- Commit messages follow conventional commits format with step info in body
- PR is always created to main branch (or current branch if not on main during setup)
- Cleanup command is manual in Phase 1-2; auto-detection added in Phase 3
- Setup agent runs in repo root; all other agents operate with explicit `worktree_path`
- Beads sync is performed **inside the worktree** (so speck bead annotations live on the PR branch, not the user’s base branch)
- Beads are closed after each successful step commit (see [D10])
- Implementation log format remains markdown; only location changes
- Git operations use `-C` flag for worktree operations from main directory

---

### Open Questions (MUST RESOLVE OR EXPLICITLY DEFER) {#open-questions}

#### [Q01] Windows Path Compatibility (RESOLVED) {#q01-windows-paths}

**Question:** How do we handle path prefixing on Windows where path separators differ?

**Why it matters:** Agents need to construct absolute paths; incorrect separators will cause file operation failures.

**Options (if known):**
- Use forward slashes everywhere (git and most tools accept them on Windows)
- Use `std::path::PathBuf` consistently and let Rust handle it
- Normalize all paths through a utility function

**Plan to resolve:** Use `std::path::Path` / `PathBuf` for all filesystem path construction and serialization; avoid manual separator logic. For commands, prefer `git -C {worktree_path}`; where a tool lacks `-C`, use `cd {worktree_path} && ...` within a single command invocation only (no persistent cwd reliance). Avoid shell heredocs for PR bodies by using `--body-file`.

**Resolution:** RESOLVED - PathBuf + `git -C` + single-command `cd` prefix (when necessary) + `gh --body-file` is the Phase 1 cross-platform strategy.

---

#### [Q02] Where Beads Sync Writes (RESOLVED) {#q02-beads-sync-location}

**Question:** When we run `specks beads sync`, which working tree should it modify—the user’s base checkout, or the speck’s worktree branch?

**Why it matters:** `specks beads sync` writes bead IDs into the speck. If it runs in the base checkout, it will dirty the user’s current branch and create changes that are not part of the implementation PR. This breaks the “worktree as session” model.

**Proposed resolution:** Run beads sync **in the worktree** after worktree creation, so bead annotations are committed/pushed with the implementation PR.

**Resolution:** RESOLVED - beads sync runs **inside the worktree** after worktree creation, so speck bead annotations are committed on the PR branch (see assumptions and Step 4 tasks).

---

#### [Q03] Partial Failure Semantics (RESOLVED) {#q03-partial-failures}

**Question:** What is the system’s behavior if:

- A step commit succeeds, but bead close fails?
- All step commits succeed, but push fails?
- Push succeeds, but PR creation fails?

**Why it matters:** Git and Beads/GitHub are separate systems; true atomicity is not possible. We must define “done” and reconciliation steps so we don’t re-run code changes or silently lose accounting.

**Proposed resolution:** Define explicit statuses and retry/reconcile behavior in `session.json` and in agent output contracts.

**Resolution:** RESOLVED (Phase 1):

- **Commit succeeds, bead close fails:** mark session `status = needs_reconcile` and HALT. Do **not** re-run code changes. Remediation is to retry bead close for the recorded `bead_id` for that step (see [D10]).
- **All step commits succeed, push fails:** mark session `status = failed` and HALT. Worktree/branch remains intact for manual remediation (retry push) and/or re-running the final publish operation.
- **Push succeeds, PR creation fails:** mark session `status = failed` and HALT. Branch is pushed; remediation is to rerun PR creation (manual `gh pr create` using Spec S03, or re-running the final publish operation).

---

#### [Q04] Artifact Retention (RESOLVED) {#q04-artifact-retention}

**Question:** Do we still want structured per-step artifacts (architect/coder/reviewer/auditor outputs) persisted on disk for debugging, or is the git history + implementation log sufficient?

**Why it matters:** Removing `.specks/runs/.../step-N/*.json` entirely reduces debuggability, especially when investigating drift/review loops.

**Options:**
- Keep a minimal `.specks/step-artifacts/step-N/` inside the worktree (optionally committed).
- Keep artifacts uncommitted but present in the worktree directory.
- Drop artifacts entirely (log-only).

**Resolution:** RESOLVED (Phase 1): keep **optional, uncommitted** step artifacts under `{worktree_path}/.specks/step-artifacts/step-N/` for local debugging, but do **not** commit them. The committed record is code changes + `.specks/specks-implementation-log.md` (per [D04]) + beads annotations.

---

#### [Q05] Definition of “Merged” for Cleanup (DEFERRED) {#q05-merged-definition}

**Question:** After Phase 1’s git-only behavior, do we want Phase 3 to optionally use GitHub PR state (via `gh`) for cleanup?

**Why it matters:** Phase 1 is intentionally git-only ([D09]). PR-state cleanup can improve UX (cleanup by PR merge) but adds auth and API dependency.

**Resolution:** DEFERRED to Phase 3.

---

### Risks and Mitigations {#risks}

| Risk | Impact | Likelihood | Mitigation | Trigger to revisit |
|------|--------|------------|------------|--------------------|
| Worktree creation fails on network drives | high | low | Detect and warn; require local repos | User reports failure on network path |
| Large repos make worktree creation slow | med | med | Document performance expectations; consider shallow clone option | User reports >30s worktree creation |
| Orphaned worktrees accumulate | low | high | `cleanup --merged` command; docs on manual cleanup | Disk space complaints |
| Bead close fails after commit | high | med | Define reconcile path; retry bead close; mark session as needs_reconcile | Users see “done in git but not in beads” |
| Push/PR creation fails at end | med | med | Define publish step with retries; persist publish info; allow re-run publish without re-running steps | Users have local branch but no PR |

**Risk R01: Worktree State Corruption** {#r01-worktree-corruption}

- **Risk:** Interrupted implementation leaves worktree in inconsistent state
- **Mitigation:**
  - Each step is atomic (commit or no changes)
  - `session.json` tracks next step index (`current_step`)
  - Resume support in Phase 2 will detect and continue from last good state
- **Residual risk:** User must manually clean up if failure occurs before Phase 2 resume support

---

### 1.0.0 Design Decisions {#design-decisions}

#### [D01] Worktrees Replace Session Directories (DECIDED) {#d01-worktrees-replace-sessions}

**Decision:** Use `.specks-worktrees/<worktree_dir_name>/` (derived from branch name; see [D08]) instead of `.specks/runs/<session-id>/` for implementation sessions.

**Rationale:**
- Git worktrees provide native branch isolation
- Changes are tracked by git, not custom metadata
- PR review is the natural approval gate
- Eliminates need for complex session state management

**Implications:**
- `.specks-worktrees/` must be added to `.gitignore`
- Session cleanup becomes worktree cleanup
- Multiple concurrent implementations require separate branches

---

#### [D02] One Commit Per Step (DECIDED) {#d02-one-commit-per-step}

**Decision:** Each execution step produces exactly one git commit, and that commit closes the corresponding bead (see [D10] for partial-failure semantics).

**Rationale:**
- Matches bead granularity (1:1 mapping)
- Git history becomes implementation log
- Easy to review individual step changes
- Atomic: if commit fails, step is not complete

**Implications:**
- Committer agent runs after every step (not batched)
- Commit message format must include step reference
- Bead close happens atomically with commit

---

#### [D03] Path Prefixing Over Directory Changes (DECIDED) {#d03-path-prefixing}

**Decision:** All agents receive `worktree_path` in their input and use absolute path prefixes for all file operations. No agent relies on persistent working-directory state; `cd {worktree_path} && ...` is allowed only as a single-command prefix when a tool lacks `-C`.

**Rationale:**
- Shell state does not persist between bash calls in Claude Code
- Explicit paths are more debuggable
- Avoids "which directory am I in?" confusion
- Git `-C` flag provides same isolation for git commands

**Implications:**
- All agent input contracts must include `worktree_path` field
- File operations use `{worktree_path}/{relative_path}` pattern
- Git commands use `git -C {worktree_path} <command>` pattern
- Tests must verify no reliance on persistent `cd` state; `cd {worktree_path} && ...` is allowed only as a single-command prefix when a tool lacks `-C`

---

#### [D04] Implementation Log Lives in Worktree (DECIDED) {#d04-log-in-worktree}

**Decision:** The implementation log file lives in the worktree and is committed with each step, appearing in the final PR.

**Rationale:**
- Log is part of the implementation record
- PR reviewers can see what was done
- Log survives worktree cleanup via merge
- Eliminates separate log sync step

**Implications:**
- Log path: `{worktree_path}/.specks/specks-implementation-log.md`
- Logger agent appends to log and stages it
- Committer agent includes log in step commit

---

#### [D05] Always Auto-Create PR (DECIDED) {#d05-always-auto-pr}

**Decision:** After all steps complete successfully, automatically create a PR to the base branch. No manual/auto option.

**Rationale:**
- Simplifies workflow (no decisions needed)
- PR is the review gate; always want one
- User can close PR if they change their mind
- Consistent behavior across all implementations

**Implications:**
- Workflow always creates a PR at the end by invoking an agent with shell capability (e.g., committer-agent in “publish” mode)
- PR title/body generated from speck metadata
- Base branch determined at worktree creation time

---

#### [D06] Minimal Session State (DECIDED) {#d06-minimal-session-state}

**Decision:** Replace complex `metadata.json` with minimal `session.json` containing only: speck_path, branch_name, base_branch, current_step, status.

**Rationale:**
- Most state is in git (commits, branch)
- Beads track step completion
- Reduces state synchronization bugs
- Easier to reason about

**Worktree Discovery:**
Note that `worktree_path` is stored IN the session, but the session is stored AT the worktree path. This is not circular because worktrees are discovered by **scanning the `.specks-worktrees/` directory**, not by reading session files:

```
list_worktrees():
  for dir in .specks-worktrees/*/:
    session = load_session(dir/.specks/session.json)
    yield session
```

The `worktree_path` field in session.json provides the **absolute path** for use by agents, avoiding relative path ambiguity.

**Implications:**
- `session.json` location: `{worktree_path}/.specks/session.json`
- Session state is: `pending`, `in_progress`, `completed`, `failed`, `needs_reconcile` (see [D10])
- No committed step-level output directories; outputs are in commits. Optional local debug artifacts may live under `{worktree_path}/.specks/step-artifacts/` and are ignored by git (see [Q04]).

---

#### [D07] Manual Cleanup in Phase 1 (DECIDED) {#d07-manual-cleanup}

**Decision:** Worktree cleanup is manual via `specks worktree cleanup --merged`. Automatic detection added in Phase 3.

**Rationale:**
- Simpler implementation for MVP
- User has control over when cleanup happens
- Avoids accidental cleanup of in-progress work
- Phase 3 can add `--auto` flag

**Implications:**
- User must run cleanup command after PR merge
- Orphaned worktrees may accumulate
- Documentation must cover cleanup workflow

---

#### [D08] Filesystem-Safe Worktree Directory Names (DECIDED) {#d08-worktree-dir-names}

**Decision:** Worktree directories are derived from branch names using a filesystem-safe mapping:

```
fn sanitize_branch_name(branch_name: &str) -> String {
    let sanitized: String = branch_name
        .replace("/", "__")           // Git path separators
        .replace("\\", "__")          // Windows path separators
        .replace(":", "_")            // Windows drive letters
        .replace(" ", "_")            // Spaces (shell escaping)
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect();

    if sanitized.is_empty() {
        // Extremely defensive fallback (should never happen with our branch format).
        "specks-worktree".to_string()
    } else {
        sanitized
    }
}
```

- Worktree path: `.specks-worktrees/{sanitized_name}/`
- Example: `specks/auth-feature` → `.specks-worktrees/specks__auth-feature/`

**Rationale:**
- Branch names commonly contain `/` and may contain other problematic characters
- We want predictable, flat worktree directories
- Avoid accidental nested directories and platform-specific oddities
- Sanitization ensures paths work on macOS, Linux, and Windows

**Implications:**
- Session records must store both `branch_name` and `worktree_path`
- CLI `list` must present both branch and directory
- Add `sanitize_branch_name()` utility function to `worktree.rs`

---

#### [D09] Phase 1 "Merged" Definition is Git-Only (DECIDED) {#d09-merged-is-git-only}

**Decision:** In Phase 1, `cleanup --merged` determines merged status using git history only (no GitHub API dependency):

- A branch is considered merged if `git merge-base --is-ancestor <branch> <base_branch>` returns success.

**Rationale:**
- Safe and deterministic offline behavior
- Avoid requiring `gh` auth for cleanup
- Avoid ambiguity around PR state vs merge state

**Known Limitations:**
- **Squash merges:** If a PR is squash-merged, the branch commits are not ancestors of main. The branch will appear "not merged" even though the work is in main.
- **Rebase merges:** Similar issue - commit SHAs change during rebase, so original branch commits are not ancestors.
- **Stale local refs:** If `main` is not up-to-date with `origin/main`, merged branches may appear unmerged.

**Workarounds:**
- Run `git fetch origin main` before cleanup
- Use `--force` flag (Phase 2) to bypass the check for known-merged branches
- Phase 3 may add GitHub PR-based cleanup that handles squash/rebase correctly

**Implications:**
- Cleanup may require that the base branch is up-to-date locally; docs should recommend `git fetch` before cleanup
- Add `--force` flag in Phase 2 for manual override
- Phase 3 may add GitHub PR-based cleanup as an option
- Cleanup MUST use git-native worktree removal (`git worktree remove`) to avoid leaving orphaned entries under `.git/worktrees/`

---

#### [D10] Step Completion Semantics (DECIDED) {#d10-step-completion-semantics}

**Decision:** A step is considered **complete** only when:

1. The step commit is created successfully, **and**
2. The corresponding bead is closed successfully.

If (1) succeeds but (2) fails, the session status becomes `needs_reconcile` and the workflow halts with a clear remediation action: “retry bead close for bead X for commit Y”.

**Rationale:**
- Prevent “git says done, beads says not done” divergence
- Make partial failures explicit and recoverable without re-running code changes

**Implications:**
- `session.json` status enum must include `needs_reconcile`
- Committer agent must emit enough data to support retry (commit hash, bead id, close reason)

---

### Deep Dives {#deep-dives}

#### Worktree Lifecycle Flow {#worktree-lifecycle}

**Diagram Diag01: Worktree Lifecycle** {#diag01-worktree-lifecycle}

```
Main Branch (user working directory)
    |
    v
[1] specks implementer .specks/specks-N.md
    |
    +-- setup-agent: sync beads, determine steps
    |
    v
[2] Create branch: specks/<speck-slug>-<timestamp>
    |
    v
[3] Create worktree: .specks-worktrees/<sanitized-branch-name>/
    |    (e.g., specks/auth-20260208 → specks__auth-20260208)
    |
    +-- Copy relevant files to worktree
    +-- Initialize session.json
    |
    v
[4] For each step (in worktree via path prefixing):
    |
    +-- architect-agent: plan step
    +-- coder-agent: implement
    +-- reviewer-agent: verify
    +-- auditor-agent: check quality
    +-- logger-agent: update log
    +-- committer-agent: commit + close bead
    |
    v
[5] All steps complete
    |
    v
[6] Push branch, create PR
    |
    v
[7] User reviews and merges PR
    |
    v
[8] specks worktree cleanup --merged
    |
    +-- Remove worktree directory
    +-- Remove branch (if merged)
```

---

#### Session JSON Schema {#session-json-schema}

**Spec S01: session.json Schema** {#s01-session-json}

```json
{
  "schema_version": "1",
  "speck_path": ".specks/specks-5.md",
  "speck_slug": "auth",
  "branch_name": "specks/auth-20260208-143022",
  "base_branch": "main",
  "worktree_path": "/abs/path/to/.specks-worktrees/specks__auth-20260208-143022",
  "created_at": "2026-02-08T14:30:22Z",
  "status": "in_progress",
  "current_step": 2,
  "total_steps": 5,
  "beads_root": "bd-abc123"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `schema_version` | string | yes | Schema version for forward compatibility |
| `speck_path` | string | yes | Relative path to speck file from repo root |
| `speck_slug` | string | yes | Short name derived from speck for branch naming |
| `branch_name` | string | yes | Full branch name created for this implementation |
| `base_branch` | string | yes | Branch to merge back to (usually main) |
| `worktree_path` | string | yes | Absolute path to worktree directory |
| `created_at` | string | yes | ISO 8601 timestamp of session creation |
| `status` | string | yes | One of: pending, in_progress, completed, failed, needs_reconcile |
| `current_step` | integer | yes | Index of the **next step to execute** (0-based). After successfully completing step N, set `current_step = N + 1`. |
| `total_steps` | integer | yes | Total number of steps in speck |
| `beads_root` | string | no | Root bead ID if beads are synced |

---

#### Agent Input Contract Changes {#agent-input-changes}

**Spec S02: Worktree Path in Agent Inputs** {#s02-agent-inputs}

All implementation agents must accept `worktree_path` in their input:

```json
{
  "worktree_path": "/abs/path/to/.specks-worktrees/specks__auth-20260208-143022",
  "speck_path": ".specks/specks-5.md",
  "step": { ... },
  ...other fields...
}
```

Agents MUST:
- Prefix all file reads with `worktree_path`
- Prefix all file writes with `worktree_path`
- Use `git -C {worktree_path}` for git commands
- Never rely on persistent `cd` state; `cd {worktree_path} && ...` is allowed only as a single-command prefix when a tool lacks `-C`

**Example: Logger Agent Input**

```json
{
  "worktree_path": "/abs/path/to/.specks-worktrees/specks__auth-20260208-143022",
  "speck_path": ".specks/specks-5.md",
  "step_anchor": "#step-1",
  "summary": "Implemented user login endpoint",
  "files_changed": ["src/api/auth.rs", "src/api/mod.rs"]
}
```

The logger agent writes to `{worktree_path}/.specks/specks-implementation-log.md`.

---

#### PR Creation Details {#pr-creation-details}

**Spec S03: PR Creation** {#s03-pr-creation}

After all steps complete, generate a PR body file in the worktree (recommended path: `{worktree_path}/.specks/pr-body.md`), then create the PR using:

```bash
gh pr create \
  --base {base_branch} \
  --head {branch_name} \
  --title "feat(specks): {speck_title}" \
  --body-file {worktree_path}/.specks/pr-body.md
```

---

#### Committer Agent Publish Mode {#committer-publish-mode}

**Spec S04: Committer Agent Operation Modes** {#s04-committer-modes}

The committer-agent supports two operation modes, distinguished by the `operation` field:

**Mode 1: `commit` (default)** - Commit step changes and close bead

```json
{
  "operation": "commit",
  "worktree_path": "/abs/path/to/.specks-worktrees/specks__auth-20260208-143022",
  "step_anchor": "#step-N",
  "commit_message": "feat(scope): description",
  "files_to_stage": ["src/file.rs", ".specks/specks-implementation-log.md"],
  "bead_id": "bd-abc123",
  "close_reason": "Step N complete: description"
}
```

Output:
```json
{
  "operation": "commit",
  "success": true,
  "commit_hash": "abc1234def",
  "bead_closed": true,
  "bead_id": "bd-abc123",
  "needs_reconcile": false
}
```

**Mode 2: `publish`** - Push branch and create PR

```json
{
  "operation": "publish",
  "worktree_path": "/abs/path/to/.specks-worktrees/specks__auth-20260208-143022",
  "branch_name": "specks/auth-20260208-143022",
  "base_branch": "main",
  "repo": "owner/repo",
  "speck_title": "Add user authentication",
  "speck_path": ".specks/specks-auth.md",
  "step_summaries": ["Step 0: Added login endpoint", "Step 1: Added logout endpoint"]
}
```

`repo` is optional and may be `null`. When present, it MUST be in `owner/repo` format (GitHub).

The agent MUST:
1. Check `gh auth status` before attempting PR creation
2. Generate `{worktree_path}/.specks/pr-body.md` from step_summaries
3. Push branch: `git -C {worktree_path} push -u origin {branch_name}`
4. Create the PR using one of the following approaches (either is acceptable):
   - **Preferred** (explicit repo): `gh pr create --repo {repo} --base {base_branch} --head {branch_name} --title "..." --body-file ...`
   - **Fallback** (git context): `cd {worktree_path} && gh pr create --base {base_branch} --head {branch_name} --title "..." --body-file ...`

Output:
```json
{
  "operation": "publish",
  "success": true,
  "pushed": true,
  "pr_created": true,
  "repo": "owner/repo",
  "pr_url": "https://github.com/owner/repo/pull/123",
  "pr_number": 123,
  "error": null
}
```

If push fails (branch not published):
```json
{
  "operation": "publish",
  "success": false,
  "pushed": false,
  "pr_created": false,
  "repo": "owner/repo",
  "pr_url": null,
  "pr_number": null,
  "error": "git push failed: <details>"
}
```

If push succeeds but PR creation fails:
```json
{
  "operation": "publish",
  "success": false,
  "pushed": true,
  "pr_created": false,
  "repo": "owner/repo",
  "pr_url": null,
  "pr_number": null,
  "error": "gh pr create failed: <details>"
}
```

If `gh auth status` fails:
```json
{
  "operation": "publish",
  "success": false,
  "pushed": false,
  "pr_created": false,
  "repo": "owner/repo",
  "pr_url": null,
  "pr_number": null,
  "error": "GitHub CLI not authenticated. Run 'gh auth login' first."
}
```

**`repo` field derivation:**
```bash
# Get repo from git remote
git -C {worktree_path} remote get-url origin | sed -E 's|.*[:/]([^/]+/[^/]+)(\.git)?$|\1|'
# Example: "git@github.com:owner/repo.git" → "owner/repo"
# Example: "https://github.com/owner/repo.git" → "owner/repo"
```
If remote is not GitHub or parsing fails, set `repo = null` and use fallback approach (cd to worktree for `gh` context).

**`step_summaries` format:**
- Array of strings, one per completed step
- Format: `"Step {N}: {first line of commit message or logger summary}"`
- Example: `["Step 0: Added .gitignore entries", "Step 1: Implemented session module"]`
- Collected by implementer skill from each step's commit message

---

#### Speck Slug Derivation {#slug-derivation}

**Spec S05: Speck Slug Derivation** {#s05-slug-derivation}

The speck slug is used in branch naming (`specks/<slug>-<timestamp>`). It is derived from the speck filename:

```rust
fn derive_speck_slug(speck_path: &Path) -> String {
    // Input: ".specks/specks-auth-feature.md"
    // Output: "auth-feature"
    let filename = speck_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    filename
        .strip_prefix("specks-")
        .unwrap_or(filename)
        .to_string()
}
```

**Examples:**
| Speck Path | Slug |
|------------|------|
| `.specks/specks-auth.md` | `auth` |
| `.specks/specks-worktree-integration.md` | `worktree-integration` |
| `.specks/specks-1.md` | `1` |
| `.specks/my-feature.md` | `my-feature` |

**Timestamp format:** `YYYYMMDD-HHMMSS` in **UTC** (e.g., `20260208-143022`)

**Full branch name:** `specks/{slug}-{timestamp}` (e.g., `specks/auth-20260208-143022`)

---

#### Updated Setup Agent Output {#setup-agent-output}

**Spec S06: Setup Agent Output with Worktree** {#s06-setup-output}

After worktree creation and beads sync, the setup agent returns:

```json
{
  "status": "ready",
  "worktree_path": "/abs/path/to/.specks-worktrees/specks__auth-20260208-143022",
  "branch_name": "specks/auth-20260208-143022",
  "base_branch": "main",
  "speck_path": ".specks/specks-auth.md",
  "resolved_steps": ["#step-0", "#step-1", "#step-2"],
  "beads": {
    "sync_performed": true,
    "root_bead": "bd-abc123",
    "bead_mapping": {
      "#step-0": "bd-abc123.1",
      "#step-1": "bd-abc123.2",
      "#step-2": "bd-abc123.3"
    }
  },
  "beads_committed": true,
  "error": null
}
```

| Field | Type | Description |
|-------|------|-------------|
| `status` | string | `"ready"`, `"error"`, or `"needs_clarification"` |
| `worktree_path` | string | Absolute path to created worktree |
| `branch_name` | string | Git branch name (with `/`) |
| `base_branch` | string | Branch to merge back to |
| `speck_path` | string | Relative path to speck file |
| `resolved_steps` | array | Step anchors to execute |
| `beads` | object | Bead sync results |
| `beads_committed` | bool | Whether bead annotations were committed |
| `error` | string? | Error message if status is `"error"` |

**Error Cases:**

If speck has 0 execution steps:
```json
{
  "status": "error",
  "error": "Speck has no execution steps",
  "worktree_path": null,
  "branch_name": null
}
```

If worktree already exists:
```json
{
  "status": "error",
  "error": "Worktree already exists for this speck. Run 'specks worktree cleanup' or use a different speck.",
  "worktree_path": null,
  "branch_name": null
}
```

---

### 1.0.1 Specification {#specification}

#### 1.0.1.1 CLI Commands {#cli-commands}

**Table T01: New CLI Commands** {#t01-cli-commands}

| Command | Description | Arguments |
|---------|-------------|-----------|
| `specks worktree create <speck>` | Create worktree for implementation | `--base <branch>` (default: main) |
| `specks worktree list` | List active worktrees with status | `--json` for JSON output |
| `specks worktree cleanup` | Remove worktrees for merged PRs | `--merged` (required), `--dry-run` |

**Table T02: Worktree Command Exit Codes** {#t02-exit-codes}

| Exit Code | Meaning |
|-----------|---------|
| 0 | Success |
| 1 | General error (unexpected failure) |
| 2 | Invalid arguments or usage |
| 3 | Worktree already exists for this speck |
| 4 | Git version insufficient (<2.15) |
| 5 | Not in a git repository |
| 6 | Base branch does not exist |
| 7 | Speck file not found or invalid |
| 8 | Speck has no execution steps |

**Usage Examples:**

```bash
# Create worktree for a speck
specks worktree create .specks/specks-auth.md

# Create worktree with different base branch
specks worktree create .specks/specks-auth.md --base develop

# List all active worktrees
specks worktree list
specks worktree list --json

# Clean up merged worktrees (dry run first)
specks worktree cleanup --merged --dry-run
specks worktree cleanup --merged
```

#### 1.0.1.2 Terminology {#terminology}

- **Worktree**: A git worktree providing an isolated working directory for a branch
- **Session**: The state of an in-progress implementation, tracked in `session.json`
- **Base branch**: The branch that the implementation will merge into (usually main)
- **Speck slug**: A short identifier derived from the speck filename (e.g., `auth` from `specks-auth.md`)

---

### 1.0.2 Symbol Inventory {#symbol-inventory}

#### 1.0.2.1 New Files {#new-files}

| File | Purpose |
|------|---------|
| `crates/specks/src/commands/worktree.rs` | Worktree CLI commands |
| `crates/specks-core/src/worktree.rs` | Worktree operations |
| `crates/specks-core/src/session.rs` | Session state management |

#### 1.0.2.2 Modified Files {#modified-files}

| File | Changes |
|------|---------|
| `skills/implementer/SKILL.md` | Add worktree creation and PR creation |
| `agents/implementer-setup-agent.md` | Create worktree via `specks worktree create` (Bash); run beads sync in worktree; commit bead annotations; return `worktree_path` |
| `agents/committer-agent.md` | Add `worktree_path`; add `operation` field (commit/publish); use `git -C`; implement Spec S04 |
| `agents/coder-agent.md` | Add worktree_path to input contract; no `cd` rule |
| `agents/architect-agent.md` | Add worktree_path to input contract; no `cd` rule |
| `agents/reviewer-agent.md` | Add worktree_path to input contract; no `cd` rule |
| `agents/auditor-agent.md` | Add worktree_path to input contract; no `cd` rule |
| `agents/logger-agent.md` | Add worktree_path; write log in worktree |
| `.gitignore` | Add `.specks-worktrees/` |

#### 1.0.2.3 Symbols to Add {#symbols}

| Symbol | Kind | Location | Notes |
|--------|------|----------|-------|
| `WorktreeConfig` | struct | `worktree.rs` | Configuration for worktree creation |
| `create_worktree` | fn | `worktree.rs` | Create a worktree for a speck |
| `list_worktrees` | fn | `worktree.rs` | List active worktrees |
| `cleanup_worktrees` | fn | `worktree.rs` | Remove merged worktrees |
| `derive_speck_slug` | fn | `worktree.rs` | Derive slug from speck path (Spec S05) |
| `sanitize_branch_name` | fn | `worktree.rs` | Filesystem-safe name from branch (D08) |
| `Session` | struct | `session.rs` | Session state struct |
| `SessionStatus` | enum | `session.rs` | pending/in_progress/completed/failed/needs_reconcile |
| `load_session` | fn | `session.rs` | Load session from worktree |
| `save_session` | fn | `session.rs` | Save session to worktree |

---

### 1.0.3 Test Plan Concepts {#test-plan-concepts}

#### Test Categories {#test-categories}

| Category | Purpose | When to use |
|----------|---------|-------------|
| **Unit** | Test worktree and session functions in isolation | Path construction, session serialization |
| **Integration** | Test full worktree lifecycle | Create, list, cleanup commands |
| **Golden** | Verify session.json format | Schema stability |

#### Test Fixtures {#test-fixtures}

**List L01: Test Scenarios** {#l01-test-scenarios}

1. Create worktree for valid speck
2. Create worktree when one already exists (error, exit code 3)
3. Create worktree for 0-step speck (error, exit code 8)
4. Create worktree partial failure cleans up (branch created, worktree fails → branch deleted)
5. List worktrees (empty, one, multiple)
6. List worktrees handles orphaned directories gracefully
7. Cleanup with no merged PRs (no-op)
8. Cleanup with merged PR (removes worktree)
9. Session state transitions (valid and invalid)
10. Path prefixing in agent inputs
11. Speck slug derivation (per Spec S05 examples)
12. Branch name sanitization (per D08, including fallback for empty result)

**Golden Tests:**
- [ ] `session.json` schema matches Spec S01 exactly
- [ ] `cleanup --merged --dry-run` output format is stable
- [ ] Exit codes match Table T02

---

### 1.0.4 Execution Steps {#execution-steps}

#### Step 0: Add Worktree Directory to Gitignore {#step-0}

**Commit:** `chore: ignore specks worktrees and step artifacts`

**References:** [D01] Worktrees replace session directories, (#context)

**Artifacts:**
- Modified `.gitignore`

**Tasks:**
- [ ] Add `.specks-worktrees/` entry to `.gitignore`
- [ ] Add `.specks/step-artifacts/` entry to `.gitignore` (optional local debug artifacts; see [Q04])

**Tests:**
- [ ] *(optional)* Add a lightweight regression test if we already have a pattern for gitignore validation

**Checkpoint:**
- [ ] `grep -q 'specks-worktrees' .gitignore` returns success
- [ ] `grep -q 'specks/step-artifacts' .gitignore` returns success

**Rollback:**
- Revert gitignore change

**Commit after all checkpoints pass.**

---

#### Step 1: Implement Session State Module {#step-1}

**Depends on:** #step-0

**Commit:** `feat(core): add session state management for worktrees`

**References:** [D06] Minimal session state, Spec S01, (#session-json-schema)

**Artifacts:**
- New file: `crates/specks-core/src/session.rs`
- Modified: `crates/specks-core/src/lib.rs` (export session module)

**Tasks:**
- [ ] Create `Session` struct with fields from Spec S01
- [ ] Create `SessionStatus` enum: pending, in_progress, completed, failed, needs_reconcile (see [D10])
- [ ] Implement `load_session(worktree_path: &Path) -> Result<Session>`
- [ ] Implement `save_session(session: &Session) -> Result<()>`
- [ ] Add serde derives for JSON serialization
- [ ] Export from lib.rs

**Tests:**
- [ ] Unit test: Session serialization roundtrip
- [ ] Unit test: SessionStatus transitions
- [ ] Unit test: load_session with missing file returns error
- [ ] Unit test: save_session creates file

**Checkpoint:**
- [ ] `cargo build -p specks-core`
- [ ] `cargo nextest run -p specks-core session`

**Rollback:**
- Remove `session.rs`, revert lib.rs changes

**Commit after all checkpoints pass.**

---

#### Step 2: Implement Worktree Core Module {#step-2}

**Depends on:** #step-1

**Commit:** `feat(core): add worktree creation and management`

**References:** [D01] Worktrees replace sessions, [D03] Path prefixing, Spec S02, Diagram Diag01, (#worktree-lifecycle)

**Artifacts:**
- New file: `crates/specks-core/src/worktree.rs`
- Modified: `crates/specks-core/src/lib.rs` (export worktree module)

**Tasks:**
- [ ] Create `WorktreeConfig` struct: speck_path, base_branch, repo_root
- [ ] Implement `derive_speck_slug(speck_path: &Path) -> String` per Spec S05
- [ ] Implement `create_worktree(config: &WorktreeConfig) -> Result<Session>`
  - Validate speck has at least 1 execution step (exit code 8 if empty)
  - Generate branch name: `specks/<slug>-<timestamp>` (UTC)
  - Check if branch already exists → return error (exit code 3)
  - Check if worktree directory already exists → return error (exit code 3)
  - Create branch from base
  - Create worktree in `.specks-worktrees/<worktree_dir_name>/` (see [D08])
  - Initialize session.json with status `pending`
  - **Partial failure recovery:**
    - If branch creation succeeds but worktree creation fails: delete the branch, return error
    - If worktree creation succeeds but session.json write fails: remove worktree, delete branch, return error
- [ ] Implement `list_worktrees(repo_root: &Path) -> Result<Vec<Session>>`
  - Run `git worktree prune` first to clean stale entries
  - Scan `.specks-worktrees/*/` for `session.json` files
  - Skip entries where directory doesn't exist (orphaned)
- [ ] Implement `cleanup_worktrees(repo_root: &Path, dry_run: bool) -> Result<Vec<String>>`
  - Check each worktree branch for merged status (see [D09])
  - Remove worktree via git-native command (preferred): `git -C {repo_root} worktree remove <worktree_path>`
  - Prune stale worktree metadata: `git -C {repo_root} worktree prune`
  - Delete local branch
- [ ] Export from lib.rs

**Tests:**
- [ ] Integration test: create_worktree creates directory and branch
- [ ] Integration test: create_worktree with 0-step speck returns error
- [ ] Integration test: create_worktree when worktree exists returns error
- [ ] Integration test: create_worktree partial failure cleans up
- [ ] Integration test: list_worktrees finds created worktrees
- [ ] Integration test: list_worktrees handles orphaned directories
- [ ] Integration test: cleanup_worktrees removes merged worktrees
- [ ] Unit test: branch name generation format
- [ ] Unit test: speck slug derivation (Spec S05 examples)

**Checkpoint:**
- [ ] `cargo build -p specks-core`
- [ ] `cargo nextest run -p specks-core worktree`

**Rollback:**
- Remove `worktree.rs`, revert lib.rs changes
- Delete any test worktrees: `rm -rf .specks-worktrees/`
- Prune git worktree metadata: `git worktree prune`

**Commit after all checkpoints pass.**

---

#### Step 3: Implement Worktree CLI Commands {#step-3}

**Depends on:** #step-2

**Commit:** `feat(cli): add specks worktree commands`

**References:** [D07] Manual cleanup, Table T01, (#cli-commands)

**Artifacts:**
- New file: `crates/specks/src/commands/worktree.rs`
- Modified: `crates/specks/src/cli.rs` (add worktree subcommand)
- Modified: `crates/specks/src/commands/mod.rs` (export worktree)

**Tasks:**
- [ ] Add `worktree` subcommand to CLI with sub-subcommands: create, list, cleanup
- [ ] Implement `specks worktree create <speck>` command
  - Parse speck path
  - Call core::create_worktree
  - Output created worktree path
- [ ] Implement `specks worktree list` command
  - Call core::list_worktrees
  - Format output (table or JSON with --json)
- [ ] Implement `specks worktree cleanup --merged` command
  - Require --merged flag (safety)
  - Support --dry-run
  - Call core::cleanup_worktrees
  - Report what was removed

**Tests:**
- [ ] Integration test: `specks worktree create` succeeds
- [ ] Integration test: `specks worktree list` shows worktrees
- [ ] Integration test: `specks worktree cleanup --merged` removes merged
- [ ] Integration test: `specks worktree cleanup` without --merged errors

**Checkpoint:**
- [ ] `cargo build -p specks`
- [ ] `specks worktree --help` shows create, list, cleanup
- [ ] `cargo nextest run -p specks worktree`

**Rollback:**
- Remove `commands/worktree.rs`, revert cli.rs and mod.rs

**Commit after all checkpoints pass.**

---

#### Step 4: Update Agent Input Contracts {#step-4}

**Depends on:** #step-2, #step-3

*Why Step 3?* The setup-agent calls `specks worktree create` via Bash, so CLI commands must exist before agents can use them.

**Commit:** `feat(agents): add worktree_path to agent input contracts`

**References:** [D03] Path prefixing, [D10] Step completion semantics, Spec S02, Spec S04, Spec S05, Spec S06, (#agent-input-changes, #committer-publish-mode, #setup-agent-output)

**Artifacts:**
- Modified: `agents/implementer-setup-agent.md`
- Modified: `agents/architect-agent.md`
- Modified: `agents/coder-agent.md`
- Modified: `agents/reviewer-agent.md`
- Modified: `agents/auditor-agent.md`
- Modified: `agents/logger-agent.md`
- Modified: `agents/committer-agent.md`

**Tasks:**
- [ ] Update implementer-setup-agent to:
  - **Create worktree via Bash**: The setup-agent (which has Bash tool access) calls `specks worktree create <speck_path>` to create the worktree and branch. The implementer skill itself does NOT have Bash access.
  - Verify speck file exists in worktree: `test -f {worktree_path}/.specks/<speck_file>`
  - Run `specks beads sync <speck_path>` **inside the worktree** via `cd {worktree_path} && specks beads sync ...` (see [Q02])
  - Verify beads were written to speck: grep for `**Bead:**` in the speck file
  - Stage and commit bead annotations as the **first commit** on the branch: `git -C {worktree_path} add .specks/<speck_file> && git -C {worktree_path} commit -m "chore: sync beads for implementation"`
  - Return output per **Spec S06**: `worktree_path`, `branch_name`, `base_branch`, `beads`, `beads_committed`, `resolved_steps`
  - Handle error cases per Spec S06 (0-step speck, worktree exists)
- [ ] Add `worktree_path` to input contract for each agent
- [ ] Document that all file paths must be prefixed with worktree_path
- [ ] Document that git commands must use `git -C {worktree_path}`
- [ ] Add explicit rule: "Never rely on persistent `cd` state between commands; `cd {worktree_path} && ...` is allowed only as a single-command prefix when a tool lacks `-C`"
- [ ] Update committer-agent to use `-C` flag for all git operations
- [ ] Add `operation` field to committer-agent input contract (see Spec S04):
  - `"commit"` mode: commit step changes and close bead (default, existing behavior + worktree_path)
  - `"publish"` mode: push branch and create PR
- [ ] Implement publish mode per Spec S04:
  - Check `gh auth status` before attempting PR creation
  - Generate `{worktree_path}/.specks/pr-body.md` from step_summaries
  - Push branch: `git -C {worktree_path} push -u origin {branch_name}`
  - Create PR via `gh pr create --body-file` (Spec S03)
- [ ] Update logger-agent to write to `{worktree_path}/.specks/specks-implementation-log.md`
- [ ] Update committer-agent failure semantics to match [D10]

**Tests:**
- [ ] Manual review: each agent has worktree_path in input contract
- [ ] Manual review: each agent documents path prefixing requirement

**Checkpoint:**
- [ ] `grep -l "worktree_path" agents/*-agent.md | wc -l` shows 7 agents
- [ ] `grep -l "Never rely on persistent" agents/*-agent.md | wc -l` shows 7 agents

**Rollback:**
- Revert agent markdown files

**Commit after all checkpoints pass.**

---

#### Step 5: Update Implementer Skill {#step-5}

**Depends on:** #step-3, #step-4

**Commit:** `feat(skills): integrate worktrees into implementer workflow`

**References:** [D01] Worktrees replace sessions, [D04] Log in worktree, [D05] Always auto PR, Spec S03, Diagram Diag01, (#worktree-lifecycle, #pr-creation-details)

**Artifacts:**
- Modified: `skills/implementer/SKILL.md`

**Tasks:**
- [ ] Add worktree creation at start of implementation:
  - Spawn setup agent to create/load worktree + session and return `worktree_path`
- [ ] Update all agent Task invocations to include worktree_path in input
- [ ] Add PR creation at end of implementation:
  - Spawn committer-agent to push + create PR (using Spec S03)
  - Ensure PR body file is generated in worktree (`.specks/pr-body.md`)
  - Include step commits in PR body
  - Include beads information
- [ ] Update session.json status throughout:
  - pending -> in_progress at start
  - in_progress with current_step after each step
  - completed after PR creation
  - failed on error
- [ ] Remove references to `.specks/runs/` session directories
- [ ] Remove Done/Abort prompts (PR is the approval gate)

**Tests:**
- [ ] Manual test: run implementer and verify worktree created
- [ ] Manual test: verify PR is created after all steps

**Checkpoint:**
- [ ] `grep -c "worktree" skills/implementer/SKILL.md` shows integration
- [ ] `grep -c ".specks/runs" skills/implementer/SKILL.md` returns 0

**Rollback:**
- Revert implementer SKILL.md to previous version

**Commit after all checkpoints pass.**

---

#### Step 6: Integration Testing and Documentation {#step-6}

**Depends on:** #step-5

**Commit:** `docs: add worktree workflow documentation`

**References:** [D07] Manual cleanup, (#success-criteria)

**Artifacts:**
- Modified: `CLAUDE.md` (update workflow documentation)
- New (if needed): Integration test for full workflow

**Tasks:**
- [ ] Update CLAUDE.md with new worktree workflow
- [ ] Document `specks worktree` commands
- [ ] Document cleanup procedure after PR merge
- [ ] Add troubleshooting section for common issues
- [ ] Write integration test that:
  - Creates a test speck
  - Runs implementer (mocked steps)
  - Verifies worktree created
  - Verifies commits made
  - Verifies cleanup works

**Tests:**
- [ ] Integration test: full workflow creates worktree and commits
- [ ] Manual test: documentation is accurate

**Checkpoint:**
- [ ] `cargo nextest run` all tests pass
- [ ] Documentation review complete

**Rollback:**
- Revert CLAUDE.md changes, remove integration test

**Commit after all checkpoints pass.**

---

### 1.0.5 Deliverables and Checkpoints {#deliverables}

**Deliverable:** Git worktree integration enabling isolated implementation environments with automatic PR creation, replacing the session directory model.

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

- [ ] `specks worktree create <speck>` creates worktree and branch (verify: directory exists, branch exists)
- [ ] `specks worktree list` shows active worktrees (verify: output includes created worktree)
- [ ] `specks worktree cleanup --merged` removes merged worktrees (verify: directory removed after merge)
- [ ] Implementer skill creates worktree at start (verify: worktree exists after implementer starts)
- [ ] Implementer skill creates PR after all steps (verify: PR exists on GitHub)
- [ ] Each step produces one commit (verify: `git log` in worktree shows expected commits)
- [ ] Implementation log is committed with each step (verify: log file in each commit)
- [ ] All agents use path prefixing and do not rely on persistent `cd` state (verify: no standalone `cd` usage; `cd {worktree_path} && ...` is allowed only as a single-command prefix when necessary)
- [ ] All tests pass: `cargo nextest run`

**Acceptance tests:**
- [ ] Integration test: worktree create/list/cleanup lifecycle
- [ ] Integration test: session state persistence
- [ ] Manual test: full implementer run creates PR

#### Roadmap / Follow-ons (Explicitly Not Required for Phase Close) {#roadmap}

- [ ] Phase 2: Concurrent session detection and conflict handling
- [ ] Phase 2: Session resume after failure
- [ ] Phase 2: `specks worktree status` command
- [ ] Phase 3: Multi-speck dashboard
- [ ] Phase 3: Automatic worktree cleanup on merge detection
- [ ] Phase 3: Shallow clone option for large repos

| Checkpoint | Verification |
|------------|--------------|
| Worktree creates | `ls .specks-worktrees/` shows directory |
| Branch created | `git branch` shows specks/* branch |
| Session persists | `cat .specks-worktrees/*/session.json` shows state |
| PR created | `gh pr list` shows new PR |
| Cleanup works | After merge, `specks worktree cleanup --merged` removes worktree |

**Commit after all checkpoints pass.**
