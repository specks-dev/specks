---
name: planner
description: Orchestrates the planning workflow - spawns sub-agents via Task
disable-model-invocation: true
allowed-tools: Task, AskUserQuestion, Read, Grep, Glob, Write, Bash
---

## Purpose

Orchestrates the complete planning workflow: analyzing ideas, gathering requirements, authoring specks, and ensuring quality through critic review.

## Usage

```
/specks:planner "add user authentication"
/specks:planner .specks/specks-auth.md
/specks:planner --resume 20260206-143022-plan-a1b2c3
```

## Input Handling

Parse the input to determine the operation mode:

| Input Pattern | Mode | Behavior |
|---------------|------|----------|
| `"idea text"` | New speck | Create new speck from idea |
| `.specks/specks-N.md` | Revise | Revise existing speck |
| `--resume <session-id>` | Resume | Resume incomplete session |

## Session Management

### Session ID Format

Generate session IDs as: `YYYYMMDD-HHMMSS-plan-<short-uuid>`

Example: `20260207-143022-plan-a1b2c3`

Use Bash to generate:
```bash
date +%Y%m%d-%H%M%S && head -c 3 /dev/urandom | xxd -p
```

### Session Directory Structure

Create: `.specks/runs/<session-id>/planning/`

Contents:
- `metadata.json` - Session status and state
- `clarifier-output.json` - Clarifier agent response
- `author-output.json` - Author agent response
- `critic-output.json` - Critic agent response
- `user-answers.json` - User responses to questions
- `error.json` - Error details if failed

### Conflict Detection (D06)

Before starting, scan for active sessions on the same speck:

