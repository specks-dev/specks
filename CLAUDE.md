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

Agent definitions live in `agents/` directory as markdown with YAML frontmatter:
```markdown
---
name: director
description: Central orchestrator for specks workflow
tools: Task, Skill, Read, Grep, Glob, Bash, Write
model: opus
---
```

### Skill Files

Skills live in `skills/<name>/SKILL.md` with YAML frontmatter:
```markdown
---
name: clarifier
description: Analyze ideas and generate clarifying questions
allowed-tools: Read, Grep, Glob
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

```
/specks:plan "add user authentication"    # Create a new speck
/specks:plan .specks/specks-auth.md       # Revise existing speck
/specks:execute .specks/specks-auth.md    # Execute a speck
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
| `/specks:plan` | Create or revise a speck through agent collaboration |
| `/specks:execute` | Execute a speck through agent orchestration |

### Agents (5)

Agents handle complex, multi-step workflows with isolated context:

| Agent | Role |
|-------|------|
| **director** | Pure orchestrator. Coordinates workflow via Task and Skill tools. |
| **planner** | Creates and revises speck documents. |
| **interviewer** | Single point of user interaction. Presents questions via AskUserQuestion. |
| **architect** | Creates implementation strategies with expected touch sets. |
| **implementer** | Executes architect strategies with self-monitoring for drift. |

### Skills (8)

Skills run inline for focused, single-purpose tasks:

| Skill | Role |
|-------|------|
| **plan** | Entry point. Spawns director with mode=plan. |
| **execute** | Entry point. Spawns director with mode=execute. |
| **clarifier** | Analyzes ideas, returns clarifying questions. |
| **critic** | Reviews speck for quality and implementability. |
| **reviewer** | Verifies completed step matches plan. |
| **auditor** | Checks code quality, security, error handling. |
| **logger** | Updates implementation log with completed work. |
| **committer** | Stages files, commits changes, closes beads. |

### Development Workflow

Use specks to develop specks:

```bash
cd /path/to/specks
claude --plugin-dir .
```

This loads the repo as a plugin. All skills and agents are available immediately.
