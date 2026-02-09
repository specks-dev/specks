# Getting Started with Specks

This guide will help you install specks, set up your first project, and understand the core workflows.

## Prerequisites

Before using specks, you'll need:

- **macOS**: specks currently supports macOS (arm64 and x86_64)
- **Claude Code**: Required for agent orchestration (`plan` and `execute` commands)
- **Git**: For version control integration

### Installing Claude Code

Specks uses Claude Code to orchestrate its agents. Install it from:

```bash
npm install -g @anthropic-ai/claude-code
```

Or follow the instructions at the [Claude Code documentation](https://docs.anthropic.com/claude-code).

## Installation

### Option 1: Homebrew (Recommended)

The easiest way to install specks on macOS:

```bash
brew tap specks-dev/specks https://github.com/specks-dev/specks
brew install specks
```

### Option 2: Download Binary

Download prebuilt binaries from [GitHub Releases](https://github.com/specks-dev/specks/releases):

```bash
# For Apple Silicon (M1/M2/M3)
curl -L https://github.com/specks-dev/specks/releases/latest/download/specks-latest-macos-arm64.tar.gz | tar xz
sudo mv bin/specks /usr/local/bin/

# For Intel Mac
curl -L https://github.com/specks-dev/specks/releases/latest/download/specks-latest-macos-x86_64.tar.gz | tar xz
sudo mv bin/specks /usr/local/bin/
```

### Option 3: Build from Source

Requires Rust 1.70+ and Cargo:

```bash
git clone https://github.com/specks-dev/specks.git
cd specks
cargo install --path crates/specks
```

## Initial Setup

### 1. Initialize Your Project

Navigate to your project directory and initialize specks:

```bash
cd your-project
specks init
```

This creates:
- `.specks/` directory with configuration and the skeleton template
- `.claude/skills/` directory with Claude Code skills (if skills are available)

### 2. Install Claude Code Skills

If you installed via binary download, you may need to manually install the Claude Code skills:

```bash
specks setup claude
```

Verify the installation:

```bash
specks setup claude --check
```

### 3. Verify Installation

Check that everything is working:

```bash
specks --version
specks list       # Should show no specks yet
```

## Core Concepts

### What is a Speck?

A **speck** is a structured markdown document that describes a software change—from high-level idea to detailed implementation steps. Specks live in the `.specks/` directory and follow a defined format (see `.specks/specks-skeleton.md`).

Key sections in a speck:
- **Plan Metadata**: Owner, status, tracking info
- **Phase Overview**: Context, strategy, scope, success criteria
- **Design Decisions**: Recorded decisions with rationale
- **Execution Steps**: Step-by-step implementation with tasks, tests, and checkpoints

### The Agent Suite

Specks uses a multi-agent architecture where specialized agents collaborate:

| Agent | Role |
|-------|------|
| **Director** | Central orchestrator—coordinates all other agents |
| **Planner** | Transforms ideas into structured specks |
| **Critic** | Reviews plan quality and completeness |
| **Interviewer** | Gathers requirements and presents feedback |
| **Architect** | Creates implementation strategies for steps |
| **Implementer** | Writes code following architect's strategy |
| **Monitor** | Tracks progress and detects drift |
| **Reviewer** | Checks plan adherence after each step |
| **Auditor** | Verifies code quality |
| **Committer** | Handles git operations |

### Two Invocation Paths

You can invoke specks workflows in two ways:

**External CLI (terminal workflow):**
```bash
specks plan "add user authentication"
specks execute .specks/specks-auth.md
```

**Internal Claude Code (session workflow):**
```
/specks-plan "add user authentication"
/specks-execute .specks/specks-auth.md
```

Both paths produce identical outcomes—choose based on your workflow preferences.

## Workflow Overview

### 1. Planning: Idea to Speck

The planning workflow transforms an idea into a structured speck through an iterative refinement loop:

```
specks plan "your idea here"
         |
    INTERVIEWER (gather requirements)
         |
    PLANNER (create speck)
         |
    CRITIC (review quality)
         |
    INTERVIEWER (present results, ask: "ready or revise?")
         |
    user says ready? --> speck saved as active
    user has feedback? --> loop back with feedback
```

**Key features:**
- No arbitrary iteration limit—loop continues until you approve
- Punch list tracks open items across iterations
- Supports both new ideas and revision of existing specks

### 2. Execution: Speck to Code

The execution workflow implements a speck step-by-step:

```
specks execute .specks/specks-feature.md
         |
    FOR each step (in dependency order):
         |
    ARCHITECT (create implementation strategy)
         |
    IMPLEMENTER + MONITOR (write code, watch for drift)
         |
    REVIEWER + AUDITOR (verify quality)
         |
    COMMITTER (prepare commit)
         |
    (checkpoint or continue)
```

**Key features:**
- Steps execute in dependency order
- Monitor can halt execution if drift is detected
- Supports manual or automatic commits

## Quick Start: Build a Python Calculator

The fastest way to learn specks is to build something. Let's create a Python command-line calculator from scratch.

### 1. Create and Initialize Your Project

```bash
mkdir py-calc
cd py-calc
specks init
```

You'll see that specks creates the `.specks/` directory and installs Claude Code skills.

### 2. Plan Your Application

Start the planning loop with a clear description:

```bash
specks plan "create a python command-line calculator that supports +, -, *, /"
```

The interviewer asks clarifying questions, the planner creates a speck, and the critic reviews it. When you're satisfied with the plan, say "ready" to finalize.

### 3. Validate the Speck

Check that the generated speck is valid:

```bash
specks validate specks-py-calc.md
```

### 4. Execute the Speck

Implement the calculator by executing the speck:

```bash
specks execute .specks/specks-py-calc.md
```

The director orchestrates the agent suite to implement each step. You'll see progress updates and be prompted at checkpoints.

### 5. Track Progress

```bash
specks status specks-py-calc.md   # Detailed status
specks list                        # All specks overview
```

For the complete walkthrough, see the [Python Calculator Tutorial](tutorials/py-calc-example.md).

## Using Specks Inside Claude Code

If you're already in a Claude Code session, you can use slash commands:

```
/specks-plan "add caching to the database layer"
```

This enters the same iterative planning loop but runs directly in your Claude Code session, which can be more convenient than shelling out to the CLI.

For execution:

```
/specks-execute .specks/specks-caching.md
```

## Common Options

### Plan Command Options

```bash
specks plan [OPTIONS] [INPUT]

Options:
  --name <NAME>        Name for the speck file
  --context <FILE>     Additional context files (repeatable)
  --timeout <SECS>     Timeout per agent invocation (default: 300)
  --json               Output result as JSON
  --quiet              Suppress progress messages
```

### Execute Command Options

```bash
specks execute [OPTIONS] <SPECK>

Options:
  --start-step <ANCHOR>   Start from this step (e.g., #step-2)
  --end-step <ANCHOR>     Stop after this step
  --commit-policy <P>     manual or auto (default: manual)
  --checkpoint-mode <M>   step, milestone, or continuous
  --dry-run               Show plan without executing
  --timeout <SECS>        Timeout per step (default: 600)
  --json                  Output result as JSON
```

## Troubleshooting

### "Claude CLI not installed"

The `plan` and `execute` commands require Claude Code. Install it:

```bash
npm install -g @anthropic-ai/claude-code
```

Then verify:

```bash
which claude
```

### "Not initialized"

Run `specks init` in your project directory to create the `.specks/` directory.

### "Skills not found"

If you installed specks via binary download, run:

```bash
specks setup claude
```

This copies the Claude Code skills from the share directory to your project.

### Validation Errors

Run `specks validate` to see specific issues:

```bash
specks validate --json specks-problem.md
```

Common issues:
- Missing required sections (check against `.specks/specks-skeleton.md`)
- Invalid anchor format (use lowercase, kebab-case)
- Broken references (ensure cited anchors exist)

### Agent Timeout

Increase the timeout for complex operations:

```bash
specks execute .specks/specks-complex.md --timeout 900
```

## Next Steps

- **Tutorial**: [Build a Python Calculator](tutorials/py-calc-example.md) — Complete greenfield example
- **Tutorial**: [Create Your First Speck](tutorials/first-speck.md) — Deep dive into the planning loop
- **Tutorial**: [Execute a Plan](tutorials/execute-plan.md) — Understanding the execution workflow
- **Contributing**: See [CONTRIBUTING.md](../CONTRIBUTING.md) for development setup
- **Reference**: Check `.specks/specks-skeleton.md` for the full speck format
