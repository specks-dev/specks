---
name: merge
description: Merge a speck's PR and clean up worktree with verification
allowed-tools: Bash, AskUserQuestion, Read
---

## Purpose

Wraps the `specks merge` CLI command with a dry-run preview, user confirmation, and post-merge health checks. This is the final step in the `/specks:planner` → `/specks:implementer` → `/specks:merge` flow.

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
specks merge <speck_path> --dry-run
```

Parse the output. If the command fails (exit code non-zero), report the error and halt. Common errors:
- "No worktree found" — implementation hasn't been run or worktree was already cleaned up
- "No PR found" — PR hasn't been created yet
- "PR already merged" — nothing to do
- "Main branch has unpushed commits" — user needs to push first
- "Uncommitted non-infrastructure files" — user needs to commit or stash first

### 2. Ask for Confirmation

Present the dry-run results and ask the user to confirm:

```
AskUserQuestion(
  questions: [{
    question: "Ready to merge? This will commit infrastructure files, squash-merge the PR, and clean up the worktree.",
    header: "Merge",
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
specks merge <speck_path>
```

If the command fails, report the error. Include any partial progress information (e.g., if infrastructure was committed but PR merge failed).

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

Summarize:
- PR URL and number
- Whether infrastructure files were committed (and which ones)
- Whether the worktree was cleaned up
- Any health check warnings
- Confirm that main is clean and ready for the next project

---

## Error Handling

If any step fails, report the error clearly with the step that failed and what the user can do to fix it. Do not retry automatically.

Common recovery paths:
- **Unpushed commits**: `git push origin main`
- **Uncommitted non-infrastructure files**: commit or stash them, then retry
- **PR checks failing**: wait for CI or fix the issues
- **Worktree cleanup failed**: `specks worktree cleanup --merged` or manual removal

---

## Output

**On success:**
- PR merged (URL + number)
- Infrastructure files committed (if any)
- Worktree cleaned up
- Health check status
- "Main is clean and ready."

**On failure:**
- Step where failure occurred
- Error details
- Suggested recovery action
