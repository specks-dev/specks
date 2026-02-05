# Tutorial: Building a Python Calculator from Scratch

This tutorial demonstrates the complete specks workflow by creating a Python command-line calculator from a greenfield project. It's the canonical "first project" example that validates the full onboarding experience.

## What You'll Learn

- How to set up specks in a brand new project
- How to use `specks plan` to design your application
- How the planning loop captures requirements
- How to execute a speck to generate working code

## Prerequisites

- macOS with specks installed (`brew install specks` or binary download)
- Claude Code installed and configured
- Python 3.8+ (for running the generated calculator)

## Step 1: Verify Your Installation

Before starting, verify that specks is properly installed:

```bash
specks --version
```

If installed via Homebrew, verify the agents and skills are available:

```bash
ls /opt/homebrew/share/specks/agents/   # Should show 11 agent files
ls /opt/homebrew/share/specks/skills/   # Should show 2 skill directories
```

## Step 2: Create Your Project

Create a new directory for the calculator project:

```bash
mkdir py-calc
cd py-calc
```

## Step 3: Initialize Specks

Initialize specks in your new project:

```bash
specks init
```

You should see output like:

```
Initialized specks in /path/to/py-calc
  Created: .specks/
  Created: .specks/specks-skeleton.md
  Created: .specks/config.toml

Skills:
  Created: .claude/skills/specks-plan/SKILL.md
  Created: .claude/skills/specks-execute/SKILL.md

Agents:
  Found 11 agents in /opt/homebrew/share/specks/agents/
    specks-director, specks-planner, specks-critic, specks-interviewer,
    specks-architect, specks-implementer, specks-monitor, specks-reviewer,
    specks-auditor, specks-logger, specks-committer
```

This confirms:
- The `.specks/` directory was created for your specks
- Claude Code skills were installed for `/specks-plan` and `/specks-execute`
- All 11 agents are available from the installation

## Step 4: Plan the Calculator

Now start the planning loop to design your calculator:

```bash
specks plan "create a python command-line calculator that supports +, -, *, /"
```

### What Happens During Planning

The planning loop involves several agents working together:

1. **Interviewer** gathers your requirements by asking clarifying questions
2. **Planner** creates a structured speck based on your answers
3. **Critic** reviews the plan for quality and completeness
4. **Interviewer** presents the results and asks if you're satisfied

### Example Interaction

The interviewer might ask questions like:

> **Interviewer**: I'm gathering requirements for your Python calculator. Let me clarify a few things:
>
> 1. Should this be an interactive REPL (read-eval-print loop) or accept single expressions?
> 2. How should it handle invalid input (division by zero, syntax errors)?
> 3. Should it support parentheses for grouping operations?
> 4. Any specific Python version requirements?

You might respond:

> Make it an interactive REPL that prompts for expressions and prints results.
> Division by zero should print an error message and continue.
> Support parentheses for grouping.
> Target Python 3.8+.

### Review and Iterate

After the planner generates a speck, the interviewer presents a summary:

> **Interviewer**: The planner has created a speck for your calculator:
>
> **Phase: Python Command-Line Calculator**
> - 4 execution steps planned
> - Step 0: Create expression parser
> - Step 1: Implement calculator operations
> - Step 2: Build REPL interface
> - Step 3: Add error handling and tests
>
> **Critic's assessment**: APPROVE
>
> **Punch list (open items)**: None
>
> What would you like to do?
> - Say "ready" to finalize
> - Provide feedback to revise
> - Say "abort" to cancel

If you're satisfied, say "ready". If you want changes, provide feedback and the loop continues.

### Finalize the Speck

Once approved, the speck is saved:

```
Speck saved to .specks/specks-py-calc.md with status "active"
```

## Step 5: Review the Generated Speck

Examine what was created:

```bash
specks list
```

Output:

```
Specks in /path/to/py-calc:

  specks-py-calc.md
    Status: active
    Steps: 4 (0 complete)
```

View the speck contents:

```bash
cat .specks/specks-py-calc.md
```

You'll see a structured document with:
- Phase overview and context
- Success criteria
- Design decisions
- Detailed execution steps with tasks and checkpoints

## Step 6: Validate the Speck

Verify the speck follows the correct format:

```bash
specks validate specks-py-calc.md
```

A valid speck shows:

```
Validated specks-py-calc.md
  Status: ok
  Errors: 0
  Warnings: 0
```

## Step 7: Execute the Speck

Run the execution workflow to implement the calculator:

```bash
specks execute .specks/specks-py-calc.md
```

The director orchestrates the agent suite for each step:

1. **Architect** creates an implementation strategy
2. **Implementer** writes the code (with Monitor watching)
3. **Reviewer** checks that the implementation matches the plan
4. **Auditor** verifies code quality
5. **Committer** prepares the git commit

By default, execution pauses after each step for your review (checkpoint mode: step).

### Track Progress

Check the status during or after execution:

```bash
specks status specks-py-calc.md
```

## Step 8: Test Your Calculator

After execution completes, you should have a working calculator. Test it:

```bash
python calculator.py
```

Example session:

```
Python Calculator
Type an expression or 'quit' to exit.

> 2 + 3
5
> 10 * (4 - 2)
20
> 100 / 0
Error: Division by zero
> quit
Goodbye!
```

## Alternative: Using Claude Code Internal Path

Instead of the CLI, you can run the same workflow inside a Claude Code session:

```
/specks-plan "create a python command-line calculator that supports +, -, *, /"
```

This enters the same iterative planning loop directly in your Claude Code session.

For execution:

```
/specks-execute .specks/specks-py-calc.md
```

Both paths produce identical outcomes—choose based on your workflow preference.

## Troubleshooting

### "Claude CLI not installed"

The `plan` and `execute` commands require Claude Code:

```bash
npm install -g @anthropic-ai/claude-code
```

### "Missing required agents"

If agents aren't found, verify your installation:

```bash
# Check agents are installed
ls /opt/homebrew/share/specks/agents/

# Or set the share directory explicitly
export SPECKS_SHARE_DIR=/opt/homebrew/share/specks
```

### "Skills not found"

Install or reinstall skills:

```bash
specks setup claude
```

For global installation (works across all projects):

```bash
specks setup claude --global
```

### Validation Errors in Generated Speck

If the planner generates an invalid speck, provide more specific requirements. You can also revise an existing speck:

```bash
specks plan .specks/specks-py-calc.md
```

## Summary

In this tutorial, you:

1. Created a new project from scratch
2. Initialized specks with `specks init`
3. Used the planning loop to design a calculator
4. Reviewed and approved the generated speck
5. Executed the speck to generate working code
6. Tested the resulting application

This workflow—**plan, validate, execute**—is the core specks experience. You can apply it to any software development task, from small features to complex systems.

## Next Steps

- **Learn more about planning**: [Create Your First Speck](first-speck.md)
- **Understand execution**: [Execute a Plan](execute-plan.md)
- **Explore the skeleton**: View `.specks/specks-skeleton.md` for the full speck format
- **Contribute**: See [CONTRIBUTING.md](../../CONTRIBUTING.md) for development setup
