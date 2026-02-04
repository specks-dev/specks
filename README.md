# Specks

From ideas to implementation via multi-agent orchestration. Specks transforms ideas into working software through a suite of specialized LLM agents that plan, implement, review, and track progress to completion.

## Installation

### From source

```bash
git clone https://github.com/yourusername/specks.git
cd specks
cargo install --path crates/specks
```

### Requirements

- Rust 1.70+
- Cargo

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

## Configuration

Project configuration lives in `.specks/config.toml`:

```toml
[specks]
skeleton_file = "specks-skeleton.md"
default_status = "draft"
naming_pattern = "specks-*.md"

[beads]
enabled = false
root_issue_type = "epic"
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
