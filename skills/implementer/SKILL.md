---
name: implementer
description: Orchestrates the implementation workflow - spawns sub-agents via Task
disable-model-invocation: true
allowed-tools: Task, AskUserQuestion, Read, Grep, Glob, Write, Bash
---

## Purpose

Orchestrates the complete implementation workflow: analyzing speck steps, generating strategies, executing code, detecting drift, reviewing quality, logging progress, and committing changes.

## Usage

```
/specks:implementer .specks/specks-3.md
/specks:implementer .specks/specks-3.md --start-step #step-2
/specks:implementer .specks/specks-3.md --end-step #step-4
/specks:implementer .specks/specks-3.md --commit-policy manual
/specks:implementer --resume 20260206-150145-impl-d4e5f6
```

## Input Handling

Parse the input to determine the operation mode:

| Input Pattern | Mode | Behavior |
|---------------|------|----------|
| `.specks/specks-N.md` | Full execution | Execute all steps |
| `--start-step #step-N` | Partial | Start from specified step |
| `--end-step #step-N` | Partial | Stop after specified step |
| `--commit-policy auto\|manual` | Commit mode | Auto-commit or stage only |
| `--resume <session-id>` | Resume | Resume incomplete session |

Default commit policy is `manual` (stage files, user commits).

## Prerequisites Check

**Before any other work, verify specks is initialized:**

Check that `.specks/specks-skeleton.md` exists:

```bash
test -f .specks/specks-skeleton.md && echo "initialized" || echo "not initialized"
```

If the skeleton file does not exist, **auto-initialize** by running:

```bash
specks init
```

This creates the required `.specks/` directory with skeleton, config, and implementation log files.

If `specks init` fails (e.g., specks CLI not installed), halt with:
```
Failed to initialize specks. Ensure the specks CLI is installed and in PATH.
```

**Then verify beads availability:**

```bash
specks beads status
```

If beads is unavailable, halt immediately:
```
Beads not installed or not initialized. Run `bd init` first.
```

Do NOT proceed without both prerequisites. They are hard requirements.

## Session Management

### Session ID Format

Generate session IDs as: `YYYYMMDD-HHMMSS-impl-<short-uuid>`

Example: `20260207-150145-impl-d4e5f6`

Use Bash to generate:
```bash
date +%Y%m%d-%H%M%S && head -c 3 /dev/urandom | xxd -p
```

### Session Directory Structure

Create: `.specks/runs/<session-id>/execution/`

Per-step subdirectories: `.specks/runs/<session-id>/execution/step-N/`

Contents of session root:
- `metadata.json` - Session status and state
- `error.json` - Error details if failed

Contents per step directory:
- `architect-output.json` - Architect agent strategy
- `coder-output.json` - Coder agent results + drift assessment
- `reviewer-output.json` - Reviewer agent verification
- `auditor-output.json` - Auditor agent quality check
- `logger-output.json` - Logger agent result
- `committer-output.json` - Committer agent result

### Conflict Detection (D06)

Before starting, scan for active sessions on the same speck:

1. Use Glob to find: `.specks/runs/*/execution/metadata.json`
2. Read each metadata.json
3. Check for entries where:
   - `status == "in_progress"` AND
   - `speck_path` matches current speck
4. If found with `last_updated_at` < 1 hour old:
   - Use AskUserQuestion: "Another session is active on this speck. Continue anyway?"
5. If found with `last_updated_at` > 1 hour old:
   - Report as stale and continue
6. If no active sessions: proceed normally

### Metadata Lifecycle

**On session start:**
```json
{
  "session_id": "<session-id>",
  "speck_path": "<path>",
  "commit_policy": "auto|manual",
  "status": "in_progress",
  "created_at": "<ISO timestamp>",
  "last_updated_at": "<ISO timestamp>",
  "current_step": "#step-0",
  "steps_completed": [],
  "steps_remaining": ["#step-0", "#step-1", "#step-2"]
}
```

**On step completion:**
```json
{
  ...
  "last_updated_at": "<ISO timestamp>",
  "current_step": "#step-1",
  "steps_completed": ["#step-0"],
  "steps_remaining": ["#step-1", "#step-2"]
}
```

**On all steps complete:**
```json
{
  ...
  "status": "completed",
  "last_updated_at": "<ISO timestamp>",
  "steps_completed": ["#step-0", "#step-1", "#step-2"],
  "steps_remaining": []
}
```

**On failure:**
```json
{
  ...
  "status": "failed",
  "last_updated_at": "<ISO timestamp>",
  "failed_at_step": "#step-N",
  "error": "<error message>"
}
```

