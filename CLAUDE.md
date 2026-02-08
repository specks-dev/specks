# Claude Code Guidelines for Specks

## Project Overview

Specks transforms ideas into working software through orchestrated LLM agents. A multi-agent suite collaborates to create structured plans and execute them to completion—from initial idea through implementation, review, and delivery.

## Crate Structure

```
crates/
├── specks/          # CLI binary crate
│   └── src/
│       ├── main.rs      # Entry point
│       ├── cli.rs       # Clap argument parsing
│       ├── output.rs    # JSON/text output types
│       └── commands/    # Subcommand implementations
└── specks-core/     # Library crate
    └── src/
        ├── lib.rs       # Public exports
        ├── types.rs     # Core data types (Speck, Step, etc.)
        ├── parser.rs    # Markdown speck parser
        ├── validator.rs # Validation rules
        ├── config.rs    # Configuration handling
        └── error.rs     # Error types
```

## Build Policy

**WARNINGS ARE ERRORS.** This project enforces `-D warnings` via `.cargo/config.toml`.

- `cargo build` will fail if there are any warnings
- `cargo nextest run` will fail if tests have any warnings
- Fix warnings immediately; do not leave them for later
- Use `#[allow(dead_code)]` sparingly and only with a comment explaining why

If you see warnings, fix them before completing your task. No exceptions.

## Key Conventions

### Speck Format

Specks are structured markdown files in `.specks/` directory:
- Filename pattern: `specks-*.md` (e.g., `specks-1.md`, `specks-auth.md`)
- Reserved files: `specks-skeleton.md`, `config.toml`

### Anchors

- Use explicit anchors: `### Section {#section-name}`
- Anchor format: lowercase, kebab-case, no phase numbers
- Step anchors: `{#step-0}`, `{#step-1}`, `{#step-2-1}` (substeps)
- Decision anchors: `{#d01-decision-slug}`

### References in Steps

Every execution step must have a `**References:**` line citing plan artifacts:
- Decisions: `[D01] Decision name`
- Anchors: `(#anchor-name, #another-anchor)`
- Tables/Specs: `Table T01`, `Spec S01`

### Dependencies

Steps declare dependencies with:
```markdown
**Depends on:** #step-0, #step-1
```

### Agent Files

Sub-agent definitions live in `agents/` directory as markdown with YAML frontmatter:
```markdown
---
name: clarifier-agent
description: Analyze ideas and generate clarifying questions
tools: Read, Grep, Glob
---
```

### Skill Files

Orchestrator skills live in `skills/<name>/SKILL.md` with YAML frontmatter:
```markdown
---
name: planner
description: Orchestrates the planning workflow - spawns sub-agents via Task
disable-model-invocation: true
allowed-tools: Task, AskUserQuestion, Read, Grep, Glob, Write, Bash
---
```

## Testing

Run tests with:
```bash
cargo nextest run
```

Test fixtures are in `tests/fixtures/`:
- `valid/` - Valid specks for success cases
- `invalid/` - Invalid specks for error cases
- `golden/` - Expected JSON output

## Common Commands

### CLI (Utility Commands)

```bash
specks init                    # Initialize project
specks validate                # Validate all specks
specks validate specks-1.md    # Validate specific file
specks list                    # List all specks
specks status specks-1.md      # Show progress
specks beads sync specks-1.md  # Sync steps to beads
specks beads status            # Show bead completion status
specks beads close bd-xxx      # Close a bead
```

### Claude Code Skills (Planning and Execution)

**IMPORTANT: Initialize first!** Before using the planner or implementer, run:
```bash
specks init
```

This creates the `.specks/` directory with required files:
- `specks-skeleton.md` - Template for speck structure
- `config.toml` - Configuration settings
- `specks-implementation-log.md` - Progress tracking
- `runs/` - Session artifacts (gitignored)

Then use the skills:
```
/specks:planner "add user authentication"    # Create a new speck
/specks:planner .specks/specks-auth.md       # Revise existing speck
/specks:implementer .specks/specks-auth.md   # Execute a speck
```

## Error Codes

| Code | Description |
|------|-------------|
| E001 | Parse error |
| E002 | Missing required field |
| E005 | Invalid anchor format |
| E006 | Duplicate anchor |
| E009 | Not initialized |
| E010 | Broken reference |
| E011 | Circular dependency |

## Implementation Log

The implementation log at `.specks/specks-implementation-log.md` tracks completed work. The `/specks:logger` skill updates this log after completing steps during execution.

## Agent and Skill Architecture

Specks is a Claude Code plugin. Planning and execution are invoked via skills, not CLI commands.

### Primary Interface

| Skill | Purpose |
|-------|---------|
| `/specks:planner` | Create or revise a speck through agent collaboration |
| `/specks:implementer` | Execute a speck through agent orchestration |

### Orchestrator Skills (2)

Two orchestrator skills contain the main workflow logic and spawn sub-agents via Task tool:

| Skill | Role |
|-------|------|
| **planner** | Orchestrates planning loop: clarifier → author → critic |
| **implementer** | Orchestrates implementation loop: architect → coder → reviewer → auditor → logger → committer |

### Sub-Agents (9)

Sub-agents are invoked via Task tool and return JSON results. Each has specific tools and contracts.

**Planning agents (invoked by planner):**

| Agent | Role | Tools |
|-------|------|-------|
| **clarifier-agent** | Analyzes ideas, generates clarifying questions | Read, Grep, Glob |
| **author-agent** | Creates and revises speck documents | Read, Grep, Glob, Write, Edit |
| **critic-agent** | Reviews speck quality and skeleton compliance | Read, Grep, Glob |

**Implementation agents (invoked by implementer):**

| Agent | Role | Tools |
|-------|------|-------|
| **architect-agent** | Creates implementation strategies, defines expected_touch_set | Read, Grep, Glob |
| **coder-agent** | Executes strategies with drift detection, self-halts on drift | Read, Grep, Glob, Write, Edit, Bash |
| **reviewer-agent** | Verifies completed step matches plan | Read, Grep, Glob |
| **auditor-agent** | Checks code quality, security, error handling | Read, Grep, Glob |
| **logger-agent** | Updates implementation log with completed work | Read, Grep, Glob, Edit |
| **committer-agent** | Stages files, commits changes, closes beads | Read, Grep, Glob, Bash |

### Development Workflow

Use specks to develop specks:

```bash
cd /path/to/specks
claude --plugin-dir .
```

This loads the repo as a plugin. All skills and agents are available immediately.
