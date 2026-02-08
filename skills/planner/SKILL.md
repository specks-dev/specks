---
name: planner
description: Orchestrates the planning workflow - spawns sub-agents via Task
disable-model-invocation: true
allowed-tools: Task, AskUserQuestion, Read, Grep, Glob, Write, Bash
---

## CRITICAL: You Are an Orchestrator

**DO NOT answer the user's request directly.**
**DO NOT analyze the idea yourself.**
**DO NOT skip any agent invocation.**

You MUST spawn subagents via the Task tool. This skill exists solely to orchestrate a sequence of agent calls that produce a speck.

**GOAL:** Produce a speck file at `.specks/specks-N.md`

---

## Orchestration Loop

```
  planner-setup-agent (one-shot)
       │
       ▼
  ┌─────────────────────────────────────┐
  │                                     │
  │  clarifier-agent                    │◄────┐
  │  (idea + critic_feedback if any)    │     │
  │                                     │     │
  └─────────────────────────────────────┘     │
       │                                      │
       ▼                                      │
  AskUserQuestion (if questions exist)        │
       │                                      │
       ▼                                      │
  author-agent                                │
       │                                      │
       ▼                                      │
  critic-agent                                │
       │                                      │
       ├── APPROVE ──► DONE (return speck)    │
       │                                      │
       └── REVISE/REJECT ─────────────────────┘
```

**The loop goes back to clarifier, NOT author.** The clarifier re-analyzes with critic feedback.

---

## Execute This Sequence

### 1. Setup (one-shot)

```
Task(
  subagent_type: "specks:planner-setup-agent",
  prompt: '{"mode": "<new|revise|resume>", "idea": "<idea or null>", "speck_path": "<path or null>", "resume_session_id": "<id or null>"}',
  description: "Initialize planning session"
)
```

Parse response. If `success: false`, halt with error. Otherwise, extract `session_id` and `session_dir`.

If `conflicts.active_sessions` is non-empty:
```
AskUserQuestion(
  questions: [{
    question: "Another session is active on this speck. Continue anyway?",
    header: "Conflict",
    options: [
      { label: "Continue", description: "Start new session" },
      { label: "Abort", description: "Cancel" }
    ],
    multiSelect: false
  }]
)
```

### 2. Clarifier Loop Entry

Initialize: `critic_feedback = null`

### 3. Spawn Clarifier

```
Task(
  subagent_type: "specks:clarifier-agent",
  prompt: '{"idea": "<idea>", "speck_path": "<path or null>", "critic_feedback": <critic_feedback or null>}',
  description: "Analyze idea and generate clarifying questions"
)
```

Save response to `<session_dir>/clarifier-output.json`.

If `questions` array is non-empty, present to user:

```
AskUserQuestion(
  questions: [
    {
      question: "<clarifier question>",
      header: "<short label from question>",
      options: [
        { label: "<option 1>", description: "" },
        { label: "<option 2>", description: "" }
      ],
      multiSelect: false
    }
  ]
)
```

Save answers to `<session_dir>/user-answers.json`.

### 4. Spawn Author

```
Task(
  subagent_type: "specks:author-agent",
  prompt: '{
    "idea": "<idea or null>",
    "speck_path": "<path or null>",
    "user_answers": <answers from step 3>,
    "clarifier_assumptions": <assumptions from clarifier>,
    "critic_feedback": <critic_feedback or null>
  }',
  description: "Create speck document"
)
```

Save response to `<session_dir>/author-output.json`.

If `validation_status == "errors"`: halt with error.

### 5. Spawn Critic

```
Task(
  subagent_type: "specks:critic-agent",
  prompt: '{"speck_path": "<path from author>", "skeleton_path": ".specks/specks-skeleton.md"}',
  description: "Review speck quality"
)
```

Save response to `<session_dir>/critic-output.json`.

### 6. Handle Critic Recommendation

**APPROVE:**
- Update metadata: `status: "completed"`
- Return success with speck path

**REVISE:**
```
AskUserQuestion(
  questions: [{
    question: "The critic found issues. How should we proceed?",
    header: "Review",
    options: [
      { label: "Revise (Recommended)", description: "Re-run clarifier with feedback" },
      { label: "Accept as-is", description: "Proceed despite issues" },
      { label: "Abort", description: "Cancel planning" }
    ],
    multiSelect: false
  }]
)
```
- If "Revise": set `critic_feedback = critic response`, **GO TO STEP 3**
- If "Accept": update metadata to completed, return success
- If "Abort": update metadata to failed, return abort

**REJECT:**
```
AskUserQuestion(
  questions: [{
    question: "The speck was rejected due to critical issues. What next?",
    header: "Rejected",
    options: [
      { label: "Start over", description: "Re-run clarifier with feedback" },
      { label: "Abort", description: "Cancel planning" }
    ],
    multiSelect: false
  }]
)
```
- If "Start over": set `critic_feedback = critic response`, **GO TO STEP 3**
- If "Abort": update metadata to failed, return abort

---

## Input Handling

Parse the user's input to determine mode:

| Input Pattern | Mode | Behavior |
|---------------|------|----------|
| `"idea text"` | new | Create new speck from idea |
| `.specks/specks-N.md` | revise | Revise existing speck |
| `--resume <session-id>` | resume | Resume incomplete session |

---

## Error Handling

If Task tool fails or returns unparseable JSON:

1. Write to `<session_dir>/error.json`:
   ```json
   {
     "agent": "<agent-name>",
     "raw_output": "<raw response>",
     "error": "<parse error or failure reason>",
     "timestamp": "<ISO timestamp>"
   }
   ```

2. Update metadata: `status: "failed"`

3. Halt with:
   ```
   Agent [name] failed: [reason]
   See <session_dir>/error.json for details.
   ```

---

## Output

**On success:**
- Session ID
- Speck path created/revised
- Critic recommendation

**On failure:**
- Session ID
- Phase where failure occurred
- Path to error.json
