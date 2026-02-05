---
name: specks-execute
description: |
  Execute a speck through the multi-agent orchestration loop.
  Implements steps via architect, implementer, monitor, reviewer, auditor, logger, and committer agents.
argument-hint: "path/to/speck.md [options]"
---

## Summary

Execute a speck through the multi-agent orchestration loop. Invoke with `/specks-execute path/to/speck.md` to implement the speck's steps sequentially with full agent coordination.

## Your Role

You orchestrate the specks execution workflow by invoking the director agent with `mode=execute`. The director coordinates the full agent suite (architect, implementer, monitor, reviewer, auditor, logger, committer) to implement each step.

## Invocation

```
/specks-execute .specks/specks-1.md
/specks-execute .specks/specks-1.md --start-step #step-2
/specks-execute .specks/specks-1.md --start-step #step-2 --end-step #step-4
/specks-execute .specks/specks-1.md --commit-policy auto
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
│ ARCHITECT creates implementation strategy                    │
│ → Produces architect-plan.md with expected_touch_set         │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ PHASE 2: IMPLEMENTATION + MONITORING                         │
├─────────────────────────────────────────────────────────────┤
│ IMPLEMENTER writes code (background)                         │
│ MONITOR watches for drift (parallel)                         │
│ → Monitor can HALT if drift detected                         │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ PHASE 3: REVIEW                                              │
├─────────────────────────────────────────────────────────────┤
│ REVIEWER checks plan adherence                               │
│ AUDITOR checks code quality                                  │
│ → Reports written to run directory                           │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ PHASE 4: DOCUMENTATION + COMMIT                              │
├─────────────────────────────────────────────────────────────┤
│ LOGGER updates implementation log                            │
│ COMMITTER prepares commit message                            │
│ → If manual: pause for user to commit                        │
│ → If auto: commit automatically                              │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ PHASE 5: BEAD CLOSURE                                        │
├─────────────────────────────────────────────────────────────┤
│ Close bead for completed step                                │
│ Sync state with beads                                        │
│ → Move to next ready step                                    │
└─────────────────────────────────────────────────────────────┘
```

## How to Execute

Invoke the director agent with `mode=execute`:

```
Task(
  subagent_type: "specks-director",
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
5. **Agents available**: All agent definitions present in `agents/`

## Run Directory

A run directory is created at `.specks/runs/{uuid}/` containing:

```
.specks/runs/{uuid}/
├── invocation.json      # Director config at start
├── architect-plan.md    # Architect's strategy for current step
├── monitor-log.jsonl    # Monitor observations (append-only)
├── reviewer-report.md   # Reviewer assessment
├── auditor-report.md    # Auditor findings
├── committer-prep.md    # Commit preparation details
├── .halt                # Halt signal file (if monitor halted)
└── status.json          # Final run status
```

## Checkpoint Modes

| Mode | Behavior |
|------|----------|
| `step` | Pause after every step for user confirmation |
| `milestone` | Pause only at milestone boundaries (M01, M02, etc.) |
| `continuous` | No pauses; only stop on error or HALT |

## Commit Policies

| Policy | Behavior |
|--------|----------|
| `manual` | Write commit message to `git-commit-message.txt`, pause for user to commit |
| `auto` | Automatically stage and commit after each step |

With `commit-policy=manual`, you'll see prompts like:
```
Step N complete. Commit and type 'done' (or 'skip' / 'abort'):
```

## Halt Handling

If the monitor detects drift during implementation:

1. A `.halt` file is written with drift details
2. Implementer stops cooperatively
3. Director routes based on halt recommendation:
   - `return_to_architect`: Revise implementation strategy
   - `return_to_planner`: Revise the plan itself
   - `continue`: Override and continue (if safe)

## Error Handling

| Error | Action |
|-------|--------|
| Speck not found | Exit with error, suggest checking path |
| Speck validation fails | Exit with error, suggest fixing speck |
| Speck not active | Exit with error, suggest running plan first |
| Implementer failure | Route to architect or retry |
| Monitor HALT | Route per drift severity |
| Reviewer failure | Route to architect or planner |
| Auditor failure | Route back to implementer |

## Example Usage

**Execute all steps:**
```
/specks-execute .specks/specks-1.md
```

**Execute from a specific step:**
```
/specks-execute .specks/specks-1.md --start-step #step-3
```

**Execute a range of steps:**
```
/specks-execute .specks/specks-1.md --start-step #step-2 --end-step #step-5
```

**Execute with automatic commits:**
```
/specks-execute .specks/specks-1.md --commit-policy auto
```

**Execute continuously without pauses:**
```
/specks-execute .specks/specks-1.md --checkpoint-mode continuous --commit-policy auto
```

## Integration with CLI

This skill provides the same functionality as `specks execute` from the command line:

| CLI Command | Equivalent Skill |
|-------------|------------------|
| `specks execute .specks/specks-1.md` | `/specks-execute .specks/specks-1.md` |
| `specks execute .specks/specks-1.md --start-step #step-2` | `/specks-execute .specks/specks-1.md --start-step #step-2` |
| `specks execute .specks/specks-1.md --commit-policy auto` | `/specks-execute .specks/specks-1.md --commit-policy auto` |

Both paths invoke the same director workflow and produce identical outcomes.

## Output

Upon completion, the director returns:

1. **status.json** in run directory with:
   - `outcome`: success, failure, halted, or partial
   - `steps_completed`: list of completed step anchors
   - `steps_remaining`: list of remaining step anchors
   - `commits_created`: number of commits made

2. **Summary** of what was accomplished

3. **Issues** that need user attention (if any)

4. **Next steps** if work remains
