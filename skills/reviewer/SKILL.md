---
name: reviewer
description: Verify step completion matches plan
allowed-tools: Read, Grep, Glob
---

## Purpose

Verify a completed step matches the plan specification. This skill checks that tasks were completed, tests match the plan, and artifacts were produced.

## Input

The director invokes this skill with a JSON payload:

```json
{
  "speck_path": "string",
  "step_anchor": "string",
  "implementer_output": {
    "files_created": ["string"],
    "files_modified": ["string"],
    "tests_passed": true,
    "drift_assessment": {
      "expected_files": ["string"],
      "actual_changes": ["string"],
      "unexpected_changes": [...],
      "drift_severity": "none|minor|moderate|major"
    }
  }
}
```

**Fields:**
- `speck_path`: Path to the speck
- `step_anchor`: Which step was just completed (e.g., `#step-2`)
- `implementer_output`: Results from the implementer agent (includes drift_assessment)

## Output

Return JSON-only output (no prose, no markdown, no code fences):

```json
{
  "tasks_complete": true,
  "tests_match_plan": true,
  "artifacts_produced": true,
  "issues": [{"type": "string", "description": "string"}],
  "drift_notes": "string | null",
  "recommendation": "APPROVE|REVISE|ESCALATE"
}
```

**Fields:**
- `tasks_complete`: Were all tasks in the step completed?
- `tests_match_plan`: Do the tests match what the plan specified?
- `artifacts_produced`: Were all specified artifacts created?
- `issues`: List of problems found
- `drift_notes`: If implementer had minor drift (1-2 yellow touches), mention it here for visibility
- `recommendation`: Overall verdict

## Behavior

1. **Read the speck**: Load the speck and find the step by anchor
2. **Extract step requirements**: Tasks, artifacts, checkpoints
3. **Compare against implementer output**: Check files created/modified
4. **Verify task completion**: Were all checkboxes addressed?
5. **Check artifacts**: Do declared artifacts exist?
6. **Assess drift**: Review the drift_assessment from implementer
7. **Generate verdict**: APPROVE, REVISE, or ESCALATE

## Recommendation Logic

- **APPROVE**: All tasks complete, artifacts exist, drift is none or minor
- **REVISE**: Missing tasks or artifacts that implementer can fix
- **ESCALATE**: Conceptual issues requiring user input or plan changes

## Drift Notes

If the implementer's drift_assessment shows minor drift (1-2 yellow touches), include a `drift_notes` field to flag it for visibility. This prevents silent creep across multiple steps.

Example:
```json
"drift_notes": "Implementer touched crates/specks/src/output.rs (yellow: adjacent module). Change appears consistent with step goals."
```

## Example Output

```json
{
  "tasks_complete": true,
  "tests_match_plan": true,
  "artifacts_produced": true,
  "issues": [],
  "drift_notes": null,
  "recommendation": "APPROVE"
}
```

```json
{
  "tasks_complete": false,
  "tests_match_plan": true,
  "artifacts_produced": true,
  "issues": [
    {"type": "missing_task", "description": "Task 'Update CLAUDE.md' was not completed"}
  ],
  "drift_notes": null,
  "recommendation": "REVISE"
}
```

```json
{
  "tasks_complete": true,
  "tests_match_plan": false,
  "artifacts_produced": true,
  "issues": [
    {"type": "design_mismatch", "description": "Implementation uses different API pattern than plan specified"}
  ],
  "drift_notes": "Implementer touched 3 files outside expected set",
  "recommendation": "ESCALATE"
}
```
