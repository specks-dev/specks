---
name: committer
description: Commit changes and close bead to finalize step completion
allowed-tools: Read, Grep, Glob, Bash
---

## Purpose

Finalize a completed step: stage files, commit changes, and close the associated bead. This skill handles git operations and bead closure after implementation is complete.

## Input

The director invokes this skill with a JSON payload:

```json
{
  "speck_path": "string",
  "step_anchor": "string",
  "proposed_message": "string",
  "files_to_stage": ["string"],
  "auto_commit": true,
  "bead_id": "string | null",
  "close_reason": "string | null"
}
```

**Fields:**
- `speck_path`: Path to the speck (for commit message context)
- `step_anchor`: Which step was completed (e.g., `#step-2`)
- `proposed_message`: Commit message from the step's `**Commit:**` line
- `files_to_stage`: Files to add (from implementer output)
- `auto_commit`: If true, execute commit; if false, only prepare message
- `bead_id`: Bead ID to close after commit (null if no bead linked)
- `close_reason`: Reason for closing bead (e.g., "Step completed per speck")

## Output

Return JSON-only output (no prose, no markdown, no code fences):

```json
{
  "commit_message": "string",
  "files_staged": ["string"],
  "committed": true,
  "commit_hash": "string",
  "bead_closed": true,
  "bead_id": "string | null"
}
```

**Fields:**
- `commit_message`: The commit message used (or prepared)
- `files_staged`: List of files that were staged
- `committed`: Whether a commit was actually made
- `commit_hash`: Git commit hash (null if not committed)
- `bead_closed`: Whether the bead was closed
- `bead_id`: The bead ID that was closed (null if none)

## Behavior

### When auto_commit is true

1. **Check git status**: Run `git status` to see current state
2. **Stage files**: Run `git add` for each file in files_to_stage
3. **Create commit**: Run `git commit -m "proposed_message"`
4. **Close bead**: If bead_id provided, run `specks beads close <bead_id> --json`
5. **Return result**: Include commit hash and bead closure status

### When auto_commit is false

1. **Check git status**: Run `git status` to see current state
2. **Write message**: Write proposed_message to `git-commit-message.txt`
3. **Return result**: Include files that would be staged, committed=false

## Commit Message Format

Use the proposed_message from input. The message should follow this format:
```
<type>(<scope>): <description>

- <detail>
- <detail>
- Completes specks-N.md Step M: Title
```

## Bead Closure

If bead_id is provided, close it after a successful commit:

```bash
specks beads close <bead_id> --reason "Step completed per speck" --json
```

The bead closure happens AFTER the commit succeeds. If the commit fails, do not close the bead.

## Git Safety Rules

- Stage only the files specified in files_to_stage
- Never force push
- Never amend previous commits
- If pre-commit hooks fail, report failure (don't retry with --no-verify)
- Check git status before and after operations

## Timing in Workflow

The committer runs:
1. AFTER reviewer and auditor approve
2. AFTER logger updates the implementation log
3. As the final step in completing a speck step

## Example Output (auto_commit=true)

```json
{
  "commit_message": "feat(skills): add analysis skills (clarifier, critic, reviewer, auditor)",
  "files_staged": [
    "skills/clarifier/SKILL.md",
    "skills/critic/SKILL.md",
    "skills/reviewer/SKILL.md",
    "skills/auditor/SKILL.md"
  ],
  "committed": true,
  "commit_hash": "abc1234",
  "bead_closed": true,
  "bead_id": "bd-specks-3.step-2"
}
```

## Example Output (auto_commit=false)

```json
{
  "commit_message": "feat(skills): add analysis skills (clarifier, critic, reviewer, auditor)",
  "files_staged": [],
  "committed": false,
  "commit_hash": null,
  "bead_closed": false,
  "bead_id": null
}
```

## Error Handling

If git operations fail:

```json
{
  "commit_message": "string",
  "files_staged": ["string"],
  "committed": false,
  "commit_hash": null,
  "bead_closed": false,
  "bead_id": null,
  "error": "description of the failure",
  "recommendation": "PAUSE"
}
```

Common errors:
- Pre-commit hook failure: Report and don't retry
- Merge conflict: Report and pause for user intervention
- Bead not found: Report warning but don't fail the commit