## Orchestration Flow

### Phase 1: Setup

1. Parse input to determine speck path and options
2. Check beads availability: `specks beads status` (fail fast)
3. Scan for active session conflicts (D06)
4. Generate session ID
5. Read speck file and identify all steps to execute
6. Create session directory: `.specks/runs/<session-id>/execution/`
7. Write initial `metadata.json` with `status: "in_progress"`

### Phase 2: Step Execution Loop

For each step (respecting `--start-step` and `--end-step`):

#### 2a. Step Preparation

1. Create step directory: `.specks/runs/<session-id>/execution/step-N/`
2. Sync bead for this step: `specks beads sync <speck_path> --step #step-N`
3. Get bead ID from sync output
4. Update `metadata.json` with `current_step`

#### 2b. Architecture Phase

Spawn architect-agent:

```
Task(
  subagent_type: "specks:architect-agent",
  prompt: '{
    "speck_path": "<path>",
    "step_anchor": "#step-N",
    "revision_feedback": null
  }',
  description: "Create implementation strategy for step N"
)
```

Parse the JSON response. Save to `step-N/architect-output.json`.

Check for errors in `risks` array. If critical risks, use AskUserQuestion to confirm.

#### 2c. Implementation Phase

Spawn coder-agent:

```
Task(
  subagent_type: "specks:coder-agent",
  prompt: '{
    "speck_path": "<path>",
    "step_anchor": "#step-N",
    "architect_strategy": { ... },
    "session_id": "<session-id>"
  }',
  description: "Execute implementation for step N"
)
```

Parse the JSON response. Save to `step-N/coder-output.json`.

#### 2d. Drift Check

Evaluate `drift_assessment` from coder output:

| Severity | Action |
|----------|--------|
| `none` or `minor` | Continue to review |
| `moderate` | AskUserQuestion: "Moderate drift detected. Continue, revise, or abort?" |
| `major` | AskUserQuestion: "Major drift detected. Revise architect strategy or abort?" |

If user chooses to revise: loop back to 2b with `revision_feedback`.
If user chooses to abort: update metadata to failed, halt.
If user chooses to continue: proceed to review.

If `halted_for_drift` is true and user didn't pre-approve drift, present options.

#### 2e. Review Phase

Spawn reviewer-agent:

```
Task(
  subagent_type: "specks:reviewer-agent",
  prompt: '{
    "speck_path": "<path>",
    "step_anchor": "#step-N",
    "coder_output": { ... }
  }',
  description: "Verify step completion"
)
```

Parse the JSON response. Save to `step-N/reviewer-output.json`.

Handle recommendation:

| Recommendation | Action |
|----------------|--------|
| `APPROVE` | Proceed to auditor |
| `REVISE` | Re-spawn coder with reviewer issues as feedback |
| `ESCALATE` | AskUserQuestion to get user decision |

Maximum 3 coder retries for REVISE before escalating.

#### 2f. Audit Phase

Spawn auditor-agent:

```
Task(
  subagent_type: "specks:auditor-agent",
  prompt: '{
    "speck_path": "<path>",
    "step_anchor": "#step-N",
    "files_to_audit": [ ... ],
    "drift_assessment": { ... }
  }',
  description: "Check code quality"
)
```

Parse the JSON response. Save to `step-N/auditor-output.json`.

Handle recommendation:

| Recommendation | Action |
|----------------|--------|
| `APPROVE` | Proceed to logger/committer |
| `FIX_REQUIRED` | Re-spawn coder with auditor issues |
| `MAJOR_REVISION` | AskUserQuestion: "Major issues found. Revise architect or abort?" |

Maximum 2 coder retries for FIX_REQUIRED before escalating.

#### 2g. Logging Phase

Spawn logger-agent:

```
Task(
  subagent_type: "specks:logger-agent",
  prompt: '{
    "speck_path": "<path>",
    "step_anchor": "#step-N",
    "summary": "Brief description of what was implemented",
    "files_changed": [ ... ],
    "commit_hash": null
  }',
  description: "Update implementation log"
)
```

Parse the JSON response. Save to `step-N/logger-output.json`.

Note: Logger updates `.specks/specks-implementation-log.md`. This file must be included in the commit.

#### 2h. Commit Phase

Spawn committer-agent:

```
Task(
  subagent_type: "specks:committer-agent",
  prompt: '{
    "speck_path": "<path>",
    "step_anchor": "#step-N",
    "proposed_message": "feat(<scope>): <description>",
    "files_to_stage": [
      ...files_created,
      ...files_modified,
      ".specks/specks-implementation-log.md"
    ],
    "commit_policy": "auto|manual",
    "confirmed": false,
    "bead_id": "<bead-id>",
    "close_reason": "Step N complete: <summary>"
  }',
  description: "Commit changes and close bead"
)
```

