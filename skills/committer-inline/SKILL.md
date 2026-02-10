---
name: committer-inline
description: Stage files, commit changes, close beads, and publish PRs. Inline execution — no subagent spawn.
user-invocable: false
---

## Committer Procedure

Execute this commit/publish procedure inline. Input:

$ARGUMENTS

The input JSON has an `operation` field: either `"commit"` or `"publish"`.

---

## Commit Mode (`operation: "commit"`)

### Required Fields

| Field | Description |
|-------|-------------|
| `worktree_path` | Absolute path to worktree |
| `speck_path` | Path to speck file relative to repo root |
| `step_anchor` | Anchor of the completed step |
| `proposed_message` | Commit message to use |
| `files_to_stage` | Files to stage (relative paths, includes implementation log) |
| `bead_id` | Bead ID to close |
| `close_reason` | Reason for closing the bead |
| `log_entry` | Log entry data (summary, tasks_completed, tests_run, checkpoints_verified) |

### Commit Workflow

#### 1. Check Log Size

Read `.specks/specks-implementation-log.md` in the worktree. Count lines and check byte size.

**Thresholds:** 500 lines OR 100KB (102400 bytes).

If over threshold, rotate:
1. Create `.specks/archive/` directory if missing
2. Move current log to `.specks/archive/implementation-log-YYYY-MM-DD-HHMMSS.md`
3. Create fresh log with header:
   ```markdown
   # Implementation Log

   This log tracks completed implementation work.

   ```
4. Add both archived file and fresh log to `files_to_stage`
5. Record `log_rotated = true`

#### 2. Update Implementation Log

Prepend a log entry to `.specks/specks-implementation-log.md` in the worktree using the Edit tool.

**Entry format:**
```markdown
---
step: {step_anchor}
date: {ISO8601 timestamp}
bead: {bead_id}
---

## {step_anchor}: {summary}

{tasks_completed as bulleted list}

**Tests:** {tests_run}

**Checkpoints:** {checkpoints_verified as bulleted list}

```

**Prepend strategy:** Use Edit tool to find the header and insert the new entry after it, before any existing entries. New entries go at the top (reverse-chronological order).

If the log file doesn't exist, create it with the header template first, then prepend.

If log update fails, record warning but continue with commit.

#### 3. Verify and Stage Files

Before staging, confirm all files in `files_to_stage` exist in worktree.

```bash
git -C {worktree_path} add <file1> <file2> ...
```

#### 4. Create Commit

```bash
git -C {worktree_path} commit -m "{proposed_message}"
```

Get the commit hash:
```bash
git -C {worktree_path} rev-parse HEAD
```

**No AI attribution:** NEVER include Co-Authored-By lines or AI/agent attribution in commit messages.

#### 5. Close Bead

Only after commit succeeds:
```bash
specks beads close {bead_id} --reason "{close_reason}"
```

**Edge cases:**
- If `bead_id` is missing or null: HALT with error "No bead_id provided"
- If bead already closed: record warning, continue
- If bead not found: HALT with error
- If commit succeeds but bead close fails: record `needs_reconcile = true`, HALT with error including the commit hash and bead ID for manual remediation

#### 6. Report Results

Record in memory for the orchestrator:
- `commit_message`: the message used
- `files_staged`: files that were staged
- `committed`: true/false
- `commit_hash`: git hash or null
- `bead_closed`: true/false
- `bead_id`: the bead that was closed
- `log_updated`: true/false
- `log_rotated`: true/false
- `needs_reconcile`: true/false (commit succeeded but bead close failed)
- Any warnings

---

## Publish Mode (`operation: "publish"`)

### Required Fields

| Field | Description |
|-------|-------------|
| `worktree_path` | Absolute path to worktree |
| `branch_name` | Git branch name |
| `base_branch` | Branch to merge back to (usually main) |
| `speck_title` | Title for PR |
| `speck_path` | Path to speck file |
| `step_summaries` | Array of step completion summaries |

### Publish Workflow

#### 1. Check Authentication

```bash
gh auth status
```

If fails, HALT with error: "GitHub CLI not authenticated. Run 'gh auth login' first."

#### 2. Derive Repo (if not provided)

```bash
git -C {worktree_path} remote get-url origin | sed -E 's|.*[:/]([^/]+/[^/]+)(\.git)?$|\1|'
```

#### 3. Generate PR Body

Create `{worktree_path}/.specks/pr-body.md`:

```markdown
## Summary
- Step 0: <summary>
- Step 1: <summary>
...

## Test plan
All tests passed during implementation.

🤖 Generated with [Claude Code](https://claude.com/claude-code)
```

#### 4. Push Branch

```bash
git -C {worktree_path} push -u origin {branch_name}
```

If push fails, HALT with error. Do not attempt PR creation.

#### 5. Create PR

```bash
gh pr create --repo {repo} --base {base_branch} --head {branch_name} --title "feat(specks): {speck_title}" --body-file {worktree_path}/.specks/pr-body.md
```

Fallback if repo parsing fails:
```bash
cd {worktree_path} && gh pr create --base {base_branch} --head {branch_name} --title "feat(specks): {speck_title}" --body-file .specks/pr-body.md
```

#### 6. Report Results

Parse the PR URL and number from `gh pr create` output. Record in memory:
- `pushed`: true/false
- `pr_created`: true/false
- `pr_url`: the PR URL
- `pr_number`: the PR number
- Any errors

---

## Path Handling Reminder

All git operations use `git -C {worktree_path}`:
- `git -C {worktree_path} add <file>`
- `git -C {worktree_path} commit -m "message"`
- `git -C {worktree_path} push -u origin <branch>`

**Never rely on persistent `cd` state between commands.** Use `cd {path} && <cmd>` within a single command only when needed.
