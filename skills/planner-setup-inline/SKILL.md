---
name: planner-setup-inline
description: Check prerequisites and determine planning mode. Inline execution — no subagent spawn.
user-invocable: false
---

## Planner Setup Procedure

Execute this setup procedure inline. Input:

$ARGUMENTS

### Step 1: Check Prerequisites

Check that `.specks/specks-skeleton.md` exists:

```bash
test -f .specks/specks-skeleton.md && echo "initialized" || echo "not initialized"
```

If not initialized, run:

```bash
specks init
```

If `specks init` fails, HALT and report error: "Failed to initialize specks. Ensure the specks CLI is installed and in PATH."

### Step 2: Validate Mode and Input

Parse the input JSON. Extract `mode`, `idea`, and `speck_path`.

**For `mode == "new"`:**
- Require `idea` to be a non-empty string
- Set `speck_path = null`

**For `mode == "revise"`:**
- Require `speck_path` to be a non-empty string
- Set `idea = null`

If validation fails, HALT with error: "Mode validation failed: <reason>"

### Step 3: Done

Setup is complete. Hold these values in memory for the orchestrator to use:
- `mode`: "new" or "revise"
- `initialized`: whether initialization was performed
- `speck_path`: path or null
- `idea`: idea text or null

Continue with the next phase of the orchestration workflow.
