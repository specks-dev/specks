# Tutorial: Execute a Plan

This tutorial walks you through executing a speck using the agent orchestration workflow.

## What You'll Learn

- How to start the execution workflow
- How the architect-implementer-monitor flow works
- How to use checkpoints and commit policies
- How to handle execution issues and drift

## Prerequisites

- specks installed and initialized (`specks init`)
- Claude Code installed and configured
- An active speck ready for execution (see [Create Your First Speck](first-speck.md))

## Scenario

Let's say you have a speck called `specks-health-check.md` with three execution steps. We'll execute it step by step and watch the agents collaborate.

## Step 1: Verify Your Speck is Ready

Before executing, ensure your speck is valid and active:

```bash
specks validate specks-health-check.md
specks status specks-health-check.md
```

The speck should:
- Pass validation with no errors
- Have status "active" in the Plan Metadata

## Step 2: Start Execution

Run the execute command:

```bash
specks execute .specks/specks-health-check.md
```

Alternatively, inside a Claude Code session:

```
/specks-execute .specks/specks-health-check.md
```

## Step 3: Watch the Agent Workflow

For each step, the agents work in sequence:

### Architect Phase

The architect reads the step and creates an implementation strategy:

> **Architect**: Creating strategy for Step 0: Create Health Service Module
>
> **Expected touch set:**
> - `src/services/health.js` (create)
> - `src/services/index.js` (modify)
>
> **Approach:** Create a dedicated health service module that exports functions for checking database connectivity and calculating uptime...

### Implementer Phase

The implementer executes the strategy while the monitor watches:

> **Implementer**: Implementing Step 0...
> - Created `src/services/health.js`
> - Updated `src/services/index.js` to export health module
> - Running tests...
>
> **Monitor**: Progress nominal. No drift detected.

### Review Phase

The reviewer and auditor verify the work:

> **Reviewer**: Checking Step 0 against plan...
> - All tasks completed
> - Tests pass
> - Checkpoints verified
>
> **Auditor**: Code quality check...
> - No security issues
> - Follows project conventions
> - Documentation adequate

### Commit Phase

Based on your commit policy, the committer prepares or executes a commit:

> **Committer**: Commit prepared for Step 0
> - Message: "feat(health): add health service module"
> - Files staged: 2
>
> [Waiting for checkpoint confirmation]

## Step 4: Handle Checkpoints

By default, execution pauses at each step for your confirmation. You'll see:

> **Checkpoint**: Step 0 complete
>
> Continue to Step 1? [y/n/abort]

Options:
- **y** or **continue**: Proceed to next step
- **n** or **pause**: Stop here, keep progress
- **abort**: Stop and rollback uncommitted changes

## Step 5: Execution Completes

When all steps finish:

> **Director**: Execution complete
>
> **Summary:**
> - Steps completed: 3/3
> - Commits created: 3
>
> All execution steps passed review and audit.

Check the final status:

```bash
specks status specks-health-check.md
```

## Execution Options

### Partial Execution

Execute only specific steps:

```bash
# Start from a specific step
specks execute .specks/specks-feature.md --start-step "#step-2"

# Stop after a specific step
specks execute .specks/specks-feature.md --end-step "#step-3"

# Execute a range
specks execute .specks/specks-feature.md --start-step "#step-1" --end-step "#step-2"
```

### Commit Policy

Control when commits are created:

```bash
# Manual: commits are prepared but you decide when to apply
specks execute .specks/specks-feature.md --commit-policy manual

# Auto: commits are created automatically after each step
specks execute .specks/specks-feature.md --commit-policy auto
```

### Checkpoint Mode

Control when execution pauses:

```bash
# Step: pause after every step (default)
specks execute .specks/specks-feature.md --checkpoint-mode step

# Milestone: pause only at milestone boundaries
specks execute .specks/specks-feature.md --checkpoint-mode milestone

# Continuous: no pauses, run to completion
specks execute .specks/specks-feature.md --checkpoint-mode continuous
```

### Dry Run

Preview execution without making changes:

```bash
specks execute .specks/specks-feature.md --dry-run
```

This shows:
- Which steps would execute
- Expected order (based on dependencies)
- Estimated scope

## Handling Execution Issues

### Monitor Halts Execution

If the monitor detects significant drift, execution halts:

> **Monitor**: HALT - Drift detected
>
> Reason: Implementer modified `src/config.js` which is not in the expected touch set.
>
> Options:
> 1. Review and continue
> 2. Return to architect with feedback
> 3. Abort execution

To continue after reviewing:

```bash
specks execute .specks/specks-feature.md --start-step "#step-2"
```

### Step Fails Validation

If a step fails review:

> **Reviewer**: Step 2 FAILED
>
> Issues:
> - Task "Add unit tests" not completed
> - Test file `health.test.js` not found

The execution pauses. You can:
1. Manually fix the issue and continue
2. Return to the implementer with specific feedback
3. Return to the planner to revise the step

### Agent Timeout

For complex steps, increase the timeout:

```bash
specks execute .specks/specks-complex.md --timeout 900
```

### Resume After Interruption

If execution is interrupted, restart from where you left off:

```bash
# Check which steps completed
specks status specks-feature.md

# Resume from the next pending step
specks execute .specks/specks-feature.md --start-step "#step-3"
```

## Tips for Successful Execution

### Review the Architect Strategy

Before the implementer starts, the architect shows its strategy. If something looks wrong, you can provide feedback:

> The architect should use the existing database pool from `src/db.js` instead of creating a new connection.

### Start Small

For your first execution, try a speck with 1-2 simple steps. This helps you understand the flow before tackling complex plans.

### Use Dry Run First

For important specks, always preview with `--dry-run`:

```bash
specks execute .specks/specks-critical.md --dry-run
```

### Keep Specks Focused

Specks that try to do too much are harder to execute successfully. If a speck has more than 5-6 steps, consider splitting it.

### Monitor the Touch Set

The expected touch set helps catch scope creep. If the implementer modifies unexpected files, the monitor flags it. This is a feature, not a bug.

## Next Steps

- **Track progress** across multiple specks: `specks list`
- **Integrate with beads** for issue tracking: `specks beads sync`
- **Contribute** to specks itself: See [CONTRIBUTING.md](../../CONTRIBUTING.md)
- **Reference** the speck format: `.specks/specks-skeleton.md`

## Common Issues

### "Not initialized"

Run `specks init` in your project directory first.

### "Speck not found"

Ensure you're using the correct path:
- Relative from `.specks/`: `specks status specks-health-check.md`
- Absolute path: `specks execute .specks/specks-health-check.md`

### "Validation errors"

Fix validation errors before executing:

```bash
specks validate specks-health-check.md
```

### "Claude CLI not installed"

Install Claude Code:

```bash
npm install -g @anthropic-ai/claude-code
```
