---
name: implementer-setup-inline
description: Initialize implementation session — create worktree, sync beads, determine state, resolve steps. Inline execution — no subagent spawn.
user-invocable: false
---

## Implementer Setup Procedure

Execute this setup procedure inline. Input:

$ARGUMENTS

**FORBIDDEN:** Do NOT spawn planning agents (clarifier, author, critic). If something is wrong, report the error and HALT.

---

### Phase 0: Create or Reuse Worktree

**Step 0a: Check for existing worktree**

```bash
specks worktree list --json
```

Parse the JSON output. Look for an entry where `speck_path` matches the input speck. If found:
- Extract `worktree_path`, `branch_name`, and `base_branch`
- Skip to Phase 1

**Step 0b: Create new worktree (only if no existing worktree found)**

```bash
specks worktree create --reuse-existing <speck_path>
```

Parse the CLI output to extract:
- `worktree_path` (absolute path)
- `branch_name`
- `base_branch`

If worktree creation fails, HALT with the error.

---

### Phase 1: Prerequisites Check

1. Check if specks is initialized (in repo root):
   ```bash
   test -f .specks/specks-skeleton.md && echo "initialized" || echo "not initialized"
   ```

2. If not initialized, auto-init:
   ```bash
   specks init
   ```
   If this fails, HALT with error.

3. Check beads availability:
   ```bash
   specks beads status
   ```
   If this fails, HALT with error.

4. **Sync beads INSIDE the worktree:**
   ```bash
   cd {worktree_path} && specks beads sync <speck_path>
   ```
   **CRITICAL:** Beads sync must run inside the worktree so bead annotations are written to the worktree branch, not the base branch.

5. Verify beads were written to the speck file in worktree:
   ```bash
   grep "**Bead:**" {worktree_path}/<speck_path>
   ```
   If no beads found, HALT with error.

6. Commit bead annotations as first commit on the branch:
   ```bash
   git -C {worktree_path} add {worktree_path}/<speck_file>
   git -C {worktree_path} commit -m "chore: sync beads for implementation"
   ```
   Record `beads_committed = true` on success.

---

### Phase 2: Validate Speck File Exists

```bash
test -f {worktree_path}/<speck_path> && echo "exists" || echo "not found"
```

If the file does NOT exist, HALT with error: "Speck file not found in worktree: <speck_path>. Run /specks:planner first to create a speck."

Do NOT attempt to create the speck. Do NOT spawn planning agents.

---

### Phase 3: Determine State

1. Read the speck file IN THE WORKTREE to extract all step anchors (look for `{#step-N}` patterns in section headers).

2. Get completion status:
   ```bash
   specks beads status <speck_path>
   ```

3. Parse beads output to determine:
   - Which steps have closed beads → `completed_steps`
   - Which steps have open or no beads → `remaining_steps`

4. Compute `next_step` as first item in `remaining_steps` (or null if empty).

---

### Phase 4: Extract Bead IDs from Speck

Read the speck file and look for bead ID annotations in step headers.
Expected pattern: `### Step N {#step-N} <!-- bd-xxxxxx -->`

Build a `bead_mapping` dictionary:
- Key: step anchor (e.g., `#step-0`)
- Value: bead ID (e.g., `bd-abc123`)

Identify the `root_bead` (typically the bead for `#step-0`).

**If any step in `all_steps` is missing a bead ID, HALT with error:** "Beads sync failed: some steps are missing bead IDs"

---

### Phase 5: Parse User Intent

Analyze the `user_input` field from the input to determine intent:

| Pattern | Intent |
|---------|--------|
| `null` or empty | `ambiguous` |
| `next` / `next step` | `next` |
| `step N` / `#step-N` | `specific` |
| `steps N-M` / `from N to M` | `range` |
| `remaining` / `finish` / `all remaining` | `remaining` |
| `all` / `start over` / `from beginning` | `all` |

If `user_answers.step_selection` is provided, use that instead of parsing raw input.

---

### Phase 6: Resolve Steps

Based on intent, compute `resolved_steps`:

| Intent | Resolution |
|--------|------------|
| `next` | `[next_step]` (or empty if none remaining) |
| `remaining` | `remaining_steps` |
| `all` | `all_steps` |
| `specific` | Parse step number(s) from input |
| `range` | Parse start/end, generate sequence |
| `ambiguous` | Cannot resolve → needs clarification |

If `user_answers.specific_steps` is provided, use those directly.

---

### Phase 7: Validate

For each step in `resolved_steps`:

1. **Check exists**: Verify step anchor is in `all_steps`
2. **Check dependencies**: Read step content for `**Depends on:**` line, verify those steps are in `completed_steps`
3. **Check not already done**: If step is in `completed_steps`, flag as `already_completed`
4. **Check has bead ID**: Verify step anchor exists in `bead_mapping`. If missing, this is a critical error.

Record any validation issues found.

---

### Phase 8: Determine Result

- If prerequisites failed → HALT with error
- If intent is `ambiguous` and no `user_answers` → report `needs_clarification` with appropriate question

**Clarification templates:**

**Step Selection** (intent is ambiguous):
```
"Speck has N total steps. X completed, Y remaining. What would you like to do?"
Options: Next step, All remaining, Specific step or range
```

**Dependency Not Met:**
```
"Step #step-N depends on #step-M which isn't complete. What should we do?"
Options: Do dependency first (Recommended), Skip dependency check
```

**Already Completed:**
```
"Step #step-N already has a closed bead. Re-execute anyway?"
Options: Skip it, Re-execute
```

- If everything is valid → setup is complete

### Phase 9: Done

Hold all setup values in memory for the orchestrator to use:
- `worktree_path`, `branch_name`, `base_branch`
- `all_steps`, `completed_steps`, `remaining_steps`, `next_step`
- `resolved_steps`
- `root_bead`, `bead_mapping`
- `beads_committed`
- Any clarification needed or validation issues

Continue with the next phase of the orchestration workflow.
