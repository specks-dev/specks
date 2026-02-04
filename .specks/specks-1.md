## Phase 1.0: Specks - Agent-Centric Technical Specifications {#phase-1}

**Purpose:** Deliver an agent-centric system for transforming ideas into comprehensive, actionable technical specifications. A suite of specialized agents—orchestrated by a central director—produces, validates, implements, and tracks specks conforming to a defined format, supported by CLI utilities for validation, listing, and status tracking.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | TBD |
| Status | draft |
| Target branch | main |
| Tracking issue/PR | TBD |
| Last updated | 2026-02-03 |
| Beads Root | *(optional; written by `specks beads sync`)* |

---

### Phase Overview {#phase-overview}

#### Context {#context}

Specks is a system for turning ideas into actionable technical specifications via LLM agents. The core value is the **multi-agent suite**—a team of specialized agents orchestrated by a central **director** agent. The planner takes brief ideas or detailed descriptions, explores codebases, asks clarifying questions, and produces comprehensive specks. The architect designs implementation strategies. The implementer writes code while the monitor watches for drift. The reviewer and auditor provide quality gates. The logger and committer handle documentation and commits.

The skeleton (`.specks/specks-skeleton.md`) defines a **format specification**, not a template for mechanical substitution. It establishes what a good speck looks like: structured sections for decisions, specifications, execution steps, and validation criteria. The CLI provides utilities to support the workflow: initializing projects, validating speck structure, listing specks, and tracking completion status.

This approach differs from template-based documentation tools. The intelligence lives in the agent suite, which understands context, asks the right questions, and produces specifications tailored to each project and feature. The CLI ensures specks conform to the format and provides visibility into progress.

#### Strategy {#strategy}

- **Agent-first**: The multi-agent suite (director, planner, architect, implementer, etc.) is the primary interface for creating and executing specifications
- **Director as orchestrator**: All agents report to the director; the director makes all decisions
- **Format over template**: The skeleton defines structure and conventions, not fill-in-the-blank text
- **CLI as utility layer**: Commands support the workflow without replacing agent-driven creation
- **Validation as quality gate**: Ensure specks conform to the format for consistency and tooling
- **Standalone operation**: CLI works independently; Claude Code integration is one usage pattern

#### Stakeholders / Primary Customers {#stakeholders}

1. Developers using LLM agents to plan complex features before implementation
2. Teams wanting structured, consistent technical specifications
3. AI coding assistants that produce and implement from specifications

#### Success Criteria (Measurable) {#success-criteria}

- Planner agent produces specks that pass `specks validate` without errors
- `specks init` creates a working `.specks/` directory with skeleton and config
- `specks validate` detects all structural errors in a test speck with known issues
- `specks list` accurately shows all specks with status and progress
- `specks status <file>` reports correct step-by-step completion
- Director agent orchestrates the full per-step loop: architect → implementer+monitor → reviewer+auditor → logger → committer

#### Scope {#scope}

1. Multi-agent suite: director, planner, architect, implementer, monitor, reviewer, auditor, logger, committer
2. CLI infrastructure with clap-based command parsing
3. `specks init` command for project initialization
4. `specks validate [file]` command for structure validation
5. `specks list` command for listing all specks with status
6. `specks status <file>` command for completion tracking
7. Speck format validation rules and error codes
8. Configuration file support (`.specks/config.toml`)
9. `specks beads sync <file>` command to create/update beads from steps (and optionally substeps) and write bead IDs back
10. `specks beads link <file> <step-anchor> <bead-id>` command to manually link steps to beads
11. `specks beads status [file]` command to show beads execution status
12. `specks beads pull [file]` command to update checkboxes from bead completion status
13. Run persistence: UUID-based `.specks/runs/` directory for agent reports and audit trail

#### Non-goals (Explicitly out of scope) {#non-goals}

- Template substitution system for `specks new` (agent creates specks, not templates)
- Web UI or GUI
- Collaborative editing features
- Version control integration beyond file operations
- MCP server implementation - Phase 2

#### Dependencies / Prerequisites {#dependencies}

- Rust toolchain (1.70+)
- `.specks/specks-skeleton.md` format specification exists
- Agent definitions in `agents/specks-*.md` (director, planner, architect, implementer, monitor, reviewer, auditor, logger, committer)
- Skills available: implement-plan, update-plan-implementation-log, prepare-git-commit-message

#### Constraints {#constraints}

- Must work on macOS, Linux, and Windows
- Configuration should be local to project (`.specks/` directory)
- Core CLI commands (`init`, `validate`, `list`, `status`) work fully offline
- Beads commands (`sync`, `link`, `beads status`, `beads pull`) require network connectivity
- Must handle large speck files efficiently (100KB+)

#### Assumptions {#assumptions}

- Users have access to LLM agents (Claude Code, or future MCP integration) for speck creation
- Specks follow Markdown format with specific structural conventions
- The skeleton represents the authoritative format specification
- Project root is identifiable by presence of `.specks/` directory

---

### Section Numbering Convention {#section-numbering}

| Placeholder | Meaning | Example |
|-------------|---------|---------|
| `1` | Major phase number | `1` |
| `0` | Minor phase number | `1.0` |
| `1.0.N` | Numbered section within phase | `1.0.1`, `1.0.2` |
| `1.0.N.M` | Subsection within a numbered section | `1.0.1.1`, `1.0.2.3` |

**Standard section numbers:**
- `1.0.0` - Design Decisions
- `1.0.1` - Specification
- `1.0.2` - Symbol Inventory
- `1.0.3` - Documentation Plan
- `1.0.4` - Test Plan Concepts
- `1.0.5` - Execution Steps
- `1.0.6` - Deliverables and Checkpoints

---

### Open Questions (MUST RESOLVE OR EXPLICITLY DEFER) {#open-questions}

#### [Q01] Binary distribution strategy (DEFERRED) {#q01-binary-distribution}

**Question:** How should specks be distributed? Cargo install, prebuilt binaries, or both?

**Why it matters:** Affects installation instructions and CI/CD setup.

**Options (if known):**
- Cargo install only (simplest, requires Rust)
- Prebuilt binaries via GitHub releases
- Both approaches

**Plan to resolve:** Defer to Phase 2; cargo install sufficient for initial development.

**Resolution:** DEFERRED (Phase 2 will address distribution after core is stable)

#### [Q02] Minimal `specks new` behavior (DECIDED) {#q02-specks-new}

**Question:** Should `specks new` exist in Phase 1, and what should it do?

**Why it matters:** The core value is agent-driven speck creation, not template substitution.

**Options:**
- Remove `specks new` entirely - agents create specks directly
- Minimal scaffold - create a file with just the required structure headings
- Instructions only - print guidance on using the planner agent

**Resolution:** DECIDED - Phase 1 omits `specks new`. The planner agent creates specks. Users can copy the skeleton manually if needed. Phase 2 may add a minimal scaffold command.

---

### Risks and Mitigations {#risks}

| Risk | Impact | Likelihood | Mitigation | Trigger to revisit |
|------|--------|------------|------------|--------------------|
| Skeleton format evolves during development | med | med | Version the skeleton, document format version | Format changes break existing specks |
| Validation too strict for real-world documents | med | low | Start lenient, add strictness flags | User complaints about false positives |
| Agent produces specks that don't validate | med | med | Test agent output against validation; iterate on agent prompts | Validation failures on agent-produced specks |

---

### 1.0.0 Design Decisions {#design-decisions}

#### [D01] Rust implementation with clap for CLI (DECIDED) {#d01-rust-clap}

**Decision:** Build specks CLI as a Rust application using clap with derive macros.

**Rationale:**
- Consistency with beads (also Rust) simplifies future integration
- Excellent cross-platform support
- clap derive provides clean, maintainable command definitions
- Strong type system catches errors at compile time

**Implications:**
- Need Rust toolchain for building
- CLI structure defined via Rust structs with clap attributes
- Error handling uses thiserror or anyhow

---

#### [D02] Project structure with `.specks/` directory (DECIDED) {#d02-specks-directory}

**Decision:** All specks-related files live in a `.specks/` directory at project root.

**Rationale:**
- Keeps specification files organized and separate from source code
- Hidden directory reduces clutter in file listings
- Single location simplifies tooling and glob patterns
- Mirrors patterns like `.git/`, `.github/`

**Implications:**
- `specks init` creates `.specks/` with initial structure
- All commands operate relative to `.specks/` directory
- Configuration lives at `.specks/config.toml`

---

#### [D03] Speck file naming and discovery (DECIDED) {#d03-file-naming}

**Decision:** Speck files use the naming pattern `specks-<name>.md` where `<name>` matches `^[a-z][a-z0-9-]{1,49}$`. Reserved files are explicitly excluded from speck discovery.

**Rationale:**
- Clear prefix makes specks easily identifiable
- Standard `.md` extension ensures editor support
- Explicit exclusion list prevents reserved files from being treated as specks

**Implications:**
- **Reserved files (never treated as specks):**
  - `specks-skeleton.md` - the format specification
  - `specks-implementation-log.md` - the implementation log
- **Speck discovery rule:** files matching `specks-*.md` EXCEPT reserved files

---

#### [D04] Anchor format for cross-references (DECIDED) {#d04-anchor-format}

**Decision:** Use explicit Markdown anchors with specific prefix conventions.

**Rationale:**
- Explicit anchors survive heading text changes
- Prefixes enable automated validation and linking
- Consistent with skeleton format conventions

**Implications:**
- Step anchors: `{#step-N}`, `{#step-N-M}` for substeps
- Decision anchors: `{#dNN-slug}` (e.g., `{#d01-rust-clap}`)
- Question anchors: `{#qNN-slug}` (e.g., `{#q01-distribution}`)
- Validation checks for anchor format compliance

---

#### [D05] Checkbox-based completion tracking (DECIDED) {#d05-checkbox-tracking}

**Decision:** Track execution step completion via Markdown checkboxes (`- [ ]` / `- [x]`).

**Rationale:**
- Standard Markdown syntax, no custom formats
- Visible in any Markdown renderer
- Easy for humans and tools to update
- Matches skeleton format conventions

**Implications:**
- `specks status` counts checked vs unchecked boxes in Tasks/Checkpoints
- Status calculation: `completed / total` items
- Empty checkbox = pending, checked = done

---

#### [D06] Configuration via TOML (DECIDED) {#d06-config-toml}

**Decision:** Use TOML format for `.specks/config.toml` configuration file.

**Rationale:**
- Human-readable and writable
- Standard in Rust ecosystem (Cargo.toml)
- Good support via `toml` crate
- Hierarchical structure for organized settings

**Implications:**
- Config file is optional (sensible defaults)
- Settings can be overridden by CLI flags
- Config includes: validation strictness and beads integration settings

---

#### [D07] Project root resolution via upward search (DECIDED) {#d07-root-resolution}

**Decision:** Commands search upward from current working directory to find `.specks/` directory, stopping at filesystem root.

**Rationale:**
- Matches git behavior (familiar to developers)
- Allows running commands from any subdirectory
- Single `.specks/` directory per project (no nesting)

**Implications:**
- `specks` commands work from any subdirectory of the project
- If no `.specks/` found, commands exit with E009 ("not initialized")
- `specks init` always creates `.specks/` in current directory

---

#### [D08] JSON output schema with shared envelope (DECIDED) {#d08-json-schema}

**Decision:** All commands with `--json` output use a shared response envelope for consistency.

**Rationale:**
- Consistent structure simplifies tooling and scripting
- Schema version enables forward compatibility
- Separating `data` from `issues` makes parsing predictable

**Implications:**
- All JSON responses follow the envelope structure defined in Spec S05
- `schema_version` starts at "1" and increments on breaking changes
- `status` is "ok" or "error" based on exit code

---

#### [D09] Agent-driven speck creation (DECIDED) {#d09-agent-creation}

**Decision:** Specks are created by LLM agents (planner agent), not by template substitution commands.

**Rationale:**
- The intelligence is in the agent, not the template
- Agents can understand context, ask questions, and produce tailored specifications
- Template substitution produces low-value boilerplate that still needs substantial editing
- The skeleton is a format specification for agents to follow, not a fill-in-the-blank template

**Implications:**
- No `specks new` command in Phase 1
- Planner agent is the primary way to create specks
- Users can manually copy skeleton if they prefer to write specks by hand
- Phase 2 may add minimal scaffold command for edge cases

---

#### [D10] Beads-compatible step dependencies (DECIDED) {#d10-step-dependencies}

**Decision:** Execution steps declare dependencies using `**Depends on:**` lines with anchor references.

**Rationale:**
- Anchor references are stable (survive title changes)
- Format matches existing Reference conventions
- Machine-parseable for validation and beads sync
- Maps directly to beads `needs` relationships

**Implications:**
- Steps (except Step 0) should have explicit dependencies
- Dependencies validated by `specks validate` (E010, E011)
- `specks beads sync` converts dependencies to bead edges
- Bead IDs stored in `**Bead:**` line after sync

---

#### [D11] Director orchestrates execution with configurable commit policy (DECIDED) {#d11-commit-policy}

