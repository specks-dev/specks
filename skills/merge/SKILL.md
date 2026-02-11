---
name: merge
description: Merge a speck's implementation and clean up worktree with verification
allowed-tools: Bash, AskUserQuestion, Read
---

## Purpose

Wraps the `specks merge` CLI command with a dry-run preview, user confirmation, and post-merge health checks. This is the final step in the `/specks:planner` → `/specks:implementer` → `/specks:merge` flow.

The merge command auto-detects the mode based on whether the repository has an 'origin' remote and an open PR:
- **Remote mode**: Has origin + open PR → squash-merge the PR via `gh pr merge`
- **Local mode**: No origin, or no open PR → `git merge --squash` directly

---

## Input Handling

Parse the user's input to extract the speck path:

| Input Pattern | Example |
|---------------|---------|
| `.specks/specks-N.md` | `.specks/specks-12.md` |
| `specks-N.md` | `specks-12.md` (prepend `.specks/`) |

If no speck path is provided, search for specks with `ls .specks/specks-*.md`. If exactly one speck exists, use it. Otherwise halt with: "Usage: /specks:merge .specks/specks-N.md"

---

## Execute This Sequence

### 1. Dry Run Preview

Run the merge command in dry-run mode:

```bash
specks merge <speck_path> --dry-run --json 2>&1
```

Parse the JSON output. Key fields:

| Field | Description |
|-------|-------------|
| `status` | `"ok"` or `"error"` |
| `merge_mode` | `"remote"` or `"local"` |
| `branch_name` | The implementation branch |
| `worktree_path` | Path to the worktree directory |
| `pr_url` | PR URL (remote mode only) |
| `pr_number` | PR number (remote mode only) |
| `dirty_files` | Uncommitted files in main (if any) |
| `error` | Error message (if status is error) |
| `message` | Human-readable summary |

If the command fails (exit code non-zero), report the error and halt. The error message tells the user what went wrong.

### 2. Handle Dirty Files

If the dry-run output includes `dirty_files`, the user has uncommitted changes in main. These need to be committed before merging to avoid conflicts.

Commit them automatically:

```bash
git add -A && git commit -m "chore: pre-merge sync"
```

If git add/commit fails (nothing to commit), that's fine — continue.

### 3. Ask for Confirmation

Present the dry-run results and ask the user to confirm:

**Remote mode:**
```
AskUserQuestion(
  questions: [{
    question: "Ready to merge? This will squash-merge the PR and clean up the worktree.",
    header: "Merge PR",
    options: [
      { label: "Merge (Recommended)", description: "Proceed with the merge" },
      { label: "Cancel", description: "Abort without making changes" }
    ],
    multiSelect: false
  }]
)
```

**Local mode:**
```
AskUserQuestion(
  questions: [{
    question: "Ready to merge? This will squash-merge the branch into main and clean up the worktree.",
    header: "Merge Branch",
    options: [
      { label: "Merge (Recommended)", description: "Proceed with the merge" },
      { label: "Cancel", description: "Abort without making changes" }
    ],
    multiSelect: false
  }]
)
```

If user selects "Cancel", halt with: "Merge cancelled."

### 4. Execute Merge

Run the actual merge:

```bash
specks merge <speck_path> --json 2>&1
```

Parse the JSON output. Key fields for the result:

| Field | Description |
|-------|-------------|
| `status` | `"ok"` or `"error"` |
| `merge_mode` | `"remote"` or `"local"` |
| `squash_commit` | Commit hash (local mode only) |
| `pr_url` | PR URL (remote mode only) |
| `worktree_cleaned` | Whether worktree was removed |
| `error` | Error message (if failed) |

If the command fails, report the error and suggest recovery.

### 5. Post-Merge Health Check

Run health checks:

```bash
specks doctor
```

```bash
specks worktree list
```

If doctor reports issues, present them as warnings (the merge itself succeeded).

### 6. Report Results

**Remote mode success:**
- PR merged (URL + number)
- Worktree cleaned up
- Health check status
- "Main is clean and ready."

**Local mode success:**
- Branch squash-merged (commit hash)
- Worktree cleaned up
- Health check status
- "Main is clean and ready."

---

## Error Handling

If any step fails, report clearly and suggest recovery. Do not retry automatically.

**Common errors:**
- **No worktree found**: Implementation hasn't run or worktree was already cleaned up
- **Merge conflicts** (local): User must resolve manually, then retry
- **PR merge failed** (remote): Check PR status on GitHub
- **Worktree cleanup failed**: Run `git worktree remove <path> --force`

---

## Output

**On success:**
- Merge result (PR URL or commit hash)
- Worktree cleanup status
- Health check warnings (if any)
- "Main is clean and ready."

**On failure:**
- Error details
- Suggested recovery action
