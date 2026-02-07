---
name: logger
description: Update implementation log with completed work
allowed-tools: Read, Grep, Glob, Edit
---

## Purpose

Update the implementation log with completed work. This skill adds a structured entry to `.specks/specks-implementation-log.md` after a step is completed and approved.

## Input

The director invokes this skill with a JSON payload:

```json
{
  "speck_path": "string",
  "step_anchor": "string",
  "summary": "string",
  "files_changed": ["string"],
  "commit_hash": "string | null"
}
```

**Fields:**
- `speck_path`: Path to the speck (e.g., `.specks/specks-3.md`)
- `step_anchor`: Which step was completed (e.g., `#step-2`)
- `summary`: Brief description of what was done
- `files_changed`: List of files created/modified
- `commit_hash`: Git commit hash if committed (from committer output, null if not yet committed)

## Output

Return JSON-only output (no prose, no markdown, no code fences):

```json
{
  "success": true,
  "log_file": "string",
  "entry_added": {
    "step": "string",
    "timestamp": "string",
    "summary": "string"
  }
}
```

**Fields:**
- `success`: Whether the log was updated successfully
- `log_file`: Path to the updated log file
- `entry_added.step`: The step that was logged
- `entry_added.timestamp`: ISO timestamp of when the entry was added
- `entry_added.summary`: The summary that was logged

## Behavior

1. **Read the speck**: Get the step title from the step_anchor
2. **Read the log file**: Open `.specks/specks-implementation-log.md`
3. **Generate entry**: Create a structured entry with the provided information
4. **Prepend entry**: Use Edit tool to insert the new entry after the header
5. **Return result**: Confirm success with entry details

## Entry Format

The logger creates entries in this machine-parseable format:

```markdown
## [speck-file.md] Step N: Title | COMPLETE | YYYY-MM-DD

**Completed:** YYYY-MM-DD

**Summary:** Brief description of what was done

**Files Changed:**
- file1.rs
- file2.md

**Commit:** abc1234 (if available)

---
```

**Header format:** `## [PLAN_FILE] STEP: TITLE | STATUS | DATE`

This enables grep/sed operations:
- `grep "^\## \[specks-3.md\]"` - all entries for a specific speck
- `grep "| COMPLETE |"` - all completed entries
- `grep "| 2026-02-06$"` - all entries from a specific date

## Prepend Strategy

The log file structure is:
```
Line 1: # Specks Implementation Log
...
Line 7: Entries are sorted newest-first.
Line 8: (blank)
Line 9: ## [first existing entry...]
```

Use the Edit tool to insert the new entry after "Entries are sorted newest-first." and before the first existing entry.

## Timing in Workflow

The logger runs:
1. AFTER reviewer and auditor approve the implementation
2. BEFORE the committer prepares/executes the commit

This ensures only approved work gets logged, and the log is updated before the commit.

## Example Output

```json
{
  "success": true,
  "log_file": ".specks/specks-implementation-log.md",
  "entry_added": {
    "step": "Step 2: Create analysis skills",
    "timestamp": "2026-02-06T14:30:22Z",
    "summary": "Created clarifier, critic, reviewer, and auditor skills"
  }
}
```

## Error Handling

If the log file cannot be read or updated:

```json
{
  "success": false,
  "error": "description of the failure",
  "recommendation": "PAUSE"
}
```