**Decision:** The director agent orchestrates execution with a configurable commit policy. The committer agent has the *ability* to commit, but whether it actually commits is controlled by `commit-policy: manual|auto` specified at director invocation time.

**Rationale:**
- Provides flexibility: human oversight when needed, autonomous operation when trusted
- Clear policy boundary: the decision is made upfront, not per-commit
- Consistent with existing skill constraints (implement-plan, prepare-git-commit-message)
- Supports both learning/high-risk scenarios and trusted/CI scenarios

**Commit policies:**

| Policy | Behavior | Use case |
|--------|----------|----------|
| `manual` | Committer writes commit message; user commits | Learning, high-risk changes, initial development |
| `auto` | Committer writes commit message AND commits | Trusted implementations, CI scenarios |

**Phase 1 constraint:** `commit-policy=auto` commits only (`git commit`). It never pushes (`git push`) or opens PRs automatically. Push/PR automation is deferred to Phase 2+.

**Implications:**
- Director invocation includes `commit-policy` parameter
- With `manual`: committer prepares message, director pauses for user signal
- With `auto`: committer commits immediately, director proceeds to next step
- Bead closure happens after commit (manual or auto)
- Policy is logged in run persistence for audit trail

**Future direction (Phase 2+):** Additional policies may include:
- `auto-on-green`: auto-commit if tests pass, pause if tests fail
- `batch`: accumulate changes, commit at milestone boundaries

---

#### [D12] Multi-agent architecture with director as orchestrator (DECIDED) {#d12-multi-agent-architecture}

**Decision:** Use a suite of nine specialized agents orchestrated by a central director agent.

**Rationale:**
- Clear separation of concerns mirrors how effective software teams work
- Each agent can be specialized and optimized for its specific task
- Monitor agent provides real-time drift detection during implementation
- Dual review (plan adherence + code quality) catches different categories of issues
- Director as hub enables clear escalation paths and coordinated decision-making
- Aligns with Claude Code's subagent model for practical implementation

**The nine agents:**

| Agent | Phase | Role |
|-------|-------|------|
| **director** | both | Orchestrates all other agents, makes decisions, handles escalation |
| **planner** | planning | Takes idea → structured plan following skeleton format |
| **architect** | planning | Takes step → implementation strategy + test plan |
| **implementer** | execution | Writes code, focused doer |
| **monitor** | execution | Runs parallel to implementer, detects drift, can signal halt |
| **reviewer** | execution | Checks plan adherence: did implementation match spec? |
| **auditor** | both | Checks code quality/performance, runs at step/milestone/completion |
| **logger** | execution | Writes detailed change log entries |
| **committer** | execution | Writes commit message and commits |

**Implications:**
- Director explicitly invokes each agent (hub-and-spoke, not peer-to-peer)
- All reports flow back to director; director makes all decisions
- Monitor can return early to director with "halt" signal before implementer completes
- Reviewer and auditor both run after each step, with different focus areas
- Agent definitions stored as markdown files in `agents/specks-*.md`
- Leverages Claude Code subagent infrastructure for isolation and coordination

---

#### [D13] Reviewer vs Auditor: complementary quality gates (DECIDED) {#d13-reviewer-vs-auditor}

**Decision:** Separate plan adherence checking (reviewer) from code quality checking (auditor) as distinct agents with different responsibilities.

**Rationale:**
- **Plan adherence** and **code quality** are orthogonal concerns
- Implementation can match the plan perfectly but still have quality issues
- Implementation can have excellent quality but drift from the plan
- Separating concerns allows each agent to be deeply focused
- Both perspectives are valuable for the director's decision-making

**Reviewer responsibilities (plan adherence):**
- Does the implementation match what was specified in the plan step?
- Are all tasks from the step completed?
- Do the tests match the test plan?
- Are the artifacts produced as expected?
- Were the references followed correctly?

**Auditor responsibilities (code quality/performance):**
- Is the code well-structured and maintainable?
- Are there performance concerns?
- Are there security issues?
- Does the code follow project conventions?
- Are there edge cases or error conditions not handled?

**When they run:**
- Both run after each step completes
- Auditor also runs at milestone boundaries for holistic review
- Auditor runs at plan completion for final quality assessment
- Both report to director; director decides on actions

**Implications:**
- Two separate agent definitions with focused prompts
- Director receives two reports per step and synthesizes them
- Different failure modes lead to different escalation paths:
  - Reviewer failures → likely back to architect or planner (conceptual miss)
  - Auditor failures → likely back to implementer (quality fix)

---

#### [D14] Cooperative halt protocol for monitor agent (DECIDED) {#d14-cooperative-halt}

**Decision:** Use cooperative halting with signal files and worktree isolation as the **reliable** stop mechanism. When running in Claude Code interactive mode, the director MAY additionally request **interactive cancellation** (foreground: `Ctrl+C`; background: `/tasks`) to reduce halt latency, but Specks MUST NOT depend on cancellation being available or reliable.

**Rationale:**
- Claude Code does not document a **programmatic** “cancel subagent” capability via the Task tool API; interactive cancellation is user-mediated and may be unreliable across versions
- Cooperative halting achieves the goal without requiring undocumented or brittle features
- Worktree isolation provides a safety net: changes can be discarded if needed
- Signal files are simple, debuggable, and cross-process

**Protocol:**

1. **Implementer runs in background**, writes to temp worktree or staged changes only
2. **Monitor polls** for uncommitted changes at intervals
3. **If monitor detects significant drift:**
   - Monitor writes `.specks/runs/{uuid}/.halt` signal file
   - Monitor returns to director with HALT status and drift report
4. **Director halts implementation**:
   - Preferred: rely on cooperative halt (below)
   - Optional (Claude Code interactive): instruct user to cancel the running implementer task (background: `/tasks`; foreground: `Ctrl+C`)
5. **Implementer checks for halt signal** between major operations (file edits, test runs, long commands)
6. **If halt signal found:**
   - Implementer stops immediately
   - Implementer returns partial status to director
7. **Director decides:** discard changes, review, or continue

**Signal file format:**
```json
{
  "reason": "drift_detected",
  "drift_type": "wrong_files",
  "drift_severity": "high",
  "timestamp": "2026-02-04T12:34:56Z"
}
```

**Implications:**
- Implementer agent must be coded to check for halt signal
- Latency between check points (seconds, not milliseconds)
- Changes not committed until director approves
- Monitor is advisory + interruptive (cooperative); interactive cancellation is a best-effort accelerator

---

#### [D15] Run persistence with UUID-based directories (DECIDED) {#d15-run-persistence}

**Decision:** Each director invocation creates a UUID-based run directory under `.specks/runs/` to store agent reports and provide an audit trail.

**Rationale:**
- Persistent reports enable debugging and post-mortem analysis
- Director can reference past runs for context
- Clear separation between runs prevents confusion
- Supports "replay" and "resume" scenarios

**Directory structure:**
```
.specks/
  runs/
    {uuid}/
      invocation.json      # Director config, commit-policy, timestamp, speck path
      planner-report.md    # Planner output (planning phase only)
      architect-plan.md    # Architect's implementation strategy per step
      monitor-log.jsonl    # Monitor observations (append-only)
      reviewer-report.md   # Reviewer assessment after each step
      auditor-report.md    # Auditor findings after each step
      committer-prep.md    # Commit message and files staged
      .halt                # Halt signal file (if monitor halted)
      status.json          # Final run status (success/failure/halted)
```

**Lifecycle:**
1. Director creates `{uuid}/` at start of invocation
2. Each agent writes its report to the run directory
3. Monitor appends observations to `monitor-log.jsonl`
4. On completion, director writes `status.json` with final outcome
5. Old runs can be cleaned up by user or retention policy

**Implications:**
- All agent reports are durable, not ephemeral
- Run directory passed to agents as context
- `.specks/runs/` MUST be in `.gitignore` (always ignored, never committed)
- UUID generated at director invocation time
- No retention policy: runs accumulate until user deletes them manually

---

#### [D16] Director invocation protocol (DECIDED) {#d16-director-invocation}

**Decision:** Define a standard invocation protocol for the director agent, including required and optional parameters.

**Invocation parameters:**

| Parameter | Required | Default | Description |
|-----------|----------|---------|-------------|
| `speck` | yes | - | Path to speck file |
| `mode` | no | `execute` | `plan` (create/refine speck) or `execute` (implement steps) |
| `start-step` | no | first ready | Step anchor to start from |
| `end-step` | no | all | Step anchor to stop after |
| `commit-policy` | no | `manual` | `manual` or `auto` |
| `checkpoint-mode` | no | `step` | `step`, `milestone`, or `continuous` |

**Example invocations:**
```
# Planning mode: create a new speck from an idea
director --mode plan --idea "Add user authentication"

# Execution mode: implement all ready steps
director --speck .specks/specks-auth.md --commit-policy manual

# Execution mode: implement through specific step
director --speck .specks/specks-auth.md --end-step step-3 --commit-policy auto

# Execution mode: resume from specific step
director --speck .specks/specks-auth.md --start-step step-4
```

**Implications:**
- Director validates parameters before starting
- Run UUID generated and logged at invocation
- Parameters stored in `invocation.json` for audit trail
- Invalid parameters result in immediate failure with clear error

---

### Deep Dives {#deep-dives}

#### CLI Command Structure {#cli-structure}

