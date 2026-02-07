---
name: planner
description: Orchestrates the planning loop from idea to approved speck
allowed-tools: Skill, Task, Read, Grep, Glob, Write, Bash
---

## Your Job: Run the Complete Planning Loop

You are the **planner orchestration skill**. Your job is to run the **entire planning loop** from start to finish, producing an approved speck document.

**YOU MUST NOT EXIT UNTIL THE LOOP COMPLETES.**

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         PLANNING PHASE                                       │
│                                                                              │
│  User invokes /specks:planner "idea" or /specks:planner path/to/speck.md    │
│                                                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────┐                                                                │
│  │  INPUT   │  idea text OR existing speck path                              │
│  └────┬─────┘                                                                │
│       │                                                                      │
│       ▼                                                                      │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ (PLANNER) orchestration skill receives input, runs INLINE            │   │
│  │                                                                       │   │
│  │ 1. Invoke (CLARIFIER) skill                                          │   │
│  │    → Returns: analysis{}, questions[], assumptions[]                  │   │
│  │    → PERSIST to 001-clarifier.json                                    │   │
│  │    → CONTINUE TO STEP 2                                               │   │
│  │                                                                       │   │
│  │ 2. IF questions exist:                                                │   │
│  │    → Invoke (INTERVIEWER) skill with questions                        │   │
│  │    → Interviewer uses AskUserQuestion                                 │   │
│  │    → Returns: user_answers{}                                          │   │
│  │    → PERSIST to 002-interviewer.json                                  │   │
│  │    → CONTINUE TO STEP 3                                               │   │
│  │                                                                       │   │
│  │ 3. Invoke (AUTHOR) skill with:  ← TRY SKILL FIRST                     │   │
│  │    - Original idea/speck                                              │   │
│  │    - User answers (if any)                                            │   │
│  │    - Clarifier assumptions                                            │   │
│  │    → Returns: draft speck path                                        │   │
│  │    → PERSIST to NNN-author.json                                       │   │
│  │                                                                       │   │
│  │    IF task is complex → ESCALATE to [AUTHOR-AGENT] via Task tool      │   │
│  │    → CONTINUE TO STEP 4                                               │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│       │                                                                      │
│       ▼                                                                      │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ (PLANNER) review loop                                                 │   │
│  │                                                                       │   │
│  │ 4. Invoke (CRITIC) skill with draft speck                             │   │
│  │    → Returns: skeleton_compliant, areas{}, issues[], recommendation   │   │
│  │    → PERSIST to NNN-critic.json                                       │   │
│  │                                                                       │   │
│  │ 5. IF recommendation == REJECT or REVISE:                             │   │
│  │    → Invoke (INTERVIEWER) skill with critic issues                    │   │
│  │    → Present issues, get user decision: revise? accept anyway? abort? │   │
│  │    → PERSIST to NNN-interviewer.json                                  │   │
│  │                                                                       │   │
│  │    IF user says revise:                                               │   │
│  │    → Go back to step 3 with critic feedback                           │   │
│  │                                                                       │   │
│  │ 6. IF recommendation == APPROVE (or user accepts):                    │   │
│  │    → Planning complete                                                │   │
│  │    → Update metadata.json with status: "completed"                    │   │
│  │    → Return final output JSON                                         │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  OUTPUT: Approved speck at .specks/specks-{name}.md                          │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## CRITICAL: After Every Sub-Task

After **EVERY** sub-task skill returns:

1. **PERSIST** the output to the run directory (Write tool)
2. **CONTINUE** immediately to the next step
3. **DO NOT** output intermediate results to the user
4. **DO NOT** stop and wait

The loop continues until: **CRITIC APPROVES** or **USER ACCEPTS** or **USER ABORTS**

## Setup Phase

```bash
MODE="plan"
SESSION_ID="$(date +%Y%m%d-%H%M%S)-${MODE}-$(head -c 3 /dev/urandom | xxd -p)"
mkdir -p .specks/runs/${SESSION_ID}/planning
```

Write `metadata.json`:
```json
{
  "session_id": "...",
  "mode": "plan",
  "started_at": "ISO8601",
  "idea": "...",
  "speck_path": null,
  "status": "in_progress",
  "completed_at": null
}
```

## Sub-Task Invocations

### Step 1: Clarifier
```
Skill(skill: "specks:clarifier", args: '{"idea": "...", "speck_path": null}')
```
Persist output → Continue to Step 2

### Step 2: Interviewer (if questions exist)
```
Skill(skill: "specks:interviewer", args: '{"context": "clarifier", "payload": {...}}')
```
Persist output → Continue to Step 3

### Step 3: Author (skill-first, escalate if complex)
```
Skill(skill: "specks:author", args: '{"idea": "...", "user_answers": {...}, "clarifier_assumptions": [...]}')
```
If complex or creating new speck from scratch:
```
Task(subagent_type: "specks:author-agent", prompt: "Create speck for: ...")
```
Persist output → Continue to Step 4

### Step 4: Critic
```
Skill(skill: "specks:critic", args: '{"speck_path": ".specks/specks-N.md"}')
```
Persist output → Evaluate recommendation

### Step 5: Handle Critic Response
- **APPROVE**: Go to Finalize
- **REVISE**: Go back to Step 3 with feedback
- **REJECT**: Invoke Interviewer for user decision

## Finalize

Update `metadata.json`:
```json
{
  "status": "completed",
  "completed_at": "ISO8601"
}
```

Return final output:
```json
{
  "success": true,
  "speck_path": ".specks/specks-{name}.md",
  "session_id": "20260207-143022-plan-abc123",
  "critic_recommendation": "APPROVE",
  "notes": null
}
```

## State Reconstruction

Skills are stateless. Reconstruct state from run directory:

1. **Counter**: Count files in `planning/` to get next number
2. **Previous outputs**: Read persisted JSON files for context
3. **Session ID**: From metadata.json

## Error Handling

If a sub-task fails:
1. Persist error to `NNN-<subtask>-error.json`
2. Retry once
3. If retry fails: escalate to agent variant (if available)
4. If agent fails: invoke interviewer for user decision

## Constraints

- ONE sub-task at a time (sequential)
- Maximum 1 agent context active
- Persist ALL outputs to run directory
- Loop until APPROVE, ACCEPT, or ABORT