**CRITICAL**: Include `.specks/specks-implementation-log.md` in `files_to_stage`. This ensures the log entry from the logger is committed atomically with the code changes.

Parse the JSON response. Save to `step-N/committer-output.json`.

If `commit_policy` is `manual`:
- Files are staged but not committed
- User reviews and commits manually
- Use AskUserQuestion: "Files staged. Review and commit when ready, then confirm."
- Wait for confirmation before proceeding to next step

If `commit_policy` is `auto`:
- Commit is created automatically
- Bead is closed
- Proceed to next step

#### 2i. Step Completion

1. Update `metadata.json`: move step from `steps_remaining` to `steps_completed`
2. Update `current_step` to next step (or null if done)
3. If more steps remain: loop back to 2a
4. If all steps complete: proceed to Phase 3

### Phase 3: Session Completion

1. Update `metadata.json` with `status: "completed"`
2. Report summary:
   - Session ID
   - Speck path
   - Steps completed
   - Any warnings or notes

## Beads Integration

### Sync Before Step

```bash
specks beads sync <speck_path> --step #step-N
```

Returns bead ID (e.g., `bd-a1b2c3`). Store for use in committer.

### Close After Commit

The committer-agent handles bead closing via:

```bash
specks beads close <bead_id> --reason "<reason>"
```

### Status Check

At any time, check progress:

```bash
specks beads status <speck_path>
```

## Drift Threshold Evaluation

From coder-agent output, evaluate `drift_assessment`:

```json
{
  "drift_severity": "none | minor | moderate | major",
  "drift_budget": {
    "yellow_used": N,
    "yellow_max": 4,
    "red_used": N,
    "red_max": 2
  }
}
```

**Threshold rules:**
- `none` or `minor` (0-2 yellow, 0 red): auto-approve, continue
- `moderate` (3-4 yellow OR 1 red): prompt user
- `major` (5+ yellow OR 2+ red): prompt user with stronger warning

**User prompt for drift:**
```
AskUserQuestion(
  questions: [
    {
      question: "Drift detected: <N> yellow files, <N> red files. How should we proceed?",
      header: "Drift",
      options: [
        { label: "Continue anyway", description: "Accept drift and proceed" },
        { label: "Revise strategy", description: "Re-run architect with expanded scope" },
        { label: "Abort", description: "Stop implementation" }
      ],
      multiSelect: false
    }
  ]
)
```

## Retry Logic

### Reviewer REVISE Loop

```
attempts = 0
max_attempts = 3

while reviewer.recommendation == "REVISE" and attempts < max_attempts:
    coder = spawn_coder(feedback=reviewer.issues)
    reviewer = spawn_reviewer(coder.output)
    attempts++

if reviewer.recommendation == "REVISE":
    escalate to user
```

### Auditor FIX_REQUIRED Loop

```
attempts = 0
max_attempts = 2

while auditor.recommendation == "FIX_REQUIRED" and attempts < max_attempts:
    coder = spawn_coder(feedback=auditor.issues)
    auditor = spawn_auditor(coder.output)
    attempts++

if auditor.recommendation == "FIX_REQUIRED":
    escalate to user
```

## Error Handling (D08)

If Task tool fails or returns unparseable JSON:

1. Write raw output to `<session>/error.json`:
   ```json
   {
     "agent": "<agent-name>",
     "step": "#step-N",
     "raw_output": "<raw response>",
     "error": "<parse error or failure reason>",
     "timestamp": "<ISO timestamp>"
   }
   ```

2. Update `metadata.json` with `status: "failed"` and `failed_at_step`

3. Halt with descriptive message:
   ```
   Agent [coder-agent] failed at step #step-2: [reason]
   See .specks/runs/<session-id>/error.json for details.
   ```

Do NOT retry automatically - user must intervene.

## Resume Support

When `--resume <session-id>` is provided:

1. Read `metadata.json` from `.specks/runs/<session-id>/execution/`
2. Check `status`:
   - If "completed": report already done
   - If "failed": ask user if they want to retry from failed step
   - If "in_progress": resume from `current_step`
3. Load existing outputs for completed steps
4. Continue from where the session left off

## Output

On success, report:
- Session ID
- Speck path
- Steps completed
- Commit hashes (if auto policy)
- Any warnings from auditor

On failure, report:
- Session ID
- Step where failure occurred
- Error details
- Path to error.json for debugging
- Partial progress (steps completed before failure)
