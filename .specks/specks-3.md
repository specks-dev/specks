## Phase 3.0: Claude Code Plugin Architecture {#phase-3}

**Purpose:** Restructure specks as a Claude Code plugin for clean distribution, eliminating custom installation code while enabling "specks to develop specks" via `--plugin-dir`.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | specks-team |
| Status | draft |
| Target branch | main |
| Tracking issue/PR | - |
| Last updated | 2026-02-06 |

---

### Agents and Skills Summary {#agents-skills-summary}

This phase defines 5 agents and 8 skills. Use this table as a quick reference.

#### Agents {#agent-summary}

| Agent | Remit |
|-------|-------|
| **director** | Pure orchestrator. Coordinates workflow via Task (spawn agents) and Skill (invoke analysis) tools. Writes only audit trail files (run directory). Never edits code or interacts with users directly. |
| **planner** | Creates and revises speck documents. Writes structured markdown plans to `.specks/`. Receives user decisions from director, not directly. |
| **interviewer** | Single point of user interaction. Presents clarifying questions and critic feedback via AskUserQuestion. Returns structured decisions to director. |
| **architect** | Creates implementation strategies for individual steps. Returns JSON with expected touch set. Read-only analysis, no file writes. |
| **implementer** | Executes architect strategies with self-monitoring for drift. Writes code, runs tests, creates artifacts. Self-halts when drift thresholds exceeded. |

#### Skills {#skill-summary}

| Skill | Spec | Remit |
|-------|------|-------|
| **plan** | S01 | Entry point. User invokes `/specks:plan`. Spawns director with mode=plan. |
| **execute** | S02 | Entry point. User invokes `/specks:execute`. Spawns director with mode=execute. |
| **clarifier** | S03 | Analyzes idea or critic feedback. Returns clarifying questions with options. JSON output. |
| **critic** | S04 | Reviews speck for skeleton compliance, completeness, implementability. Returns APPROVE/REVISE/REJECT. JSON output. |
| **reviewer** | S05 | Verifies completed step matches plan. Checks tasks, tests, artifacts. Returns APPROVE/REVISE/ESCALATE. JSON output. |
| **auditor** | S06 | Checks code quality, error handling, security. Returns severity-ranked issues. JSON output. |
| **logger** | S07 | Updates `.specks/specks-implementation-log.md` with completed work. JSON output. |
| **committer** | S08 | Finalizes step: stages files, commits changes, closes associated bead. JSON output. |

---

### Phase Overview {#phase-overview}

#### Context {#context}

The current architecture has multiple problems:

1. **Competing orchestration layers**: Rust CLI orchestrates agents by shelling out to `claude`, while the director agent was designed to orchestrate via Task tool but is never invoked. This creates 6+ minute execution times.

2. **Custom distribution complexity**: The plan was to use `share/specks/skills/` and `share/specks/agents/` with a custom `specks setup claude` command to install to `.claude/`. This reinvents what Claude Code plugins already do.

3. **Path confusion**: Skills and agents have multiple locations (share/, .claude/, agents/) creating confusion about source of truth.

**The solution is simple: Make specks a Claude Code plugin.**

From the Claude Code docs:
> "Plugins let you extend Claude Code with custom functionality that can be shared across projects and teams."

A plugin provides:
- `skills/` directory at repo root - automatically discovered
- `agents/` directory at repo root - automatically discovered
- `.claude-plugin/plugin.json` - metadata and versioning
- Namespacing: `/specks:plan`, `specks:director`, etc.
- Distribution via marketplaces or `--plugin-dir`

This is a brand new library with ZERO external users. No deprecation, no migration, clean breaks only.

#### Strategy {#strategy}

