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

# Worktree commands (for isolated implementation environments)
specks worktree create <speck>      # Create isolated worktree for implementation
specks worktree list                # List active worktrees
specks worktree cleanup --merged    # Remove worktrees for merged PRs
specks merge <speck>                # Merge PR and clean up (recommended approach)
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
| E035 | Beads sync failed |
| E036 | Bead commit failed |

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
| **implementer** | Orchestrates implementation loop: architect → coder → reviewer → committer |

### Sub-Agents (7)

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
| **reviewer-agent** | Verifies completed step matches plan and audits code quality, security, error handling | Read, Grep, Glob, Edit |
| **committer-agent** | Stages files, commits changes, updates implementation log, closes beads | Read, Grep, Glob, Write, Edit, Bash |

### Development Workflow

Use specks to develop specks:

```bash
cd /path/to/specks
claude --plugin-dir .
```

This loads the repo as a plugin. All skills and agents are available immediately.

## Worktree Workflow

The implementer skill uses git worktrees to isolate implementation work in separate directories with dedicated branches. This provides:

- **Isolation**: Each speck implementation gets its own branch and working directory
- **Parallel work**: Multiple specks can be implemented concurrently
- **Clean history**: One commit per step, matching bead granularity
- **PR-based review**: Implementation is complete when the PR is merged

### How It Works

When you run `/specks:implementer .specks/specks-N.md`:

1. **Worktree created**: A new git worktree is created at `.specks-worktrees/specks__<name>-<timestamp>/`
2. **Branch created**: A new branch `specks/<name>-<timestamp>` is created from main
3. **Beads synced**: Bead annotations are synced and committed to the worktree
4. **Steps executed**: Each step is implemented and committed separately
5. **PR created**: After all steps complete, a PR is automatically created to main

### Merge Workflow (Recommended)

After implementation completes and a PR is created, use the `specks merge` command to automate the merge workflow:

```bash
# Preview what will happen
specks merge .specks/specks-12.md --dry-run

# Merge the PR and clean up
specks merge .specks/specks-12.md
```

The merge command:
1. Finds the worktree for the speck
2. Checks that main is synced with origin (no unpushed commits)
3. Finds the PR for the worktree's branch
4. Verifies all PR checks have passed
5. Detects uncommitted changes in main
6. Auto-commits infrastructure files (agents/, .claude/skills/, CLAUDE.md, etc.)
7. Pushes main to origin
8. Merges the PR via squash
9. Pulls main to get the squashed commit
10. Cleans up the worktree and branch

**Infrastructure files** are auto-committed automatically:
- `agents/*.md` - Agent definition files
- `.claude/skills/**/` - Skill directories and contents
- `.specks/specks-skeleton.md` - Speck template
- `.specks/config.toml` - Configuration
- `.specks/specks-implementation-log.md` - Implementation log
- `.beads/*` - Beads tracking files
- `CLAUDE.md` - Project instructions

The command aborts if non-infrastructure files have uncommitted changes (use `--force` to override, though not recommended).

### Manual Cleanup (Alternative)

If you prefer manual control or the merge command is unavailable:

```bash
# Fetch latest main to ensure merge is detected
git fetch origin main

# Remove worktrees for merged PRs (dry run first)
specks worktree cleanup --merged --dry-run

# Actually remove them
specks worktree cleanup --merged
```

The cleanup command:
- Uses git-native worktree removal (`git worktree remove`)
- Prunes stale worktree metadata (`git worktree prune`)
- Deletes the local branch

### Troubleshooting

#### "Worktree already exists"

If you see this error, it means a worktree for this speck already exists:

```bash
# List all worktrees to see what exists
specks worktree list

# If the worktree is stale, remove it manually
rm -rf .specks-worktrees/specks__<name>-<timestamp>
git worktree prune
```

#### "Branch not merged" after PR merge

This can happen with squash or rebase merges, where the original commits are not ancestors of main:

```bash
# Update your local main branch
git fetch origin main
git checkout main
git pull origin main

# Try cleanup again
specks worktree cleanup --merged
```

If cleanup still fails, you may need to remove the worktree manually:

```bash
# Remove the worktree
git worktree remove .specks-worktrees/specks__<name>-<timestamp>

# Prune stale entries
git worktree prune

# Delete the branch manually
git branch -d specks/<name>-<timestamp>
```

#### Session in "needs_reconcile" state

This happens when a step commit succeeds but the bead close fails. The worktree is left in a consistent state, but beads tracking is out of sync. To fix:

1. Check the implementation log in the worktree for the bead ID
2. Close the bead manually: `specks beads close bd-xxx`
3. If continuing implementation, the next step should proceed normally
