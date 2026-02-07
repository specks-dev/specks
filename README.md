# Specks

Go from ideas to implementation via multi-agent orchestration. Specks transforms ideas into working software through a suite of specialized LLM agents that plan, implement, review, and track progress to completion.

## Installation

### Homebrew (macOS)

The easiest way to install specks on macOS:

```bash
brew tap specks-dev/specks https://github.com/specks-dev/specks
brew install specks
```

### Download Binary

Download prebuilt binaries from [GitHub Releases](https://github.com/specks-dev/specks/releases):

```bash
# For Apple Silicon (M1/M2/M3)
curl -L https://github.com/specks-dev/specks/releases/latest/download/specks-latest-macos-arm64.tar.gz | tar xz
sudo mv bin/specks /usr/local/bin/

# For Intel Mac
curl -L https://github.com/specks-dev/specks/releases/latest/download/specks-latest-macos-x86_64.tar.gz | tar xz
sudo mv bin/specks /usr/local/bin/
```

### From Source

Requires Rust 1.70+ and Cargo:

```bash
git clone https://github.com/specks-dev/specks.git
cd specks
cargo install --path crates/specks
```

### Post-Install Setup

After installation, initialize specks in your project:

```bash
cd your-project
specks init
```

This creates a `.specks/` directory with the skeleton template and configuration.

Verify your installation:

```bash
specks --version
```

### Using as a Claude Code Plugin

Specks is a Claude Code plugin. For development or local use:

```bash
cd /path/to/specks
claude --plugin-dir .
```

This loads all specks skills and agents. You can then use:
- `/specks:plan "your idea"` - Create a new speck
- `/specks:execute .specks/specks-name.md` - Execute a speck

## Quick Start

1. Initialize a specks project:

```bash
specks init
```

This creates a `.specks/` directory with:
- `specks-skeleton.md` - Template for new specks
- `config.toml` - Project configuration
- `runs/` - Agent run artifacts (gitignored)

2. Create a speck via Claude Code:

```bash
claude --plugin-dir /path/to/specks
# Then in Claude Code:
/specks:plan "add a health check endpoint"
```

Or manually from the skeleton:

```bash
cp .specks/specks-skeleton.md .specks/specks-myfeature.md
```

3. Validate your speck:

```bash
specks validate specks-myfeature.md
```

4. Execute your speck:

```bash
claude --plugin-dir /path/to/specks
# Then in Claude Code:
/specks:execute .specks/specks-myfeature.md
```

5. Track progress:

```bash
specks status specks-myfeature.md
specks list
```

## Commands

### `specks init`

Initialize a specks project in the current directory.

```bash
specks init          # Create .specks/ directory
specks init --force  # Overwrite existing .specks/
```

### `specks validate`

Validate speck structure against format conventions.

```bash
specks validate                    # Validate all specks
specks validate specks-1.md        # Validate specific file
specks validate --strict           # Enable strict mode
specks validate --json             # Output as JSON
```

### `specks list`

List all specks with summary information.

```bash
specks list                  # List all specks
specks list --status draft   # Filter by status
specks list --json           # Output as JSON
```

### `specks status`

Show detailed completion status for a speck.

```bash
specks status specks-1.md      # Show status
specks status specks-1.md -v   # Verbose (show tasks)
specks status specks-1.md --json  # Output as JSON
```

### `specks beads close`

Close a bead to mark work complete.

```bash
specks beads close bd-abc123                      # Close a bead
specks beads close bd-abc123 --reason "Step done" # Close with reason
specks beads close bd-abc123 --json               # JSON output
```

## Planning and Execution (Claude Code Skills)

Planning and execution are handled via Claude Code skills, not CLI commands.

### `/specks:plan`

Create or revise a speck through agent collaboration.

```
/specks:plan "add a health check endpoint"    # Create from idea
/specks:plan .specks/specks-existing.md       # Revise existing speck
```

The planning flow:
1. **Clarifier** analyzes the idea and generates questions
2. **Interviewer** presents questions and gathers user input
3. **Planner** creates a structured speck
4. **Critic** reviews for quality and implementability
5. Loop continues until critic approves or user accepts

### `/specks:execute`

Execute a speck step-by-step with agent orchestration.

```
/specks:execute .specks/specks-feature.md
```

The execution flow for each step:
1. **Architect** creates implementation strategy
2. **Implementer** executes strategy (with self-monitoring for drift)
3. **Reviewer** and **Auditor** verify work in parallel
4. **Logger** updates implementation log
5. **Committer** stages files and commits changes

## Agent and Skill Architecture

Specks uses a multi-agent architecture implemented as a Claude Code plugin.

### Agents (5)

Agents handle complex, multi-step workflows:

| Agent | Role | Description |
|-------|------|-------------|
| **director** | Orchestrator | Coordinates workflow via Task and Skill tools |
| **planner** | Idea → Speck | Creates and revises speck documents |
| **interviewer** | User Interaction | Single point of user interaction via AskUserQuestion |
| **architect** | Step → Strategy | Creates implementation strategies with expected touch sets |
| **implementer** | Strategy → Code | Executes strategies with self-monitoring for drift |

### Skills (8)

Skills run inline for focused tasks:

| Skill | Role | Description |
|-------|------|-------------|
| **plan** | Entry Point | Spawns director with mode=plan |
| **execute** | Entry Point | Spawns director with mode=execute |
| **clarifier** | Analysis | Analyzes ideas, returns clarifying questions |
| **critic** | Review | Reviews speck quality and implementability |
| **reviewer** | Verification | Verifies completed step matches plan |
| **auditor** | Quality | Checks code quality, security, error handling |
| **logger** | Documentation | Updates implementation log |
| **committer** | Git | Stages files, commits changes, closes beads |

### Run Artifacts

Agent runs create an audit trail in `.specks/runs/<session-id>/`:
- `metadata.json` - Session info (mode, speck path, timestamps)
- `planning/NNN-<skill>.json` - Skill outputs during planning
- `execution/step-N/` - Per-step artifacts

Run directories are gitignored by default.

## Speck Format

Specks follow a structured markdown format. See `.specks/specks-skeleton.md` for the complete template.

### Key Sections

- **Plan Metadata**: Owner, status, tracking info
- **Phase Overview**: Context, strategy, scope
- **Design Decisions**: [D01], [D02], etc.
- **Execution Steps**: Step 0, Step 1, etc.
- **Deliverables**: Exit criteria, milestones

### Anchors and References

- Use explicit anchors: `### Section {#section-name}`
- Reference anchors: `**Depends on:** #step-0, #step-1`
- Reference decisions: `**References:** [D01] Decision name`

## Beads Integration

Specks integrates with [Beads](https://github.com/kocienda/beads) for issue/task tracking. This enables two-way synchronization between speck steps and external work items.

### Requirements

- **Beads CLI** (`bd`) must be installed and available in PATH
- **Beads initialized** in your project (`bd init` creates `.beads/` directory)
- **Network connectivity** for beads commands (they communicate with the beads backend)

### Commands

#### `specks beads sync`

Sync speck steps to beads—creates beads for steps and writes IDs back to the speck.

```bash
specks beads sync specks-1.md           # Sync a specific speck
specks beads sync specks-1.md --dry-run # Preview without making changes
specks beads sync specks-1.md --prune-deps  # Remove stale dependency edges
```

This creates:
- A **root bead** (epic) for the entire speck
- **Child beads** for each execution step
- **Dependency edges** matching the `**Depends on:**` lines

Bead IDs are written back to the speck file:
- `**Beads Root:** \`bd-xxx\`` in Plan Metadata
- `**Bead:** \`bd-xxx.1\`` in each step

#### `specks beads status`

Show execution status for each step based on linked beads.

```bash
specks beads status specks-1.md    # Show status for one speck
specks beads status                # Show status for all specks
specks beads status --pull         # Also update checkboxes
```

Status values:
- **complete**: Bead is closed
- **ready**: Bead is open, all dependencies are complete
- **blocked**: Waiting on dependencies
- **pending**: No bead linked yet

#### `specks beads pull`

Update speck checkboxes from bead completion status.

```bash
specks beads pull specks-1.md      # Pull completion for one speck
specks beads pull                  # Pull for all specks
specks beads pull --no-overwrite   # Don't change manually checked items
```

When a step's bead is closed, `pull` marks the checkpoint items as complete.

#### `specks beads link`

Manually link an existing bead to a step.

```bash
specks beads link specks-1.md step-3 bd-abc123
```

### Two-Way Sync Workflow

Beads integration supports a bidirectional workflow:

1. **Plan → Beads** (sync): Create beads from your speck
   ```bash
   specks beads sync specks-feature.md
   ```

2. **Work in Beads**: Team members work on beads, closing them when complete

3. **Beads → Plan** (pull): Update speck checkboxes from bead status
   ```bash
   specks beads pull specks-feature.md
   ```

4. **Check Status**: See what's ready to work on
   ```bash
   specks beads status specks-feature.md
   ```

5. **Iterate**: Re-sync after adding new steps, pull after completing work

### Example Session

```bash
# Initialize beads (one-time setup)
bd init

# Create beads from your speck
specks beads sync specks-1.md
# Output: Synced specks-1.md to beads:
#   Root bead: bd-abc123
#   Steps synced: 5
#   Dependencies added: 3

# Check what's ready to work on
specks beads status specks-1.md
# Output: Step 0: Setup     [x] complete  (bd-abc123.1)
#         Step 1: Core      [ ] ready     (bd-abc123.2)
#         Step 2: Tests     [ ] blocked   (bd-abc123.3) <- waiting on bd-abc123.2

# After completing work, close the bead
bd close bd-abc123.2

# Pull completion back to speck checkboxes
specks beads pull specks-1.md
# Output: specks-1: 3 checkboxes updated
```

### Beads Readiness Checklist

Before using beads integration, verify your setup:

1. **Specks CLI installed and on PATH:**
   ```bash
   specks --version
   # Should show: specks x.y.z
   ```

2. **Beads CLI (`bd`) installed and on PATH:**
   ```bash
   bd --version
   # Should show: bd x.y.z
   ```
   If not on PATH, set `SPECKS_BD_PATH` or configure in `.specks/config.toml`.

3. **Beads initialized in your project:**
   ```bash
   ls .beads/
   # Should show: config.toml, beads.db, etc.
   ```
   If not present, run `bd init`.

4. **Verify beads commands work:**
   ```bash
   specks beads status --json
   # Should return valid JSON (even if no specks have beads yet)
   ```

**Discovery chain for `bd` binary:**
1. `SPECKS_BD_PATH` environment variable (highest priority)
2. `config.specks.beads.bd_path` from `.specks/config.toml`
3. Default `"bd"` (expects `bd` on PATH)

## Configuration

Project configuration lives in `.specks/config.toml`:

```toml
[specks]
skeleton_file = "specks-skeleton.md"
default_status = "draft"
naming_pattern = "specks-*.md"

[specks.beads]
enabled = true
bd_path = "bd"              # Path to beads CLI
root_issue_type = "epic"    # Issue type for root bead
substeps = "none"           # Substep handling: "none" or "children"
pull_checkbox_mode = "checkpoints"  # What to check: "checkpoints" or "all"
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments or file not found |
| 3 | Validation error |
| 5 | Beads CLI not installed |
| 9 | Not initialized (.specks/ not found) |
| 13 | Beads not initialized |

## Error Codes

| Code | Description |
|------|-------------|
| E001 | Parse error |
| E002 | Missing required field |
| E005 | Invalid anchor format / Beads CLI not installed |
| E006 | Duplicate anchor |
| E009 | Not initialized (.specks/ not found) |
| E010 | Broken reference |
| E011 | Circular dependency |
| E013 | Beads not initialized (.beads/ not found) |
| E016 | Beads command failed |

## Troubleshooting

### "Not initialized"

Run `specks init` in your project directory to create the `.specks/` directory.

### "Beads CLI not installed" (E005)

The beads commands require the `bd` binary:

1. Install the beads CLI from [beads releases](https://github.com/kocienda/beads/releases)
2. Add to PATH, or set `SPECKS_BD_PATH` environment variable
3. Verify: `bd --version`

### "Beads not initialized" (E013)

Run `bd init` in your project directory to create the `.beads/` directory.

### "Beads command failed" (E016)

A beads operation failed. Check the error message for details. Common causes:
- Network connectivity issues
- Invalid bead ID
- Permission problems

### Validation Errors

Check the specific issues with:

```bash
specks validate specks-problem.md --json
```

Common issues: missing sections, invalid anchor format, broken references.

### Plugin Not Loading

If skills/agents aren't available in Claude Code:

```bash
# Verify you're loading the plugin
claude --plugin-dir /path/to/specks

# Check skills are discovered
# In Claude Code: /help
# Should list /specks:plan, /specks:execute, etc.
```

## Documentation

- **[Getting Started Guide](docs/getting-started.md)** - Installation, setup, and core concepts
- **[Tutorial: Create Your First Speck](docs/tutorials/first-speck.md)** - Walk through the planning workflow
- **[Tutorial: Execute a Plan](docs/tutorials/execute-plan.md)** - Walk through the execution workflow

## License

MIT