See (#agents-skills-summary) for the complete list of what we're building.

- Convert specks repo to a Claude Code plugin structure
- Move skills from `.claude/skills/` to `skills/` at repo root
- Keep agents at `agents/` (already correct location for plugins!)
- Add `.claude-plugin/plugin.json` manifest
- Remove `specks setup claude` command (plugins handle their own installation)
- Remove all `share/specks/` distribution complexity
- Convert read-only agents (clarifier, critic, etc.) to skills per (#skill-summary)
- Keep agents that need isolated context per (#agent-summary)
- Remove Rust orchestration code (plan, execute commands)
- CLI becomes thin utility (init, validate, list, status, beads)

#### Using Specks to Develop Specks {#dogfooding}

**During development:**
```bash
cd /path/to/specks
claude --plugin-dir .
```
This loads the repo as a plugin. All skills and agents are available immediately.

**For users (future):**
```bash
# Add specks marketplace to settings
claude plugin install specks
```

#### Stakeholders / Primary Customers {#stakeholders}

1. Specks developers using Claude Code for planning and execution
2. Future specks users who will install via plugin marketplace

#### Success Criteria (Measurable) {#success-criteria}

See (#agents-skills-summary) for the complete list of agents and skills.

- `.claude-plugin/plugin.json` exists with valid manifest
- `skills/` directory at repo root contains 8 skills per (#skill-summary)
- `agents/` directory at repo root contains 5 agents per (#agent-summary)
- `claude --plugin-dir .` loads specks as a plugin with all skills/agents available
- `/specks:plan` and `/specks:execute` skills work
- `specks:director` agent can be invoked via Task tool
- `specks plan` and `specks execute` CLI commands removed
- `specks setup claude` command removed
- `.claude/skills/` directory removed (replaced by `skills/`)
- `cargo build` succeeds with no warnings
- `cargo nextest run` passes all tests

#### Scope {#scope}

1. Create `.claude-plugin/plugin.json` manifest
2. Create `skills/` directory with 6 new skills (clarifier, critic, reviewer, auditor, logger, committer)
3. Move entry point skills from `.claude/skills/` to `skills/` (plan, execute)
4. Update agent definitions at `agents/` (keep 5, remove 7)
5. Remove Rust orchestration code (plan.rs, execute.rs, planning_loop/, streaming.rs, interaction/)
6. Remove `specks setup claude` command and share.rs
7. Remove `.claude/skills/` directory (legacy skills)
8. Update documentation

#### Non-goals (Explicitly out of scope) {#non-goals}

- Creating a public plugin marketplace (future work)
- Adding new features beyond architecture simplification
- Changing the speck file format
- Modifying beads integration beyond validation, error handling, and onboarding docs
- Testing the full planning/execution loop (Phase 4)

---

#### Beads Integration Contract (Plugin Mode) {#beads-contract}

**Problem:** Beads integration works via CLI, but agents in plugin mode may have different PATH/env than a terminal shell. This section defines the contract to prevent silent failures.

**Invocation method:** Agents call `specks beads ... --json` via Bash, NOT `bd` directly. No fire-and-forget; all calls must capture and validate output.

```bash
# Correct - uses CLI discovery and JSON output
specks beads status specks-3.md --json

# Wrong - bypasses config, may fail if bd not on PATH
bd show bd-123 --json
```

**Discovery chain (implemented in CLI):**
1. `SPECKS_BD_PATH` environment variable (highest priority)
2. `config.specks.beads.bd_path` from `.specks/config.toml`
3. Default `"bd"` (expects `bd` on PATH)

---

##### Beads JSON Contract {#beads-json-contract}

All `specks beads` subcommands support `--json` and return a standard envelope:

```json
{
  "schema_version": "1",
  "command": "beads <subcommand>",
  "status": "ok" | "error",
  "data": { /* command-specific payload */ },
  "issues": [ /* error/warning objects */ ]
}
```

**Issue object schema:**
```json
{
  "code": "E005",
  "severity": "error" | "warning",
  "message": "human-readable description",
  "file": ".specks/specks-3.md",  // optional
  "line": 42,                      // optional
  "anchor": "#step-0"              // optional
}
```

---

**Command: `specks beads status [file] --json`**

Returns completion status for each step based on linked beads.

```json
{
  "files": [
    {
      "file": ".specks/specks-3.md",
      "name": "3",
      "root_bead_id": "bd-abc123",
      "steps_complete": 2,
      "steps_total": 5,
      "steps": [
        {
          "anchor": "step-0",
          "title": "Step 0: Create plugin manifest",
          "bead_id": "bd-abc123.1",
          "status": "complete" | "ready" | "blocked" | "pending",
          "blocked_by": ["bd-abc123.0"]
        }
      ]
    }
  ]
}
```

**Status values:**
- `complete`: bead is closed (work done)
- `ready`: bead is open, all dependencies complete
- `blocked`: waiting on dependencies to complete
- `pending`: no bead linked yet

---

**Command: `specks beads sync <file> --json`**

Creates/updates beads for speck steps. Returns what was synced.

```json
{
  "file": ".specks/specks-3.md",
  "root_bead_id": "bd-abc123",
  "steps_synced": 5,
  "deps_added": 3,
  "dry_run": false
}
```

---

**Command: `specks beads pull [file] --json`**

Updates speck checkboxes from bead completion status.

```json
{
  "files": [
    {
      "file": ".specks/specks-3.md",
      "name": "3",
      "checkboxes_updated": 4,
      "steps_updated": ["Step 0: Create plugin manifest", "Step 1: ..."]
    }
  ],
  "total_updated": 4
}
```

---

**Command: `specks beads link <file> <step-anchor> <bead-id>`**

Links an existing bead to a step. Returns success/failure (no JSON-specific data payload beyond standard envelope).

---

**Closing beads:** The committer skill closes the bead after a successful commit via `specks beads close <bead-id> --json`. This happens as part of the "finalize step" sequence: commit code → close bead → step complete. The committer receives the `bead_id` in its input and reports `bead_closed` in its output.

**Note:** The `specks beads close` subcommand must be added (see Step 8.6). It wraps `bd close` and uses the same discovery chain as other beads commands.

**Command: `specks beads close <bead-id> [--reason <reason>] --json`**

Closes a bead to mark work complete.

| Flag | Required | Description |
|------|----------|-------------|
| `--reason` | Optional | Reason for closing (e.g., "Step completed per speck") |
| `--json` | Required | JSON output format |

```json
{
  "bead_id": "string",
  "closed": true,
  "reason": "string"
}
```

---

##### Beads Error Handling {#beads-error-handling}

**CLI returns structured errors:**
- `E005` / `BeadsNotInstalled` - `bd` binary not found
- `E009` / `NotInitialized` - `.specks` directory not found
- `E013` / `BeadsNotInitialized` - `.beads` directory not found
- `E016` / `BeadsCommand(msg)` - command failed with stderr

**Beads error message format (short spec):**
- Single-line, human-readable, actionable
- Prefix with `beads:` and include the failing command
- Include one next step (install, set env, or retry) and avoid stack traces
- Use this template:
  - `beads: <command> failed: <reason>. Next: <action>.`
  - Examples:
    - `beads: specks beads status --json failed: specks CLI not found. Next: install specks CLI or add it to PATH.`
    - `beads: specks beads status --json failed: bd not found. Next: install bd or set SPECKS_BD_PATH.`

**Agent error recovery:**
1. If `specks beads` fails with "not installed", inform user via interviewer
2. Do not retry beads operations without user intervention
3. Non-beads workflow (planning, execution) continues unaffected
4. Treat non-JSON or invalid JSON output as an error and report it

---

**Install contract for plugin users:**

| Capability | Requirements |
|-----------|-------------|
| Planning/execution (no beads) | Plugin only. CLI and `bd` not required. |
| Beads integration | `specks` CLI on PATH + `bd` binary + `.beads/` initialized |

**Onboarding and ergonomics requirements:**
- Provide a single "beads readiness" checklist in docs
- Include actionable error messages when `specks` or `bd` is missing
- Document how to set `SPECKS_BD_PATH` and verify with `specks beads status --json`

**Testing requirements (Step 10):**
- [ ] Verify agent can call `specks beads status --json` via Bash and parse JSON
- [ ] Verify graceful error when `bd` not installed
- [ ] Verify graceful error when `specks` CLI not installed or not on PATH
- [ ] Verify `SPECKS_BD_PATH` override works in plugin context

---

#### Dependencies / Prerequisites {#dependencies}

- Claude Code CLI must be installed and working
- Understanding of Claude Code plugin system

#### Constraints {#constraints}

- Warnings are errors: all code must compile cleanly
- Must follow Claude Code plugin structure exactly
- Skills must have `SKILL.md` in subdirectories
- Agents must be markdown files with YAML frontmatter

#### Assumptions {#assumptions}

- Claude Code's `--plugin-dir` flag works for local development
- Plugin namespacing (`/specks:skill-name`) is acceptable
- The existing planner, architect, and implementer agent definitions are solid

---

### Open Questions (MUST RESOLVE OR EXPLICITLY DEFER) {#open-questions}

#### [Q01] Plugin naming (DECIDED) {#q01-plugin-naming}

**Question:** What should the plugin name be?

**Resolution:** DECIDED - use `specks`. Skills become `/specks:plan`, `/specks:execute`, etc. Agents become `specks:director`, etc.

---

### Risks and Mitigations {#risks}

| Risk | Impact | Likelihood | Mitigation | Trigger to revisit |
|------|--------|------------|------------|--------------------|
| Plugin structure doesn't work as expected | high | low | Test with `--plugin-dir .` early | Skills/agents not discovered |
| Namespacing creates UX friction | medium | medium | Accept tradeoff for clean architecture | User feedback |

---

### 3.0.0 Design Decisions {#design-decisions}

#### [D01] Specks is a Claude Code plugin (DECIDED) {#d01-plugin-architecture}

**Decision:** Structure the specks repository as a Claude Code plugin.

**Rationale:**
- Plugins are the standard way to distribute skills and agents
- `--plugin-dir .` enables development workflow (specks develops specks)
- Eliminates custom installation code (`specks setup claude`)
- Built-in namespacing prevents conflicts
- Version management via `plugin.json`

**Implications:**
- Add `.claude-plugin/plugin.json` at repo root
- Skills live in `skills/<skill-name>/SKILL.md`
- Agents live in `agents/<agent-name>.md`
- All skills namespaced as `/specks:<skill-name>`
- All agents namespaced as `specks:<agent-name>`

**Reference:** https://code.claude.com/docs/en/plugins

---

#### [D02] Director is pure orchestrator (DECIDED) {#d02-pure-orchestrator}

**Decision:** The director agent only coordinates via Task tool and Skill tool. It writes only audit trail files (run directory), never edits files, and never interacts with users directly.

**Rationale:**
- Keeps orchestration logic separate from work execution
- All file operations delegated to specialists (planner, implementer)
- All user interaction delegated to interviewer

**Implications:**
- Director's tools: Task, Skill, Read, Grep, Glob, Bash, Write (Write for audit trail only)
- Remove Edit, AskUserQuestion from director (Write kept for audit trail)
- Director can invoke skills via Skill tool
- Director can spawn agents via Task tool

---

#### [D03] Focused-task agents become skills (DECIDED) {#d03-agents-to-skills}

**Decision:** Agents that perform focused, single-purpose tasks become skills. Agents that need isolated context or complex multi-step workflows remain agents. See (#agents-skills-summary) for the complete breakdown.

**Rationale:**
- Skills run inline (~5-10s) vs agents spawn new context (~30-60s)
- Skills can specify `allowed-tools` in frontmatter (e.g., `allowed-tools: Write, Edit, Bash`)
- Skills produce structured JSON, ideal for director consumption
- Verified: Claude Code docs confirm skills are NOT read-only; they can write files

**Implications:**
- 6 agents become skills per (#skill-summary): clarifier, critic, reviewer, auditor, logger, committer
- 5 agents remain: director, planner, interviewer, architect, implementer
- Implementer agent includes self-monitoring for drift detection (see #smart-drift)
- Skills specify `allowed-tools` as needed (all get baseline Read, Grep, Glob plus additional tools)
- Skills return JSON-only output

---

#### [D04] Interviewer handles all user interaction (DECIDED) {#d04-interviewer-role}

**Decision:** The interviewer agent is the single point of user interaction. Director passes data to interviewer, interviewer presents via AskUserQuestion, returns user decisions.

**Rationale:**
- Director stays pure orchestrator (no AskUserQuestion)
- User interaction logic consolidated in one place

**Implications:**
- Interviewer receives questions from clarifier skill, results from critic skill
- Interviewer returns structured decisions to director
- Director never calls AskUserQuestion directly

---

#### [D05] CLI becomes utility only (DECIDED) {#d05-cli-utility}

**Decision:** Remove `plan` and `execute` CLI commands. Remove `setup claude` command. Keep: init, validate, list, status, beads, version.

**Rationale:**
- Planning and execution happen inside Claude Code via `/specks:plan` and `/specks:execute`
- Plugin system handles skill/agent distribution (no need for setup command)
- Eliminates process spawning overhead
- **Beads integration stays in CLI** - it's operational tooling (like git), not orchestration
- Agents can call beads CLI commands via Bash when needed

**What stays (unchanged):**
- `specks init` - Initialize project
- `specks validate` - Validate specks
- `specks list` - List specks
- `specks status` - Show progress
- `specks beads sync|link|status|pull` - Beads integration
- `specks version` - Show version

**What goes:**
- `specks plan` → replaced by `/specks:plan` skill
- `specks execute` → replaced by `/specks:execute` skill
- `specks setup` → plugin system handles distribution

**Implications:**
- Remove plan.rs, execute.rs, setup.rs from commands/
- Remove planning_loop/, streaming.rs, interaction/, share.rs modules
- Update cli.rs to remove Plan, Execute, Setup variants
- Keep commands/beads/ entirely (unchanged)

---

#### [D06] Clean breaks only (DECIDED) {#d06-clean-breaks}

**Decision:** Remove deprecated code entirely. No deprecation warnings, no legacy shims, no migration paths.

**Rationale:**
- Zero external users means zero migration burden
- Cleaner codebase without dead paths

**Implications:**
- Delete files rather than mark deprecated
- Update tests to remove references to deleted code
- Remove `.claude/skills/` directory entirely

---

#### [D07] Skill invocation via Skill tool (DECIDED) {#d07-skill-invocation}

**Decision:** Director invokes skills using Claude Code's native Skill tool.

**Rationale (verified from docs):**
- "The Skill tool" exists and can be controlled via permissions: `Skill(name)` or `Skill(name *)`
- Skills can be auto-loaded when description matches context
- Subagents can preload skills via `skills` frontmatter field

**Implications:**
- Director uses Skill tool to invoke analysis skills (clarifier, critic, etc.)
- Skill outputs are JSON-only for easy parsing
- Director spawns agents via Task tool for write operations

**Reference:** https://code.claude.com/docs/en/skills

---

### 3.0.1 Plugin Structure {#plugin-structure}

#### Naming Conventions {#naming-conventions}

**The plugin provides the namespace.** Per Claude Code plugin docs, plugin components are accessed as `plugin-name:component-name`.

| Resource | File/Folder Name | Frontmatter Name | User Invocation | Tool Invocation |
|----------|-----------------|------------------|-----------------|-----------------|
| Skill | `skills/clarifier/SKILL.md` | `name: clarifier` | `/specks:clarifier` | `Skill(skill: "specks:clarifier")` |
| Agent | `agents/director.md` | `name: director` | N/A | `Task(subagent_type: "specks:director")` |

**Namespacing rule (applies to BOTH skills AND agents):**
- Plugin name `specks` provides the namespace prefix automatically
- Skill folder `skills/plan/` becomes `/specks:plan`
- Agent file `agents/director.md` becomes `specks:director`
- Both Skill and Task tools use the colon-namespaced format

**Consistency rule:** Always use fully-qualified namespaced names in code:
- `Skill(skill: "specks:clarifier")` not `Skill(skill: "clarifier")`
- `Task(subagent_type: "specks:director")` not `Task(subagent_type: "director")`

**Syntax verification:** The exact Skill tool invocation syntax (`Skill(skill: "specks:clarifier")`) is verified in Step 10. If the actual syntax differs, update this section and all director references before proceeding past Step 10. Step 10 is the hard gate for syntax correctness.

**Plugin directory layout:**

```
specks/                           # Plugin root (repo root)
├── .claude-plugin/
│   └── plugin.json              # Plugin manifest
├── skills/                       # Skills (auto-discovered)
│   ├── plan/
│   │   └── SKILL.md
│   ├── execute/
│   │   └── SKILL.md
│   ├── clarifier/
│   │   └── SKILL.md
│   ├── critic/
│   │   └── SKILL.md
│   ├── reviewer/
│   │   └── SKILL.md
│   ├── auditor/
│   │   └── SKILL.md
│   ├── logger/
│   │   └── SKILL.md
│   └── committer/
│       └── SKILL.md
├── agents/                       # Agents (auto-discovered, namespaced as specks:*)
│   ├── director.md
│   ├── planner.md
│   ├── interviewer.md
│   ├── architect.md
│   └── implementer.md
├── crates/                       # Rust CLI (unchanged)
│   ├── specks/
│   └── specks-core/
├── .specks/                      # Specks plans for this project
└── CLAUDE.md                     # Project instructions
```

**Plugin manifest (`.claude-plugin/plugin.json`):**

```json
{
  "name": "specks",
  "description": "Transform ideas into working software through orchestrated LLM agents",
  "version": "0.3.0",
  "author": {
    "name": "specks-team"
  },
  "repository": "https://github.com/specks-dev/specks",
  "license": "MIT",
  "keywords": ["planning", "orchestration", "agents", "implementation"]
}
```

---

### 3.0.1.1 Orchestration Flowcharts {#orchestration-flowcharts}

These flowcharts define the director's orchestration logic. All steps reference these flows.

**Legend:**
- `[AGENT]` = spawned via Task tool (isolated context)
- `(SKILL)` = invoked via Skill tool (inline, JSON output)
- `{USER}` = interaction via interviewer agent

#### Planning Phase Flow {#flow-planning}

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         PLANNING PHASE                                       │
│                                                                              │
│  User invokes /specks:plan "idea" or /specks:plan path/to/speck.md          │
│                                                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────┐                                                                │
│  │  INPUT   │  idea text OR existing speck path                              │
│  └────┬─────┘                                                                │
│       │                                                                      │
│       ▼                                                                      │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ [DIRECTOR] receives input, mode=plan                                  │   │
│  │                                                                       │   │
│  │ 1. Invoke (CLARIFIER) skill                                          │   │
│  │    → Returns: analysis{}, questions[], assumptions[]                  │   │
│  │                                                                       │   │
│  │ 2. IF questions exist:                                                │   │
│  │    → Spawn [INTERVIEWER] with questions                               │   │
│  │    → Interviewer uses AskUserQuestion                                 │   │
│  │    → Returns: user_answers{}                                          │   │
│  │                                                                       │   │
│  │ 3. Spawn [PLANNER] with:                                              │   │
│  │    - Original idea/speck                                              │   │
│  │    - User answers (if any)                                            │   │
│  │    - Clarifier assumptions                                            │   │
│  │    → Returns: draft speck path                                        │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│       │                                                                      │
│       ▼                                                                      │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ [DIRECTOR] review loop                                                │   │
│  │                                                                       │   │
│  │ 4. Invoke (CRITIC) skill with draft speck                             │   │
│  │    → Returns: skeleton_compliant, areas{}, issues[], recommendation   │   │
│  │                                                                       │   │
│  │ 5. IF recommendation == REJECT or REVISE:                             │   │
│  │    → Spawn [INTERVIEWER] with critic issues                           │   │
│  │    → Present issues, get user decision: revise? accept anyway? abort? │   │
│  │                                                                       │   │
│  │    IF user says revise:                                               │   │
│  │    → Go back to step 3 with critic feedback                           │   │
│  │                                                                       │   │
│  │ 6. IF recommendation == APPROVE (or user accepts):                    │   │
│  │    → Planning complete                                                │   │
│  │    → Ready for execution                                              │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  OUTPUT: Approved speck at .specks/specks-{name}.md                          │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Key Points:**
- Director orchestrates via Skill tool (clarifier, critic) and Task tool (interviewer, planner)
- ALL user interaction goes through interviewer agent
- Planner never asks users directly (no AskUserQuestion)
- Loop continues until critic approves OR user accepts

---

#### Execution Phase Flow {#flow-execution}

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         EXECUTION PHASE                                      │
│                                                                              │
│  User invokes /specks:execute path/to/speck.md                               │
│                                                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ [DIRECTOR] receives speck, mode=execute                               │   │
│  │                                                                       │   │
│  │ FOR EACH step in speck.execution_steps:                               │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│       │                                                                      │
│       ▼                                                                      │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ STEP 1: Get Implementation Strategy                                   │   │
│  │                                                                       │   │
│  │ Spawn [ARCHITECT] with step details                                   │   │
│  │ → Returns: strategy, expected_touch_set[], test_plan                  │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│       │                                                                      │
│       ▼                                                                      │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ STEP 2: Implementation (with Self-Monitoring)                         │   │
│  │                                                                       │   │
│  │ Spawn [IMPLEMENTER] agent with architect strategy                     │   │
│  │ → Implementer reads strategy, writes code, runs tests                 │   │
│  │ → Implementer self-monitors against expected_touch_set (see #smart-   │   │
│  │   drift)                                                              │   │
│  │ → Returns: success/failure + drift_assessment                         │   │
│  │                                                                       │   │
│  │ IF implementer.halted_for_drift:                                      │   │
│  │   → Spawn [INTERVIEWER] with drift details                            │   │
│  │   → User decides: continue anyway? back to architect? abort?          │   │
│  │   → Director acts on user decision                                    │   │
│  │                                                                       │   │
│  │ IF implementer.success == false (non-drift failure):                  │   │
│  │   → Handle error, may retry or escalate                               │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│       │                                                                      │
│       ▼                                                                      │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ STEP 3: Review + Audit                                                │   │
│  │                                                                       │   │
│  │ PARALLEL INVOCATION:                                                  │   │
│  │                                                                       │   │
│  │   (REVIEWER) skill                 (AUDITOR) skill                    │   │
│  │   ├─ Checks plan adherence         ├─ Checks code quality             │   │
│  │   ├─ Tasks completed?              ├─ Performance concerns?           │   │
│  │   ├─ Tests match plan?             ├─ Security issues?                │   │
│  │   ├─ Artifacts produced?           ├─ Conventions followed?           │   │
│  │   └─ Returns: APPROVE|REVISE|      └─ Returns: APPROVE|FIX_REQUIRED|  │   │
│  │              ESCALATE                         MAJOR_REVISION          │   │
│  │                                                                       │   │
│  │ Director evaluates both reports                                       │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│       │                                                                      │
│       ▼                                                                      │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ STEP 4: Resolution                                                    │   │
│  │                                                                       │   │
│  │ IF issues found:                                                      │   │
│  │   ├─ Minor quality issues → Re-spawn [IMPLEMENTER] with fixes         │   │
│  │   ├─ Design issues → Back to [ARCHITECT] for new strategy             │   │
│  │   └─ Conceptual issues → Spawn [INTERVIEWER], may need re-planning    │   │
│  │                                                                       │   │
│  │ IF both reports clean:                                                │   │
│  │   1. Invoke (LOGGER) skill → Updates implementation log               │   │
│  │   2. Invoke (COMMITTER) skill → Commits changes                       │   │
│  │   3. Mark step complete                                               │   │
│  │   4. Proceed to next step                                             │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│       │                                                                      │
│       ▼                                                                      │
│  NEXT STEP (loop back to STEP 1)                                             │
│                                                                              │
│  WHEN all steps complete:                                                    │
│  → Invoke (LOGGER) with phase completion                                     │
│  → Report success to user                                                    │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Key Points:**
- Director coordinates all skills and agents
- Implementer includes **self-monitoring** for drift detection (see #smart-drift)
- Implementer runs to completion OR self-halts when drift thresholds exceeded
- On drift halt, director spawns interviewer for user decision
- Reviewer and auditor run in parallel (both are inline skills)
- ALL escalation decisions go through interviewer for user input
- Logger and committer are invoked after each successful step

---

#### Tool Invocation Summary {#flow-tools}

| Component | Type | Invocation | Purpose |
|-----------|------|------------|---------|
| **director** | Agent | Entry point (spawned by plan/execute skills) | Orchestrates entire workflow |
| **planner** | Agent | `Task(subagent_type: "specks:planner")` | Creates/revises speck documents |
| **interviewer** | Agent | `Task(subagent_type: "specks:interviewer")` | All user interaction |
| **architect** | Agent | `Task(subagent_type: "specks:architect")` | Implementation strategies |
| **implementer** | Agent | `Task(subagent_type: "specks:implementer")` | Executes architect strategies with self-monitoring for drift |
| **clarifier** | Skill | `Skill(skill: "specks:clarifier")` | Generates clarifying questions |
| **critic** | Skill | `Skill(skill: "specks:critic")` | Reviews speck quality |
| **reviewer** | Skill | `Skill(skill: "specks:reviewer")` | Verifies step completion |
| **auditor** | Skill | `Skill(skill: "specks:auditor")` | Code quality/security checks |
| **logger** | Skill | `Skill(skill: "specks:logger")` | Updates implementation log |
| **committer** | Skill | `Skill(skill: "specks:committer")` | Git commit operations |

---

### 3.0.1.2 Run Directory Audit Trail {#run-directory}

Each planning or execution session creates an audit trail in `.specks/runs/`.

#### Directory Structure {#run-structure}

```
.specks/runs/<session-id>/
├── metadata.json              # Session info, start time, mode, speck path
├── planning/                  # Planning phase artifacts
│   ├── 001-clarifier.json     # Clarifying questions generated
│   ├── 002-interviewer.json   # User answers received
│   ├── 003-planner.json       # Draft speck produced
│   ├── 004-critic.json        # Quality review
│   └── ...                    # Numbered by invocation order
└── execution/                 # Execution phase artifacts
    ├── step-0/
    │   ├── architect.json     # Implementation strategy
    │   ├── implementer.json   # Code changes made (includes drift_assessment)
    │   ├── reviewer.json      # Plan adherence check
    │   ├── auditor.json       # Code quality check
    │   ├── logger.json        # Log entry added
    │   └── committer.json     # Commit details
    ├── step-1/
    │   └── ...
    └── summary.json           # Overall execution status
```

#### Write Permissions {#run-permissions}

| Component | Access | Responsibility |
|-----------|--------|----------------|
| **director** | Write tool | Creates run directory (Bash), writes metadata.json and all skill outputs (Write) |
| **planner** | Write | Writes draft speck to `.specks/` (not runs/) |
| **architect** | Read | Reads speck; director persists strategy to runs/ |
| **All skills** | None | Return JSON to director; director persists to runs/ |

**Key design:** Skills return JSON to director. Director persists all outputs to runs directory using the Write tool. This keeps skills stateless and director as single source of truth for audit trail.

#### Session ID Format {#session-id}

Format: `YYYYMMDD-HHMMSS-<mode>-<short-uuid>`

Examples:
- `20260206-143022-plan-a1b2c3`
- `20260206-150145-execute-d4e5f6`

**Generation method:** Director generates session ID at start via Bash:
```bash
# Generate session ID
SESSION_ID="$(date +%Y%m%d-%H%M%S)-${MODE}-$(uuidgen | tr '[:upper:]' '[:lower:]' | cut -c1-6)"
```

Director passes session ID to all components. Fallback chain if primary fails:
```bash
# Fallback 1: /dev/urandom
SESSION_ID="$(date +%Y%m%d-%H%M%S)-${MODE}-$(head -c 3 /dev/urandom | xxd -p)"

# Fallback 2: PID + RANDOM (final fallback, always works on macOS/Linux)
SESSION_ID="$(date +%Y%m%d-%H%M%S)-${MODE}-$$${RANDOM}"
```
*Note: `date +%N` not used because it's unsupported on macOS.*

#### Metadata Schema {#run-metadata}

```json
{
  "session_id": "20260206-143022-plan-a1b2c3",
  "mode": "plan",
  "started_at": "2026-02-06T14:30:22Z",
  "speck_path": ".specks/specks-3.md",
  "status": "in_progress",
  "completed_at": null
}
```

#### JSON Persistence Pattern {#json-persistence}

Director writes JSON to the runs directory using the **Write tool** (not Bash). This avoids all escaping issues and is the natural tool for file creation.

```
Write(file_path: ".specks/runs/20260206-143022-plan-a1b2c3/metadata.json", content: <json-string>)
```

**Why Write, not Bash:**
- Write tool handles content exactly as provided - no escaping needed
- More reliable than heredocs or echo for JSON with special characters
- Audit trail is director's responsibility, not delegation - Write is appropriate
- Bash is reserved for: `mkdir -p` (directory creation), `uuidgen`/`date` (ID generation)

**Director tool usage:**
| Tool | Used for |
|------|----------|
| Bash | `mkdir -p .specks/runs/<session-id>/planning`, session ID generation |
| Write | All JSON file writes (metadata.json, skill outputs, agent outputs) |
| Read | Reading speck files, checking existing state |

---

### 3.0.2 Skill Specifications {#skill-specs}

Each skill lives at `skills/<skill-name>/SKILL.md`.

**Skill frontmatter requirements:**
- `name`: matches directory name
- `description`: helps Claude decide when to auto-invoke
- `allowed-tools`: explicit tool permissions (prevents mysterious denials)
- `disable-model-invocation: true` for entry points (user invokes explicitly)

**Output format:** All analysis skills return JSON-only (no prose).

#### Skill Permissions Summary {#skill-permissions}

| Skill | allowed-tools | Reason |
|-------|---------------|--------|
| **plan** | Task | Spawns director |
| **execute** | Task | Spawns director |
| **clarifier** | Read, Grep, Glob | Reads codebase for context |
| **critic** | Read, Grep, Glob | Reads speck for review |
| **reviewer** | Read, Grep, Glob | Checks plan artifacts |
| **auditor** | Read, Grep, Glob | Reads code for quality |
| **logger** | Read, Grep, Glob, Edit | Updates implementation log |
| **committer** | Read, Grep, Glob, Bash | `git add`, `git commit` |

**Baseline:** All skills get `Read, Grep, Glob` for codebase access. Additional tools added only where needed.

**Note:** Implementer is an agent, not a skill. It includes self-monitoring for drift detection. See (#implementer-agent-contract) for its tool configuration and drift detection heuristics.

#### Skill Tool Invocation Contract {#skill-invocation-contract}

**Invocation:** The director invokes skills via the Skill tool using the skill's frontmatter `name:` and a single argument string or JSON payload.

**Input format:**
- Provide only the inputs listed in each skill spec
- If multiple inputs are needed, encode as JSON (preferred) or a structured string
- Paths are repo-relative when possible

**Output format:**
- Output is JSON-only and must conform to the spec schema
- No surrounding prose, markdown, or code fences
- On failure, return a valid JSON object with:
  - `error`: "string" describing the failure
  - `recommendation`: "REVISE|PAUSE|HALT" (use the closest matching enum for the spec)

**Director parsing rules:**
- Reject any non-JSON output and re-run the skill once
- If the second run fails, escalate to the interviewer with an error summary
- Persist the raw JSON in the run directory for audit

#### plan Skill (Entry Point) {#skill-plan}

**Spec S01: plan** {#s01-plan}

**Flow:** See (#flow-planning) for complete orchestration.

```yaml
---
name: plan
description: Create or revise a speck through agent collaboration
disable-model-invocation: true
allowed-tools: Task
---
```
*Note: Task required to spawn director agent.*

**Behavior:**
1. Accepts: idea text OR path to existing speck
2. Spawns director agent with `mode=plan` via Task tool
3. Director executes Planning Phase Flow (#flow-planning)
4. Returns: path to approved speck

---

#### execute Skill (Entry Point) {#skill-execute}

**Spec S02: execute** {#s02-execute}

**Flow:** See (#flow-execution) for complete orchestration.

```yaml
---
name: execute
description: Execute a speck through agent orchestration
disable-model-invocation: true
allowed-tools: Task
---
```
*Note: Task required to spawn director agent.*

**Behavior:**
1. Accepts: path to approved speck
2. Spawns director agent with `mode=execute` via Task tool
3. Director executes Execution Phase Flow (#flow-execution) for each step
4. Returns: completion status, log of changes

---

#### clarifier Skill {#skill-clarifier}

**Spec S03: clarifier** {#s03-clarifier}

**Purpose:** Analyze an idea or critic feedback to generate clarifying questions.

**Frontmatter:**
```yaml
---
name: clarifier
description: Analyze ideas and generate clarifying questions
allowed-tools: Read, Grep, Glob
---
```

**Input JSON:**
```json
{
  "idea": "string",
  "speck_path": "string | null",
  "critic_feedback": {
    "issues": [{"priority": "string", "description": "string"}],
    "recommendation": "string"
  } | null
}
```
- `idea`: The original idea text (required for new specks)
- `speck_path`: Path to existing speck (for revisions)
- `critic_feedback`: If critic triggered a revision cycle, contains their issues

**Output JSON:**
```json
{
  "analysis": {
    "understood_intent": "string",
    "ambiguities": ["string"]
  },
  "questions": [
    {
      "question": "string",
      "options": ["string"],
      "default": "string"
    }
  ],
  "assumptions": ["string"]
}
```

---

#### critic Skill {#skill-critic}

**Spec S04: critic** {#s04-critic}

**Purpose:** Review a speck for skeleton compliance, quality, and implementability.

**Frontmatter:**
```yaml
---
name: critic
description: Review speck quality and implementability
allowed-tools: Read, Grep, Glob
---
```

**Input JSON:**
```json
{
  "speck_path": "string",
  "skeleton_path": "string"
}
```
- `speck_path`: Path to the speck to review (required)
- `skeleton_path`: Path to skeleton template for compliance check (defaults to `.specks/specks-skeleton.md`)

**Output JSON:**
```json
{
  "skeleton_compliant": true,
  "areas": {
    "completeness": "PASS|WARN|FAIL",
    "implementability": "PASS|WARN|FAIL",
    "sequencing": "PASS|WARN|FAIL"
  },
  "issues": [
    {
      "priority": "HIGH|MEDIUM|LOW",
      "description": "string"
    }
  ],
  "recommendation": "APPROVE|REVISE|REJECT"
}
```

---

#### reviewer Skill {#skill-reviewer}

**Spec S05: reviewer** {#s05-reviewer}

**Purpose:** Verify a completed step matches the plan specification.

**Frontmatter:**
```yaml
---
name: reviewer
description: Verify step completion matches plan
allowed-tools: Read, Grep, Glob
---
```

**Input JSON:**
```json
{
  "speck_path": "string",
  "step_anchor": "string",
  "implementer_output": {
    "files_created": ["string"],
    "files_modified": ["string"],
    "tests_passed": true,
    "drift_assessment": { ... }
  }
}
```
- `speck_path`: Path to the speck
- `step_anchor`: Which step was just completed
- `implementer_output`: Results from the implementer agent (includes drift_assessment)

**Output JSON:**
```json
{
  "tasks_complete": true,
  "tests_match_plan": true,
  "artifacts_produced": true,
  "issues": [{"type": "string", "description": "string"}],
  "drift_notes": "string | null",
  "recommendation": "APPROVE|REVISE|ESCALATE"
}
```
- `drift_notes`: If implementer had minor drift (1-2 yellow touches), mention it here for visibility. Prevents silent creep.

---

#### auditor Skill {#skill-auditor}

**Spec S06: auditor** {#s06-auditor}

**Purpose:** Check code quality, performance, and security of recent changes.

**Frontmatter:**
```yaml
---
name: auditor
description: Check code quality, performance, and security
allowed-tools: Read, Grep, Glob
---
```

**Input JSON:**
```json
{
  "speck_path": "string",
  "step_anchor": "string",
  "files_to_audit": ["string"],
  "drift_assessment": { ... }
}
```
- `speck_path`: Path to the speck (for context)
- `step_anchor`: Which step was just completed
- `files_to_audit`: Files that were created or modified by implementer
- `drift_assessment`: From implementer output (for context on any unexpected changes)

**Output JSON:**
```json
{
  "categories": {
    "structure": "PASS|WARN|FAIL",
    "error_handling": "PASS|WARN|FAIL",
    "security": "PASS|WARN|FAIL"
  },
  "issues": [
    {
      "severity": "critical|major|minor",
      "file": "string",
      "description": "string"
    }
  ],
  "drift_notes": "string | null",
  "recommendation": "APPROVE|FIX_REQUIRED|MAJOR_REVISION"
}
```
- `drift_notes`: If implementer had minor drift, note any concerns about the unexpected changes. Flags potential creep for human awareness.

---

#### logger Skill {#skill-logger}

**Spec S07: logger** {#s07-logger}

**Purpose:** Update the implementation log with completed work.

**Frontmatter:**
```yaml
---
name: logger
description: Update implementation log with completed work
allowed-tools: Read, Grep, Glob, Edit
---
```
*Note: Edit required to update `.specks/specks-implementation-log.md`. Write not needed because `specks init` creates the log file.*

**Input JSON:**
```json
{
  "speck_path": "string",
  "step_anchor": "string",
  "summary": "string",
  "files_changed": ["string"],
  "commit_hash": "string | null"
}
```
- `speck_path`: Path to the speck
- `step_anchor`: Which step was completed
- `summary`: Brief description of what was done
- `files_changed`: List of files created/modified
- `commit_hash`: Git commit hash if committed (from committer output)

**Output JSON:**
```json
{
  "success": true,
  "log_file": "string",
  "entry_added": {
    "step": "string",
    "timestamp": "string",
    "summary": "string"
  }
}
```

---

#### committer Skill {#skill-committer}

**Spec S08: committer** {#s08-committer}

**Purpose:** Finalize a completed step: stage files, commit changes, and close the associated bead.

**Frontmatter:**
```yaml
---
name: committer
description: Commit changes and close bead to finalize step completion
allowed-tools: Read, Grep, Glob, Bash
---
```
*Note: Bash required for `git add`, `git commit`, `git status`, and `specks beads close`.*

**Input JSON:**
```json
{
  "speck_path": "string",
  "step_anchor": "string",
  "proposed_message": "string",
  "files_to_stage": ["string"],
  "auto_commit": true,
  "bead_id": "string | null",
  "close_reason": "string | null"
}
```
- `speck_path`: Path to the speck (for commit message context)
- `step_anchor`: Which step was completed
- `proposed_message`: Commit message from the step's `**Commit:**` line
- `files_to_stage`: Files to add (from implementer output)
- `auto_commit`: If true, execute commit; if false, only prepare message
- `bead_id`: Bead ID to close after commit (null if no bead linked)
- `close_reason`: Reason for closing bead (e.g., "Step completed per speck")

**Output JSON:**
```json
{
  "commit_message": "string",
  "files_staged": ["string"],
  "committed": true,
  "commit_hash": "string",
  "bead_closed": true,
  "bead_id": "string | null"
}
```

---

#### implementer (Agent, not Skill) {#implementer-note}

**Note:** Implementer is an **agent**, not a skill. It includes self-monitoring for drift detection. See (#implementer-agent-contract) for the input/output contract and drift detection heuristics.

---

### 3.0.3 Files to Remove {#files-to-remove}

**Table T01: Rust Files to Remove** {#t01-rust-removal}

| File | Reason |
|------|--------|
| `crates/specks/src/commands/plan.rs` | CLI orchestration eliminated |
| `crates/specks/src/commands/execute.rs` | CLI orchestration eliminated |
| `crates/specks/src/commands/setup.rs` | Plugin system handles installation |
| `crates/specks/src/planning_loop/` (entire directory) | Rust orchestration eliminated |
| `crates/specks/src/streaming.rs` | Not needed |
| `crates/specks/src/interaction/` (entire directory) | Skills use AskUserQuestion |
| `crates/specks/src/share.rs` | Plugin system handles distribution |

**Table T02: Directories to Remove** {#t02-dir-removal}

| Directory | Reason |
|-----------|--------|
| `.claude/skills/` | Replaced by `skills/` at repo root |

**Table T03: Agent Files to Remove** {#t03-agent-removal}

These agents become skills per (#agents-skills-summary). Note: implementer stays as an agent (includes self-monitoring). Monitor is eliminated entirely (no skill replacement).

| File | Replacement |
|------|-------------|
| `agents/specks-clarifier.md` | `skills/clarifier/SKILL.md` |
| `agents/specks-critic.md` | `skills/critic/SKILL.md` |
| `agents/specks-monitor.md` | None (eliminated) |
| `agents/specks-reviewer.md` | `skills/reviewer/SKILL.md` |
| `agents/specks-auditor.md` | `skills/auditor/SKILL.md` |
| `agents/specks-logger.md` | `skills/logger/SKILL.md` |
| `agents/specks-committer.md` | `skills/committer/SKILL.md` |

---

### 3.0.4 Agent Updates {#agent-updates}

Tool changes for agents that remain per (#agent-summary).

**Table T04: Agent Tool Changes** {#t04-agent-tools}

| Agent | Current Tools | New Tools |
|-------|---------------|-----------|
| director | Task, Read, Grep, Glob, Bash, Write, Edit | Task, Skill, Read, Grep, Glob, Bash, Write |
| planner | Read, Grep, Glob, Bash, Write, Edit, AskUserQuestion | Read, Grep, Glob, Bash, Write, Edit |
| interviewer | Read, Grep, Glob, Bash, AskUserQuestion | Read, Grep, Glob, Bash, AskUserQuestion (unchanged) |
| architect | Read, Grep, Glob, Bash | Read, Grep, Glob, Bash (unchanged) |

#### Architect Agent Output Contract {#architect-output}

The architect agent returns JSON to the director, which is persisted to the runs directory and passed to the implementer agent.

**Architect Output JSON:**
```json
{
  "step_anchor": "string",
  "approach": "string",
  "expected_touch_set": ["string"],
  "implementation_steps": [
    {
      "order": 1,
      "description": "string",
      "files": ["string"]
    }
  ],
  "test_plan": "string",
  "risks": ["string"]
}
```
- `step_anchor`: Which step this strategy is for
- `approach`: High-level description of the implementation approach
- `expected_touch_set`: All files expected to be created or modified
- `implementation_steps`: Ordered list of implementation actions
- `test_plan`: How to verify the implementation works
- `risks`: Potential issues to watch for

#### Interviewer Agent Contract {#interviewer-contract}

The interviewer agent handles all user interaction. Director spawns it with different input contexts; output format mirrors input structure.

**Input JSON (from director):**
```json
{
  "context": "clarifier | critic | drift | review",
  "speck_path": "string",
  "step_anchor": "string | null",
  "payload": { ... }
}
```

The `payload` structure depends on `context`:

| Context | Payload | What interviewer presents |
|---------|---------|---------------------------|
| `clarifier` | `{questions: [...], assumptions: [...]}` | Clarifying questions before planning |
| `critic` | `{issues: [...], recommendation: "..."}` | Critic feedback on draft speck |
| `drift` | `{drift_assessment: {...}, files_touched: [...]}` | Implementer self-halted due to drift |
| `review` | `{issues: [...], source: "reviewer\|auditor"}` | Conceptual issues from review |

**Output JSON (to director):**
```json
{
  "context": "clarifier | critic | drift | review",
  "decision": "continue | halt | revise",
  "user_answers": { ... },
  "notes": "string | null"
}
```

The `user_answers` structure mirrors the input payload - answers keyed to questions, resolutions keyed to issues, etc.

#### Implementer Agent Contract {#implementer-agent-contract}

The implementer agent executes architect strategies. It includes **self-monitoring** for drift detection: after each implementation sub-step, the implementer checks its own changes against the expected_touch_set and halts if drift thresholds are exceeded.

**Implementer Agent Definition:**
```yaml
---
name: implementer
description: Execute architect strategies with self-monitoring. Writes code, runs tests, creates artifacts. Self-halts when drift detected.
tools: Read, Grep, Glob, Write, Edit, Bash
model: inherit
---
```

**Input (via Task prompt):**
```json
{
  "speck_path": "string",
  "step_anchor": "string",
  "architect_strategy": {
    "approach": "string",
    "expected_touch_set": ["string"],
    "implementation_steps": [{"order": 1, "description": "string", "files": ["string"]}],
    "test_plan": "string"
  },
  "session_id": "string"
}
```
- `speck_path`: Path to the speck being executed
- `step_anchor`: Which step to implement
- `architect_strategy`: Strategy from the architect agent
- `session_id`: For persisting artifacts to `.specks/runs/<session_id>/`

##### Smart Drift Detection (Self-Monitoring) {#smart-drift}

The implementer uses smart heuristics to self-monitor, not just mechanical file comparison. After each implementation sub-step, it evaluates whether its changes stay within acceptable bounds.

**1. Proximity scoring:**
| Category | Description | Budget Impact |
|----------|-------------|---------------|
| **Green** | Same directory as expected files | No impact (automatic leeway) |
| **Yellow** | Adjacent directories (sibling, parent, child) | Counts toward budget |
| **Red** | Unrelated subsystem | Counts double |

**2. File type modifiers:**
- Test files (`*_test.rs`, `tests/`) -> +2 leeway
- Config files (`Cargo.toml`, `*.toml`) -> +1 leeway
- Documentation (`*.md`) -> +1 leeway
- Core logic in unexpected areas -> no leeway

**3. Drift budget thresholds:**
| Severity | Condition | Action |
|----------|-----------|--------|
| `none` | All files in expected set | Continue implementation |
| `minor` | 1-2 yellow touches | Continue (note in output) |
| `moderate` | 3-4 yellow OR 1 red | HALT and report to director |
| `major` | 5+ yellow OR 2+ red | HALT and report to director |

**4. Qualitative check:**
The implementer evaluates whether unexpected changes are *consistent with the architect's approach*. Adding a helper function in the same module = OK. Refactoring unrelated subsystems = HALT.

**5. Self-halt behavior:**
When drift thresholds are exceeded, the implementer:
1. Stops further implementation work immediately
2. Returns with `success: false` and `halted_for_drift: true`
3. Includes full drift assessment in output for director to escalate via interviewer

**Output (returned when agent completes or halts):**
```json
{
  "success": true,
  "halted_for_drift": false,
  "files_created": ["string"],
  "files_modified": ["string"],
  "tests_run": true,
  "tests_passed": true,
  "artifacts": ["string"],
  "notes": "string",
  "drift_assessment": {
    "expected_files": ["string"],
    "actual_changes": ["string"],
    "unexpected_changes": [
      {
        "file": "string",
        "category": "green|yellow|red",
        "reason": "string"
      }
    ],
    "drift_budget": {
      "yellow_used": 0,
      "yellow_max": 4,
      "red_used": 0,
      "red_max": 2
    },
    "drift_severity": "none|minor|moderate|major",
    "qualitative_assessment": "string"
  }
}
```

**Note:** `drift_assessment` is **mandatory** in all implementer output, even when `halted_for_drift: false` and `drift_severity: none`. This improves debuggability, gives reviewer/auditor context about minor drift, and supports the "no fire-and-forget / audit-first" principle.

When `halted_for_drift: true`, the director spawns the interviewer to present drift details and get user decision: continue anyway, back to architect for new strategy, or abort.

---

### 3.0.5 Execution Steps {#execution-steps}

#### Step 0: Create plugin manifest {#step-0}

**Commit:** `feat(plugin): add .claude-plugin/plugin.json manifest`

**References:** [D01] Specks is a Claude Code plugin, (#plugin-structure)

**Artifacts:**
- `.claude-plugin/plugin.json`

**Tasks:**
- [x] Create `.claude-plugin/` directory
- [x] Create `plugin.json` with name, description, version, author, repository, license, keywords

**Checkpoint:**
- [x] File exists at `.claude-plugin/plugin.json`
- [x] JSON is valid and contains required fields

**Rollback:**
- Delete `.claude-plugin/` directory

**Commit after all checkpoints pass.**

---

#### Step 1: Create skills directory and entry point skills {#step-1}

**Depends on:** #step-0

**Commit:** `feat(skills): create skills directory and move entry point skills`

**References:** [D01] Specks is a Claude Code plugin, Spec S01, Spec S02, (#plugin-structure)

**Artifacts:**
- `skills/plan/SKILL.md`
- `skills/execute/SKILL.md`

**Tasks:**
- [x] Create `skills/` directory at repo root
- [x] Create `skills/plan/` directory
- [x] Move/adapt content from `.claude/skills/specks-plan/SKILL.md` to `skills/plan/SKILL.md`
- [x] Create `skills/execute/` directory
- [x] Move/adapt content from `.claude/skills/specks-execute/SKILL.md` to `skills/execute/SKILL.md`
- [x] Update both to spawn director via Task tool

**Checkpoint:**
- [x] `skills/plan/SKILL.md` exists with valid frontmatter
- [x] `skills/execute/SKILL.md` exists with valid frontmatter
- [x] `claude --plugin-dir . --help` shows specks plugin loaded

**Rollback:**
- Delete `skills/` directory

**Commit after all checkpoints pass.**

---

#### Step 2: Create analysis skills {#step-2}

**Depends on:** #step-1

**Commit:** `feat(skills): add analysis skills (clarifier, critic, reviewer, auditor)`

**References:** [D03] Read-only agents become skills, Specs S03-S06, (#skill-specs)

**Artifacts:**
- `skills/clarifier/SKILL.md`
- `skills/critic/SKILL.md`
- `skills/reviewer/SKILL.md`
- `skills/auditor/SKILL.md`

**Tasks:**
- [x] Create `skills/clarifier/SKILL.md` per S03 spec
  - `allowed-tools: Read, Grep, Glob`
- [x] Create `skills/critic/SKILL.md` per S04 spec
  - `allowed-tools: Read, Grep, Glob`
- [x] Create `skills/reviewer/SKILL.md` per S05 spec
  - `allowed-tools: Read, Grep, Glob`
- [x] Create `skills/auditor/SKILL.md` per S06 spec
  - `allowed-tools: Read, Grep, Glob`

**Checkpoint:**
- [x] All 4 skill files exist with valid YAML frontmatter
- [x] Each skill has correct `allowed-tools` per spec
- [x] Each skill specifies JSON-only output format

**Rollback:**
- Delete the created skill directories

**Commit after all checkpoints pass.**

---

#### Step 3: Create utility skills {#step-3}

**Depends on:** #step-1

**Commit:** `feat(skills): add utility skills (logger, committer)`

**References:** [D03] Focused-task agents become skills, Specs S07-S08, (#skill-specs)

**Artifacts:**
- `skills/logger/SKILL.md`
- `skills/committer/SKILL.md`

**Tasks:**
- [x] Create `skills/logger/SKILL.md` based on existing update-plan-implementation-log
  - Add `allowed-tools: Read, Grep, Glob, Edit` in frontmatter
- [x] Create `skills/committer/SKILL.md` based on existing prepare-git-commit-message
  - Add `allowed-tools: Read, Grep, Glob, Bash` in frontmatter
  - Add git add/commit execution

**Note:** Implementer is an agent, not a skill. See Step 4.1 for implementer agent update.

**Checkpoint:**
- [x] Both skill files exist with valid YAML frontmatter
- [x] Logger skill has `allowed-tools: Read, Grep, Glob, Edit`
- [x] Committer skill has `allowed-tools: Read, Grep, Glob, Bash`

**Rollback:**
- Delete the created skill directories

**Commit after all checkpoints pass.**

---

#### Step 4: Update director agent {#step-4}

Step 4 is split into substeps to manage complexity. Each substep builds on the previous.

---

##### Step 4.1: Update director tools and remove legacy CLI {#step-4-1}

**Depends on:** #step-2, #step-3

**References:** [D02] Director is pure orchestrator, [D07] Skill invocation, Table T04, (#agent-updates)

**Artifacts:**
- Updated `agents/director.md` (tools and legacy removal only)

**Tasks:**
- [x] Change tools line to: `tools: Task, Skill, Read, Grep, Glob, Bash, Write`
- [x] Remove Edit from tools (keep Write for audit trail)
- [x] Add `skills` frontmatter field to preload analysis skills (clarifier, critic, reviewer, auditor, logger, committer)
- [x] Remove "Path 1: External CLI" section entirely from body
- [x] Remove all references to `specks plan "idea"` CLI command (replaced by `/specks:plan`)
- [x] Remove all references to `specks execute` CLI command (replaced by `/specks:execute`)
- [x] Remove any direct file writing logic (director delegates to skills/agents)

**Checkpoint:**
- [x] `grep "^tools:" agents/director.md` shows Task, Skill, Read, Grep, Glob, Bash, Write
- [x] No Edit or AskUserQuestion in tools (Write is allowed for audit trail)
- [x] `grep -c "specks plan\|specks execute" agents/director.md` returns 0
- [x] `grep "^skills:" agents/director.md` lists preloaded skills

**Rollback:**
- Revert from git

**Commit:** `refactor(agents): update director tools and remove legacy CLI references`

---

##### Step 4.2: Implement director Planning Phase Flow {#step-4-2}

**Depends on:** #step-4-1

**References:** [D02] Director is pure orchestrator, [D04] Interviewer handles user interaction, **(#flow-planning)**, (#flow-tools)

**Artifacts:**
- Updated `agents/director.md` (planning flow implementation)

**Tasks:**
- [x] Implement Planning Phase Flow per (#flow-planning):
  - [x] **Step 1**: Receive idea/context from plan skill
  - [x] **Step 2**: Invoke clarifier skill → if questions exist, spawn interviewer agent
  - [x] **Step 3**: Spawn planner agent with idea + user_answers + assumptions
  - [x] **Step 4**: Invoke critic skill on draft speck
  - [x] **Step 5**: If critic has issues, spawn interviewer → loop back to step 3
  - [x] **Step 6**: On critic approval, spawn interviewer for final user approval
  - [x] **Step 7**: Return approved speck path
- [x] Use exact invocation syntax from (#flow-tools):
  - [x] `Skill(skill: "specks:clarifier")` for clarifier
  - [x] `Task(subagent_type: "specks:interviewer")` for interviewer
  - [x] `Task(subagent_type: "specks:planner")` for planner
  - [x] `Skill(skill: "specks:critic")` for critic
- [x] ALL user interaction delegated to interviewer agent (never AskUserQuestion directly)

**Checkpoint:**
- [x] Planning flow in director body matches (#flow-planning) diagram
- [x] Clarifier invoked via Skill tool
- [x] Interviewer spawned via Task tool for all user interaction
- [x] Planner spawned via Task tool
- [x] Critic invoked via Skill tool
- [x] Loop structure present for critic issues

**Rollback:**
- Revert from git

**Commit:** `feat(agents): implement director planning phase flow`

---

##### Step 4.3: Implement director Execution Phase Flow {#step-4-3}

**Depends on:** #step-4-2

**References:** [D02] Director is pure orchestrator, **(#flow-execution)**, (#flow-tools), (#implementer-agent-contract)

**Artifacts:**
- Updated `agents/director.md` (execution flow implementation)

**Tasks:**
- [x] Implement Execution Phase Flow per (#flow-execution):
  - [x] **For each step** in speck (iterate in order, respecting dependencies):
    - [x] **Architect**: Spawn architect agent -> receive strategy JSON
    - [x] **Implementer**: Spawn implementer agent -> wait for completion
    - [x] **Drift handling**: If implementer returns halted_for_drift, spawn interviewer for escalation
    - [x] **Review**: Invoke reviewer + auditor skills in parallel
    - [x] **Finalize**: Invoke logger skill, invoke committer skill (with bead_id if present)
  - [x] Handle step completion and move to next step
- [x] Use exact invocation syntax from (#flow-tools):
  - [x] `Task(subagent_type: "specks:architect")` for architect
  - [x] `Task(subagent_type: "specks:implementer")` for implementer
  - [x] `Skill(skill: "specks:reviewer")` and `Skill(skill: "specks:auditor")` in parallel
  - [x] `Skill(skill: "specks:logger")` then `Skill(skill: "specks:committer")`

**Checkpoint:**
- [x] Execution flow in director body matches (#flow-execution) diagram
- [x] Implementer spawned via Task tool, runs to completion or self-halts
- [x] Drift escalation path to interviewer exists (when implementer.halted_for_drift)
- [x] Reviewer and auditor invoked in parallel
- [x] Logger and committer invoked sequentially at step end

**Rollback:**
- Revert from git

**Commit:** `feat(agents): implement director execution phase flow`

---

##### Step 4.4: Add director run directory audit trail {#step-4-4}

**Depends on:** #step-4-3

**References:** **(#run-directory)**, (#run-structure)

**Artifacts:**
- Updated `agents/director.md` (audit trail implementation)

**Tasks:**
- [ ] **Session initialization**:
  - [ ] Generate session ID at start using format: `YYYYMMDD-HHMMSS-<mode>-<short-uuid>`
  - [ ] UUID generation: `uuidgen` → fallback `/dev/urandom` → fallback `date +%N`
  - [ ] Mode is `plan` or `execute` based on entry point
  - [ ] Create `.specks/runs/<session-id>/` directory via Bash
  - [ ] Create `planning/` or `execution/` subdirectory based on mode
- [ ] **Metadata management**:
  - [ ] Write `metadata.json` at session start with: session_id, mode, speck_path, started_at, status: "in_progress"
  - [ ] Update `metadata.json` with status: "completed"/"failed" and completed_at at end
- [ ] **Skill output persistence**:
  - [ ] After each skill invocation, write output to run directory
  - [ ] Naming: `NNN-<skill-name>.json` (e.g., `001-clarifier.json`, `002-critic.json`)
  - [ ] Increment counter for each invocation
- [ ] **Agent output persistence**:
  - [ ] After each agent completion, write summary to run directory
  - [ ] Naming: `NNN-<agent-name>.json` (e.g., `003-planner.json`)

**Checkpoint:**
- [ ] Director creates run directory on session start
- [ ] `metadata.json` written with correct structure
- [ ] Skill outputs persisted with sequential numbering
- [ ] `metadata.json` updated on session end

**Rollback:**
- Revert from git

**Commit:** `feat(agents): add director run directory audit trail`

---

**Step 4 Summary:** After completing substeps 4.1-4.4, the director agent is fully updated as a pure orchestrator with both planning and execution flows implemented, plus audit trail support.

---

#### Step 5: Update other agents {#step-5}

**Depends on:** #step-4-4

**Commit:** `refactor(agents): update planner, interviewer, and implementer`

**References:** [D04] Interviewer handles all user interaction, Table T04, (#agent-updates), (#flow-planning), (#flow-execution), (#implementer-agent-contract)

**Artifacts:**
- Updated `agents/planner.md`
- Updated `agents/interviewer.md`
- Updated `agents/implementer.md`

**Tasks:**
- [ ] Remove AskUserQuestion from planner's tools
- [ ] **Update planner body content** (per #flow-planning step 3):
  - [ ] Remove "Ask Clarifying Questions" workflow (interviewer handles this now)
  - [ ] Receives: idea, user_answers, clarifier_assumptions from director
  - [ ] Returns: draft speck path
  - [ ] Ensure workflow focuses on speck creation/revision only
- [ ] **Update interviewer body content** (per #flow-planning steps 2, 5):
  - [ ] Emphasize role as single point of user interaction
  - [ ] Receives: questions from clarifier skill OR issues from critic skill
  - [ ] Uses AskUserQuestion to present to user
  - [ ] Returns: structured user_answers{} or decisions to director
- [ ] **Update interviewer for execution phase** (per #flow-execution):
  - [ ] Handles drift escalation when implementer self-halts
  - [ ] Handles conceptual issue escalation from reviewer/auditor
- [ ] **Update implementer agent** (per #implementer-agent-contract):
  - [ ] Update tools to: `tools: Read, Grep, Glob, Write, Edit, Bash`
  - [ ] Add description per contract (includes self-monitoring)
  - [ ] Update body to accept architect strategy JSON input
  - [ ] Implement self-monitoring for drift detection per (#smart-drift)
  - [ ] Return structured JSON output per contract (includes drift_assessment)
- [ ] Verify architect doesn't need changes (read-only analysis, no user interaction)

**Checkpoint:**
- [ ] Planner tools do not include AskUserQuestion
- [ ] Planner body has no "Ask Clarifying Questions" section
- [ ] Planner receives clarifier output as input parameter
- [ ] Interviewer tools include AskUserQuestion
- [ ] Interviewer body describes user interaction workflow per flowcharts
- [ ] Implementer has correct tools and accepts architect strategy

**Rollback:**
- Revert from git

**Commit after all checkpoints pass.**

---

#### Step 6: Remove agent files that became skills {#step-6}

**Depends on:** #step-2, #step-3

**Commit:** `refactor(agents): remove agents that became skills`

**References:** [D03] Read-only agents become skills, [D06] Clean breaks, Table T03, (#files-to-remove)

**Artifacts:**
- Deleted agent files per Table T03

**Tasks:**
- [ ] Delete `agents/specks-clarifier.md`
- [ ] Delete `agents/specks-critic.md`
- [ ] Delete `agents/specks-monitor.md` (eliminated, no skill replacement)
- [ ] Delete `agents/specks-reviewer.md`
- [ ] Delete `agents/specks-auditor.md`
- [ ] Delete `agents/specks-logger.md`
- [ ] Delete `agents/specks-committer.md`
- [ ] **Rename remaining agents** to remove `specks-` prefix (per #naming-conventions):
  - [ ] `mv agents/specks-director.md agents/director.md`
  - [ ] `mv agents/specks-planner.md agents/planner.md`
  - [ ] `mv agents/specks-interviewer.md agents/interviewer.md`
  - [ ] `mv agents/specks-architect.md agents/architect.md`
  - [ ] `mv agents/specks-implementer.md agents/implementer.md`
- [ ] Update frontmatter `name:` field in each renamed agent (e.g., `name: director` not `name: specks-director`)

**Checkpoint:**
- [ ] Only 5 agent files remain: director, planner, interviewer, architect, implementer
- [ ] `ls agents/*.md | wc -l` returns 5
- [ ] No `agents/specks-*.md` files exist

**Rollback:**
- Restore from git

**Commit after all checkpoints pass.**

---

#### Step 7: Remove legacy skill directories {#step-7}

**Depends on:** #step-1

**Commit:** `refactor(skills): remove legacy .claude/skills directory`

**References:** [D06] Clean breaks, Table T02, (#files-to-remove)

**Artifacts:**
- Deleted `.claude/skills/` directory

**Tasks:**
- [ ] Delete `.claude/skills/specks-plan/` (moved to `skills/plan/`)
- [ ] Delete `.claude/skills/specks-execute/` (moved to `skills/execute/`)
- [ ] Delete `.claude/skills/implement-plan/` (CLI infrastructure, no replacement needed)
- [ ] Delete `.claude/skills/update-plan-implementation-log/` (replaced by `skills/logger/`)
- [ ] Delete `.claude/skills/prepare-git-commit-message/` (replaced by `skills/committer/`)
- [ ] Delete `.claude/skills/` directory if empty

**Checkpoint:**
- [ ] `.claude/skills/` directory does not exist
- [ ] `skills/` at repo root contains all 8 skills

**Rollback:**
- Restore from git

**Commit after all checkpoints pass.**

---

#### Step 8: Refactor CLI (remove orchestration, add beads close) {#step-8}

**Depends on:** #step-4-4

**Commit:** `refactor(cli): remove orchestration commands, add beads close`

**References:** [D05] CLI becomes utility only, Table T01, (#files-to-remove), (#beads-contract)

**Artifacts:**
- Deleted files per Table T01
- Updated `crates/specks/src/commands/mod.rs`
- Updated `crates/specks/src/cli.rs`
- Updated `crates/specks/src/main.rs`
- New `specks beads close` subcommand

**Commit strategy:** Complete all substeps 8.1-8.6, verify all checkpoints pass, then make a single commit with the message above. Substeps 8.1-8.5 remove orchestration code; substep 8.6 adds the beads close command needed by the committer skill.

##### Step 8.1: Remove planning_loop module {#step-8-1}

**Tasks:**
- [ ] Delete `crates/specks/src/planning_loop/` directory entirely
- [ ] Remove `mod planning_loop;` declaration

**Checkpoint:**
- [ ] `cargo build` succeeds

---

##### Step 8.2: Remove interaction module {#step-8-2}

**Depends on:** #step-8-1

**Tasks:**
- [ ] Delete `crates/specks/src/interaction/` directory entirely
- [ ] Remove `mod interaction;` declaration

**Checkpoint:**
- [ ] `cargo build` succeeds

---

##### Step 8.3: Remove streaming and share modules {#step-8-3}

**Depends on:** #step-8-2

**Tasks:**
- [ ] Delete `crates/specks/src/streaming.rs`
- [ ] Delete `crates/specks/src/share.rs`
- [ ] Remove module declarations

**Checkpoint:**
- [ ] `cargo build` succeeds

---

##### Step 8.4: Remove plan, execute, setup commands {#step-8-4}

**Depends on:** #step-8-3

**Tasks:**
- [ ] Delete `crates/specks/src/commands/plan.rs`
- [ ] Delete `crates/specks/src/commands/execute.rs`
- [ ] Delete `crates/specks/src/commands/setup.rs`
- [ ] Remove `mod plan;`, `mod execute;`, `mod setup;` from commands/mod.rs
- [ ] Remove Plan, Execute, Setup variants from Commands enum in cli.rs
- [ ] Remove match arms in main.rs
- [ ] Remove tests referencing these commands

**Checkpoint:**
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo nextest run` passes

---

##### Step 8.5: Clean up unused dependencies {#step-8-5}

**Depends on:** #step-8-4

**Tasks:**
- [ ] Remove unused dependencies from Cargo.toml (inquire, indicatif, owo-colors, ctrlc, etc.)
- [ ] Remove agent.rs if no longer needed
- [ ] Run `cargo build` to verify no missing dependencies

**Checkpoint:**
- [ ] `cargo build` succeeds with no warnings
- [ ] No unused import warnings

---

##### Step 8.6: Add specks beads close subcommand {#step-8-6}

**Depends on:** #step-8-5

**Tasks:**
- [ ] Create `crates/specks/src/commands/beads/close.rs`
- [ ] Add `Close` variant to `BeadsCommands` enum in `mod.rs`
- [ ] Implement `run_close(bead_id, reason, json_output)` function
- [ ] Use `BeadsCli.close()` with proper error handling
- [ ] Return JSON output matching beads contract schema

**Checkpoint:**
- [ ] `specks beads close --help` shows the command with `--reason` and `--json` flags
- [ ] `specks beads close bd-test-123 --json` returns valid JSON
- [ ] `specks beads close bd-test-123 --reason "Step completed" --json` works
- [ ] `cargo build` succeeds with no warnings

**Rollback:**
- Revert git changes

**Commit after all checkpoints pass.**

---

#### Step 8 Summary {#step-8-summary}

After completing Steps 8.1-8.6:
- All Rust orchestration code removed
- CLI has only utility commands (init, validate, list, status, beads)
- No plan, execute, or setup commands
- New `specks beads close` subcommand added for committer skill

**Final Step 8 Checkpoint:**
- [ ] `specks plan` returns error (unknown command)
- [ ] `specks execute` returns error (unknown command)
- [ ] `specks setup` returns error (unknown command)
- [ ] `specks --help` shows only init, validate, list, status, beads, version
- [ ] `specks beads close --help` shows the close subcommand

---

#### Step 9: Update documentation {#step-9}

**Depends on:** #step-8

**Commit:** `docs: update for Claude Code plugin architecture`

**References:** [D01] Specks is a Claude Code plugin, (#context, #strategy)

**Artifacts:**
- Updated `CLAUDE.md`
- Updated `README.md`

**Tasks:**
- [ ] Update CLAUDE.md agent list (5 agents, not 11)
- [ ] Update CLAUDE.md to mention skills
- [ ] Remove references to `specks plan`, `specks execute`, `specks setup claude`
- [ ] Document `/specks:plan` and `/specks:execute` as primary interface
- [ ] Document `claude --plugin-dir .` for development
- [ ] Update README installation instructions
- [ ] Add a "Beads readiness checklist" section (CLI install, bd install, SPECKS_BD_PATH)
- [ ] Document error messages and next steps when `specks` or `bd` is missing

**Checkpoint:**
- [ ] CLAUDE.md reflects new architecture
- [ ] README documents plugin installation and beads readiness

**Rollback:**
- Revert from git

**Commit after all checkpoints pass.**

---

#### Step 10: Verify plugin works {#step-10}

**Depends on:** #step-9

**Commit:** N/A (verification only)

**References:** (#beads-contract), (#orchestration-flowcharts), (#run-directory)

**Tasks:**
- [ ] Run `claude --plugin-dir .` from repo root
- [ ] Verify `/specks:plan` skill appears in `/help`
- [ ] Verify `/specks:execute` skill appears in `/help`
- [ ] Verify `specks:director` agent appears in `/agents`
- [ ] Test invoking `/specks:plan "test idea"`

**Skill tool syntax verification (HARD GATE):**
- [ ] Test minimal Skill invocation from director: `Skill(skill: "specks:clarifier")`
- [ ] Record exact working syntax in run artifacts
- [ ] If syntax differs from plan, STOP and update (#naming-conventions) and all director references before proceeding
- [ ] Verify Task tool syntax: `Task(subagent_type: "specks:director")`

**Flow verification (per #orchestration-flowcharts):**
- [ ] Director uses Skill tool for: clarifier, critic, reviewer, auditor, logger, committer
- [ ] Director uses Task tool for: planner, interviewer, architect, implementer
- [ ] Implementer includes self-monitoring for drift detection
- [ ] Interviewer handles ALL user interaction (director never uses AskUserQuestion)
- [ ] Planning flow matches (#flow-planning)
- [ ] Execution flow matches (#flow-execution)

**Run directory verification (per #run-directory):**
- [ ] After `/specks:plan "test idea"`, verify `.specks/runs/<session-id>/` created
- [ ] Verify `metadata.json` exists with correct schema
- [ ] Verify `planning/` subdirectory contains skill outputs (clarifier, critic, etc.)
- [ ] Skill outputs are valid JSON matching their output specs

**Beads integration verification (per #beads-contract):**
- [ ] Test agent can call `specks beads status --json` via Bash in plugin context
- [ ] Verify graceful error when `bd` not installed (unset `SPECKS_BD_PATH`, remove bd from PATH)
- [ ] Verify graceful error when `specks` CLI not installed or not on PATH
- [ ] Verify `SPECKS_BD_PATH` override works in plugin context

**Checkpoint:**
- [ ] Plugin loads without errors
- [ ] All 8 skills discoverable
- [ ] All 5 agents discoverable
- [ ] `/specks:plan` can be invoked
- [ ] Orchestration flows match flowcharts
- [ ] Run directory created with audit trail
- [ ] Beads CLI callable from plugin context with JSON output parsed

**Rollback:**
- N/A (verification step)

---

### 3.0.6 Deliverables and Checkpoints {#deliverables}

**Deliverable:** Specks as a Claude Code plugin with pure orchestrator director and skill-based analysis.

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

All agents and skills per (#agents-skills-summary) are implemented and functional.

- [ ] `.claude-plugin/plugin.json` exists with valid manifest
- [ ] 5 agent definitions exist in `agents/` per (#agent-summary)
- [ ] 8 skill directories exist in `skills/` per (#skill-summary)
- [ ] Director has tools: Task, Skill, Read, Grep, Glob, Bash, Write (no Edit, AskUserQuestion)
- [ ] CLI has no plan, execute, or setup commands
- [ ] `.claude/skills/` directory removed
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo nextest run` passes all tests
- [ ] `claude --plugin-dir .` loads specks as a plugin
- [ ] `/specks:plan` and `/specks:execute` work

#### Milestones (Within Phase) {#milestones}

**Milestone M01: Plugin Structure Created** {#m01-plugin-created}
- [ ] Plugin manifest exists
- [ ] All 8 skills created in `skills/`
- Steps 0-3 complete

**Milestone M02: Agents Updated** {#m02-agents-updated}
- [ ] Director is pure orchestrator with Skill tool
- [ ] 7 agent files removed (6 became skills, 1 eliminated)
- [ ] 5 agents remain (implementer includes self-monitoring)
- Steps 4-6 complete

**Milestone M03: Legacy Removed** {#m03-legacy-removed}
- [ ] `.claude/skills/` removed
- [ ] Rust orchestration code removed
- Steps 7-8 complete

**Milestone M04: Documentation Complete** {#m04-docs-complete}
- [ ] All docs updated
- [ ] Plugin verified working
- Steps 9-10 complete

#### Roadmap / Follow-ons (Explicitly Not Required for Phase Close) {#roadmap}

- [ ] Phase 4: Test full planning loop inside Claude Code
- [ ] Phase 5: Test full execution loop inside Claude Code
- [ ] Create public plugin marketplace for specks distribution
- [ ] Performance benchmarking

| Checkpoint | Verification |
|------------|--------------|
| Plugin manifest | `.claude-plugin/plugin.json` exists and is valid JSON |
| Skills count | `ls skills/*/SKILL.md \| wc -l` returns 8 |
| Agents count | `ls agents/*.md \| wc -l` returns 5 |
| Director tools | `grep "^tools:" agents/director.md` has Write but no Edit/AskUserQuestion |
| CLI simplified | `specks --help` shows no plan/execute/setup |
| Build clean | `cargo build` with no warnings |
| Tests pass | `cargo nextest run` passes |
| Plugin loads | `claude --plugin-dir .` succeeds |
| Run directory | `.specks/runs/<session-id>/` created with metadata.json and skill outputs |
| Beads callable | Agent can invoke `specks beads status --json` via Bash and parse output |

**Commit after all checkpoints pass.**