**Diagram Diag01: Command Hierarchy** {#diag01-command-hierarchy}

```
specks [global-options] <command> [command-options]
├── init [--force]              # Initialize .specks/ directory
├── validate [file] [--strict]  # Validate speck structure against format
├── list [--status]             # List all specks with status
├── status <file> [--verbose]   # Show step-by-step completion
└── beads
    ├── sync <file>         # Create/update beads from steps (idempotent)
    ├── link <file> <anchor> <id>  # Link step to existing bead
    ├── status [file] [--pull]     # Show beads execution status
    └── pull [file]         # Update checkboxes from bead completion
```

**Global options (apply to all commands):**

| Option | Description |
|--------|-------------|
| `--json` | Output in JSON format per Spec S05 envelope |
| `--verbose` | Increase output verbosity |
| `--quiet` | Suppress non-error output |
| `--version` | Print version and exit |
| `--help` | Print help and exit |

---

#### Validation Rules {#validation-rules}

**List L01: Structural Validation Rules** {#l01-validation-rules}

Validation checks the structure of specks against the format defined in the skeleton.

**Metadata field presence rules:**
- A required metadata field is **present** if: (1) the table row exists AND (2) the value cell is non-empty
- The literal value `"TBD"` is considered **present** (not missing) for Owner and Tracking issue/PR fields
- Values matching `<...>` pattern (angle-bracket placeholders) are treated as **unfilled placeholders** - warning, not error
- Status must be one of: `draft`, `active`, `done` (case-insensitive)

**Errors (must fix):**
1. Missing required sections: Plan Metadata, Phase Overview, Design Decisions, Execution Steps, Deliverables
2. Plan Metadata missing required fields: Owner, Status, Last updated
3. Plan Metadata Status field has invalid value (not draft/active/done)
4. Execution steps without References line
5. Anchors with invalid characters (only `a-z`, `0-9`, `-` allowed)
6. Duplicate anchor names within document
7. Dependency references non-existent step anchor (E010)
8. Circular dependency detected (E011)
9. Invalid bead ID format (E012). When beads integration is enabled and `validate_bead_ids` is true: prefer validation via `bd show <id> --json` (bead must exist). Fallback regex: `^[a-z0-9][a-z0-9-]*-[a-z0-9]+(\.[0-9]+)*$`.
10. When beads integration is enabled and Plan Metadata has a Beads Root ID: root bead must exist (E014). Check via `bd show <root-id> --json`.
11. When beads integration is enabled and a step has a `**Bead:**` line: that bead must exist (E015). Check via `bd show <id> --json`.

**Warnings (should fix):**
1. Decisions without DECIDED/OPEN status
2. Questions without resolution status
3. Steps without checkpoint items
4. Steps without test items
5. References citing non-existent anchors
6. Metadata fields with unfilled placeholders (`<...>` pattern)
7. Step (other than Step 0) has no dependencies (W007)
8. Bead ID present but beads integration not enabled (W008)

**Info (optional):**
1. Document exceeds recommended size (2000+ lines)
2. Deep dive sections exceed 50% of document
3. Missing optional sections (Risks, Rollout)

---

#### Speck Lifecycle States {#speck-lifecycle}

**Concept C01: Speck Status Model** {#c01-status-model}

A speck progresses through these states based on metadata and checkbox completion:

| Status | Condition |
|--------|-----------|
| `draft` | Metadata Status = "draft" |
| `active` | Metadata Status = "active", completion < 100% |
| `done` | Metadata Status = "done" OR completion = 100% |

Completion percentage = (checked boxes) / (total boxes) in Execution Steps section.

**Status consistency warning:** If metadata Status = "done" but checkbox completion < 100%, this is a conflict. The `specks status` command should:
- Show a warning: "Status is 'done' but only X% of checkboxes are checked"
- Display both "declared status" (from metadata) and "computed status" (from checkboxes)
- Example output: `specks-feature.md: done (declared) / active (computed: 75%)`

---

#### Skills Workflow {#skills-workflow}

**Concept C02: Agent-Skill Mapping** {#c02-agent-skill-mapping}

The director orchestrates agents, and agents invoke skills as needed:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Per-Step Execution Flow                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  DIRECTOR ─→ ARCHITECT ─→ IMPLEMENTER ─→ REVIEWER ─→ AUDITOR    │
│                              │                                   │
│                              ▼                                   │
│                    [implement-plan skill]                        │
│                    Code + Tests + Checkboxes                     │
│                              │                                   │
│                              ▼                                   │
│              DIRECTOR ─→ LOGGER ─→ [update-log skill]            │
│                              │                                   │
│                              ▼                                   │
│              DIRECTOR ─→ COMMITTER ─→ [prepare-commit skill]     │
│                              │                                   │
│                              ▼                                   │
│              IF commit-policy=auto: git commit                   │
│              IF commit-policy=manual: USER COMMITS               │
│                              │                                   │
│                              ▼                                   │
│              DIRECTOR ─→ bd close + bd sync                      │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**Agent-skill mapping:**

| Agent | Primary Skill | Does |
|-------|---------------|------|
| implementer | implement-plan | Write code, run tests, check plan checkboxes |
| logger | update-plan-implementation-log | Prepend entry to implementation log |
| committer | prepare-git-commit-message | Write commit message; optionally commit |

**Commit policy (D11):** With `commit-policy=manual`, committer writes the message and director pauses. With `commit-policy=auto`, committer writes AND commits.

**Skill invocation pattern:**
```
/implement-plan <speck-path> step-<N>
/update-plan-implementation-log <log-path>
/prepare-git-commit-message
```

For specks projects, `<log-path>` is `.specks/specks-implementation-log.md`. The skill accepts a path argument to remain generic and reusable across different project structures.

---

#### Multi-Agent Architecture {#multi-agent-architecture}

**Concept C03: Agent Suite Design** {#c03-agent-suite}

The specks system uses a suite of nine specialized agents orchestrated by a central director. This replaces simpler monolithic-agent approaches with an architecture that mirrors how effective software teams operate.

**Architecture Diagram:**

```
                              ┌──────────────┐
                              │   DIRECTOR   │
                              │ (orchestrator)│
                              └──────┬───────┘
                                     │
        ┌────────────────────────────┼────────────────────────────┐
        │                            │                            │
        │         PLANNING           │         EXECUTION          │
        │                            │                            │
   ┌────▼────┐                  ┌────▼────┐                  ┌────▼────┐
   │ PLANNER │                  │ARCHITECT│                  │IMPLEMENTER│
   │idea→plan│                  │step→strat│                  │writes code│
   └─────────┘                  └─────────┘                  └─────┬─────┘
                                                                   │
                                                            (runs parallel)
                                                                   │
                                                            ┌──────▼──────┐
                                                            │   MONITOR   │
                                                            │detects drift│
                                                            └─────────────┘
        │                            │                            │
        │         QUALITY            │          SUPPORT           │
        │                            │                            │
   ┌────▼────┐    ┌─────────┐   ┌────▼────┐    ┌────────┐   ┌────▼────┐
   │ REVIEWER│    │ AUDITOR │   │ LOGGER  │    │COMMITTER│   │ AUDITOR │
   │plan adh.│    │code qual│   │changelog│    │ commit  │   │(holistic)│
   └─────────┘    └─────────┘   └─────────┘    └─────────┘   └─────────┘
```

**Key Principles:**

1. **Hub-and-spoke topology**: Director is the hub; all other agents are spokes. No peer-to-peer communication.
2. **Director decides**: All reports flow to director. Director makes all decisions about next steps.
3. **Specialized focus**: Each agent excels at one thing. No agent tries to do everything.
4. **Parallel monitoring**: Monitor runs alongside implementer, can signal halt before implementer completes.
5. **Dual quality gates**: Reviewer (plan adherence) and auditor (code quality) provide complementary perspectives.

---

**Concept C04: Planning Phase Flow** {#c04-planning-flow}

The planning phase transforms an idea into an implementable plan.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         PLANNING PHASE                                   │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐              │
│  │  IDEA   │───▶│ PLANNER │───▶│ AUDITOR │───▶│DIRECTOR │              │
│  │(input)  │    │         │    │(quality)│    │(decide) │              │
│  └─────────┘    └────┬────┘    └────┬────┘    └────┬────┘              │
│                      │              │              │                    │
│                      ▼              ▼              ▼                    │
│                   PLAN          QUALITY       GO/NO-GO                  │
│                 (draft)         REPORT        DECISION                  │
│                                                   │                     │
│                      ┌──────────────────────────◀─┘                     │
│                      │                                                  │
│                      ▼                                                  │
│            ┌──────────────────┐                                         │
│            │ IF NO-GO:        │                                         │
│            │ → Back to PLANNER│                                         │
│            │   with feedback  │                                         │
│            └──────────────────┘                                         │
│                                                                          │
│            ┌──────────────────┐                                         │
│            │ IF GO:           │                                         │
│            │ → For each step: │                                         │
│            │   ARCHITECT      │                                         │
│            └──────────────────┘                                         │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

**Planning phase agents:**

| Agent | Input | Output | Focus |
|-------|-------|--------|-------|
| **planner** | Idea + codebase context | Structured plan | Breaking down work, sequencing, defining steps |
| **architect** | Plan step | Implementation strategy + test plan | How to build it, what could go wrong |
| **auditor** | Plan | Quality report | Is this plan sound? Complete? Implementable? |

**Planner responsibilities:**
- Explore codebase to understand context
- Ask clarifying questions when requirements are ambiguous
- Break idea into implementable steps with clear boundaries
- Define dependencies between steps
- Produce plan following skeleton format

**Architect responsibilities (runs per step):**
- Analyze step requirements
- Design implementation approach
- Identify files to create/modify (machine-readable "expected touch set")
- Define unit test strategy
- Define integration test strategy
- Anticipate edge cases and error conditions
- Document assumptions and constraints

**Architect output contract (machine-readable):**
The architect MUST emit an `expected_touch_set` block in `architect-plan.md` to provide **advisory expectations** to the monitor and director about what files/directories are likely to be touched. This is a *hint*, not a hard constraint:

```yaml
# architect-plan.md (excerpt)
expected_touch_set:
  create:
    - src/commands/auth.rs
    - tests/auth_test.rs
  modify:
    - src/cli.rs
    - src/lib.rs
  directories:
    - src/commands/
    - tests/
```

This helps the monitor reason about drift (and ask better questions) without brittle, hard-fail gating.

---

**Concept C05: Execution Phase Flow** {#c05-execution-flow}

The execution phase implements each step with continuous quality monitoring.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    EXECUTION PHASE (per step)                            │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │ 1. DIRECTOR invokes ARCHITECT with step                           │  │
│  │    → Receives implementation strategy + test plan                 │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                              │                                           │
│                              ▼                                           │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │ 2. DIRECTOR spawns IMPLEMENTER + MONITOR in parallel              │  │
│  │                                                                    │  │
│  │    IMPLEMENTER                    MONITOR                          │  │
│  │    ├─ writes code                 ├─ polls for uncommitted changes │  │
│  │    ├─ runs tests                  ├─ evaluates against plan        │  │
│  │    └─ checks task boxes           └─ can return EARLY with HALT    │  │
│  │                                                                    │  │
│  │    IF MONITOR returns HALT:                                        │  │
│  │    → DIRECTOR stops IMPLEMENTER                                    │  │
│  │    → DIRECTOR decides: back to ARCHITECT? PLANNER?                 │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                              │                                           │
│                              ▼                                           │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │ 3. DIRECTOR invokes REVIEWER + AUDITOR                            │  │
│  │                                                                    │  │
│  │    REVIEWER                       AUDITOR                          │  │
│  │    ├─ checks plan adherence       ├─ checks code quality           │  │
│  │    ├─ tasks completed?            ├─ performance concerns?         │  │
│  │    ├─ tests match plan?           ├─ security issues?              │  │
│  │    └─ artifacts produced?         └─ conventions followed?         │  │
│  │                                                                    │  │
│  │    Both report to DIRECTOR. DIRECTOR decides next action.          │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                              │                                           │
│                              ▼                                           │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │ 4. IF issues found:                                               │  │
│  │    → Minor quality issues: back to IMPLEMENTER                    │  │
│  │    → Design issues: back to ARCHITECT                             │  │
│  │    → Conceptual issues: back to PLANNER                           │  │
│  │                                                                    │  │
│  │    IF both reports clean:                                         │  │
│  │    → DIRECTOR invokes LOGGER (write change log)                   │  │
│  │    → DIRECTOR invokes COMMITTER (commit changes)                  │  │
│  │    → Proceed to next step                                         │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

**Concept C06: Monitor Agent Protocol** {#c06-monitor-protocol}

The monitor agent is unique in that it runs parallel to the implementer and can interrupt execution.

**How monitor works:**

1. Director spawns implementer as a background task (using Claude Code's `run_in_background`)
2. Director spawns monitor, providing it the task ID of the implementer
3. Monitor polls for uncommitted changes at intervals (e.g., every 30 seconds or after tool calls)
4. Monitor evaluates changes against:
   - The plan step specification
   - The architect's implementation strategy
   - Project conventions
5. If monitor detects risk of drift or needs clarification:
   - Monitor returns early to director with a `PAUSE` or `HALT` signal and a drift/concern report
   - Director receives monitor's return and decides whether to continue, pause for questions, or halt the implementer
   - Director decides how to proceed based on severity and recommendation

**Drift detection criteria:**

| Drift Type | Severity | Likely Escalation | Detection Method |
|------------|----------|-------------------|------------------|
| Wrong files being modified | High | Architect | Use `expected_touch_set` as a signal; investigate and ask: is the new file touch justified by the step? |
| Implementation approach differs from strategy | Medium | Architect | Code review against architect-plan.md |
| Tests not following test plan | Medium | Architect | Check test additions vs the test plan (and expected touch hints) |
| Code quality concerns | Low | Implementer (continue, note for review) | Heuristic analysis |
| Scope creep (doing more than specified) | High | Planner | Qualitative: work clearly beyond step scope, new features, large refactors not implied |
| Missing required functionality | Medium | Let implementer continue | Compare checkboxes + deltas vs spec; ask whether gaps remain |

**Touch set is advisory (NOT a gate):** `expected_touch_set` is an *expectation hint* from the architect, not a strict allowlist. Modifying files outside the touch set MUST NOT automatically trigger HALT. Instead, the monitor should:
1. Identify the mismatch (what was touched vs expected)
2. Assess whether it is plausibly justified by the step
3. If unclear or high-risk, return to the director with questions and a recommendation (continue vs pause vs revisit architect)

**PAUSE vs HALT:**
- Recommend `PAUSE` when you have concerns or questions but drift is not yet clear (e.g., a surprising file touch that might be justified; an unclear interpretation of a requirement; early signs of scope creep).
- Recommend `HALT` only when there is strong evidence of drift (e.g., sustained work in unrelated subsystems, new user-facing scope not in the step, or refactors that materially change architecture without justification).

**Monitor return structure:**

```
{
  "status": "CONTINUE" | "PAUSE" | "HALT",
  "drift_detected": boolean,
  "drift_severity": "none" | "low" | "medium" | "high",
  "drift_type": "...",
  "drift_description": "...",
  "questions": ["..."], 
  "recommendation": "continue" | "pause_for_director" | "return_to_architect" | "return_to_planner"
}
```

---

**Concept C07: Escalation Paths** {#c07-escalation-paths}

When issues are detected, the director routes back to the appropriate agent.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        ESCALATION DECISION TREE                          │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ISSUE DETECTED                                                          │
│       │                                                                  │
│       ▼                                                                  │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ Is this a CONCEPTUAL problem?                                    │    │
│  │ (wrong understanding of requirements, scope miss, missing step)  │    │
│  └───────────────────────────┬─────────────────────────────────────┘    │
│                              │                                           │
│            YES               │               NO                          │
│             │                │                │                          │
│             ▼                │                ▼                          │
│     ┌───────────┐            │      ┌─────────────────────────────┐     │
│     │ → PLANNER │            │      │ Is this a DESIGN problem?   │     │
│     │  (revise  │            │      │ (wrong approach, bad arch,  │     │
│     │   plan)   │            │      │  missing test strategy)     │     │
│     └───────────┘            │      └─────────────┬───────────────┘     │
│                              │                    │                      │
│                              │      YES           │          NO          │
│                              │       │            │           │          │
│                              │       ▼            │           ▼          │
│                              │  ┌──────────┐      │    ┌─────────────┐   │
│                              │  │→ARCHITECT│      │    │ Is this a   │   │
│                              │  │ (revise  │      │    │ QUALITY     │   │
│                              │  │ strategy)│      │    │ problem?    │   │
│                              │  └──────────┘      │    └──────┬──────┘   │
│                              │                    │           │          │
│                              │                    │    YES    │    NO    │
│                              │                    │     │     │     │    │
│                              │                    │     ▼     │     ▼    │
│                              │                    │ ┌───────┐ │ ┌──────┐ │
│                              │                    │ │→IMPL. │ │ │ LOG  │ │
│                              │                    │ │(fix)  │ │ │& SKIP│ │
│                              │                    │ └───────┘ │ └──────┘ │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

**Concept C08: Agent Definition Format** {#c08-agent-definition}

Each agent is defined as a markdown file following Claude Code's subagent format.

**File location:** `agents/specks-<agent-name>.md`

**Example: specks-director.md**

```markdown
---
name: specks-director
description: Orchestrates all specks agents. Use when executing a speck plan.
tools: Task, Read, Grep, Glob, Bash
model: opus
---

You are the specks director agent, the central orchestrator for all specks work.

## Your Role

You coordinate the work of all other specks agents:
- planner: creates plans from ideas
- architect: creates implementation strategies from steps
- implementer: writes code
- monitor: watches for drift during implementation
- reviewer: checks plan adherence
- auditor: checks code quality
- logger: writes change log entries
- committer: commits changes

## Principles

1. You invoke agents explicitly - they do not invoke each other
2. All reports come to you - you make all decisions
3. You can halt work at any time if quality is at risk
4. You prioritize completing work correctly over completing quickly

## Workflow

[Detailed workflow instructions...]
```

**Example: specks-monitor.md**

```markdown
---
name: specks-monitor
description: Monitors implementation for drift. Runs parallel to implementer.
tools: Read, Grep, Glob, Bash
model: haiku
---

You are the specks monitor agent. You watch implementation work in progress
and detect when it drifts from the plan.

## Your Role

You run in parallel with the implementer. Your job is to:
1. Poll for uncommitted changes
2. Evaluate changes against the plan and architect's strategy
3. Return IMMEDIATELY to director if you detect significant drift
4. Otherwise, continue monitoring until implementer completes

## Drift Detection

[Criteria for detecting drift...]

## Return Format

When you detect drift that warrants halting:
{
  "status": "HALT",
  "drift_detected": true,
  ...
}
```

---

### 1.0.1 Specification {#specification}

#### 1.0.1.1 Inputs and Outputs {#inputs-outputs}

**Inputs:**
- Speck files: `.specks/specks-<name>.md` (see [D03](#d03-file-naming))
- Skeleton: `.specks/specks-skeleton.md` (format specification, reserved)
- Implementation log: `.specks/specks-implementation-log.md` (reserved)
- Configuration: `.specks/config.toml` (optional)
- Command-line arguments

**Outputs:**
- Initialized project structure (from `specks init`)
- Validation reports (from `specks validate`)
- Status summaries (from `specks list`, `specks status`)
- JSON output when `--json` flag is used (see [S05](#s05-json-schema))

**Key invariants:**
- Speck files are valid UTF-8 Markdown
- Skeleton and implementation log are never modified by commands
- All file operations are within `.specks/` directory
- Project root is found by searching upward for `.specks/` directory (see [D07](#d07-root-resolution))

---

#### 1.0.1.2 Terminology and Naming {#terminology}

- **Speck**: A structured technical specification document following the skeleton format
- **Skeleton**: The format specification defining speck structure (`.specks/specks-skeleton.md`)
- **Anchor**: An explicit Markdown anchor for cross-referencing (`{#anchor-name}`)
- **Step**: An execution step within a speck (numbered, with tasks and checkpoints)
- **Substep**: A nested step (e.g., Step 2.1, Step 2.2) within a parent step
- **Checkpoint**: A verification item within a step (checkbox format)
- **Run**: A single director invocation, identified by UUID, with persistent reports in `.specks/runs/{uuid}/`

**Agent Suite:**
- **director**: Central orchestrator; invokes all other agents, makes all decisions, handles escalation
- **planner**: Takes idea → structured plan following skeleton format
- **architect**: Takes step → implementation strategy + test plan
- **implementer**: Writes code; focused doer that follows architect's strategy
- **monitor**: Runs parallel to implementer; detects drift, can signal halt
- **reviewer**: Checks plan adherence after each step (did implementation match spec?)
- **auditor**: Checks code quality/performance; runs at step, milestone, and completion
- **logger**: Writes detailed change log entries to implementation log
- **committer**: Writes commit message; commits changes when commit-policy allows

---

#### 1.0.1.3 Command Specifications {#command-specs}

**Spec S01: `specks init`** {#s01-init}

| Aspect | Specification |
|--------|---------------|
| Synopsis | `specks init [--force]` |
| Purpose | Initialize a specks project in current directory |
| Precondition | Current directory is writable |
| Postcondition | `.specks/` directory exists with skeleton and config |
| Idempotent | Yes (with `--force`), No (without, fails if exists) |

Creates:
- `.specks/` directory
- `.specks/specks-skeleton.md` (copy of embedded format specification)
- `.specks/config.toml` (with defaults)
- `.specks/specks-implementation-log.md` (empty with header)
- `.specks/runs/` directory (for agent reports, always gitignored)
- Appends `.specks/runs/` to project `.gitignore` (creates if needed)

---

**Spec S02: `specks validate [file]`** {#s02-validate}

| Aspect | Specification |
|--------|---------------|
| Synopsis | `specks validate [file] [--strict] [--json]` |
| Purpose | Validate speck structure against format conventions |
| Precondition | File exists and is readable |
| Postcondition | None (read-only) |
| Default | Validate all specks if no file specified |

Output format (default):
```
specks-feature.md: 2 errors, 3 warnings

Errors:
  Line 45: Missing References line in Step 3
  Line 89: Invalid anchor format: {#Step 4}

Warnings:
  Line 12: Decision [D02] missing status
  Line 67: Step 5 has no checkpoint items
```

Exit codes:
- 0: Valid (warnings allowed)
- 1: Validation errors found
- 2: File not found or unreadable

**Speck discovery:** When no file specified, validates all files matching `specks-*.md` in `.specks/` directory, excluding reserved files. See [D03](#d03-file-naming).

---

**Spec S03: `specks list`** {#s03-list}

| Aspect | Specification |
|--------|---------------|
| Synopsis | `specks list [--json] [--status <status>]` |
| Purpose | List all specks with summary information |
| Precondition | `.specks/` exists |
| Postcondition | None (read-only) |

Output format (default):
```
SPECK                    STATUS    PROGRESS   UPDATED
specks-1                 active    12/47      2026-02-03
specks-feature-x         draft     0/23       2026-02-01
specks-refactor          done      35/35      2026-01-28
```

Columns:
- SPECK: Name without prefix/extension
- STATUS: From metadata (draft/active/done)
- PROGRESS: Checked/total checkboxes in execution steps
- UPDATED: Last updated date from metadata

---

**Spec S04: `specks status <file>`** {#s04-status}

| Aspect | Specification |
|--------|---------------|
| Synopsis | `specks status <file> [--json] [--verbose]` |
| Purpose | Show detailed completion status for a speck |
| Precondition | File exists |
| Postcondition | None (read-only) |

Output format (default):
```
specks-1.md: active (25% complete)

Step 0: Project Setup                    [x] 3/3
Step 1: Core Types                       [x] 5/5
Step 2: CLI Framework                    [ ] 2/8
  Step 2.1: Command Parsing              [x] 2/2
  Step 2.2: Init Command                 [ ] 0/3
Step 3: Validation Engine                [ ] 0/12
...

Total: 12/47 tasks complete
```

Verbose mode adds:
- Individual task text
- References for each step
- Checkpoint details

---

**Spec S05: JSON Output Schema** {#s05-json-schema}

All commands with `--json` use a shared response envelope:

```json
{
  "schema_version": "1",
  "command": "init|validate|list|status",
  "status": "ok|error",
  "data": { /* command-specific payload */ },
  "issues": [ /* validation issues, warnings, etc. */ ]
}
```

**Issue object structure:**
```json
{
  "code": "E001",
  "severity": "error|warning|info",
  "message": "Missing required section: Plan Metadata",
  "file": ".specks/specks-foo.md",
  "line": 45,
  "anchor": "#step-3"
}
```

**Path and anchor normalization rules:**
- `file` fields are **project-root-relative** paths using forward slashes
- `anchor` fields are either `null` or a string that **always starts with `#`**

**Command-specific `data` payloads:**

| Command | `data` structure |
|---------|------------------|
| `init` | `{ "path": ".specks/", "files_created": ["specks-skeleton.md", "config.toml", "specks-implementation-log.md"] }` |
| `validate` | `{ "files": [{ "path": "...", "valid": true, "error_count": 0, "warning_count": 0 }] }` |
| `list` | `{ "specks": [{ "name": "...", "status": "...", "progress": { "done": 12, "total": 47 }, "updated": "2026-02-03" }] }` |
| `status` | `{ "name": "...", "status": "...", "progress": {...}, "steps": [{ "title": "...", "anchor": "#step-0", "done": 3, "total": 3, "substeps": [...] }] }` |

---

**Spec S06: `specks beads sync <file>`** {#s06-beads-sync}

| Aspect | Specification |
|--------|---------------|
| Synopsis | `specks beads sync <file> [--dry-run]` |
| Purpose | Create/update a root bead and step beads from the speck, write bead IDs back to speck |
| Precondition | File exists and is a valid speck; beads CLI is installed; `.beads/` directory exists |
| Postcondition | Root bead and step beads created/updated; `**Beads Root:**` and `**Bead:**` lines written to speck file |
| Idempotent | Yes (converges to desired state on re-run) |

**Beads context discovery:**
- Beads operations require `.beads/` to be initialized in the project
- The beads project root is the same directory as the specks project root
- If `.beads/` not found, exit with error E013: "Beads not initialized (run `bd init`)"

**Contract (MUST):**
- **Stable identity**: a step/substep is identified by its **anchor** (e.g., `#step-2`, `#step-2-1`). Titles may change; anchors must not.
- **Single source of truth**: the speck is authoritative for dependencies and (optionally) bead title/body; beads are the execution substrate.
- **Writeback is canonical**: the speck stores root linkage in Plan Metadata `**Beads Root:**` and step linkage in each step’s `**Bead:**` line; tools must update these in place.
- **Safety by default**: by default, sync is additive (creates missing beads, adds missing deps) and does not delete beads or remove deps unless explicitly requested.

**Root bead (invariant):**
- Each speck has exactly one **root bead** (epic/molecule). It groups all step beads as children and enables `bd ready --parent <root-id>` workflows.
- Root bead is stored in Plan Metadata as a new row: `**Beads Root:** \`<bead-id>\`` (or equivalent table cell). Written by sync; optional until first sync.
- Root bead is created as `bd create --type epic --json` (or configurable `root_issue_type` in config), title = phase title from the speck, description = stable header (see below).
- If Plan Metadata already has a Beads Root ID, verify it exists via `bd show <id> --json`; if missing/deleted, create a new root bead and update the speck.

**Behavior (converge, don't skip):**

1. **Ensure root bead exists:**
   - If no `**Beads Root:**` in Plan Metadata: create root via `bd create --type epic --json` with title = phase title, description = `Specks: <speck-path>` (and optional commit/scope). Write `**Beads Root:** \`<id>\`` to Plan Metadata.
   - If root ID present: verify via `bd show <id> --json`; if not found, create new root and replace ID in speck.

2. **For each step in the speck:**
   - **If step has no bead ID:** Create bead via `bd create --parent <root-id> --json` with title = step title, description = stable header (see below) + human-friendly summary. Write `**Bead:** \`<bead-id>\`` to the step. Parse `bd create --json` output as a **single JSON object** (Issue).
   - **If step already has bead ID:** Verify bead exists via `bd show <id> --json`. **CLI JSON shape:** `bd show` may return a **JSON array** of IssueDetails (one element) or a single object; Specks MUST accept both (normalize to one issue). If missing/deleted: create new bead with `--parent <root-id>` and replace `**Bead:**` in speck. With `--update-title`/`--update-body`, update bead; otherwise leave as-is. Reconcile dependencies (see below).

3. **Dependency edges:**
   - Desired deps come from the step’s `**Depends on:**` anchors. For each dep anchor, resolve to bead ID (must exist after step pass).
   - Add missing deps: `bd dep add <this-bead> <dep-bead>` (type `blocks`).
   - **Reconciliation:** Use `bd dep list <this-bead> --json` to obtain current direct dependencies (returns a **JSON array** of IssueWithDependencyMetadata). If `--prune-deps` is set, remove edges present in beads but not in speck via `bd dep remove <this-bead> <dep-bead>`.

**Substep handling (LOCKED):**
- Default is **no substep beads**. Substeps are reflected in the parent step bead’s description as a checklist/outline.
- Optional: `--substeps children` creates one bead per substep via `bd create --parent <step-bead-id> --json`. Child IDs are assigned by beads; write them back to each substep’s `**Bead:**` line. Substep dependencies: if no explicit `**Depends on:**`, inherit parent step’s deps.

**Bead line placement in speck file:**
- **Beads Root:** In Plan Metadata table, add row "Beads Root" with value `\`<bead-id>\`` (or a dedicated `**Beads Root:**` line after the table if preferred; specks implementation chooses one canonical place).
- **Per-step Bead:** After `**Depends on:**` (if present), before `**Commit:**`; else after step heading, before `**Commit:**`. Format: `**Bead:** \`<bead-id>\`` (backticks). Update in place if line exists.

**Bead description header (MUST be present on created beads):**
- First lines of description: `Specks: <speck-path>#<step-anchor>` (or for root: `Specks: <speck-path>`), `Commit: <planned-commit-message>`, `Depends on: <comma-separated step anchors>` (step anchors only).

**Beads JSON Contract:** See (§1.0.1.6 Beads JSON Contract). Specks and the mock-bd test harness MUST conform to that contract.

**Options:**
- `--dry-run`: Show what would be created/updated without making changes
- `--update-title`: Update bead titles for already-linked steps
- `--update-body`: Update bead descriptions for already-linked steps
- `--prune-deps`: Remove beads deps not present in the speck (destructive; use with care)
- `--substeps <mode>`: `none` (default) or `children`

**Exit codes:**
- 0: Success (beads created/updated and IDs written)
- 1: Some bead operations failed
- 2: File not found or unreadable
- 5: Beads CLI not installed
- 13: Beads not initialized (E013)

---

**Spec S07: `specks beads link <file> <step-anchor> <bead-id>`** {#s07-beads-link}

| Aspect | Specification |
|--------|---------------|
| Synopsis | `specks beads link <file> <step-anchor> <bead-id>` |
| Purpose | Manually link an existing bead to a step |
| Precondition | File exists; step anchor exists; bead ID format is valid |
| Postcondition | `**Bead:**` line written to specified step |
| Idempotent | Yes (overwrites existing bead ID) |

**Behavior:**

- **Bead ID validation:** When beads integration is enabled and `validate_bead_ids` is true, prefer validating by calling `bd show <id> --json` (parse array or single object; success => valid). Fallback: regex format `^[a-z0-9][a-z0-9-]*-[a-z0-9]+(\.[0-9]+)*$` (allows variable-length prefix, e.g. `bd-abc12` or `bd-xyz.1`).
- Validates step anchor exists in the speck file.
- Writes the `**Bead:** \`<bead-id>\`` line to the specified step (canonical placement: after `**Depends on:**`, before `**Commit:**`).

**Exit codes:**
- 0: Success
- 1: Invalid bead ID format or bead not found
- 2: File not found or step anchor not found

---

**Spec S08: `specks beads status [file]`** {#s08-beads-status}

| Aspect | Specification |
|--------|---------------|
| Synopsis | `specks beads status [file] [--json]` |
| Purpose | Show execution status from beads aligned with speck steps |
| Precondition | Beads CLI is installed |
| Postcondition | None (read-only) |
| Default | Show status for all specks if no file specified |

**Behavior:**

- For each step/substep in the speck, reads its `**Bead:**` linkage (if any). Optionally show root bead status if `**Beads Root:**` is set.
- Queries beads via `bd show <id> --json`. **CLI JSON:** response may be a **JSON array** of IssueDetails (one element) or a single IssueDetails object; Specks MUST accept both (normalize to one issue per ID).
- Computes readiness using the speck dependency graph (and/or dependency data from `bd show` or `bd dep list`):
  - `complete`: bead status is closed
  - `ready`: bead is open and all `**Depends on:**` beads are complete
  - `blocked`: bead is open and at least one dependency bead is not complete (or missing)
  - `pending`: no bead linked yet
- Aligns this status back onto the speck step structure.

**Output format (default):**
```
specks-1.md: 3/6 steps complete

Step 0: Project Setup                    [x] complete (bd-abc123)
Step 1: Core Types                       [x] complete (bd-def456)
Step 2: Validation Engine                [ ] ready    (bd-ghi789)
Step 3: CLI Framework                    [ ] blocked  (bd-jkl012) <- waiting on bd-ghi789
Step 4: Agent Refinement                 [ ] blocked  (bd-mno345)
Step 5: Documentation                    [ ] pending  (no bead)
```

**Status values:**
- `complete`: Bead is done
- `ready`: All dependencies satisfied, can start
- `blocked`: Waiting on dependencies
- `pending`: No bead linked yet

**Exit codes:**
- 0: Success
- 2: File not found
- 5: Beads CLI not installed

---

**Spec S09: `specks beads pull [file]`** {#s09-beads-pull}

| Aspect | Specification |
|--------|---------------|
| Synopsis | `specks beads pull [file] [--json]` |
| Purpose | Update speck checkboxes based on bead completion status |
| Precondition | Beads CLI is installed; `.beads/` directory exists |
| Postcondition | Speck checkboxes updated to match bead completion states |
| Default | Pull for all specks if no file specified |

**Behavior:**

For each step/substep with a linked bead:
1. Query bead completion status via `bd show <id> --json`. Parse response as array or single object (see Beads JSON Contract); treat bead as complete if `status` is `closed`.
2. If bead is marked complete:
   - By default, check **all** checkbox items under that step’s `**Checkpoint:**` section
   - (Configurable) optionally check all checkboxes under the step (Tasks/Tests/Checkpoints)
3. If bead is not complete but step checkbox is checked:
   - Optionally warn about inconsistency (checkbox ahead of bead)

**Checkbox update rules:**
- Only update checkboxes in "Checkpoint" sections (not Tasks or Tests)
- OR update all checkboxes for the step (configurable via `config.toml`)
- Preserve manual checkbox state if `--no-overwrite` flag is used

**Output format (default):**
```
specks-1.md: 2 checkboxes updated
  Step 2: Validation Engine - marked complete
  Step 3.1: CLI Parsing - marked complete
```

**Alternative: `--pull` flag on status command:**
Instead of a separate command, this behavior can be triggered via:
`specks beads status [file] --pull`

**Exit codes:**
- 0: Success (checkboxes updated or already in sync)
- 1: Some updates failed
- 2: File not found
- 5: Beads CLI not installed
- 13: Beads not initialized (E013)

---

**Spec S10: Director Execution Protocol** {#s10-director-protocol}

| Aspect | Specification |
|--------|---------------|
| Synopsis | Orchestrate execution of an approved speck via the agent suite and beads |
| Input | Speck file path, commit-policy, checkpoint-mode (see D16) |
| Output | Implemented steps, updated checkboxes, closed beads, run reports |
| Checkpoint mode | `step` (default), `milestone`, `continuous` |

**Preconditions:**
- Speck file exists and passes `specks validate`
- Speck metadata Status = "active"
- Beads have been synced (`**Beads Root:**` exists in Plan Metadata)
- All agent definitions available in `agents/specks-*.md`

**Execution loop (per step):**

```
1. DIRECTOR creates run directory: .specks/runs/{uuid}/
2. DIRECTOR writes invocation.json with parameters
3. DIRECTOR validates speck (status = active, Beads Root exists)
4. DIRECTOR builds bead→step map from speck's **Bead:** lines
5. DIRECTOR queries ready steps: `bd ready --parent <root-bead-id> --json`
6. DIRECTOR sorts ready beads by speck dependency graph
7. FOR each ready bead (in topological order):
   a. DIRECTOR invokes ARCHITECT with step
      → Receives implementation strategy, writes to architect-plan.md
   b. DIRECTOR spawns IMPLEMENTER (background) + MONITOR (parallel)
      - IMPLEMENTER writes code, runs tests, checks task boxes
      - MONITOR polls for changes, evaluates against plan
      - IF MONITOR signals HALT: DIRECTOR stops, escalates per C07
   c. DIRECTOR invokes REVIEWER + AUDITOR
      → Both write reports; DIRECTOR synthesizes
   d. IF issues found: escalate per C07 (back to architect/planner/implementer)
   e. IF clean: DIRECTOR invokes LOGGER (writes to implementation log)
   f. DIRECTOR invokes COMMITTER
      - IF commit-policy=manual: writes message, DIRECTOR pauses for user
      - IF commit-policy=auto: writes message AND commits
   g. DIRECTOR closes bead: `bd close <step-bead-id> --reason "Completed"`
   h. DIRECTOR syncs state: `bd sync`
8. REPEAT until `bd ready` returns no more steps
9. DIRECTOR updates speck metadata Status to "done" when all steps complete
10. DIRECTOR writes status.json with final outcome
```

**Bead ID resolution:** The director constructs a `bead_id → step_anchor` map by parsing each step's `**Bead:**` line from the speck. When `bd ready` returns bead IDs, the director looks up corresponding step anchors.

**Topological ordering:** The director sorts ready beads using the speck's `**Depends on:**` lines. Steps whose dependencies are all complete come first.

**Checkpoint modes:**

| Mode | Behavior | Use case |
|------|----------|----------|
| `step` | Pause after every step (default) | Learning, high-risk changes |
| `milestone` | Pause only at milestone boundaries (M01, M02, etc.) | Trusted implementation |
| `continuous` | No pauses between steps; pause only on error | Well-tested specks, CI scenarios |

**With commit-policy=manual:** Director prompts: "Step N complete. Commit and type 'done' (or 'skip' / 'abort'):" and waits for user signal.

**With commit-policy=auto:** Director proceeds immediately after committer commits. User can still abort via interrupt.

**Error handling:**

| Error Type | Escalation |
|------------|------------|
| Implementer failure | Back to architect (design issue) or retry (transient) |
| Monitor HALT | Per drift severity: architect, planner, or implementer |
| Reviewer failure | Back to architect or planner (spec adherence) |
| Auditor failure | Back to implementer (quality fix) |
| Bead not found (E015) | Log, suggest re-running `specks beads sync` |
| Bead already closed | Log info, skip to next step |

**Context provision:**
Before invoking implementer, director provides (via architect):
- Full speck content for overall context
- Step-specific: title, anchor, References, Tasks, Tests, Checkpoints
- Architect's implementation strategy
- All referenced material (design decisions, specs, external files)
- Previous step context (what was implemented in dependencies)

**Run persistence:** All agent reports are written to `.specks/runs/{uuid}/` per D15. This provides an audit trail and enables debugging/replay.

---

#### 1.0.1.4 Error and Warning Model {#errors-warnings}

**Table T01: Error Codes** {#t01-error-codes}

| Code | Severity | Message |
|------|----------|---------|
| E001 | error | Missing required section: {section} |
| E002 | error | Missing or empty required metadata field: {field} |
| E003 | error | Invalid metadata Status value: {value} (must be draft/active/done) |
| E004 | error | Step missing References line |
| E005 | error | Invalid anchor format: {anchor} |
| E006 | error | Duplicate anchor: {anchor} |
| E009 | error | .specks directory not initialized |
| E010 | error | Dependency references non-existent step anchor: {anchor} |
| E011 | error | Circular dependency detected: {cycle} |
| E012 | error | Invalid bead ID format: {id} |
| E013 | error | Beads not initialized in project (run `bd init`) |
| E014 | error | Beads Root bead does not exist: {id} |
| E015 | error | Step bead does not exist: {id} (step anchor: {anchor}) |
| W001 | warning | Decision missing status |
| W002 | warning | Question missing resolution |
| W003 | warning | Step missing checkpoint items |
| W004 | warning | Step missing test items |
| W005 | warning | Reference to non-existent anchor: {anchor} |
| W006 | warning | Unfilled placeholder in metadata: {field} contains {value} |
| W007 | warning | Step (other than Step 0) has no dependencies |
| W008 | warning | Bead ID present but beads integration not enabled |
| I001 | info | Document exceeds recommended size |
| I002 | info | Deep dives exceed 50% of document |

**Exit codes:**
| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Validation errors found |
| 2 | File not found or unreadable |
| 3 | Feature not implemented |
| 4 | Configuration error |
| 5 | Beads CLI not installed |
| 13 | Beads not initialized |

---

#### 1.0.1.5 Configuration Schema {#config-schema}

**Config file:** `.specks/config.toml`

```toml
[specks]
# Validation strictness: "lenient", "normal", "strict"
validation_level = "normal"

# Include info-level messages in validation output
show_info = false

[specks.naming]
# Speck file prefix (default: "specks-")
prefix = "specks-"

# Allowed name pattern (regex)
name_pattern = "^[a-z][a-z0-9-]{1,49}$"

[specks.beads]
# Enable beads integration
enabled = true  # Beads is core functionality

# Validate bead IDs when present
validate_bead_ids = true

# Path to beads CLI binary (default: "bd" on PATH)
bd_path = "bd"

# Sync behavior defaults (safe, non-destructive)
update_title = false
update_body = false
prune_deps = false

# Root bead type (epic recommended for bd ready --parent)
root_issue_type = "epic"

# Substep mapping: "none" (default) or "children"
substeps = "none"

# Pull behavior: which checkboxes to update when a bead is complete
# - "checkpoints": update only **Checkpoint:** items (default)
# - "all": update Tasks/Tests/Checkpoints
pull_checkbox_mode = "checkpoints"

# Warn when checkboxes and bead status disagree
pull_warn_on_conflict = true
```

---

#### 1.0.1.6 Beads JSON Contract (Normative) {#beads-json-contract-normative}

All Specks code that invokes `bd` with `--json`, and any mock/fake `bd` used in tests, MUST conform to this contract. It defines the minimal Beads CLI JSON shapes Specks depends on.

**Issue (from `bd create --json`):** Single JSON object.
- **Required fields Specks reads:** `id`, `title`, `status`, `priority`, `issue_type`
- **Optional fields Specks writes/reads:** `description`

**IssueDetails (from `bd show <id> --json`):** Response MAY be a **JSON array** of one element or a **single object**. Specks MUST accept both; normalize to one issue per ID.
- **Required fields Specks reads:** `id`, `title`, `status`, `priority`, `issue_type`
- **Required for dependency reconciliation:** `dependencies` array; each element: `id`, `dependency_type`
- **Optional:** `dependents`, `description`

**IssueWithDependencyMetadata (from `bd dep list <id> --json`):** JSON array of direct dependencies/dependents.
- **Required fields Specks reads:** `id`, `dependency_type` (and optionally `title`, `status` for display)

**bd dep add / bd dep remove --json:** Small object with at least `status`; may include `issue_id`, `depends_on_id`, `type` for success/failure.

**ReadyIssue (from `bd ready [--parent <id>] --json`):** JSON array of issues available for work.
- **Required fields Specks reads:** `id`, `title`, `status`, `priority`
- **Output shape:** Array of Issue objects (may be empty if no work is ready)
- **Example:**
  ```json
  [
    {"id": "bd-fake-1.2", "title": "Step 2: Validation Engine", "status": "open", "priority": 1},
    {"id": "bd-fake-1.3", "title": "Step 3: CLI Framework", "status": "open", "priority": 1}
  ]
  ```

**bd close <id> [--reason "..."]:** Closes an issue (sets status to "closed"). No `--json` flag required; command succeeds silently or returns error. When used with `--json`, returns small object with `status` field.

**bd sync:** Flushes database state to JSONL, commits if daemon enabled. No output on success. Used to ensure state is visible to other worktrees/agents.

**Parsing rules:**
- When parsing `bd show` output: if value is array, use first element; if object, use as-is.
- Only rely on the fields listed above; other fields may be present but must be ignored for contract compliance.
- All bead IDs must match format `^[a-z0-9][a-z0-9-]*-[a-z0-9]+(\.[0-9]+)*$` for regex validation; when Beads is available, existence can be checked via `bd show <id> --json`.

---

### 1.0.2 Symbol Inventory {#symbol-inventory}

#### 1.0.2.1 New crates {#new-crates}

| Crate | Purpose |
|-------|---------|
| `specks` | Main CLI binary crate |
| `specks-core` | Core library (parsing, validation, types) |

---

#### 1.0.2.2 New files {#new-files}

| File | Purpose |
|------|---------|
| `Cargo.toml` | Workspace manifest |
| `crates/specks/Cargo.toml` | CLI crate manifest |
| `crates/specks/src/main.rs` | CLI entry point |
| `crates/specks/src/cli.rs` | clap command definitions |
| `crates/specks/src/output.rs` | JSON output formatting (Spec S05) |
| `crates/specks/src/commands/mod.rs` | Command implementations |
| `crates/specks/src/commands/init.rs` | Init command |
| `crates/specks/src/commands/validate.rs` | Validate command |
| `crates/specks/src/commands/list.rs` | List command |
| `crates/specks/src/commands/status.rs` | Status command |
| `crates/specks-core/Cargo.toml` | Core library manifest |
| `crates/specks-core/src/lib.rs` | Library entry point |
| `crates/specks-core/src/parser.rs` | Speck parsing |
| `crates/specks-core/src/validator.rs` | Validation logic |
| `crates/specks-core/src/types.rs` | Core data types |
| `crates/specks-core/src/config.rs` | Configuration handling |
| `crates/specks-core/src/error.rs` | Error types |
| `agents/specks-director.md` | Director agent: orchestrates all other agents |
| `agents/specks-planner.md` | Planner agent: idea → structured plan |
| `agents/specks-architect.md` | Architect agent: step → implementation strategy |
| `agents/specks-implementer.md` | Implementer agent: writes code |
| `agents/specks-monitor.md` | Monitor agent: detects drift, signals halt |
| `agents/specks-reviewer.md` | Reviewer agent: checks plan adherence |
| `agents/specks-auditor.md` | Auditor agent: checks code quality |
| `agents/specks-logger.md` | Logger agent: writes change log entries |
| `agents/specks-committer.md` | Committer agent: handles commits |
| `tests/fixtures/` | Test fixture specks |
| `crates/specks/src/commands/beads/mod.rs` | Beads subcommand module |
| `crates/specks/src/commands/beads/sync.rs` | Sync command |
| `crates/specks/src/commands/beads/link.rs` | Link command |
| `crates/specks/src/commands/beads/status.rs` | Status command |
| `crates/specks/src/commands/beads/pull.rs` | Pull command (bead completion -> checkboxes) |
| `tests/bin/bd-fake` | Mock beads CLI for deterministic CI tests |
| `tests/bin/bd-fake-state.json` | State storage for mock-bd (temp, per-test) |

---

#### 1.0.2.3 Symbols to add {#symbols}

| Symbol | Kind | Location | Notes |
|--------|------|----------|-------|
| `Cli` | struct | `cli.rs` | Top-level clap command |
| `Commands` | enum | `cli.rs` | Subcommand variants |
| `InitCommand` | struct | `commands/init.rs` | Init args |
| `ValidateCommand` | struct | `commands/validate.rs` | Validate args |
| `ListCommand` | struct | `commands/list.rs` | List args |
| `StatusCommand` | struct | `commands/status.rs` | Status args |
| `Speck` | struct | `types.rs` | Parsed speck document |
| `SpeckMetadata` | struct | `types.rs` | Plan metadata section; includes optional `beads_root_id` (from **Beads Root:**) |
| `Step` | struct | `types.rs` | Execution step. Fields include: `depends_on: Vec<String>` (step anchor refs from **Depends on:** line), `bead_id: Option<String>` (from **Bead:** line if present) |
| `Substep` | struct | `types.rs` | Nested substep. Default: no bead; optional: child bead when `specks beads sync --substeps children` is used |
| `Checkpoint` | struct | `types.rs` | Checkbox item |
| `ValidationResult` | struct | `validator.rs` | Validation output |
| `ValidationIssue` | struct | `validator.rs` | Single issue |
| `Severity` | enum | `validator.rs` | Error/Warning/Info |
| `Config` | struct | `config.rs` | Configuration |
| `SpecksError` | enum | `error.rs` | Error variants |
| `JsonResponse` | struct | `output.rs` | Shared JSON envelope |
| `parse_speck` | fn | `parser.rs` | Parse speck file |
| `validate_speck` | fn | `validator.rs` | Validate parsed speck |
| `find_specks` | fn | `lib.rs` | Find all speck files |
| `find_project_root` | fn | `lib.rs` | Upward search for `.specks/` |
| `is_reserved_file` | fn | `lib.rs` | Check if filename is reserved |
| `BeadsCommands` | enum | `commands/beads/mod.rs` | Beads subcommand variants |
| `SyncCommand` | struct | `commands/beads/sync.rs` | Sync command args |
| `LinkCommand` | struct | `commands/beads/link.rs` | Link command args |
| `BeadsStatusCommand` | struct | `commands/beads/status.rs` | Beads status command args |
| `sync_to_beads` | fn | `commands/beads/sync.rs` | Create beads from steps |
| `link_step_to_bead` | fn | `commands/beads/link.rs` | Link step to existing bead |
| `get_beads_status` | fn | `commands/beads/status.rs` | Query beads execution status |
| `PullCommand` | struct | `commands/beads/pull.rs` | Pull command args |
| `pull_bead_status` | fn | `commands/beads/pull.rs` | Update checkboxes from bead completion |
| `specks-director` | agent | `agents/specks-director.md` | Central orchestrator for all specks work |
| `specks-planner` | agent | `agents/specks-planner.md` | Creates plans from ideas |
| `specks-architect` | agent | `agents/specks-architect.md` | Creates implementation strategies |
| `specks-implementer` | agent | `agents/specks-implementer.md` | Writes code |
| `specks-monitor` | agent | `agents/specks-monitor.md` | Monitors for drift during implementation |
| `specks-reviewer` | agent | `agents/specks-reviewer.md` | Checks plan adherence |
| `specks-auditor` | agent | `agents/specks-auditor.md` | Checks code quality |
| `specks-logger` | agent | `agents/specks-logger.md` | Writes change log entries |
| `specks-committer` | agent | `agents/specks-committer.md` | Handles commits |

---

### 1.0.3 Documentation Plan {#documentation-plan}

- [ ] README.md with installation and quick start
- [ ] `specks --help` comprehensive help text
- [ ] Skeleton format documentation (inline comments explaining each section)
- [ ] Agent suite instructions and examples (director, planner, architect, implementer, monitor, reviewer, auditor, logger, committer)
- [ ] CLAUDE.md section for specks conventions
- [ ] Example specks in repository

---

### 1.0.4 Test Plan Concepts {#test-plan-concepts}

#### Test Categories {#test-categories}

| Category | Purpose | When to use |
|----------|---------|-------------|
| **Unit** | Test parsing, validation rules in isolation | Core logic, edge cases |
| **Integration** | Test CLI commands end-to-end | Command behavior, file operations |
| **Golden** | Compare validation output against snapshots | Parser output, error messages |
| **Agent** | Verify agent-produced specks pass validation | Agent quality assurance |

---

#### Test Fixtures {#test-fixtures}

**Fixture Directory Structure:**

```
tests/fixtures/
├── valid/
│   ├── minimal.md          # Smallest valid speck
│   ├── complete.md         # All sections populated
│   └── with-substeps.md    # Nested step structure
├── invalid/
│   ├── missing-metadata.md
│   ├── bad-anchors.md
│   ├── duplicate-anchors.md
│   └── missing-references.md
└── golden/
    ├── minimal.validated.json
    └── complete.status.json
```

---

### 1.0.5 Execution Steps {#execution-steps}

#### Step 0: Project Bootstrap {#step-0}

**Commit:** `feat: initialize specks project structure`

**Depends on:** (none - root step)

**References:** [D01] Rust/clap, [D02] .specks directory, (#scope, #new-crates)

**Artifacts:**
- Cargo workspace with two crates
- Basic project structure
- CI configuration

**Tasks:**
- [x] Create `Cargo.toml` workspace manifest
- [x] Create `crates/specks/` CLI crate with minimal main.rs
- [x] Create `crates/specks-core/` library crate with lib.rs
- [x] Add dependencies: clap, serde, toml, thiserror, anyhow
- [x] Create `.github/workflows/ci.yml` for basic CI
- [x] Add `.gitignore` for Rust projects

**Tests:**
- [x] Unit test: `cargo build` succeeds
- [x] Integration test: `cargo run -- --help` shows usage

**Checkpoint:**
- [x] `cargo build` completes without errors
- [x] `cargo test` passes (empty test suite OK)
- [x] `./target/debug/specks --version` prints version

**Rollback:**
- Delete created files and directories

**Commit after all checkpoints pass.**

---

#### Step 1: Core Types and Parser {#step-1}

**Depends on:** #step-0

**Commit:** `feat(core): add core types and speck parser`

**References:** [D04] Anchor format, [D05] Checkbox tracking, Table T01 error codes, (#symbols, #terminology)

**Artifacts:**
- Core data types in specks-core
- Speck parser implementation
- Error type definitions

**Tasks:**
- [x] Implement `Speck`, `SpeckMetadata`, `Step`, `Substep`, `Checkpoint` structs
- [x] Implement `SpecksError` enum with all error variants
- [x] Implement `parse_speck()` function
- [x] Parse Plan Metadata table (including optional `Beads Root` row)
- [x] Parse section headings with anchors
- [x] Extract execution steps and substeps
- [x] Parse `**Depends on:**` lines from steps (anchor references)
- [x] Parse `**Bead:**` lines from steps (bead ID if present)
- [x] Parse optional `**Beads:**` hints block (type, priority, labels, estimate_minutes)
- [x] Parse checkbox items (Tasks, Tests, Checkpoints)
- [x] Extract References lines from steps

**Tests:**
- [x] Unit test: Parse minimal valid speck
- [x] Unit test: Parse complete speck with all sections
- [x] Unit test: Extract anchors correctly
- [x] Unit test: Parse checkbox states
- [x] Unit test: Parse `**Depends on:**` lines correctly
- [x] Unit test: Parse `**Bead:**` line (extracts bead ID)
- [x] Unit test: Parse `**Beads:**` hints block (type, priority, labels, estimate)
- [x] Unit test: Parse `Beads Root` from Plan Metadata table
- [x] Unit test: Handle malformed markdown gracefully

**Checkpoint:**
- [x] `cargo build -p specks-core` succeeds
- [x] `cargo test -p specks-core` passes
- [x] Parser handles all fixture files without panic

**Rollback:**
- Revert to Step 0 commit

**Commit after all checkpoints pass.**

---

#### Step 2: Validation Engine {#step-2}

**Depends on:** #step-1

**Commit:** `feat(core): implement validation engine`

**References:** List L01 validation rules, Table T01 error codes, (#errors-warnings, #validation-rules)

**Artifacts:**
- Validation logic
- Rule implementations
- Validation result aggregation

**Tasks:**
- [x] Implement `validate_speck()` function
- [x] Implement `ValidationResult` and `ValidationIssue` structs
- [x] Implement `Severity` enum (Error, Warning, Info)
- [x] Implement required section checks (E001)
- [x] Implement metadata field checks (E002, E003)
- [x] Implement step References check (E004)
- [x] Implement anchor format validation (E005)
- [x] Implement duplicate anchor detection (E006)
- [x] Implement warning rules (W001-W006)
- [x] Implement info rules (I001-I002)
- [x] Support validation levels (lenient/normal/strict)
- [x] Implement dependency anchor validation (E010)
- [x] Implement cycle detection algorithm (E011)
- [x] Implement bead ID format validation (E012): regex fallback; when beads enabled, optional `bd show` check
- [x] Implement E014 (Beads Root exists when set), E015 (step bead exists when set) when beads integration enabled
- [x] Implement dependency warning rules (W007-W008)

**Tests:**
- [x] Unit test: Each validation rule in isolation
- [x] Integration test: Validate fixture files
- [x] Golden test: Validation output matches expected for invalid fixtures

**Checkpoint:**
- [x] Valid fixtures pass validation
- [x] Invalid fixtures produce expected errors
- [x] `cargo test -p specks-core` passes

**Rollback:**
- Revert to Step 1 commit

**Commit after all checkpoints pass.**

---

#### Step 3: CLI Framework and Commands {#step-3}

**Depends on:** #step-1, #step-2

**Commit:** `feat(cli): implement CLI with init, validate, list, status commands`

**References:** [D01] Rust/clap, [D07] Root resolution, [D08] JSON schema, Specs S01-S05, Diag01, (#cli-structure)

**Artifacts:**
- clap command structure
- All four commands implemented
- JSON output formatting

**Tasks:**
- [x] Implement `Cli` struct with clap derive
- [x] Implement `Commands` enum with all subcommands
- [x] Add global options (--verbose, --quiet, --json, --version)
- [x] Implement project root discovery (upward search for `.specks/`)
- [x] Implement `specks init` command (Spec S01)
- [x] Implement `specks validate` command (Spec S02)
- [x] Implement `specks list` command (Spec S03)
- [x] Implement `specks status` command (Spec S04)
- [x] Implement JSON output formatting (Spec S05)
- [x] Implement configuration loading

**Tests:**
- [x] Integration test: `specks init` creates expected files
- [x] Integration test: `specks validate` on valid/invalid files
- [x] Integration test: `specks list` shows all specks
- [x] Integration test: `specks status` shows step breakdown
- [x] Integration test: JSON output format for all commands

**Checkpoint:**
- [x] `specks --help` lists all commands
- [x] `specks init` creates .specks/ directory
- [x] `specks validate` catches known errors in test fixtures
- [x] `specks list` shows all specks with accurate progress
- [x] `specks status <file>` shows per-step breakdown
- [x] All commands support --json output

**Rollback:**
- Revert to Step 2 commit

**Commit after all checkpoints pass.**

---

#### Step 4: Director + Planning Agents {#step-4}

**Depends on:** #step-3

**Commit:** `feat(agent): add director, planner, and architect agents`

**References:** [D12] Multi-agent architecture, [D13] Reviewer vs auditor, [D14] Cooperative halt, [D15] Run persistence, [D16] Director invocation, (#c03-agent-suite, #c04-planning-flow, #terminology)

**Artifacts:**
- `agents/specks-director.md` - Central orchestrator agent
- `agents/specks-planner.md` - Idea → structured plan agent
- `agents/specks-architect.md` - Step → implementation strategy agent
- Run directory structure under `.specks/runs/`
- Agent testing workflow

**Tasks:**
- [x] Create `agents/specks-director.md` with:
  - [x] Role: central orchestrator, invokes all other agents
  - [x] Invocation protocol per D16 (speck, mode, commit-policy, etc.)
  - [x] Run persistence setup (create UUID directory, write invocation.json)
  - [x] Planning flow: idea → planner → auditor → approve/revise
  - [x] Execution flow: per-step loop per S10
  - [x] Escalation decision tree per C07
  - [x] Hub-and-spoke principle: all reports flow to director
- [x] Create `agents/specks-planner.md` with:
  - [x] Role: takes idea → structured plan following skeleton format
  - [x] Instructions for codebase exploration
  - [x] Instructions for asking clarifying questions
  - [x] Format compliance requirements (all required sections)
  - [x] Output: complete speck or revision to existing speck
- [x] Create `agents/specks-architect.md` with:
  - [x] Role: takes step → implementation strategy + test plan
  - [x] Design implementation approach
  - [x] Emit machine-readable `expected_touch_set` (files to create/modify/directories)
  - [x] Define unit and integration test strategy
  - [x] Anticipate edge cases and error conditions
  - [x] Output: architect-plan.md written to run directory (includes expected_touch_set block)
- [x] Implement run directory structure (`.specks/runs/{uuid}/`)
- [x] Create test workflow: planner produces speck, validate passes
- [x] Document agent invocation patterns (Claude Code subagent model)

**Tests:**
- [x] Agent test: Planner produces a speck for a simple feature request (verified via Step 9 E2E test)
- [x] Validation test: Planner-produced speck passes `specks validate` (verified via Step 9 E2E test)
- [x] Structure test: Planner-produced speck has all required sections (verified via Step 9 E2E test)
- [x] Agent test: Architect produces implementation strategy for a step (verified via Step 9 E2E test)
- [x] Integration test: Director invokes planner in planning mode (verified via Step 9 E2E test)

**Checkpoint:**
- [x] `agents/specks-director.md` follows agent definition format (C08)
- [x] `agents/specks-planner.md` produces specks that pass validation (agent definition complete; runtime verified in Step 9)
- [x] `agents/specks-architect.md` produces actionable implementation strategies with `expected_touch_set`
- [x] Run directory created correctly with invocation.json (structure in place; runtime verified in Step 9)
- [x] Hub-and-spoke principle documented and clear

**Rollback:**
- Revert agent definitions and remove `.specks/runs/` structure

**Commit after all checkpoints pass.**

---

#### Step 5: Test Fixtures and Documentation {#step-5}

**Depends on:** #step-3, #step-4

**Commit:** `docs: add test fixtures, README, and documentation`

**References:** (#documentation-plan, #test-fixtures)

**Artifacts:**
- Test fixture files
- README.md
- CLI help improvements

**Tasks:**
- [ ] Create tests/fixtures/valid/ directory with valid specks
- [ ] Create tests/fixtures/invalid/ directory with invalid specks
- [ ] Create golden output files for validation
- [ ] Write README.md with installation, usage, agent workflow
- [ ] Review and improve all --help text
- [ ] Add CLAUDE.md section for specks conventions
- [ ] Create example speck demonstrating agent output

**Tests:**
- [ ] Golden tests for all fixtures
- [ ] Integration test: Full workflow (init, validate, list, status)

**Checkpoint:**
- [ ] All fixtures validate as expected
- [ ] README covers all commands and agent workflow
- [ ] `specks --help` is clear and complete
- [ ] Example speck validates successfully

**Rollback:**
- Revert to Step 4 commit

**Commit after all checkpoints pass.**

---

#### Step 6: Beads Integration Commands {#step-6}

**Depends on:** #step-3, #step-4

**Commit:** `feat(cli): implement beads integration commands`

**References:** [D10] Step dependencies, Specs S06-S09, (#cli-structure)

**Artifacts:**
- beads subcommand with sync, link, status, pull
- Bead creation from steps (and optionally substeps)
- Bead ID writeback to speck files
- Two-way sync between beads and checkboxes

**Tasks:**
- [ ] Implement `BeadsCommands` enum and subcommand routing
- [ ] Implement beads context discovery (check for `.beads/` directory)
- [ ] Implement `specks beads sync` command (Spec S06)
  - [ ] Create or verify root bead (`bd create --type epic` or config `root_issue_type`); write `**Beads Root:**` to Plan Metadata
  - [ ] Create step beads as children of root (`bd create --parent <root-id>`)
  - [ ] Optional: create child beads for substeps (`--substeps children`)
  - [ ] Converge existing beads (update title, description, edges); reconcile deps via `bd dep list`
  - [ ] Handle case where bead ID exists but bead was deleted (recreate, replace ID)
  - [ ] Parse `bd create` / `bd show` / `bd dep list` JSON per Beads JSON Contract (array or single object)
- [ ] Implement dependency edge creation via `bd dep add`; use `bd dep list <id> --json` for reconciliation when `--prune-deps`
- [ ] Implement bead ID writeback: Beads Root in Plan Metadata; per-step `**Bead:**` after `**Depends on:**`, before `**Commit:**`; update in place
- [ ] Implement `specks beads link` command (Spec S07)
- [ ] Implement `specks beads status` command (Spec S08)
- [ ] Implement `specks beads pull` command (Spec S09)
  - [ ] Update checkboxes based on bead completion
  - [ ] Support `--pull` flag on status command as alternative
- [ ] Handle beads CLI not installed gracefully (exit code 5)
- [ ] Handle beads not initialized (exit code 13, E013)

**Tests:**
- [ ] Integration test: sync creates beads with correct titles
- [ ] Integration test: sync creates child beads for substeps when enabled
- [ ] Integration test: sync creates dependency edges
- [ ] Integration test: sync writes bead IDs back to file in correct position
- [ ] Integration test: re-running sync converges (idempotent)
- [ ] Integration test: sync handles deleted beads (recreates)
- [ ] Integration test: link updates speck file
- [ ] Integration test: status shows correct bead states
- [ ] Integration test: pull updates checkboxes from bead completion
- [ ] Error test: E013 when `.beads/` not found

**Checkpoint:**
- [ ] `specks beads sync` creates root bead and step beads (and substeps when enabled)
- [ ] `**Beads Root:**` and `**Bead:**` IDs appear in speck file after sync in correct positions
- [ ] Re-running sync converges (idempotent)
- [ ] `specks beads status` shows step/bead alignment; parses `bd show --json` as array or object
- [ ] `specks beads pull` updates checkboxes from bead completion
- [ ] Dependencies in beads match speck dependencies
- [ ] E013 / E014 / E015 validation when beads enabled

**Rollback:**
- Revert to Step 5 commit
- Manually delete created beads if needed

**Commit after all checkpoints pass.**

---

#### Step 6.5: Mock-bd Test Harness {#step-6-5}

**Depends on:** #step-6

**Commit:** `test: add mock-bd harness for deterministic CI`

**References:** Spec S06-S09, (#beads-json-contract-normative)

**Artifacts:**
- Fake `bd` binary for tests
- Deterministic test state storage
- CI-friendly beads integration tests

**Tasks:**
- [ ] Create `tests/bin/bd-fake` (or Rust binary) that implements the Beads JSON Contract subset
- [ ] Implement `bd create --json [--parent <id>] [--type <type>]` → returns `Issue` JSON object
- [ ] Implement `bd show <id> --json` → returns `[IssueDetails]` array (match real bd behavior)
- [ ] Implement `bd dep add <child> <parent> --json` → returns status object
- [ ] Implement `bd dep remove <child> <parent> --json` → returns status object
- [ ] Implement `bd dep list <id> --json` → returns `[IssueWithDependencyMetadata]` array
- [ ] Implement `bd ready [--parent <id>] --json` → returns array of open issues with no unmet deps
- [ ] Implement `bd close <id> [--reason "..."]` → sets issue status to closed
- [ ] Implement `bd sync` → no-op in mock (state already persisted), returns success
- [ ] State storage: JSON file in temp dir (issues + edges), or in-memory per test
- [ ] Deterministic ID generation: counter-based with `--parent` producing `.1`, `.2` suffixes
- [ ] Add config/env override: `SPECKS_BD_PATH` or `bd_path` in config.toml to point to fake
- [ ] Write integration tests using mock-bd that exercise sync/status/pull without network

**Tests:**
- [ ] Mock-bd test: `bd create` returns valid Issue JSON
- [ ] Mock-bd test: `bd show` returns array of IssueDetails
- [ ] Mock-bd test: `bd dep add/list` track dependencies correctly
- [ ] Integration test: sync with mock-bd creates root + step beads, writes IDs back
- [ ] Integration test: re-running sync with mock-bd converges (idempotent)
- [ ] Integration test: dependency edges in mock-bd match speck `**Depends on:**`
- [ ] Integration test: status with mock-bd computes readiness correctly
- [ ] Integration test: pull with mock-bd updates checkboxes

**Checkpoint:**
- [ ] Mock-bd passes all Beads JSON Contract requirements
- [ ] All beads integration tests pass with mock-bd in CI (no network required)
- [ ] Tests are deterministic (no flakiness from external beads state)

**Rollback:**
- Revert to Step 6 commit; remove mock-bd binary and related tests

**Commit after all checkpoints pass.**

---

#### Step 7: Final Documentation {#step-7}

**Depends on:** #step-5, #step-6, #step-6-5

**Commit:** `docs: finalize documentation with beads integration`

**References:** (#documentation-plan)

**Artifacts:**
- Updated README with beads integration
- Complete CLI help for beads commands

**Tasks:**
- [ ] Update README.md with beads integration documentation
- [ ] Add beads workflow examples to documentation
  - [ ] Document sync command (create/update beads)
  - [ ] Document pull command (update checkboxes from bead completion)
  - [ ] Document two-way sync workflow
- [ ] Document beads CLI dependency and `.beads/` requirement
- [ ] Document network requirements (beads commands require connectivity)
- [ ] Review and improve beads command --help text

**Tests:**
- [ ] Documentation review: all beads commands covered (sync, link, status, pull)
- [ ] Integration test: Full workflow including beads sync and pull

**Checkpoint:**
- [ ] README documents beads integration including two-way sync
- [ ] `specks beads --help` is clear and complete
- [ ] Example workflow with beads sync and pull works end-to-end

**Rollback:**
- Revert to Step 6 commit

**Commit after all checkpoints pass.**

---

#### Step 8: Execution Agents {#step-8}

**Depends on:** #step-4, #step-6, #step-7

**Commit:** `feat(agent): add execution agents (implementer, monitor, reviewer, auditor, logger, committer)`

**References:** [D11] Commit policy, [D12] Multi-agent architecture, [D13] Reviewer vs auditor, [D14] Cooperative halt, Spec S10, (#c05-execution-flow, #c06-monitor-protocol, #c07-escalation-paths, #c02-agent-skill-mapping)

**Artifacts:**
- `agents/specks-implementer.md` - Writes code, focused doer
- `agents/specks-monitor.md` - Detects drift, signals halt
- `agents/specks-reviewer.md` - Checks plan adherence
- `agents/specks-auditor.md` - Checks code quality
- `agents/specks-logger.md` - Writes change log entries
- `agents/specks-committer.md` - Handles commits
- Updated director agent with full execution loop
- Integration tests with mock-bd

**Tasks:**
- [ ] Create `agents/specks-implementer.md` with:
  - [ ] Role: writes code following architect's strategy
  - [ ] Invokes implement-plan skill
  - [ ] Checks for halt signal between major operations (D14)
  - [ ] Returns partial status if halted
  - [ ] Checks task boxes in speck as work completes
- [ ] Create `agents/specks-monitor.md` with:
  - [ ] Role: runs parallel to implementer, detects drift
  - [ ] Polls for uncommitted changes at intervals
  - [ ] Compares changed files against architect's `expected_touch_set` for objective drift detection
  - [ ] Evaluates changes against plan and architect strategy
  - [ ] Drift detection criteria per C06
  - [ ] Writes halt signal file if significant drift detected
  - [ ] Returns HALT/CONTINUE status to director
- [ ] Create `agents/specks-reviewer.md` with:
  - [ ] Role: checks plan adherence after step completes
  - [ ] Verifies all tasks from step completed
  - [ ] Verifies tests match test plan
  - [ ] Verifies artifacts produced as expected
  - [ ] Writes reviewer-report.md to run directory
- [ ] Create `agents/specks-auditor.md` with:
  - [ ] Role: checks code quality and performance
  - [ ] Evaluates code structure and maintainability
  - [ ] Identifies security concerns
  - [ ] Checks project conventions
  - [ ] Writes auditor-report.md to run directory
  - [ ] Runs at step, milestone, and completion per D13
- [ ] Create `agents/specks-logger.md` with:
  - [ ] Role: writes change log entries to implementation log
  - [ ] Invokes update-plan-implementation-log skill
  - [ ] Documents what was implemented in this step
- [ ] Create `agents/specks-committer.md` with:
  - [ ] Role: handles commit preparation and (optionally) execution
  - [ ] Invokes prepare-git-commit-message skill
  - [ ] With commit-policy=auto: also performs git commit
  - [ ] With commit-policy=manual: writes message, returns to director
  - [ ] Writes committer-prep.md to run directory
- [ ] Update director agent with full execution loop per S10
- [ ] Implement halt signal file protocol per D14
- [ ] Add integration notes to existing skill documentation

**Tests:**
- [ ] Agent test: Implementer writes code and checks for halt signal
- [ ] Agent test: Monitor detects drift and writes halt signal
- [ ] Agent test: Reviewer produces plan adherence report
- [ ] Agent test: Auditor produces quality report
- [ ] Integration test: Director spawns implementer + monitor in parallel
- [ ] Integration test: Director handles HALT from monitor
- [ ] Integration test: Director invokes reviewer + auditor after step
- [ ] Integration test: Director escalates correctly per C07
- [ ] Integration test: Committer respects commit-policy
- [ ] Integration test: Full step execution with mock-bd

**Checkpoint:**
- [ ] All 6 execution agents follow agent definition format (C08)
- [ ] Monitor halt protocol works (signal file, implementer checks it)
- [ ] Reviewer and auditor produce complementary reports
- [ ] Committer respects commit-policy (manual vs auto)
- [ ] Director orchestrates full execution loop per S10
- [ ] Escalation paths work correctly per C07
- [ ] Run directory contains all expected reports after step

**Rollback:**
- Revert agent definitions and documentation changes

**Commit after all checkpoints pass.**

---

#### Step 9: End-to-End Validation {#step-9}

**Depends on:** #step-8

**Commit:** `test: validate full multi-agent pipeline end-to-end`

**References:** [D12] Multi-agent architecture, [D15] Run persistence, [D16] Director invocation, Spec S10, (#exit-criteria)

**Purpose:** Prove the multi-agent architecture actually works by running a complete cycle: idea → planner → speck → beads sync → director execution → implemented code → closed beads. This is the "turn the key" step.

**Artifacts:**
- A real speck created by the planner agent for a small feature
- Beads created and linked via sync
- Run directory with all agent reports
- Implemented code committed (or ready to commit with `manual` policy)
- Closed beads reflecting completed work

**Test scenario:** Add a simple, self-contained feature to the specks CLI itself. Suggested: `specks version --verbose` that shows build info (commit hash, build date, rust version). Small scope, touches real code, exercises the full loop.

**Tasks:**
- [ ] Invoke director in planning mode with the test idea
- [ ] Verify planner produces a speck that passes `specks validate`
- [ ] Run `specks beads sync` on the new speck
- [ ] Verify root bead and step beads created with correct dependencies
- [ ] Invoke director in execution mode with `commit-policy=manual`
- [ ] Verify for each step:
  - [ ] Architect produces `architect-plan.md` with `expected_touch_set`
  - [ ] Implementer writes code (check for halt signal protocol)
  - [ ] Monitor runs parallel, no false HALT on valid work
  - [ ] Reviewer produces plan-adherence report
  - [ ] Auditor produces quality report
  - [ ] Logger writes to implementation log
  - [ ] Committer prepares commit message
- [ ] Verify run directory contains all expected artifacts:
  - [ ] `invocation.json`
  - [ ] `architect-plan.md`
  - [ ] `monitor-log.jsonl`
  - [ ] `reviewer-report.md`
  - [ ] `auditor-report.md`
  - [ ] `committer-prep.md`
  - [ ] `status.json`
- [ ] Manually commit the changes (since `commit-policy=manual`)
- [ ] Verify beads closed after commit
- [ ] Run `specks beads status` to confirm alignment
- [ ] Document any issues encountered and fixes applied

**Tests:**
- [ ] End-to-end test: Full pipeline completes without manual intervention (except final commit)
- [ ] Validation test: Planner-produced speck passes all validation rules
- [ ] Persistence test: Run directory contains complete audit trail
- [ ] Beads test: All step beads closed after execution

**Checkpoint:**
- [ ] Planner produces valid speck for test feature
- [ ] Director orchestrates full execution loop without errors
- [ ] All 8 specialist agents invoked and produce expected outputs
- [ ] Monitor does not false-HALT on valid implementation
- [ ] Run directory contains complete audit trail
- [ ] Feature code is correct and tests pass
- [ ] Beads reflect execution state accurately
- [ ] No manual fixes required to agent definitions mid-run (if fixes needed, apply and re-run)

**Failure handling:**
If any agent fails or produces incorrect output:
1. Document the failure mode
2. Fix the agent definition
3. Re-run from the failed step
4. Update Step 8 tasks if the fix reveals a gap in the original spec

**Rollback:**
- Revert test feature code
- Delete test speck and associated beads
- Retain run directory for post-mortem if needed

**Commit after all checkpoints pass.**

---

### 1.0.6 Deliverables and Checkpoints {#deliverables}

**Deliverable:** An agent-centric system for creating and managing technical specifications, consisting of a nine-agent suite (director, planner, architect, implementer, monitor, reviewer, auditor, logger, committer) for speck creation and execution, plus CLI utilities for validation and status tracking.

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

- [ ] Planner agent produces specks that pass validation
- [ ] `specks init` creates valid project structure
- [ ] `specks validate` catches all List L01 errors and warnings
- [ ] `specks list` accurately shows all specks with status
- [ ] `specks status <file>` accurately reports checkbox completion
- [ ] All commands support --json for machine-readable output
- [ ] README provides clear installation and agent workflow instructions
- [ ] Test coverage includes all validation rules
- [ ] `specks beads sync` creates beads from steps and substeps with correct dependencies
- [ ] Bead IDs written back to speck after sync in correct position
- [ ] `specks beads status` shows aligned step/bead status
- [ ] `specks beads pull` updates checkboxes from bead completion
- [ ] Re-running sync converges to same state (idempotent)
- [ ] Mock-bd harness enables deterministic CI tests without network
- [ ] All 9 agent definitions complete and follow agent definition format (C08)
- [ ] Director orchestrates full execution loop per S10
- [ ] Monitor halt protocol works (D14)
- [ ] Run persistence captures agent reports (D15)
- [ ] **Step 9 complete**: End-to-end workflow proven on real feature (idea → planner → speck → beads sync → director → implemented code → closed beads)

**Acceptance tests:**
- [ ] Integration test: Full workflow (init, validate, list, status)
- [ ] Agent test: Planner produces valid, comprehensive specks
- [ ] Agent test: Director orchestrates full step execution
- [ ] Golden test: All fixtures produce expected output
- [ ] **E2E test (Step 9):** Complete pipeline on real feature with all 9 agents

#### Milestones {#milestones}

**Milestone M01: Core Infrastructure** {#m01-core-infra}
- [ ] Steps 0-2 complete (project structure, parser, validator)

**Milestone M02: CLI Complete** {#m02-cli-complete}
- [ ] Step 3 complete (all commands implemented)

**Milestone M03: Planning Agents** {#m03-planning-agents}
- [ ] Step 4 complete (director, planner, architect agents)
- [ ] Run persistence structure in place

**Milestone M04: Beads Integration** {#m04-beads-integration}
- [ ] Step 6 complete (beads commands work)
- [ ] Step 6.5 complete (mock-bd harness for CI)

**Milestone M05: Documentation** {#m05-docs}
- [ ] Steps 5, 7 complete (fixtures, docs complete)

**Milestone M06: Execution Agents** {#m06-execution-agents}
- [ ] Step 8 complete (implementer, monitor, reviewer, auditor, logger, committer)
- [ ] Full execution loop working per S10

**Milestone M07: End-to-End Validation** {#m07-e2e-validation}
- [ ] Step 9 complete (full pipeline exercised on real feature)
- [ ] End-to-end workflow proven: idea → planner → speck → beads → director → implemented code
- [ ] All agent definitions validated through actual use
- [ ] Integration with skills verified (implement-plan, update-log, prepare-commit)
- [ ] Run persistence captures complete audit trail
- [ ] **Phase 1 complete**

#### Roadmap / Follow-ons (Phase 2+) {#roadmap}

- [ ] **Enhanced commit policies**: `auto-on-green` (commit if tests pass), `batch` (commit at milestones)
- [ ] **Rollback on failure**: Automatic git reset when step fails after partial commits
- [ ] **Programmatic agent cancellation**: If Claude Code adds a documented API/tool for terminating running subagents, upgrade from cooperative halt + interactive cancellation
- [ ] `specks new` minimal scaffold command (if needed)
- [ ] MCP server for specks operations
- [ ] Speck format versioning and migration
- [ ] Editor integrations (VS Code extension)
- [ ] Pre-commit hook for validation
- [ ] Parallel multi-agent execution across worktrees with coordination protocol
- [ ] Run replay: re-execute a run from persisted state

| Checkpoint | Verification |
|------------|--------------|
| Binary builds | `cargo build --release` succeeds |
| Tests pass | `cargo test` all green |
| Commands work | Manual test of each command |
| Agent works | Agent produces valid speck |
| Docs complete | README covers all features and agent workflow |

**Commit after all checkpoints pass.**
