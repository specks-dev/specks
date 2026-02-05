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
curl -L https://github.com/specks-dev/specks/releases/latest/download/specks-0.1.3-macos-arm64.tar.gz | tar xz
sudo mv bin/specks /usr/local/bin/

# For Intel Mac
curl -L https://github.com/specks-dev/specks/releases/latest/download/specks-0.1.3-macos-x86_64.tar.gz | tar xz
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

This creates a `.specks/` directory and installs Claude Code skills to `.claude/skills/`.

If you installed via binary download (not Homebrew), you may need to manually install the skills:

```bash
specks setup claude
```

Verify your installation:

```bash
specks --version
specks setup claude --check
```

## Quick Start

1. Initialize a specks project:

```bash
specks init
```

This creates a `.specks/` directory with:
- `specks-skeleton.md` - Template for new specks
- `config.toml` - Project configuration
- `runs/` - Agent run artifacts (gitignored)

2. Create a speck from the skeleton:

```bash
cp .specks/specks-skeleton.md .specks/specks-myfeature.md
```

3. Validate your speck:

```bash
specks validate specks-myfeature.md
```

4. Track progress:

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

## Agent Workflow

Specks uses a multi-agent architecture for creating and implementing specifications:

### Agent Suite

| Agent | Role | Description |
|-------|------|-------------|
| **Director** | Orchestrator | Central hub that coordinates all other agents |
| **Planner** | Idea → Speck | Transforms ideas into structured plans |
| **Architect** | Step → Strategy | Creates implementation strategies with expected touch sets |
| **Implementer** | Strategy → Code | Executes architect strategies, writes code |
| **Monitor** | Watchdog | Tracks progress, detects drift, signals halts |
| **Reviewer** | Quality | Reviews completed work for issues |
| **Auditor** | Compliance | Verifies adherence to spec and policies |
| **Logger** | Documentation | Records run activity and decisions |
| **Committer** | Git | Handles git operations (staging, committing) |

### Workflow Phases

1. **Planning**: Director invokes Planner to create/refine a speck
2. **Execution**: For each step:
   - Director invokes Architect to create strategy
   - Director invokes Implementer to execute strategy
   - Monitor tracks progress and drift
   - Reviewer/Auditor verify quality
3. **Commit**: Committer handles git operations based on commit-policy

### Run Artifacts

Agent runs create artifacts in `.specks/runs/<uuid>/`:
- `director-plan.md` - Director's execution plan
- `architect-plan.md` - Per-step architecture strategy
- `monitor-report.md` - Progress and drift reports
- `audit-report.md` - Compliance findings

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

Specks integrates with [Beads](https://github.com/specks-dev/beads) for issue/task tracking. This enables two-way synchronization between speck steps and external work items.

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
| E005 | Invalid anchor format |
| E006 | Duplicate anchor |
| E009 | Not initialized |
| E010 | Broken reference |
| E011 | Circular dependency |
| E013 | Beads not initialized |

## License

MIT
