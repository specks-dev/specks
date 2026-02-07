---
name: execute
description: Execute a speck through agent orchestration
disable-model-invocation: true
allowed-tools: Task
argument-hint: "path/to/speck.md [options]"
---

## Summary

Execute a speck through the multi-agent orchestration loop. Invoke with `/specks:execute path/to/speck.md` to implement the speck's steps sequentially with full agent coordination.

## Your Role

You orchestrate the specks execution workflow by invoking the director agent with `mode=execute`. The director coordinates agents (architect, implementer, interviewer) and skills (reviewer, auditor, logger, committer) to implement each step.

## Invocation

```
/specks:execute .specks/specks-1.md
/specks:execute .specks/specks-1.md --start-step #step-2
/specks:execute .specks/specks-1.md --start-step #step-2 --end-step #step-4
/specks:execute .specks/specks-1.md --commit-policy auto
```

## Options

| Option | Default | Description |
|--------|---------|-------------|
| `--start-step <anchor>` | first ready | Step anchor to start from (e.g., `#step-2`) |
| `--end-step <anchor>` | all | Step anchor to stop after |
| `--commit-policy <policy>` | `manual` | `manual` (pause for user) or `auto` (commit automatically) |
| `--checkpoint-mode <mode>` | `step` | `step`, `milestone`, or `continuous` |

## Workflow

For each step in the speck (in dependency order):

```
┌─────────────────────────────────────────────────────────────┐
│ PHASE 1: ARCHITECTURE                                        │
├─────────────────────────────────────────────────────────────┤
│ [ARCHITECT] agent creates implementation strategy            │
│ → Returns: strategy, expected_touch_set[], test_plan         │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ PHASE 2: IMPLEMENTATION (with Self-Monitoring)               │
├─────────────────────────────────────────────────────────────┤
│ [IMPLEMENTER] agent writes code with drift detection         │
│ → Self-monitors against expected_touch_set                   │
│ → Self-halts if drift thresholds exceeded                    │
│ → Returns: success/failure + drift_assessment                │
│                                                              │
│ IF halted_for_drift:                                         │
│   → [INTERVIEWER] presents drift to user for decision        │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ PHASE 3: REVIEW (Parallel)                                   │
├─────────────────────────────────────────────────────────────┤
│ (REVIEWER) skill checks plan adherence                       │
│ (AUDITOR) skill checks code quality                          │
│ → Both run in parallel, return JSON                          │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ PHASE 4: DOCUMENTATION + COMMIT                              │
├─────────────────────────────────────────────────────────────┤
│ (LOGGER) skill updates implementation log                    │
│ (COMMITTER) skill prepares/executes commit                   │
│ → If manual: pause for user to commit                        │
│ → If auto: commit automatically                              │
│ → Closes bead if linked                                      │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
            NEXT STEP (loop back to PHASE 1)
```

**Legend:**
- `[AGENT]` = spawned via Task tool (isolated context)
- `(SKILL)` = invoked via Skill tool (inline, JSON output)

## How to Execute

Spawn the director agent with `mode=execute`:

```
Task(
  subagent_type: "specks:director",
  prompt: "mode=execute speck=\"$SPECK_PATH\" start-step=$START end-step=$END commit-policy=$POLICY checkpoint-mode=$MODE",
  description: "Execute speck"
)
```

Parse the arguments to extract:
- `speck`: Required path to speck file
- `start-step`: Optional step anchor
- `end-step`: Optional step anchor
- `commit-policy`: Optional, default "manual"
- `checkpoint-mode`: Optional, default "step"

## Preconditions

Before execution begins, the director verifies:

1. **Speck exists**: File at specified path is readable
2. **Speck is valid**: `specks validate` passes without errors
3. **Speck is active**: Status in Plan Metadata = "active"
4. **Beads synced**: Beads Root exists (runs `specks beads sync` if not)

## Run Directory

A run directory is created at `.specks/runs/{session-id}/` containing:

```
.specks/runs/{session-id}/
├── metadata.json           # Session info, start time, mode, status
├── execution/              # Execution phase artifacts
│   ├── step-0/
│   │   ├── architect.json     # Implementation strategy
│   │   ├── implementer.json   # Code changes made (includes drift_assessment)
│   │   ├── reviewer.json      # Plan adherence check
│   │   ├── auditor.json       # Code quality check
│   │   ├── logger.json        # Log entry added
│   │   └── committer.json     # Commit details
│   ├── step-1/
│   │   └── ...
│   └── summary.json        # Overall execution status
```

## Checkpoint Modes

| Mode | Behavior |
|------|----------|
| `step` | Pause after every step for user confirmation |
| `milestone` | Pause only at milestone boundaries (M01, M02, etc.) |
| `continuous` | No pauses; only stop on error or drift halt |

## Commit Policies

| Policy | Behavior |
|--------|----------|
| `manual` | Write commit message to `git-commit-message.txt`, pause for user to commit |
| `auto` | Automatically stage and commit after each step |

With `commit-policy=manual`, you'll see prompts like:
```
Step N complete. Commit and type 'done' (or 'skip' / 'abort'):
```

## Drift Handling

If the implementer detects drift during implementation:

1. Implementer self-halts with `halted_for_drift: true`
2. Director spawns interviewer to present drift details to user
3. User decides:
   - Continue anyway
   - Back to architect for new strategy
   - Abort

## Error Handling

| Error | Action |
|-------|--------|
| Speck not found | Exit with error, suggest checking path |
| Speck validation fails | Exit with error, suggest fixing speck |
| Speck not active | Exit with error, suggest running plan first |
| Implementer failure | Route to architect or retry |
| Implementer drift halt | Route to interviewer for user decision |
| Reviewer issues | Route to architect or planner |
| Auditor issues | Route back to implementer |

## Example Usage

**Execute all steps:**
```
/specks:execute .specks/specks-1.md
```

**Execute from a specific step:**
```
/specks:execute .specks/specks-1.md --start-step #step-3
```

**Execute a range of steps:**
```
/specks:execute .specks/specks-1.md --start-step #step-2 --end-step #step-5
```

**Execute with automatic commits:**
```
/specks:execute .specks/specks-1.md --commit-policy auto
```

**Execute continuously without pauses:**
```
/specks:execute .specks/specks-1.md --checkpoint-mode continuous --commit-policy auto
```

## Output

Upon completion, the director returns:

1. **metadata.json** in run directory with:
   - `status`: "completed" or "failed"
   - `steps_completed`: list of completed step anchors
   - `steps_remaining`: list of remaining step anchors

2. **Summary** of what was accomplished

3. **Issues** that need user attention (if any)

4. **Next steps** if work remains
