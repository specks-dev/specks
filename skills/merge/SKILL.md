---
name: merge
description: Merge a speck's implementation and clean up worktree with verification
allowed-tools: Bash, AskUserQuestion, Read
---

## Purpose

Wraps the `specks merge` CLI command with a dry-run preview, user confirmation, and post-merge health checks. This is the final step in the `/specks:planner` → `/specks:implementer` → `/specks:merge` flow.

The merge command auto-detects the mode (remote or local) based on whether the repository has an 'origin' remote configured. Remote mode merges a PR via GitHub, local mode performs a direct squash merge.

---

## Input Handling

Parse the user's input to extract the speck path:

| Input Pattern | Example |
|---------------|---------|
| `.specks/specks-N.md` | `.specks/specks-12.md` |
| `specks-N.md` | `specks-12.md` (prepend `.specks/`) |

If no speck path is provided, halt with: "Usage: /specks:merge .specks/specks-N.md"

---

## Execute This Sequence

### 1. Dry Run Preview

Run the merge command in dry-run mode to show what will happen:

```bash
specks merge <speck_path> --dry-run --json
```

Parse the JSON output to determine the merge mode and preview details. Check the `merge_mode` field:

**Remote mode (`"merge_mode": "remote"`):**
- Shows PR URL, PR number, and branch name
- Lists infrastructure files to be committed (if any)
- Indicates PR will be squash-merged

**Local mode (`"merge_mode": "local"`):**
- Shows branch name and worktree path
- Lists infrastructure files to be committed (if any)
- Indicates branch will be squash-merged directly

If the command fails (exit code non-zero), report the error and halt. Common errors:
- "No worktree found" — implementation hasn't been run or worktree was already cleaned up
- "No PR found" (remote mode only) — PR hasn't been created yet
- "PR already merged" (remote mode only) — nothing to do
- "Branch has no commits to merge" (local mode only) — branch is already up to date
- "Main branch has unpushed commits" (remote mode only) — user needs to push first
- "Uncommitted non-infrastructure files" — user needs to commit or stash first

### 2. Ask for Confirmation

Present the dry-run results and ask the user to confirm. Use mode-aware wording:

**Remote mode:**
```
AskUserQuestion(
  questions: [{
    question: "Ready to merge? This will commit infrastructure files, squash-merge the PR, and clean up the worktree.",
    header: "Merge PR",
    options: [
      { label: "Merge (Recommended)", description: "Proceed with the merge workflow" },
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
    question: "Ready to merge? This will commit infrastructure files, squash-merge the branch, and clean up the worktree.",
    header: "Merge Branch",
    options: [
      { label: "Merge (Recommended)", description: "Proceed with the merge workflow" },
      { label: "Cancel", description: "Abort without making changes" }
    ],
    multiSelect: false
  }]
)
```

If user selects "Cancel", halt with: "Merge cancelled."

### 3. Execute Merge

Run the actual merge:

```bash
specks merge <speck_path> --json
```

Parse the JSON output to check success and extract details.

If the command fails, report the error. Include any partial progress information (e.g., if infrastructure was committed but PR/branch merge failed).

Common failure scenarios:
- **Merge conflicts** (local mode): The squash merge encountered conflicts. User must resolve manually in the worktree, commit the resolution, then retry.
- **PR merge failed** (remote mode): GitHub rejected the merge. Check PR status and CI.
- **Push failed** (remote mode): Network or permission issues. User may need to push manually.

### 4. Post-Merge Health Check

Run health checks to verify everything is clean:

```bash
specks doctor
```

```bash
specks worktree list
```

If doctor reports any issues, present them to the user as warnings (not errors — the merge itself succeeded).

If the worktree list is not empty, note any remaining worktrees.

### 5. Report Results

Summarize based on merge mode. Parse the JSON output from the merge command.

**Remote mode:**
- PR URL and number (from `pr_url` and `pr_number` fields)
- Whether infrastructure files were committed (from `infrastructure_committed` field)
- List of infrastructure files (from `infrastructure_files` field)
- Whether the worktree was cleaned up (from `worktree_cleaned` field)
- Any health check warnings
- Confirm that main is clean and ready for the next project

**Local mode:**
- Squash commit hash (from `squash_commit` field)
- Branch name that was merged
- Whether infrastructure files were committed (from `infrastructure_committed` field)
- List of infrastructure files (from `infrastructure_files` field)
- Whether the worktree was cleaned up (from `worktree_cleaned` field)
- Any health check warnings
- Confirm that main is clean and ready for the next project

Note: In local mode, `pr_url` and `pr_number` will be null. In remote mode, `squash_commit` will be null. Check these fields and display accordingly.

---

## Error Handling

If any step fails, report the error clearly with the step that failed and what the user can do to fix it. Do not retry automatically.

Common recovery paths:

**Remote mode:**
- **Unpushed commits**: `git push origin main`
- **Uncommitted non-infrastructure files**: commit or stash them, then retry
- **PR checks failing**: wait for CI or fix the issues
- **PR merge failed**: Check PR status on GitHub, resolve issues, then retry
- **Worktree cleanup failed**: `specks worktree cleanup --merged` or manual removal

**Local mode:**
- **Merge conflicts**: Resolve conflicts in the worktree, commit the resolution, then retry the merge
- **Empty branch**: The branch has no commits ahead of main; nothing to merge
- **Uncommitted non-infrastructure files**: commit or stash them, then retry
- **Worktree cleanup failed**: `specks worktree cleanup` or manual removal

**Both modes:**
- **No worktree found**: Implementation hasn't been run or worktree was already cleaned up
- **Uncommitted files blocking**: Use `--force` flag if you're sure (not recommended)

---

## Output

**On success (remote mode):**
- PR merged (URL + number)
- Infrastructure files committed (if any)
- Worktree cleaned up
- Health check status
- "Main is clean and ready."

**On success (local mode):**
- Branch squash-merged (commit hash)
- Infrastructure files committed (if any)
- Worktree cleaned up
- Health check status
- "Main is clean and ready."

**On failure:**
- Step where failure occurred
- Error details
- Suggested recovery action
- Current merge mode (remote or local)