1. Use Glob to find: `.specks/runs/*/planning/metadata.json`
2. Read each metadata.json
3. Check for entries where:
   - `status == "in_progress"` AND
   - `speck_path` matches current speck (for revisions)
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
  "mode": "new|revise|resume",
  "speck_path": "<path or null>",
  "idea": "<idea text or null>",
  "status": "in_progress",
  "created_at": "<ISO timestamp>",
  "last_updated_at": "<ISO timestamp>",
  "current_phase": "clarifier"
}
```

**On successful completion:**
```json
{
  ...
  "status": "completed",
  "last_updated_at": "<ISO timestamp>",
  "result": {
    "speck_path": "<path>",
    "recommendation": "APPROVE"
  }
}
```

**On failure:**
```json
{
  ...
  "status": "failed",
  "last_updated_at": "<ISO timestamp>",
  "error": "<error message>"
}
```

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

Do NOT proceed without initialization. The skeleton file is required for author and critic agents to function correctly.

## Orchestration Flow

### Phase 1: Setup

1. Check prerequisites (skeleton file exists)
2. Parse input to determine mode (new/revise/resume)
3. Scan for active session conflicts (D06)
3. Generate session ID
4. Create session directory: `.specks/runs/<session-id>/planning/`
5. Write initial `metadata.json` with `status: "in_progress"`

### Phase 2: Clarification

Spawn clarifier-agent:

```
Task(
  subagent_type: "specks:clarifier-agent",
  prompt: '{"idea": "<idea>", "speck_path": "<path or null>", "critic_feedback": null}',
  description: "Analyze idea and generate clarifying questions"
)
```

Parse the JSON response. Save to `clarifier-output.json`.

If `questions` array is non-empty:
- Use AskUserQuestion DIRECTLY to present questions to user
- Map clarifier questions to AskUserQuestion format:
  ```
  AskUserQuestion(
    questions: [
      {
        question: "<clarifier question>",
        header: "<short label>",
        options: [
          { label: "<option 1>", description: "" },
          { label: "<option 2>", description: "" }
        ],
        multiSelect: false
      }
    ]
  )
  ```
- Save user answers to `user-answers.json`
- Update `metadata.json` with `current_phase: "author"`

If `questions` array is empty:
- Use clarifier assumptions
- Proceed directly to author phase

### Phase 3: Authoring

Spawn author-agent:

```
Task(
  subagent_type: "specks:author-agent",
  prompt: '{
    "idea": "<idea or null>",
    "speck_path": "<path or null>",
    "user_answers": { ... },
    "clarifier_assumptions": [ ... ],
    "critic_feedback": null
  }',
  description: "Create speck document"
)
```

Parse the JSON response. Save to `author-output.json`.

Check `validation_status`:
- If "errors": halt with error (author failed to produce valid speck)
- If "valid" or "warnings": proceed to critic

Update `metadata.json` with `current_phase: "critic"`

### Phase 4: Review

Spawn critic-agent:

```
Task(
  subagent_type: "specks:critic-agent",
  prompt: '{
    "speck_path": "<path from author>",
    "skeleton_path": ".specks/specks-skeleton.md"
  }',
  description: "Review speck quality"
)
```

Parse the JSON response. Save to `critic-output.json`.

Handle recommendation:

#### APPROVE
- Update `metadata.json` with `status: "completed"`
- Return success with speck path

#### REVISE
- Use AskUserQuestion to present issues to user:
  ```
  AskUserQuestion(
    questions: [
      {
        question: "The critic found issues. How should we proceed?",
        header: "Critic review",
        options: [
          { label: "Revise speck (Recommended)", description: "Author will address the issues" },
          { label: "Accept as-is", description: "Proceed despite issues" },
          { label: "Abort", description: "Cancel planning" }
        ],
        multiSelect: false
      }
    ]
  )
  ```
- If "Revise": loop back to Phase 3 with `critic_feedback`
- If "Accept": update metadata to completed, return success
- If "Abort": update metadata to failed, return abort

#### REJECT
- Use AskUserQuestion to present rejection:
  ```
  AskUserQuestion(
    questions: [
      {
        question: "The speck was rejected due to critical issues. What next?",
        header: "Rejected",
        options: [
          { label: "Start over with new clarification", description: "Re-run clarifier with feedback" },
          { label: "Abort", description: "Cancel planning" }
        ],
        multiSelect: false
      }
    ]
  )
  ```
- If "Start over": loop back to Phase 2 with `critic_feedback`
- If "Abort": update metadata to failed, return abort

### Loop Limits

- Maximum 3 author-critic cycles before forcing user decision
- After 3 cycles, ask user: "Continue revising or accept current state?"

## Error Handling (D08)

If Task tool fails or returns unparseable JSON:

1. Write raw output to `<session>/error.json`:
   ```json
   {
     "agent": "<agent-name>",
     "raw_output": "<raw response>",
     "error": "<parse error or failure reason>",
     "timestamp": "<ISO timestamp>"
   }
   ```

2. Update `metadata.json` with `status: "failed"`

3. Halt with descriptive message:
   ```
   Agent [clarifier-agent] failed: [reason]
   See .specks/runs/<session-id>/planning/error.json for details.
   ```

Do NOT retry automatically - user must intervene.

## JSON Persistence Pattern

Use Write tool for all JSON files:

```
Write(
  file_path: ".specks/runs/<session-id>/planning/metadata.json",
  content: '<JSON string>'
)
```

Ensure JSON is properly formatted with 2-space indentation for readability.

## Resume Support

When `--resume <session-id>` is provided:

1. Read `metadata.json` from `.specks/runs/<session-id>/planning/`
2. Check `status`:
   - If "completed": report already done
   - If "failed": ask user if they want to retry from last phase
   - If "in_progress": resume from `current_phase`
3. Load existing outputs (clarifier, author, critic) if present
4. Continue from where the session left off

## Output

On success, report:
- Session ID
- Speck path created/revised
- Critic recommendation
- Any warnings from validation

On failure, report:
- Session ID
- Phase where failure occurred
- Error details
- Path to error.json for debugging
