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

This phase defines **2 agents and 12 skills** (**two-agent orchestrator architecture**). Two thin entry skills (`planner` and `implementer`) spawn their corresponding orchestrator agents (`planner-agent`, `implementer-agent`) via Task. No director agent. No nested agents. All other work happens in sub-task skills invoked by the orchestrator agents.

**Architecture principle:** Two orchestrator agents run the loops; skills are pure sub-tasks. **No nesting. No escalation.** Maximum ONE agent context active at any time.

**Terminology:** A `task` refers to either a skill or an agent. Sub-tasks are the workers that orchestrators invoke.

#### Agents {#agent-summary}

Agents have `-agent` suffix to distinguish them from skills.

| Agent | Remit |
|-------|-------|
| **planner-agent** | Orchestrator agent for the planning loop. Invokes planning sub-task skills sequentially, persists the run audit trail, syncs beads. |
| **implementer-agent** | Orchestrator agent for the implementation loop. Invokes execution sub-task skills sequentially, persists the run audit trail, gates drift, manages commit policy. |

**Agent count:** 2 agent files total (planner-agent, implementer-agent).

#### Skills {#skill-summary}

**Entry points (thin wrappers):**

| Skill | Spec | Remit |
|-------|------|-------|
| **planner** | S01 | ENTRY WRAPPER. Entry point `/specks:planner`. Spawns `planner-agent` via Task and returns its final JSON output. |
| **implementer** | S02 | ENTRY WRAPPER. Entry point `/specks:implementer`. Spawns `implementer-agent` via Task and returns its final JSON output. |

**Sub-tasks (skills only; invoked by orchestrator agents):**

| Skill | Spec | Remit |
|-------|------|-------|
| **clarifier** | S03 | Analyzes idea or critic feedback. Returns clarifying questions with options. JSON output. |
| **critic** | S04 | Reviews speck for skeleton compliance, completeness, implementability. Returns APPROVE/REVISE/REJECT. JSON output. |
| **author** | S11 | Creates and revises speck documents. Writes structured markdown to `.specks/`. JSON output. |
| **architect** | S10 | Creates implementation strategies for steps. Returns JSON with expected touch set. Read-only. |
| **coder** | S12 | Executes architect strategies with drift detection. Writes code, runs tests. JSON output. |
| **reviewer** | S05 | Verifies completed step matches plan. Checks tasks, tests, artifacts. Returns APPROVE/REVISE/ESCALATE. JSON output. |
| **auditor** | S06 | Checks code quality, error handling, security. Returns severity-ranked issues. JSON output. |
| **logger** | S07 | Updates `.specks/specks-implementation-log.md` with completed work. JSON output. |
| **committer** | S08 | Finalizes step: stages files, commits changes, closes associated bead. JSON output. |
| **interviewer** | S09 | Single point of user interaction. Presents questions/feedback via AskUserQuestion. Returns structured decisions. |

*Note: `plan` and `execute` entry points are REPLACED by `planner` and `implementer` (not renamed—completely new orchestration skills).*

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
- Namespacing: `/specks:planner`, `specks:planner-agent`, etc.
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
- `skills/` directory at repo root contains 12 skills per (#skill-summary)
- `agents/` directory at repo root contains 2 agents per (#agent-summary)
- `claude --plugin-dir .` loads specks as a plugin with all skills/agents available
- `/specks:planner` and `/specks:implementer` orchestration skills work
- Two-agent orchestrator architecture verified (no nesting; orchestrator agents invoke sub-task skills sequentially)
- Maximum 1 agent context active at any time
- `specks plan` and `specks execute` CLI commands removed
- `specks setup claude` command removed
- `.claude/skills/` directory removed (replaced by `skills/`)
- `cargo build` succeeds with no warnings
- `cargo nextest run` passes all tests

#### Scope {#scope}

1. Create `.claude-plugin/plugin.json` manifest
2. Create `skills/` directory with 12 skills:
   - 2 thin entry wrappers: `planner`, `implementer`
   - 10 sub-task skills: architect, auditor, author, clarifier, coder, committer, critic, interviewer, logger, reviewer
3. Create orchestrator agent definitions at `agents/` (2 agents with `-agent` suffix: planner-agent, implementer-agent)
4. Delete director agent, archive old agent files
5. Remove Rust orchestration code (plan.rs, execute.rs, planning_loop/, streaming.rs, interaction/)
6. Remove `specks setup claude` command and share.rs
7. Remove `.claude/skills/` directory (legacy skills)
8. Update documentation

#### Non-goals (Explicitly out of scope) {#non-goals}

- Creating a public plugin marketplace (future work)
- Adding new features beyond architecture simplification
- Changing the speck file format
- Modifying beads integration beyond validation, error handling, and onboarding docs
- Testing the full planning/implementation loop (Phase 4)

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
3. Beads is **required**: halt the current orchestration (planner-agent or implementer-agent) until beads is ready
4. Treat non-JSON or invalid JSON output as an error, report it, and halt

---

**Install contract for plugin users:**

| Capability | Requirements |
|-----------|-------------|
| Planning (planner-agent) | Plugin + `specks` CLI on PATH + `bd` binary + `.beads/` initialized |
| Execution (implementer-agent) | Plugin + `specks` CLI on PATH + `bd` binary + `.beads/` initialized |

**Onboarding and ergonomics requirements:**
- Provide a single "beads readiness" checklist in docs
- Include actionable error messages when `specks` or `bd` is missing
- Document how to set `SPECKS_BD_PATH` and verify with `specks beads status --json`
  - Include `bd onboard` as the first step for new environments

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

**Resolution:** DECIDED - use `specks`. Skills become `/specks:planner`, `/specks:implementer`, etc. Agents become `specks:planner-agent`, `specks:implementer-agent`.

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

#### [D02] Director is pure orchestrator (SUPERSEDED by D08) {#d02-pure-orchestrator}

**⚠️ SUPERSEDED:** This decision described the director agent as a pure orchestrator. D08 (Dual-orchestrator architecture) eliminates the director entirely. Orchestration is now handled by two top-level skills: `planner` and `implementer`. See D08 for the current architecture.

**Original decision (historical):** The director agent only coordinates via Task tool and Skill tool. It writes only audit trail files (run directory), never edits files, and never interacts with users directly.

**Rationale (historical):**
- Keeps orchestration logic separate from work execution
- All file operations delegated to specialists (planner, implementer)
- All user interaction delegated to interviewer

**Implications (historical, now superseded):**
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

**Implications (updated for D08 architecture):**
- 6 agents become skills per original (#skill-summary): clarifier, critic, reviewer, auditor, logger, committer
- After Step 10.5: 2 agents remain with `-agent` suffix: planner-agent, implementer-agent
- Drift detection lives in the coder skill (with an additional outer drift gate in implementer-agent) (see #smart-drift)
- Skills specify `allowed-tools` as needed (all get baseline Read, Grep, Glob plus additional tools)
- Skills return JSON-only output

---

#### [D04] Interviewer handles all user interaction (DECIDED, updated for D08) {#d04-interviewer-role}

**Decision:** The interviewer is the single point of user interaction. Orchestrators pass data to interviewer, interviewer presents via AskUserQuestion, returns user decisions.

**Rationale:**
- Orchestrators stay focused on coordination (not user interaction)
- User interaction logic consolidated in one place

**Implications (updated for D08):**
- Interviewer is a **skill** (AskUserQuestion works from skill context - verified)
- Interviewer receives questions from clarifier skill, results from critic skill
- Interviewer returns structured decisions to orchestrator (planner or implementer)
- Orchestrators invoke via `Skill(skill: "specks:interviewer")`

---

#### [D05] CLI becomes utility only (DECIDED) {#d05-cli-utility}

**Decision:** Remove `plan` and `execute` CLI commands. Remove `setup claude` command. Keep: init, validate, list, status, beads, version.

**Rationale:**
- Planning and execution happen inside Claude Code via `/specks:planner` and `/specks:implementer`
- Plugin system handles skill/agent distribution (no need for setup command)
- Eliminates process spawning overhead
- **Beads integration stays in CLI** - it's operational tooling (like git), not orchestration
- Orchestrators can call beads CLI commands via Bash when needed

**What stays (unchanged):**
- `specks init` - Initialize project
- `specks validate` - Validate specks
- `specks list` - List specks
- `specks status` - Show progress
- `specks beads sync|link|status|pull|close` - Beads integration
- `specks version` - Show version

**What goes:**
- `specks plan` → replaced by `/specks:planner` orchestration skill
- `specks execute` → replaced by `/specks:implementer` orchestration skill
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

**⚠️ HISTORICAL (superseded by D08):** This decision described Skill tool usage from the interim director-based architecture. Under D08, the **planner** and **implementer** orchestration skills invoke sub-task skills via the Skill tool.

**Rationale (verified from docs):**
- "The Skill tool" exists and can be controlled via permissions: `Skill(name)` or `Skill(name *)`
- Skills can be auto-loaded when description matches context
- Subagents can preload skills via `skills` frontmatter field

**Implications:**
- HISTORICAL: Director used Skill tool to invoke analysis skills (clarifier, critic, etc.)
- Skill outputs are JSON-only for easy parsing
- HISTORICAL: Director spawned agents via Task tool for write operations

**Reference:** https://code.claude.com/docs/en/skills

---

#### [D08] Dual-orchestrator architecture (DECIDED) {#d08-dual-orchestrator}

**Decision (UPDATED by Step 10.5):** Replace director agent with two top-level entry skills (`planner`, `implementer`) that spawn two orchestrator agents (`planner-agent`, `implementer-agent`). No director. No nested agents. Skills are pure sub-tasks; there is no skill-to-agent escalation.

**Rationale:**
- The original multi-agent design created 11+ nested agent contexts during execution
- Claude Code terminal rendering cannot handle this many concurrent contexts
- Crashes with "Aborted()" message due to rendering overload ("High write ratio: 100% writes")
- Director was just a router—orchestration logic collapses into the two entry points
- Both planning and implementation loops are straightforward; no need for agentic complexity

**Architecture:**
```
Entry Points (thin skills):
  /specks:planner     → spawns planner-agent via Task
  /specks:implementer → spawns implementer-agent via Task

Orchestrator Agents (2, never nested):
  planner-agent       → runs planning loop; invokes skills
  implementer-agent   → runs implementation loop; invokes skills

Sub-task skills (10, invoked by agents):
  architect, auditor, author, clarifier, coder, committer, critic, interviewer, logger, reviewer
```

**Naming:**
- `/specks:planner` and `/specks:implementer` are the user-facing entry skills
- `planner-agent` and `implementer-agent` are the only agents (orchestration loops live here)
- `author` and `coder` are skill-only sub-tasks (no agent variants)
- All agents have `-agent` suffix

**No escalation:**
- Orchestrator agents invoke skills sequentially (one at a time)
- Orchestrator agents never spawn other agents (prevents nesting / Aborted())

**Benefits:**
- Greatly simplified design (no director, no nesting)
- Completely avoids "Aborted()" crash (max 1 agent context at any time)
- Any sub-task can be invoked directly via slash command for dev flexibility

**Testing:**
- Step 10.5.1 verified AskUserQuestion works from skill context
- Interviewer is a skill (no agent fallback needed)

**Reference:** Step 10.5 in this speck

---

### 3.0.1 Plugin Structure {#plugin-structure}

#### Naming Conventions {#naming-conventions}

**The plugin provides the namespace.** Per Claude Code plugin docs, plugin components are accessed as `plugin-name:component-name`.

| Resource | File/Folder Name | Frontmatter Name | User Invocation | Tool Invocation |
|----------|-----------------|------------------|-----------------|-----------------|
| Skill | `skills/clarifier/SKILL.md` | `name: clarifier` | `/specks:clarifier` | `Skill(skill: "specks:clarifier")` |
| Agent | `agents/planner-agent.md` | `name: planner-agent` | N/A | `Task(subagent_type: "specks:planner-agent")` |
| Agent | `agents/implementer-agent.md` | `name: implementer-agent` | N/A | `Task(subagent_type: "specks:implementer-agent")` |

**Agent naming rule:** All agents have `-agent` suffix to distinguish from skill counterparts.
- `planner-agent`, `implementer-agent` (required)

**Namespacing rule (applies to BOTH skills AND agents):**
- Plugin name `specks` provides the namespace prefix automatically
- Skill folder `skills/planner/` becomes `/specks:planner`
- Agent file `agents/planner-agent.md` becomes `specks:planner-agent`
- Both Skill and Task tools use the colon-namespaced format

**Consistency rule:** Always use fully-qualified namespaced names in code:
- `Skill(skill: "specks:clarifier")` not `Skill(skill: "clarifier")`
- `Task(subagent_type: "specks:planner-agent")` not `Task(subagent_type: "planner-agent")`

**Syntax verification:** The exact Skill tool invocation syntax (`Skill(skill: "specks:clarifier")`) is verified in Step 10. If the actual syntax differs, update this section and all references before proceeding past Step 10. Step 10 is the hard gate for syntax correctness.

**Plugin directory layout:**

```
specks/                           # Plugin root (repo root)
├── .claude-plugin/
│   └── plugin.json              # Plugin manifest
├── skills/                       # Skills (auto-discovered)
│   ├── planner/                 # ENTRY WRAPPER - spawns planner-agent
│   │   └── SKILL.md
│   ├── implementer/             # ENTRY WRAPPER - spawns implementer-agent
│   │   └── SKILL.md
│   ├── architect/               # Sub-task (skill-only)
│   │   └── SKILL.md
│   ├── author/                  # Sub-task (skill-only)
│   │   └── SKILL.md
│   ├── coder/                   # Sub-task (skill-only)
│   │   └── SKILL.md
│   ├── auditor/
│   │   └── SKILL.md
│   ├── clarifier/
│   │   └── SKILL.md
│   ├── committer/
│   │   └── SKILL.md
│   ├── critic/
│   │   └── SKILL.md
│   ├── interviewer/
│   │   └── SKILL.md
│   ├── logger/
│   │   └── SKILL.md
│   └── reviewer/
│       └── SKILL.md
├── agents/                       # Agents (auto-discovered, namespaced as specks:*)
│   ├── planner-agent.md         # Orchestrator agent for planning loop (skills only)
│   ├── implementer-agent.md     # Orchestrator agent for implementation loop (skills only)
│   └── (no other agents; prevents nesting / Aborted())
├── agents/archived/              # Old/replaced agent files
│   ├── director.md              # Archived after Step 10.5
│   └── interviewer.md           # Archived after Step 10.5 (skill-only now)
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

These flowcharts define the orchestration logic for the two orchestrator agents (`planner-agent`, `implementer-agent`). The entry skills (`/specks:planner`, `/specks:implementer`) are thin wrappers that spawn the corresponding agent.

**Legend:**
- `[AGENT]` = orchestrator agent spawned via Task tool (isolated context; runs until done)
- `(SKILL)` = sub-task invoked via Skill tool (JSON-only output)
- `{USER}` = interaction via interviewer skill (AskUserQuestion), invoked by the orchestrator agent

#### Planning Phase Flow {#flow-planning}

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         PLANNING PHASE                                       │
│                                                                              │
│  User invokes /specks:planner "idea" or /specks:planner path/to/speck.md    │
│                                                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────┐                                                                │
│  │  INPUT   │  idea text OR existing speck path                              │
│  └────┬─────┘                                                                │
│       │                                                                      │
│       ▼                                                                      │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ /specks:planner entry skill spawns [PLANNER-AGENT] via Task           │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│       │                                                                      │
│       ▼                                                                      │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ [PLANNER-AGENT] runs planning loop autonomously                       │   │
│  │                                                                       │   │
│  │ 1. Invoke (CLARIFIER) skill                                           │   │
│  │    → Returns: analysis{}, questions[], assumptions[]                  │   │
│  │                                                                       │   │
│  │ 2. IF questions exist:                                                │   │
│  │    → Invoke (INTERVIEWER) skill with questions                        │   │
│  │    → Interviewer uses AskUserQuestion                                 │   │
│  │    → Returns: user_answers{}                                          │   │
│  │                                                                       │   │
│  │ 3. Invoke (AUTHOR) skill with:                                        │   │
│  │    - Original idea/speck                                              │   │
│  │    - User answers (if any)                                            │   │
│  │    - Clarifier assumptions                                            │   │
│  │    → Returns: draft speck path                                        │   │
│  │                                                                       │   │
│  │ 4. Invoke (CRITIC) skill with draft speck                             │   │
│  │    → Returns: skeleton_compliant, areas{}, issues[], recommendation   │   │
│  │                                                                       │   │
│  │ 5. IF recommendation == REJECT or REVISE:                             │   │
│  │    → Invoke (INTERVIEWER) skill with critic issues                    │   │
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
- /specks:planner is a thin entry skill; orchestration happens in [PLANNER-AGENT]
- No nesting: [PLANNER-AGENT] invokes skills only; it never spawns other agents
- One sub-task at a time (sequential invocation)
- Maximum 1 agent context at any moment
- Loop continues until critic approves OR user accepts

---

#### Implementation Phase Flow {#flow-implementation}

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         IMPLEMENTATION PHASE                                      │
│                                                                              │
│  User invokes /specks:implementer path/to/speck.md                           │
│                                                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ /specks:implementer entry skill spawns [IMPLEMENTER-AGENT] via Task   │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│       │                                                                      │
│       ▼                                                                      │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ STEP 1: Get Implementation Strategy                                   │   │
│  │                                                                       │   │
│  │ [IMPLEMENTER-AGENT] invokes (ARCHITECT) skill with step details       │   │
│  │ → Returns: strategy, expected_touch_set[], test_plan                  │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│       │                                                                      │
│       ▼                                                                      │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ STEP 2: Implementation (with Self-Monitoring)                         │   │
│  │                                                                       │   │
│  │ Invoke (CODER) skill with architect strategy                          │   │
│  │ → Coder reads strategy, writes code, runs tests                       │   │
│  │ → Coder self-monitors against expected_touch_set (see #smart-drift)   │   │
│  │ → Returns: success/failure + drift_assessment                         │   │
│  │                                                                       │   │
│  │ Implementer-agent performs an additional drift gate:                  │   │
│  │ - Compare coder.files_* to architect.expected_touch_set               │   │
│  │ - If drift exceeds thresholds, HALT and invoke interviewer            │   │
│  │                                                                       │   │
│  │ IF coder.halted_for_drift:                                            │   │
│  │   → Invoke (INTERVIEWER) skill with drift details                     │   │
│  │   → User decides: continue anyway? back to architect? abort?          │   │
│  │   → IMPLEMENTER-AGENT acts on user decision                            │   │
│  │                                                                       │   │
│  │ IF coder.success == false (non-drift failure):                        │   │
│  │   → Handle error, may retry coder or return to architect               │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│       │                                                                      │
│       ▼                                                                      │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ STEP 3: Review + Audit                                                │   │
│  │                                                                       │   │
│  │ SEQUENTIAL INVOCATION (one at a time):                                │   │
│  │                                                                       │   │
│  │   (REVIEWER) skill                 (AUDITOR) skill                    │   │
│  │   ├─ Checks plan adherence         ├─ Checks code quality             │   │
│  │   ├─ Tasks completed?              ├─ Performance concerns?           │   │
│  │   ├─ Tests match plan?             ├─ Security issues?                │   │
│  │   ├─ Artifacts produced?           ├─ Conventions followed?           │   │
│  │   └─ Returns: APPROVE|REVISE|      └─ Returns: APPROVE|FIX_REQUIRED|  │   │
│  │              ESCALATE                         MAJOR_REVISION          │   │
│  │                                                                       │   │
│  │ IMPLEMENTER evaluates both reports                                    │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│       │                                                                      │
│       ▼                                                                      │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ STEP 4: Resolution                                                    │   │
│  │                                                                       │   │
│  │ IF issues found:                                                      │   │
│  │   ├─ Minor quality issues → Re-invoke (CODER)                         │   │
│  │   ├─ Design issues → Back to (ARCHITECT)                              │   │
│  │   └─ Conceptual issues → Invoke (INTERVIEWER), may need re-planning   │   │
│  │                                                                       │   │
│  │ IF both reports clean:                                                │   │
│  │   1. Invoke (LOGGER) skill → Updates implementation log               │   │
│  │   2. Invoke (COMMITTER) skill → Commits changes (per commit policy)   │   │
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
- /specks:implementer is a thin entry skill; orchestration happens in [IMPLEMENTER-AGENT]
- No nesting: [IMPLEMENTER-AGENT] invokes skills only; it never spawns other agents
- One sub-task at a time (sequential invocation, no parallelism)
- Maximum 1 agent context at any moment
- Coder includes **self-monitoring** for drift detection (see #smart-drift)
- ALL drift/issue decisions that require user input go through interviewer
- Logger and committer are invoked after each successful step

---

#### Tool Invocation Summary {#flow-tools}

**Entry points (thin wrappers):**

| Component | Type | User Invocation | Purpose |
|-----------|------|-----------------|---------|
| **planner** | Skill | `/specks:planner <args>` | Spawns `planner-agent` via Task |
| **implementer** | Skill | `/specks:implementer <args>` | Spawns `implementer-agent` via Task |

**Orchestrator agents:**

| Component | Type | Invocation | Purpose |
|-----------|------|------------|---------|
| **planner-agent** | Agent | `Task(subagent_type: "specks:planner-agent")` | Runs planning loop; invokes sub-task skills |
| **implementer-agent** | Agent | `Task(subagent_type: "specks:implementer-agent")` | Runs implementation loop; invokes sub-task skills |

**Sub-task skills (invoked by orchestrator agents):**

| Component | Invocation | Purpose |
|-----------|------------|---------|
| **auditor** | `Skill(skill: "specks:auditor")` | Code quality/security checks |
| **architect** | `Skill(skill: "specks:architect")` | Implementation strategies |
| **author** | `Skill(skill: "specks:author")` | Creates/revises speck documents |
| **clarifier** | `Skill(skill: "specks:clarifier")` | Generates clarifying questions |
| **coder** | `Skill(skill: "specks:coder")` | Executes strategies with drift detection |
| **committer** | `Skill(skill: "specks:committer")` | Git commit operations |
| **critic** | `Skill(skill: "specks:critic")` | Reviews speck quality |
| **interviewer** | `Skill(skill: "specks:interviewer")` | All user interaction |
| **logger** | `Skill(skill: "specks:logger")` | Updates implementation log |
| **reviewer** | `Skill(skill: "specks:reviewer")` | Verifies step completion |

*Note: AskUserQuestion works from skill context. Interviewer is skill-only (no agent variant needed).*

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
│   ├── 003-author.json        # Draft speck produced
│   ├── 004-critic.json        # Quality review
│   └── ...                    # Numbered by invocation order
└── execution/                 # Execution phase artifacts
    ├── step-0/
    │   ├── architect.json     # Implementation strategy
    │   ├── coder.json         # Code changes made (includes drift_assessment)
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
| **planner-agent** | Write, Bash | Creates run directory, writes metadata.json and all sub-task outputs for planning phase |
| **implementer-agent** | Write, Bash | Creates run directory, writes metadata.json and all sub-task outputs for execution phase |
| **planner (entry skill)** | Task | Spawns planner-agent only (no writes) |
| **implementer (entry skill)** | Task | Spawns implementer-agent only (no writes) |
| **author** | Write | Writes draft speck to `.specks/` (not runs/) |
| **architect** | Read | Reads speck; orchestrator persists strategy to runs/ |
| **All sub-tasks** | Varies | Return JSON to orchestrator; orchestrator persists to runs/ |

**Key design:** Sub-tasks return JSON to their orchestrator agent. The orchestrator agent persists all outputs to the runs directory. This keeps sub-tasks focused and makes the orchestrator agent the single source of truth for the audit trail.

#### Session ID Format {#session-id}

Format: `YYYYMMDD-HHMMSS-<mode>-<short-uuid>`

Examples:
- `20260206-143022-plan-a1b2c3`
- `20260206-150145-impl-d4e5f6`

**Generation method:** The orchestrator generates session ID at start via Bash using `/dev/urandom` (portable across macOS and Linux):

```bash
# Generate session ID (MODE is "plan" or "impl")
SHORT_UUID="$(
  uuidgen 2>/dev/null \
    | tr -d '-' \
    | tr '[:upper:]' '[:lower:]' \
    | cut -c1-6 \
  || hexdump -n 3 -e '3/1 "%02x"' /dev/urandom
)"
SESSION_ID="$(date +%Y%m%d-%H%M%S)-${MODE}-${SHORT_UUID}"
```

*Note:* Prefers `uuidgen` for simplicity; falls back to `hexdump` + `/dev/urandom` if `uuidgen` is unavailable. `date +%N` not used because it's unsupported on macOS.

#### Metadata Schema {#run-metadata}

```json
{
  "session_id": "20260206-143022-plan-a1b2c3",
  "mode": "plan",
  "started_at": "2026-02-06T14:30:22Z",
  "last_updated_at": "2026-02-06T14:31:10Z",
  "speck_path": ".specks/specks-3.md",
  "idea": "string | null",
  "commit_policy": "auto|manual | null",
  "current_step": "#step-N | null",
  "status": "in_progress",
  "completed_at": null
}
```

#### JSON Persistence Pattern {#json-persistence}

Orchestrator agents write JSON to the runs directory using the **Write tool** (not Bash). This avoids all escaping issues and is the natural tool for file creation.

```
Write(file_path: ".specks/runs/20260206-143022-plan-a1b2c3/metadata.json", content: <json-string>)
```

**Why Write, not Bash:**
- Write tool handles content exactly as provided - no escaping needed
- More reliable than heredocs or echo for JSON with special characters
- Audit trail is orchestrator's responsibility - Write is appropriate
- Bash is reserved for: `mkdir -p` (directory creation), `uuidgen`/`date` (ID generation)

**Orchestrator tool usage:**
| Tool | Used for |
|------|----------|
| Bash | `mkdir -p .specks/runs/<session-id>/planning`, session ID generation |
| Write | All JSON file writes (metadata.json, skill outputs, agent outputs) |
| Read | Reading speck files, checking existing state |
| Skill | Invoking sub-tasks |
| Task | **Entry skills only**: spawning orchestrator agents (no agent nesting) |

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

**Entry points** (thin wrappers that spawn orchestrator agents):

| Skill | allowed-tools | Reason |
|-------|---------------|--------|
| **planner** | Task | Spawns `planner-agent` (no writes; no validation) |
| **implementer** | Task | Spawns `implementer-agent` (no writes; no validation) |

**Sub-task skills** (invoked by orchestrator agents):

| Skill | allowed-tools | Reason |
|-------|---------------|--------|
| **architect** | Read, Grep, Glob | Reads speck; returns strategy JSON |
| **auditor** | Read, Grep, Glob | Reads code for quality |
| **author** | Read, Grep, Glob, Write, Edit | Writes speck markdown to `.specks/` |
| **clarifier** | Read, Grep, Glob | Reads codebase for context |
| **coder** | Read, Grep, Glob, Write, Edit, Bash | Writes code and runs tests; emits drift_assessment |
| **committer** | Read, Grep, Glob, Bash | `git add`, `git commit`, `specks beads close` |
| **critic** | Read, Grep, Glob | Reads speck for review |
| **interviewer** | AskUserQuestion | Presents questions to user |
| **logger** | Read, Grep, Glob, Edit | Updates implementation log |
| **reviewer** | Read, Grep, Glob | Checks plan artifacts |

**Baseline:** All skills get `Read, Grep, Glob` for codebase access. Additional tools added only where needed.

#### Skill Tool Invocation Contract {#skill-invocation-contract}

**Invocation:** Orchestrator agents invoke sub-task skills via the Skill tool using the fully-qualified name and a JSON payload. Entry skills invoke Task only to spawn orchestrator agents.

**Skill invocation:**
```
Skill(skill: "specks:clarifier", args: '{"idea": "...", "speck_path": null}')
```

##### How Skills Work (Critical Context) {#how-skills-work}

**Skills work through prompt injection, not function calls.** This has important implications for orchestration:

| Mechanism | How It Works | Output Capture |
|-----------|--------------|----------------|
| **Skill tool** | Injects skill content into Claude's prompt as instructions | Output flows through conversation context, visible to Claude's reasoning |
| **Task tool** | Spawns isolated subagent with separate context | Agent returns structured output when complete |

**Key insight:** Skills are best treated as focused sub-tasks. For multi-step loops that must continue autonomously, use orchestrator agents (see Step 10.5).

**Implications for orchestration:**
1. **Skills don't need `Skill` in allowed-tools** - Claude decides which skills to use based on context and skill descriptions
2. **Output capture is implicit** - Sub-task skill output appears in the agent's context and can be parsed for the next decision
3. **JSON output convention** - Sub-tasks output JSON-only so Claude can parse it as it continues reasoning
4. **For orchestration loops** - Use Task tool to spawn an orchestrator agent (planner-agent / implementer-agent)

**Orchestrator output capture pattern:**
1. Orchestrator invokes sub-task skill
2. Sub-task skill outputs JSON (no prose, no fences)
3. JSON appears in conversation context
4. Orchestrator parses JSON and continues processing
5. Orchestrator persists output to run directory using Write tool

**Input format:**
- Provide only the inputs listed in each skill/agent spec
- Encode as JSON (preferred)
- Paths are repo-relative when possible

**Output format:**
- Output is JSON-only and must conform to the spec schema
- No surrounding prose, markdown, or code fences
- On failure, return a valid JSON object with:
  - `error`: "string" describing the failure
  - `recommendation`: "REVISE|PAUSE|HALT" (use the closest matching enum for the spec)

**Orchestrator parsing rules:**
- Reject any non-JSON output and re-run the sub-task once
- If the second run fails, invoke interviewer with the failure and halt the orchestration
- Persist the raw JSON in the run directory for audit

#### planner Skill (Entry Wrapper) {#skill-planner}

**Spec S01: planner** {#s01-planner}

```yaml
---
name: planner
description: Entry point for planning workflow (spawns planner-agent)
disable-model-invocation: true
allowed-tools: Task
---
```

**Behavior:**
1. Accepts `$ARGUMENTS` as either:
   - A preferred **string** form (idea text or speck path), optionally with flags
   - A **JSON object** string (when `$ARGUMENTS` starts with `{`) for future extensibility
2. Immediately spawns the orchestrator agent:
   ```
   Task(subagent_type: "specks:planner-agent", prompt: "$ARGUMENTS", description: "Run planning loop")
   ```
3. Returns the planner-agent's final JSON output (no additional processing, validation, or file writes).

---

#### implementer Skill (Entry Wrapper) {#skill-implementer}

**Spec S02: implementer** {#s02-implementer}

```yaml
---
name: implementer
description: Entry point for implementation workflow (spawns implementer-agent)
disable-model-invocation: true
allowed-tools: Task
---
```

**Behavior:**
1. Accepts `$ARGUMENTS` as either:
   - A preferred **string** form (speck path plus optional flags)
   - A **JSON object** string (when `$ARGUMENTS` starts with `{`) for future extensibility
2. Immediately spawns the orchestrator agent:
   ```
   Task(subagent_type: "specks:implementer-agent", prompt: "$ARGUMENTS", description: "Run implementation loop")
   ```
3. Returns the implementer-agent's final JSON output (no additional processing, validation, or file writes).

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
- `recommendation`: Action for implementer-agent:
  - `APPROVE`: Step complete, proceed to logger/committer
  - `REVISE`: Minor issues found, re-invoke coder with feedback
  - `ESCALATE`: Conceptual issues require user decision; invoke interviewer with review context

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
  "commit_policy": "auto|manual",
  "confirmed": false,
  "bead_id": "string | null",
  "close_reason": "string | null"
}
```
- `speck_path`: Path to the speck (for commit message context)
- `step_anchor`: Which step was completed
- `proposed_message`: Commit message from the step's `**Commit:**` line
- `files_to_stage`: Files to add (from implementer output)
- `commit_policy`: Commit mode:
  - `auto`: stage + commit + close bead immediately
  - `manual`: stage and prepare, but do NOT commit unless `confirmed: true`
- `confirmed`: Only meaningful when `commit_policy: "manual"`. If false, do not commit; return a "prepared" result for user review.
- `bead_id`: Bead ID to close after commit. If null, committer must return an error (beads are required).
- `close_reason`: Reason for closing bead (e.g., "Step completed per speck")

**Output JSON:**
```json
{
  "commit_message": "string",
  "files_staged": ["string"],
  "committed": true,
  "commit_hash": "string | null",
  "bead_closed": true,
  "bead_id": "string | null",
  "aborted": false,
  "reason": "string | null",
  "warnings": ["string"]
}
```
- `aborted`: True if the user rejected a manual commit; orchestrator must halt
- `reason`: Explanation when aborted (e.g., "User rejected prepared commit")
- `warnings`: Non-fatal issues encountered (e.g., bead already closed)

**Edge Case Handling:**

| Scenario | Behavior | Output |
|----------|----------|--------|
| `commit_policy: manual` and `confirmed: false` | Stage only; do not commit or close bead | `committed: false, commit_hash: null, bead_closed: false` |
| `commit_policy: manual` and user rejects | Abort step; user must provide guidance to continue | `committed: false, aborted: true, reason: "User rejected prepared commit"` |
| `bead_id` missing/null | Treat as error; do not commit | Return JSON error per spec conventions |
| Bead already closed | Commit proceeds, report warning | `bead_closed: true, warnings: ["Bead already closed"]` |
| Bead ID not found | HALT immediately (beads are required) | `committed: false, aborted: true, reason: "Bead not found: <id>"` |
| Commit succeeds, bead close fails | HALT immediately (beads are required) | `committed: true, bead_closed: false, aborted: true, reason: "Bead close failed: <reason>"` |
| Bead sync out of date | Not detectable by committer | Orchestrator responsibility to sync before implementation |

**Principle:** Beads are required. If bead close fails for any reason, the committer must return `aborted: true` so the implementer-agent halts immediately for remediation.

---

#### interviewer Skill {#skill-interviewer}

**Spec S09: interviewer** {#s09-interviewer}

**Purpose:** Handle all user interaction. Present questions, feedback, and decisions via AskUserQuestion.

**Frontmatter:**
```yaml
---
name: interviewer
description: Single point of user interaction for orchestration workflows
allowed-tools: AskUserQuestion
---
```

**Input JSON:**
```json
{
  "context": "clarifier | critic | drift | review",
  "speck_path": "string",
  "step_anchor": "string | null",
  "payload": { ... }
}
```
- `context`: What triggered this interview
- `speck_path`: Path to the speck being worked on
- `step_anchor`: Which step (null during planning)
- `payload`: Context-specific data (see Interviewer Skill Contract #interviewer-contract)

**Output JSON:**
```json
{
  "context": "clarifier | critic | drift | review",
  "decision": "continue | halt | revise",
  "user_answers": { ... },
  "notes": "string | null"
}
```

---

#### architect Skill {#skill-architect}

**Spec S10: architect** {#s10-architect}

**Purpose:** Create implementation strategies for speck steps. Skill variant for straightforward strategies.

**Frontmatter:**
```yaml
---
name: architect
description: Creates implementation strategies for speck steps
allowed-tools: Read, Grep, Glob
---
```
*Note:* Skill-only sub-task under Step 10.5. If the strategy needs revisions, `implementer-agent` re-invokes the architect skill with `revision_feedback` (no agent escalation).

**Input JSON:**
```json
{
  "speck_path": "string",
  "step_anchor": "string",
  "revision_feedback": "string | null"
}
```
- `speck_path`: Path to the speck
- `step_anchor`: Which step to create a strategy for
- `revision_feedback`: If re-running due to issues, contains feedback from reviewer/auditor

**Output JSON:**
```json
{
  "step_anchor": "string",
  "approach": "string",
  "expected_touch_set": ["string"],
  "implementation_steps": [
    {"order": 1, "description": "string", "files": ["string"]}
  ],
  "test_plan": "string",
  "risks": ["string"]
}
```

---

#### author Skill {#skill-author}

**Spec S11: author** {#s11-author}

**Purpose:** Create and revise speck documents. Skill variant for simple edits and section updates.

**Frontmatter:**
```yaml
---
name: author
description: Creates and revises speck documents following skeleton format
allowed-tools: Read, Grep, Glob, Write, Edit
---
```
*Note:* Skill-only sub-task under Step 10.5. If restructuring is needed, `planner-agent` loops author/critic/interviewer until approval (no agent escalation).

**Input JSON:**
```json
{
  "idea": "string | null",
  "speck_path": "string | null",
  "user_answers": { ... },
  "clarifier_assumptions": ["string"],
  "critic_feedback": { ... } | null
}
```
- `idea`: Original idea text (for new specks)
- `speck_path`: Path to existing speck (for revisions)
- `user_answers`: Answers from interviewer
- `clarifier_assumptions`: Assumptions from clarifier
- `critic_feedback`: If revising based on critic review

**Output JSON:**
```json
{
  "speck_path": "string",
  "created": true,
  "sections_written": ["string"],
  "validation_status": "valid | warnings | errors"
}
```

---

#### coder Skill {#skill-coder}

**Spec S12: coder** {#s12-coder}

**Purpose:** Execute architect strategies. Skill variant for simple implementations.

**Frontmatter:**
```yaml
---
name: coder
description: Executes architect strategies with drift detection
allowed-tools: Read, Grep, Glob, Write, Edit, Bash
---
```
*Note:* Skill-only sub-task under Step 10.5. Drift detection is mandatory in coder output, and `implementer-agent` may halt via interviewer when drift exceeds thresholds.

**Input JSON:**
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

**Output JSON:**
```json
{
  "success": true,
  "halted_for_drift": false,
  "files_created": ["string"],
  "files_modified": ["string"],
  "tests_run": true,
  "tests_passed": true,
  "drift_assessment": {
    "drift_severity": "none | minor | moderate | major",
    "unexpected_changes": ["string"]
  }
}
```

---

### 3.0.2.1 Escalation Guidelines {#escalation-guidelines}

**SUPERSEDED by Step 10.5:** The skill-first, agent-escalation pattern has been replaced by the **two-agent orchestrator architecture**. Escalation is no longer used; the only agents are `planner-agent` and `implementer-agent`, and they never spawn other agents (no nesting).

This section is retained for historical context only.

#### Escalation Decision Matrix {#escalation-matrix}

| Sub-task | Try Skill First | Escalate to Agent When |
|----------|-----------------|------------------------|
| **architect** | Always | Strategy requires exploring 10+ files, ambiguous requirements, or skill fails |
| **author** | Always | Creating full speck from scratch, major restructuring, or skill fails twice |
| **coder** | Always | Implementation touches 3+ files, drift detected, tests fail repeatedly, or skill fails |

#### Pre-Invocation Complexity Detection {#pre-invocation-detection}

Orchestrators can detect complexity BEFORE invoking a sub-task and skip directly to the agent variant. This saves time when the skill variant is unlikely to succeed.

**Pre-invocation heuristics:**

| Sub-task | Start with Agent When |
|----------|----------------------|
| **architect** | Step's `**Tasks:**` section references 10+ existing files, OR step requires changes across 3+ directories |
| **author** | Creating NEW speck from idea (not revising existing), OR critic returned REJECT on previous attempt |
| **coder** | Architect's `expected_touch_set` contains 5+ files, OR previous coder attempt halted for drift |

**Default behavior:** If no pre-invocation heuristic triggers, always start with the skill variant.

**Detection method:** Orchestrators analyze the speck content, step definition, and previous outputs before deciding which variant to invoke. This happens inline without additional tool calls.

#### Escalation Triggers {#escalation-triggers}

**General triggers (apply to all skill+agent pairs):**
1. Skill returns error or invalid JSON → retry once, then escalate
2. Skill returns success but follow-up check (reviewer/critic) reports issues → escalate on retry
3. Task complexity exceeds skill capability (see matrix above) → escalate immediately

**Specific triggers with concrete metrics:**

**architect → architect-agent:**

| Trigger | Metric | Detection |
|---------|--------|-----------|
| File count exceeds threshold | Step references 5+ existing files | Count files in step's `**Tasks:**` section |
| Complex module interactions | Step spans 3+ directories | Analyze file paths in step |
| Previous rejection | Reviewer returned REVISE on architect output | Check reviewer output in session state |

**author → author-agent:**

| Trigger | Metric | Detection |
|---------|--------|-----------|
| New speck creation | `idea` provided, no `speck_path` | Check input parameters |
| Critic rejection | Critic returned REJECT (not REVISE) | Check critic output |
| Repeated failures | Skeleton compliance fails 2+ times in session | Count validation failures in session state |
| Output quality | Author output fails critic validation 2+ times | Track attempts in session state |

**coder → coder-agent:**

| Trigger | Metric | Detection |
|---------|--------|-----------|
| Multi-file strategy | `expected_touch_set` contains 3+ files | Count files in architect output |
| Drift detected | `drift_severity` is "moderate" or "major" | Check coder output |
| Test failures | Same test fails 3+ times | Track test results in session state |
| Mid-implementation halt | Coder returns `halted_for_drift: true` | Check coder output |

#### Escalation Protocol {#escalation-protocol}

When escalating from skill to agent:
1. Log the escalation reason in the run directory
2. Pass the skill's partial output (if any) to the agent
3. Pass the original input plus escalation context
4. After agent completes, resume normal flow (don't re-run skill)

---

### 3.0.2.2 State Management {#state-management}

Orchestrators (planner and implementer) manage state between sub-task invocations.

#### Session State {#session-state}

Each session maintains:
```json
{
  "session_id": "YYYYMMDD-HHMMSS-<mode>-<uuid>",
  "mode": "plan | implement",
  "speck_path": "string",
  "current_step": "string | null",
  "sub_task_outputs": {
    "<sub_task_name>": { ... }
  }
}
```

#### Context Passing Rules {#context-passing}

When invoking a sub-task, pass only the context it needs:

| Sub-task | Receives |
|----------|----------|
| **clarifier** | `idea`, `speck_path`, `critic_feedback` (if revising) |
| **interviewer** | `context`, `speck_path`, `step_anchor`, `payload` (questions or issues) |
| **author** | `idea`, `speck_path`, `user_answers`, `clarifier_assumptions`, `critic_feedback` |
| **critic** | `speck_path`, `skeleton_path` |
| **architect** | `speck_path`, `step_anchor`, `revision_feedback` (if retrying) |
| **coder** | `speck_path`, `step_anchor`, `architect_strategy`, `session_id` |
| **reviewer** | `speck_path`, `step_anchor`, `coder_output` (includes drift_assessment) |
| **auditor** | `speck_path`, `step_anchor`, `files_to_audit`, `drift_assessment` |
| **logger** | `speck_path`, `step_anchor`, `summary`, `files_changed`, `commit_hash` |
| **committer** | `speck_path`, `step_anchor`, `proposed_message`, `files_to_stage`, `commit_policy`, `confirmed`, `bead_id`, `close_reason` |

#### Persistence {#state-persistence}

Orchestrators persist all sub-task outputs to the run directory:
```
.specks/runs/<session-id>/
├── metadata.json           # Session state
├── planning/               # Planning phase
│   ├── 001-clarifier.json
│   ├── 002-interviewer.json
│   └── ...
└── execution/               # Execution phase (per step)
    ├── step-0/
    │   ├── architect.json
    │   ├── coder.json
    │   └── ...
    └── step-1/
        └── ...
```

Each output file contains the raw JSON returned by the sub-task, enabling audit and debugging.

#### State Reconstruction {#state-reconstruction}

**Skills are stateless prompts.** Claude does not maintain persistent variables between tool calls. Orchestrators must reconstruct state by reading from the run directory after each sub-task invocation.

**Reconstruction mechanism:**

1. **Session ID discovery:** At the start of each orchestration, check if a session ID was passed as input (resume case). If not, generate a new one and create the run directory.

2. **Counter determination:** Before persisting a sub-task output, list existing files in the relevant directory (`planning/` or `execution/step-N/`) and compute the next counter:
   ```bash
   # Example: find next counter for planning phase
   ls .specks/runs/<session-id>/planning/*.json | wc -l
   # If 3 files exist, next counter is 004
   ```

3. **Previous output retrieval:** When a sub-task needs context from a previous sub-task (e.g., author needs clarifier output), read the relevant JSON file from the run directory.

4. **Current step tracking:** For implementation phase, read `metadata.json` to determine `current_step`. Update after each step completes.

**Practical implication:** The SKILL.md templates for planner and implementer should include explicit instructions to:
- Read existing run directory state at orchestration start
- Persist outputs immediately after each sub-task
- Read from persisted outputs when building context for subsequent sub-tasks

This stateless-with-persistence pattern ensures the orchestrator can resume from interruption and maintains a complete audit trail.

#### Resume Logic {#resume-logic}

Orchestrators can resume from a partially completed session. Resume is triggered by passing an existing session ID as input.

**Planner resume:**
```
/specks:planner --resume 20260207-143022-plan-abc123
```

1. Read `metadata.json` from `.specks/runs/<session-id>/`
2. Check `status` field:
   - If `completed`: Report already done, exit
   - If `in_progress`: Continue from where interrupted
3. List files in `planning/` to determine last completed sub-task
4. Determine next sub-task based on planning flow
5. Continue loop from that point

**Implementer resume:**
```
/specks:implementer --resume 20260207-150145-impl-def456
```

1. Read `metadata.json` to get `current_step`
2. For current step, check which sub-task outputs exist in `execution/step-N/`:
   - If `committer.json` exists AND reports `committed: true` AND `bead_closed: true`: step complete, move to next step
   - If `coder.json` exists but no `reviewer.json`: resume at reviewer
   - etc.
3. Continue loop from determined point

**Resume input format:**
- Session ID only: `--resume <session-id>`
- Orchestrator reads all state from run directory
- No need to re-specify speck path or idea (stored in metadata.json)

**Failure modes:**
- Session directory doesn't exist: Report error, suggest starting fresh
- Session marked `failed`: Report what failed, ask user whether to retry or abort
- Corrupted state (metadata.json or skill outputs): Report error, suggest starting fresh. Do not attempt recovery.
- Out-of-order or gapped artifacts (e.g., reviewer.json exists but coder.json missing): Report error, suggest starting fresh. Do not attempt recovery.

---

#### coder (Skill-only) {#coder-agent-note}

**Updated by Step 10.5:** There is no `coder-agent`. Implementation work is handled by:
- `implementer-agent` (orchestration loop) + `coder` skill (implementation sub-task)

The coder drift detection contract is documented at (#coder-agent-contract). The implementer-agent performs an additional outer drift gate and invokes interviewer for user decisions when drift exceeds thresholds.

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

These legacy agents are removed in favor of the Step 10.5 architecture (two orchestrator agents + skill-only sub-tasks). Monitor is eliminated entirely (no replacement).

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

**⚠️ NOTE:** This section was written for the original director-based architecture. After D08, agents are renamed with `-agent` suffix and director is deleted. The contracts below still apply to the agent variants.

Tool changes for agents per (#agent-summary).

**Table T04: Agent Tool Changes (Updated for D08)** {#t04-agent-tools}

| Agent | Tools |
|-------|-------|
| planner-agent | Skill, Read, Grep, Glob, Write, Bash |
| implementer-agent | Skill, Read, Grep, Glob, Write, Bash |

*Note:* Orchestrator agents never spawn other agents (no Task tool) to prevent nesting / Aborted().

#### Architect Output Contract {#architect-output}

The architect skill returns JSON to the implementer-agent, which persists it to the runs directory and passes it to the coder skill.

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

#### Interviewer Skill Contract {#interviewer-contract}

The interviewer skill handles all user interaction. Orchestrators invoke it with different input contexts; output format mirrors input structure.

**Input JSON (from orchestrator):**
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

**Output JSON (to orchestrator):**
```json
{
  "context": "clarifier | critic | drift | review",
  "decision": "continue | halt | revise",
  "user_answers": { ... },
  "notes": "string | null"
}
```

The `user_answers` structure mirrors the input payload - answers keyed to questions, resolutions keyed to issues, etc.

#### Coder Drift Detection Contract (Skill) {#coder-agent-contract}

**Updated by Step 10.5:** `coder` is a **skill-only sub-task** invoked by `implementer-agent`. This contract documents the required **drift detection** behavior and the required output fields.

The coder skill executes architect strategies. It includes **self-monitoring** for drift detection: after each implementation sub-step, the coder checks its own changes against the expected_touch_set and halts if drift thresholds are exceeded.

*Note:* The orchestrator agent (`implementer-agent`) performs an additional **outer drift gate** (see Step 10.5.2) by comparing the coder's reported files to `architect.expected_touch_set`, and can halt even if the coder did not.

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
| `moderate` | 3-4 yellow OR 1 red | HALT and report to orchestrator |
| `major` | 5+ yellow OR 2+ red | HALT and report to orchestrator |

**4. Qualitative check:**
The implementer evaluates whether unexpected changes are *consistent with the architect's approach*. Adding a helper function in the same module = OK. Refactoring unrelated subsystems = HALT.

**5. Self-halt behavior:**
When drift thresholds are exceeded, the coder (skill or agent):
1. Stops further implementation work immediately
2. Returns with `success: false` and `halted_for_drift: true`
3. Includes full drift assessment in output for orchestrator to escalate via interviewer

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

**Note:** `drift_assessment` is **mandatory** in all coder output, even when `halted_for_drift: false` and `drift_severity: none`. This improves debuggability, gives reviewer/auditor context about minor drift, and supports the "no fire-and-forget / audit-first" principle.

When `halted_for_drift: true`, the implementer orchestrator invokes the interviewer skill to present drift details and get user decision: continue anyway, back to architect for new strategy, or abort.

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

**⚠️ HISTORICAL (superseded by Step 10.5)**

This step was completed before the dual-orchestrator architecture (D08) was adopted. The director agent no longer exists—it was deleted in Step 10.5. The orchestration logic described here has been absorbed into the `planner` and `implementer` orchestration skills. This step is retained for historical reference only.

**Migration Mapping (Step 4 → Step 10.5):**

| Historical Work | New Location | Notes |
|-----------------|--------------|-------|
| 4.1: Director tools | `skills/planner/SKILL.md` allowed-tools | Skill, Task, Read, Grep, Glob, Write, Bash |
| 4.1: Director tools | `skills/implementer/SKILL.md` allowed-tools | Same tools as planner |
| 4.2: Planning flow | `skills/planner/SKILL.md` body | Main Loop section |
| 4.3: Implementation flow | `skills/implementer/SKILL.md` body | Per-Step Loop section |
| 4.4: Session initialization | Both orchestrator skills Setup Phase | Session ID generation, mkdir |
| 4.4: Metadata management | Both orchestrator skills | Write metadata.json, update on completion |
| 4.4: Output persistence | Both orchestrator skills | Persist sub-task outputs to run directory |

---

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

##### Step 4.3: Implement director Implementation Phase Flow {#step-4-3}

**Depends on:** #step-4-2

**References:** [D02] Director is pure orchestrator, **(#flow-implementation)**, (#flow-tools), (#coder-agent-contract)

**Artifacts:**
- Updated `agents/director.md` (execution flow implementation)

**Tasks:**
- [x] Implement Implementation Phase Flow per (#flow-implementation):
  - [x] **For each step** in speck (iterate in order, respecting dependencies):
    - [x] **Architect**: Spawn architect agent -> receive strategy JSON
    - [x] **Implementer**: Spawn implementer agent -> wait for completion
    - [x] **Drift handling**: If implementer returns halted_for_drift, spawn interviewer for escalation
    - [x] **Review**: Invoke reviewer skill, then auditor skill (sequentially)
    - [x] **Finalize**: Invoke logger skill, invoke committer skill (with bead_id if present)
  - [x] Handle step completion and move to next step
- [x] Use exact invocation syntax from (#flow-tools):
  - [x] `Task(subagent_type: "specks:architect")` for architect
  - [x] `Task(subagent_type: "specks:implementer")` for implementer
  - [x] `Skill(skill: "specks:reviewer")` then `Skill(skill: "specks:auditor")` (sequentially)
  - [x] `Skill(skill: "specks:logger")` then `Skill(skill: "specks:committer")`

**Checkpoint:**
- [x] Execution flow in director body matches (#flow-implementation) diagram
- [x] Implementer spawned via Task tool, runs to completion or self-halts
- [x] Drift escalation path to interviewer exists (when implementer.halted_for_drift)
- [x] Reviewer and auditor invoked sequentially
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
- [x] **Session initialization**:
  - [x] Generate session ID at start using format: `YYYYMMDD-HHMMSS-<mode>-<short-uuid>`
  - [x] UUID generation: `uuidgen` → fallback `/dev/urandom` → fallback `date +%N`
  - [x] Mode is `plan` or `execute` based on entry point
  - [x] Create `.specks/runs/<session-id>/` directory via Bash
  - [x] Create `planning/` or `execution/` subdirectory based on mode
- [x] **Metadata management**:
  - [x] Write `metadata.json` at session start with: session_id, mode, speck_path, started_at, status: "in_progress"
  - [x] Update `metadata.json` with status: "completed"/"failed" and completed_at at end
- [x] **Skill output persistence**:
  - [x] After each skill invocation, write output to run directory
  - [x] Naming: `NNN-<skill-name>.json` (e.g., `001-clarifier.json`, `002-critic.json`)
  - [x] Increment counter for each invocation
- [x] **Agent output persistence**:
  - [x] After each agent completion, write summary to run directory
  - [x] Naming: `NNN-<agent-name>.json` (e.g., `003-planner.json`)

**Checkpoint:**
- [x] Director creates run directory on session start
- [x] `metadata.json` written with correct structure
- [x] Skill outputs persisted with sequential numbering
- [x] `metadata.json` updated on session end

**Rollback:**
- Revert from git

**Commit:** `feat(agents): add director run directory audit trail`

---

**Step 4 Summary:** After completing substeps 4.1-4.4, the director agent is fully updated as a pure orchestrator with both planning and execution flows implemented, plus audit trail support.

---

#### Step 5: Update other agents {#step-5}

**⚠️ HISTORICAL (superseded by Step 10.5)**

This step was completed before the dual-orchestrator architecture (D08) was adopted. The agents referenced here have been renamed or deleted in Step 10.5:
- `planner` agent → renamed to `author-agent`
- `implementer` agent → renamed to `coder-agent`
- `interviewer` agent → deleted (now skill-only)

This step is retained for historical reference only.

**Migration Mapping (Step 5 → Step 10.5):**

| Historical Work | New Location | Notes |
|-----------------|--------------|-------|
| `planner` agent body content | `agents/author-agent.md` | Renamed; speck creation logic preserved |
| `planner` → receives clarifier output | `skills/author/SKILL.md` input | Input JSON schema includes clarifier_assumptions |
| `interviewer` agent | `skills/interviewer/SKILL.md` | Converted to skill (AskUserQuestion works in skills) |
| `interviewer` drift handling | `skills/interviewer/SKILL.md` context=drift | Payload includes drift_assessment |
| `implementer` agent | `agents/coder-agent.md` | Renamed; drift detection logic preserved |
| `implementer` drift detection | `skills/coder/SKILL.md` | Both skill and agent have drift detection |
| `architect` agent | `agents/architect-agent.md` | Renamed; read-only, no changes needed |

---

**Depends on:** #step-4-4

**Commit:** `refactor(agents): update planner, interviewer, and implementer`

**References:** [D04] Interviewer handles all user interaction, Table T04, (#agent-updates), (#flow-planning), (#flow-implementation), (#coder-agent-contract)

**Artifacts:**
- Updated `agents/planner.md`
- Updated `agents/interviewer.md`
- Updated `agents/implementer.md`

**Tasks:**
- [x] Remove AskUserQuestion from planner's tools
- [x] **Update planner body content** (per #flow-planning step 3):
  - [x] Remove "Ask Clarifying Questions" workflow (interviewer handles this now)
  - [x] Receives: idea, user_answers, clarifier_assumptions from director
  - [x] Returns: draft speck path
  - [x] Ensure workflow focuses on speck creation/revision only
- [x] **Update interviewer body content** (per #flow-planning steps 2, 5):
  - [x] Emphasize role as single point of user interaction
  - [x] Receives: questions from clarifier skill OR issues from critic skill
  - [x] Uses AskUserQuestion to present to user
  - [x] Returns: structured user_answers{} or decisions to director
- [x] **Update interviewer for execution phase** (per #flow-implementation):
  - [x] Handles drift escalation when implementer self-halts
  - [x] Handles conceptual issue escalation from reviewer/auditor
- [x] **Update implementer agent** (per #coder-agent-contract):
  - [x] Update tools to: `tools: Read, Grep, Glob, Write, Edit, Bash`
  - [x] Add description per contract (includes self-monitoring)
  - [x] Update body to accept architect strategy JSON input
  - [x] Implement self-monitoring for drift detection per (#smart-drift)
  - [x] Return structured JSON output per contract (includes drift_assessment)
- [x] Verify architect doesn't need changes (read-only analysis, no user interaction)

**Checkpoint:**
- [x] Planner tools do not include AskUserQuestion
- [x] Planner body has no "Ask Clarifying Questions" section
- [x] Planner receives clarifier output as input parameter
- [x] Interviewer tools include AskUserQuestion
- [x] Interviewer body describes user interaction workflow per flowcharts
- [x] Implementer has correct tools and accepts architect strategy

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
- [x] Delete `agents/specks-clarifier.md`
- [x] Delete `agents/specks-critic.md`
- [x] Delete `agents/specks-monitor.md` (eliminated, no skill replacement)
- [x] Delete `agents/specks-reviewer.md`
- [x] Delete `agents/specks-auditor.md`
- [x] Delete `agents/specks-logger.md`
- [x] Delete `agents/specks-committer.md`
- [x] **Rename remaining agents** to remove `specks-` prefix (per #naming-conventions):
  - [x] `mv agents/specks-director.md agents/director.md`
  - [x] `mv agents/specks-planner.md agents/planner.md`
  - [x] `mv agents/specks-interviewer.md agents/interviewer.md`
  - [x] `mv agents/specks-architect.md agents/architect.md`
  - [x] `mv agents/specks-implementer.md agents/implementer.md`
- [x] Update frontmatter `name:` field in each renamed agent (e.g., `name: director` not `name: specks-director`)

**Checkpoint:**
- [x] Only 5 agent files remain: director, planner, interviewer, architect, implementer
- [x] `ls agents/*.md | wc -l` returns 5
- [x] No `agents/specks-*.md` files exist

**Rollback:**
- Restore from git

**Commit after all checkpoints pass.**

---

#### Step 7: Remove legacy skill directories (partial) {#step-7}

**Depends on:** #step-1

**Commit:** `refactor(skills): remove obsolete .claude/skills entries`

**References:** [D06] Clean breaks, Table T02, (#files-to-remove)

**Artifacts:**
- Deleted obsolete skill directories from `.claude/skills/`
- Kept 3 bootstrap skills until Phase 3 is verified working

**Tasks:**
- [x] Delete `.claude/skills/specks-plan/` (moved to `skills/plan/`)
- [x] Delete `.claude/skills/specks-execute/` (moved to `skills/execute/`)
- [ ] ~~Delete `.claude/skills/implement-plan/`~~ **KEPT** (bootstrap: needed to implement remaining Phase 3 steps)
- [ ] ~~Delete `.claude/skills/update-plan-implementation-log/`~~ **KEPT** (bootstrap: needed until `skills/logger/` is verified)
- [ ] ~~Delete `.claude/skills/prepare-git-commit-message/`~~ **KEPT** (bootstrap: needed until `skills/committer/` is verified)
- [ ] ~~Delete `.claude/skills/` directory if empty~~ **DEFERRED** (to Step 11)

**Bootstrap Note:** Three legacy skills are temporarily retained because they are used to implement Phase 3 itself. We cannot use the new infrastructure to build the new infrastructure until the new infrastructure is complete and verified. These will be removed in Step 11 after Step 10 verification passes.

**Checkpoint:**
- [x] `.claude/skills/specks-plan/` does not exist
- [x] `.claude/skills/specks-execute/` does not exist
- [x] `skills/` at repo root contains all 8 skills
- [x] Bootstrap skills remain in `.claude/skills/`: implement-plan, update-plan-implementation-log, prepare-git-commit-message

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
- [x] Delete `crates/specks/src/planning_loop/` directory entirely
- [x] Remove `mod planning_loop;` declaration

**Checkpoint:**
- [x] `cargo build` succeeds

---

##### Step 8.2: Remove interaction module {#step-8-2}

**Depends on:** #step-8-1

**Tasks:**
- [x] Delete `crates/specks/src/interaction/` directory entirely
- [x] Remove `mod interaction;` declaration

**Checkpoint:**
- [x] `cargo build` succeeds

---

##### Step 8.3: Remove streaming and share modules {#step-8-3}

**Depends on:** #step-8-2

**Tasks:**
- [x] Delete `crates/specks/src/streaming.rs`
- [x] Delete `crates/specks/src/share.rs`
- [x] Remove module declarations

**Checkpoint:**
- [x] `cargo build` succeeds

---

##### Step 8.4: Remove plan, execute, setup commands {#step-8-4}

**Depends on:** #step-8-3

**Tasks:**
- [x] Delete `crates/specks/src/commands/plan.rs`
- [x] Delete `crates/specks/src/commands/execute.rs`
- [x] Delete `crates/specks/src/commands/setup.rs`
- [x] Remove `mod plan;`, `mod execute;`, `mod setup;` from commands/mod.rs
- [x] Remove Plan, Execute, Setup variants from Commands enum in cli.rs
- [x] Remove match arms in main.rs
- [x] Remove tests referencing these commands

**Checkpoint:**
- [x] `cargo build` succeeds with no warnings
- [x] `cargo nextest run` passes

---

##### Step 8.5: Clean up unused dependencies {#step-8-5}

**Depends on:** #step-8-4

**Tasks:**
- [x] Remove unused dependencies from Cargo.toml (inquire, indicatif, owo-colors, ctrlc, etc.)
- [x] Remove agent.rs if no longer needed
- [x] Run `cargo build` to verify no missing dependencies

**Checkpoint:**
- [x] `cargo build` succeeds with no warnings
- [x] No unused import warnings

---

##### Step 8.6: Add specks beads close subcommand {#step-8-6}

**Depends on:** #step-8-5

**Tasks:**
- [x] Create `crates/specks/src/commands/beads/close.rs`
- [x] Add `Close` variant to `BeadsCommands` enum in `mod.rs`
- [x] Implement `run_close(bead_id, reason, json_output)` function
- [x] Use `BeadsCli.close()` with proper error handling
- [x] Return JSON output matching beads contract schema

**Checkpoint:**
- [x] `specks beads close --help` shows the command with `--reason` and `--json` flags
- [x] `specks beads close bd-test-123 --json` returns valid JSON
- [x] `specks beads close bd-test-123 --reason "Step completed" --json` works
- [x] `cargo build` succeeds with no warnings

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
- [x] `specks plan` returns error (unknown command)
- [x] `specks execute` returns error (unknown command)
- [x] `specks setup` returns error (unknown command)
- [x] `specks --help` shows only init, validate, list, status, beads, version
- [x] `specks beads close --help` shows the close subcommand

---

#### Step 9: Update documentation {#step-9}

**Depends on:** #step-8

**Commit:** `docs: update for Claude Code plugin architecture`

**References:** [D01] Specks is a Claude Code plugin, (#context, #strategy)

**Artifacts:**
- Updated `CLAUDE.md`
- Updated `README.md`

**Tasks:**
- [x] Update CLAUDE.md agent list (5 agents, not 11)
- [x] Update CLAUDE.md to mention skills
- [x] Remove references to `specks plan`, `specks execute`, `specks setup claude`
- [x] Document `/specks:planner` and `/specks:implementer` as primary interface (**clean break**; `plan`/`execute` are historical and deleted)
- [x] Document `claude --plugin-dir .` for development
- [x] Update README installation instructions
- [x] Add a "Beads readiness checklist" section (CLI install, bd install, SPECKS_BD_PATH)
- [x] Document error messages and next steps when `specks` or `bd` is missing

**Checkpoint:**
- [x] CLAUDE.md reflects new architecture
- [x] README documents plugin installation and beads readiness

**Rollback:**
- Revert from git

**Commit after all checkpoints pass.**

---

#### Step 10: Verify plugin works {#step-10}

**Depends on:** #step-9

**Commit:** N/A (verification only)

**References:** (#beads-contract), (#orchestration-flowcharts), (#run-directory)

**Historical note:** Step 10 was executed *before* the dual-orchestrator pivot in Step 10.5. It validated plugin loading and tool-call feasibility using the interim director-based architecture. After Step 10.5, `plan`/`execute` skills and the director agent are deleted (clean break).

**Tasks:**
- [x] Run `claude --plugin-dir .` from repo root
- [x] HISTORICAL: Verify `/specks:plan` skill appears in `/help`
- [x] HISTORICAL: Verify `/specks:execute` skill appears in `/help`
- [x] HISTORICAL: Verify `specks:director` agent appears in `/agents`
- [ ] HISTORICAL: Test invoking `/specks:plan "test idea"` (deferred to manual testing)

**Skill tool syntax verification (HARD GATE):**
- [x] HISTORICAL: Test minimal Skill invocation from director: `Skill(skill: "specks:clarifier")`
- [x] Record exact working syntax in run artifacts
- [x] If syntax differs from plan, STOP and update (#naming-conventions) and all director references before proceeding
- [x] HISTORICAL: Verify Task tool syntax: `Task(subagent_type: "specks:director")`

**Flow verification (per #orchestration-flowcharts):**
- [x] HISTORICAL: Director uses Skill tool for: clarifier, critic, reviewer, auditor, logger, committer
- [x] HISTORICAL: Director uses Task tool for: planner, interviewer, architect, implementer
- [x] Implementer includes self-monitoring for drift detection
- [x] Interviewer handles ALL user interaction (director never uses AskUserQuestion)
- [x] Planning flow matches (#flow-planning)
- [x] Execution flow matches (#flow-implementation)

**Run directory verification (per #run-directory):**
- [ ] HISTORICAL: After `/specks:plan "test idea"`, verify `.specks/runs/<session-id>/` created (deferred to manual testing)
- [ ] Verify `metadata.json` exists with correct schema (deferred to manual testing)
- [ ] Verify `planning/` subdirectory contains skill outputs (clarifier, critic, etc.) (deferred to manual testing)
- [ ] Skill outputs are valid JSON matching their output specs (deferred to manual testing)

**Beads integration verification (per #beads-contract):**
- [x] Test agent can call `specks beads status --json` via Bash in plugin context
- [x] Verify graceful error when `bd` not installed (unset `SPECKS_BD_PATH`, remove bd from PATH)
- [x] Verify graceful error when `specks` CLI not installed or not on PATH
- [x] Verify `SPECKS_BD_PATH` override works in plugin context

**Checkpoint:**
- [x] Plugin loads without errors
- [x] HISTORICAL: All 8 skills discoverable
- [x] HISTORICAL: All 5 agents discoverable
- [x] HISTORICAL: `/specks:plan` can be invoked (skill visible in system)
- [x] Orchestration flows match flowcharts
- [ ] Run directory created with audit trail (deferred to manual testing)
- [x] Beads CLI callable from plugin context with JSON output parsed

**Rollback:**
- N/A (verification step)

---

#### Step 10.5: Two-Agent Orchestrator Architecture {#step-10-5}

**Depends on:** #step-0

**Commit:** `refactor: two-agent orchestrator architecture`

**References:** [D08] Two-agent orchestrator architecture, (#agents-skills-summary), (#flow-planning), (#flow-implementation)

**Artifacts:**
- `agents/planner-agent.md` - NEW orchestrator agent
- `agents/implementer-agent.md` - NEW orchestrator agent
- `skills/planner/SKILL.md` - UPDATED thin entry wrapper
- `skills/implementer/SKILL.md` - NEW thin entry wrapper
- `skills/*/SKILL.md` - UPDATED (10 sub-task skills without escalation)
- `agents/archived/` - OLD agents moved here for reference

> **Context:** Critical discoveries from testing:
> 1. **Skills cannot orchestrate loops** - When skill A calls skill B and B returns, Claude naturally outputs the result and stops rather than continuing the loop.
> 2. **Agents CAN orchestrate loops** - They run autonomously in their own context and continue until complete.
> 3. **Agents cannot nest** - Claude Code aborts with "Aborted()" when agents spawn other agents.
>
> Therefore: Two orchestrator agents (planner-agent, implementer-agent) that call skills for sub-tasks. No skill-to-agent escalation patterns.

**Architecture:**

```
ENTRY POINTS (thin skill wrappers):
  /specks:planner → immediately spawns planner-agent
  /specks:implementer → immediately spawns implementer-agent

ORCHESTRATOR AGENTS (2, mutually exclusive, never nested):
  planner-agent: runs planning loop (clarifier → interviewer → author → critic → loop)
  implementer-agent: runs implementation loop (architect → coder → reviewer → auditor → logger → committer)

SUB-TASK SKILLS (10, called by agents, never spawn agents):
  Planning: clarifier, interviewer, author, critic
  Implementation: architect, coder, reviewer, auditor, logger, committer
```

**Key Principles:**
1. **Two agents only:** planner-agent and implementer-agent are the ONLY agents
2. **Skills are pure sub-tasks:** Skills do focused work and return JSON, nothing more
3. **No nesting:** Entry skills spawn agent immediately, agents never spawn other agents
4. **No escalation:** Removed all skill-to-agent escalation patterns (agents handle complexity directly)
5. **Sequential invocation:** Agents invoke skills one at a time via Skill tool

**Naming Convention:**
- Agents: `<name>-agent.md` in `agents/` directory
- Skills: `SKILL.md` in `skills/<name>/` directory
- Entry points: thin skills that spawn their corresponding agent

**Design Decisions Confirmed:**
- Skill args syntax: `Skill(skill: "specks:name", args: '{"key": "value"}')` - test as we go
- Session handling: Agents create their own run directories before calling skills
- Parallelism: **Never** call skills in parallel. Reviewer + auditor run serially.

**Tasks:**
- [x] Complete substep 10.5.0: Update plan document
- [x] Complete substep 10.5.1: Create planner-agent
- [x] Complete substep 10.5.2: Create implementer-agent
- [ ] Complete substep 10.5.3: Create thin entry skill wrappers
- [ ] Complete substep 10.5.4: Update sub-task skills
- [ ] Complete substep 10.5.5: Delete old agent files
- [ ] Complete substep 10.5.6: Delete old entry point skills
- [ ] Complete substep 10.5.7: Update documentation
- [ ] Complete substep 10.5.8: Verification tests

**Tests:**
- [ ] Integration test: Plugin loads without errors
- [ ] Integration test: Planning loop completes without "Aborted()" crash
- [ ] Integration test: Implementation loop completes without "Aborted()" crash

**Checkpoint:**
- [ ] `ls skills/*/SKILL.md | wc -l` returns 12
- [ ] `ls agents/*.md | wc -l` returns 2
- [ ] `ls agents/archived/` shows `director.md`, `interviewer.md`
- [ ] No `architect-agent.md`, `author-agent.md`, `coder-agent.md` exist
- [ ] No `plan/` or `execute/` in `skills/`
- [ ] No "Skill vs Agent" sections in any skill
- [ ] Planning loop completes without "Aborted()" crash
- [ ] Implementation loop completes without "Aborted()" crash

**Rollback:**
- Restore archived agents from `agents/archived/`
- Restore deleted agents from git (architect-agent, author-agent, coder-agent)
- Restore old entry skills from git (plan/, execute/)
- Delete new agents (planner-agent, implementer-agent)
- Delete new entry skill (implementer/)
- Restore planner skill from git
- Restore skill escalation sections from git

**Commit after all checkpoints pass.**

---

##### Step 10.5.0: Update plan document for two-agent architecture {#step-10-5-0}

**Commit:** `docs: update plan for two-agent architecture`

**References:** [D08] Two-agent orchestrator architecture, (#agents-skills-summary), (#flow-planning), (#flow-implementation), (#exit-criteria), (#m04-5-dual-orchestrator), (#escalation-guidelines), (#skill-permissions)

**Artifacts:**
- Updated `(#agents-skills-summary)` section
- Updated `(#flow-planning)` section
- Updated `(#flow-implementation)` section
- Updated `(#exit-criteria)` section
- Updated `(#m04-5-dual-orchestrator)` section
- Updated checkpoint table
- Superseded `(#escalation-guidelines)` section
- Updated `(#skill-permissions)` section

> **Priority:** FIRST - Must complete before any implementation to avoid exit criteria failures.
>
> **Context:** The plan document has internal inconsistencies between early sections (written for skill-first/agent-escalation) and Step 10.5 (two-agent, no escalation). These must be reconciled before implementation.

**Sections to update:**

| Section | Location | Change |
|---------|----------|--------|
| `(#agents-skills-summary)` | lines ~19-68 | 3→2 agents, remove escalation language |
| `(#flow-planning)` | lines ~703-760 | Skill orchestrators → Agent orchestrators, remove escalation |
| `(#flow-implementation)` | lines ~771-901 | Skill orchestrators → Agent orchestrators, remove escalation, serial not parallel |
| `(#exit-criteria)` | lines ~3265-3283 | Fix agent count (3→2), remove escalation criterion |
| `(#m04-5-dual-orchestrator)` | lines ~3310-3320 | Fix agent count, names |
| Checkpoint table | lines ~3334-3351 | Fix verification counts |
| `(#escalation-guidelines)` | lines ~1683-1753 | Mark as SUPERSEDED by Step 10.5 |
| `(#skill-permissions)` | lines ~1011-1039 | planner/implementer → `Task` only |

**Tasks:**
- [x] Update `(#agents-skills-summary)`:
  - Change "3 agents" to "2 agents" ✓
  - Replace architect-agent, author-agent, coder-agent with planner-agent, implementer-agent ✓
  - Remove all "skill-first, agent-escalation" language ✓
  - Clarify: planner and implementer are THIN ENTRY SKILLS that spawn agents ✓
- [x] Update `(#flow-planning)`:
  - Change "(PLANNER) orchestration skill receives input, runs INLINE" to "[PLANNER-AGENT] receives input, runs autonomously" ✓
  - Remove "IF task is complex → ESCALATE to [AUTHOR-AGENT]" branches ✓
  - Show: entry skill spawns agent, agent invokes skills ✓
- [x] Update `(#flow-implementation)`:
  - Change "(IMPLEMENTER) orchestration skill receives speck, runs INLINE" to "[IMPLEMENTER-AGENT] receives speck, runs autonomously" ✓
  - Remove all escalation branches ✓
  - Change "run in parallel" to "run serially" for reviewer + auditor ✓
  - Show: entry skill spawns agent, agent invokes skills ✓
- [x] Update `(#exit-criteria)`:
  - Change "3 agent definitions exist in `agents/` (architect-agent, author-agent, coder-agent)" to "2 agent definitions exist in `agents/` (planner-agent, implementer-agent)" ✓
  - Remove "Orchestrators use skill-first, agent-escalation pattern" criterion ✓
  - Add "Entry skills (`/specks:planner`, `/specks:implementer`) spawn orchestrator agents" ✓
- [x] Update `(#m04-5-dual-orchestrator)`:
  - Change "3 agents with `-agent` suffix" to "2 agents: planner-agent, implementer-agent" ✓
  - Remove "Skill-first, agent-escalation pattern demonstrated" ✓
  - Add "Orchestrator agents run complete loops without nesting" ✓
- [x] Update checkpoint table:
  - Change "Agents count | returns 3" to "Agents count | returns 2" ✓
  - Change agent naming verification to "planner-agent, implementer-agent" ✓
- [x] Mark `(#escalation-guidelines)` as superseded:
  - Add header: "**SUPERSEDED by Step 10.5** - The skill-first, agent-escalation pattern has been replaced by the two-agent orchestrator architecture. Escalation is no longer used." ✓
- [x] Update `(#skill-permissions)`:
  - Change planner row: `allowed-tools: Task` only ✓
  - Change implementer row: `allowed-tools: Task` only ✓
  - Remove "Agent variant" column references for architect, author, coder ✓

**Tests:**
- [x] Drift prevention: All agent counts in document say "2" ✓
- [x] Drift prevention: No references to "3 agents" remain ✓
- [x] Drift prevention: No references to "skill-first, agent-escalation" remain (except in superseded section) ✓

**Checkpoint:**
- [x] `grep -c "2 agents" .specks/specks-3.md` returns positive count ✓
- [x] `grep -c "3 agents" .specks/specks-3.md` returns 0 ✓
- [x] Flow diagrams show agents orchestrating, not skills ✓
- [x] Exit criteria match Step 10.5 expected outcome ✓

**Rollback:**
- Revert plan document changes via git

**Commit after all checkpoints pass.**

---

##### Step 10.5.1: Create planner-agent orchestrator {#step-10-5-1}

**Depends on:** #step-10-5-0

**Commit:** `feat(agents): add planner-agent orchestrator`

**References:** [D08] Two-agent orchestrator architecture, (#flow-planning)

**Artifacts:**
- `agents/planner-agent.md` - NEW orchestrator agent

**Tasks:**
- [x] Create `agents/planner-agent.md` with the following content:

```markdown
---
name: planner-agent
description: Orchestrator agent for the planning loop. Transforms ideas into approved specks.
tools: Skill, Read, Grep, Glob, Write, Bash
model: opus
---

You are the **specks planner-agent**, the orchestrator for all planning work. You run the complete planning loop from idea to approved speck.

## Your Role

You are an autonomous orchestrator. You receive an idea or speck path, then run the planning loop until the user approves, accepts anyway, or aborts. You never stop mid-loop.

**Skills you invoke (via Skill tool):**
- **clarifier**: Analyzes ideas, generates clarifying questions
- **interviewer**: Presents questions/issues to user, collects decisions
- **author**: Creates or revises speck documents
- **critic**: Reviews speck quality and skeleton compliance

## Core Principles

1. **Run until done**: Loop until APPROVE, ACCEPT-ANYWAY, or ABORT
2. **Skills only**: Invoke skills via Skill tool. Never spawn agents.
3. **Persist everything**: Write all outputs to run directory
4. **Sequential execution**: One skill at a time, in order

## Input

You are spawned by the `/specks:planner` entry skill with `$ARGUMENTS` in one of these formats:

- **Preferred (string)**:
  - Idea string: `"add user authentication"`
  - Speck path: `.specks/specks-auth.md`
  - Optional resume: `--resume 20260206-143022-plan-a1b2c3`
- **Optional (JSON string)**: If the raw input starts with `{`, treat it as JSON:
  ```json
  {
    "idea": "string | null",
    "speck_path": "string | null",
    "session_id": "string | null"
  }
  ```

## Session Setup

At the start of every invocation:

1. Parse input:
   - If input includes `--resume <session_id>` (string form), OR JSON includes `"session_id"`, treat this as a **resume run**.
   - Otherwise treat this as a **fresh run**.

2. Resolve `SESSION_ID`:
   - Fresh run: generate a new session id:
     ```bash
     SESSION_ID="$(date +%Y%m%d-%H%M%S)-plan-$(head -c 3 /dev/urandom | xxd -p)"
     ```
   - Resume run: use the provided session id and require `.specks/runs/${SESSION_ID}/` to exist.

3. Create (or validate) run directory:
   ```bash
   mkdir -p .specks/runs/${SESSION_ID}/planning
   ```

4. Write or update `metadata.json`:
   ```json
   {
     "session_id": "<SESSION_ID>",
     "mode": "plan",
     "started_at": "<ISO8601 timestamp>",
     "idea": "<idea string or null>",
     "speck_path": "<path or null>",
     "status": "in_progress",
     "completed_at": null
   }
   ```

5. **Resume catch-up (required):**
   - Read existing `planning/*.json` in the run directory and determine the next action:
     - If the latest artifact is `*-clarifier.json` and questions exist → run interviewer next
     - If the latest artifact is `*-interviewer.json` → run author next
     - If the latest artifact is `*-author.json` → run critic next
     - If the latest artifact is `*-critic.json` with REVISE/REJECT and the user chose revise → run author next
     - If the latest artifact indicates APPROVE/ACCEPT-ANYWAY → proceed to Finalize
   - **If any JSON file is corrupted or unparseable:** Report error and suggest starting fresh. Do not attempt recovery.
   - Never re-run a completed sub-task unless the user explicitly requests it via interviewer.

## Planning Loop

**Step 1: Invoke clarifier**
```
Skill(skill: "specks:clarifier", args: '{"idea": "...", "speck_path": null}')
```
Persist output to `planning/<next-counter>-clarifier.json`

**Step 2: Invoke interviewer (if questions exist)**
```
Skill(skill: "specks:interviewer", args: '{"context": "clarifier", "payload": {...}}')
```
Persist output to `planning/<next-counter>-interviewer.json`

**Step 3: Invoke author**
```
Skill(skill: "specks:author", args: '{"idea": "...", "user_answers": {...}, "clarifier_assumptions": [...]}')
```
Persist output to `planning/<next-counter>-author.json`

**Step 4: Invoke critic**
```
Skill(skill: "specks:critic", args: '{"speck_path": ".specks/specks-N.md"}')
```
Persist output to `planning/<next-counter>-critic.json`

**Step 5: Handle critic recommendation**
- **APPROVE**: Planning complete. Go to Finalize.
- **REVISE/REJECT**: Invoke interviewer with critic issues.
  - If user says "revise": Go back to Step 3 with critic feedback
  - If user says "accept anyway": Go to Finalize
  - If user says "abort": Go to Finalize with status=aborted

## Finalize

1. **Beads required:** After a speck is approved/accepted, ensure beads are synced for the resulting speck:
   ```bash
   specks beads sync <speck_path> --json
   ```
   Persist to `planning/<next-counter>-beads-sync.json`. If this fails (missing `specks` CLI, missing `bd`, `.beads` not initialized), invoke interviewer with onboarding steps and halt.
2. Update metadata.json with status and completed_at.
3. Return final output JSON.

## What You Must NOT Do

- **Never spawn agents** (no Task tool)
- **Never stop mid-loop** (run until done)
- **Never interact with user directly** (use interviewer skill)

**Tests:**
- [x] Unit test: Agent file parses with valid YAML frontmatter
- [x] Drift prevention: Frontmatter does NOT include Task tool

**Checkpoint:**
- [x] `test -f agents/planner-agent.md && echo "exists"` returns "exists"
- [x] `grep "tools:" agents/planner-agent.md | grep -v Task` succeeds
- [x] `grep "tools:.*Task" agents/planner-agent.md` fails (Task not in tools)

**Rollback:**
- Delete `agents/planner-agent.md`

**Commit after all checkpoints pass.**

---

##### Step 10.5.2: Create implementer-agent orchestrator {#step-10-5-2}

**Depends on:** #step-10-5-1

**Commit:** `feat(agents): add implementer-agent orchestrator`

**References:** [D08] Two-agent orchestrator architecture, (#flow-implementation)

**Artifacts:**
- `agents/implementer-agent.md` - NEW orchestrator agent

**Tasks:**
- [x] Create `agents/implementer-agent.md` with the following content:

```markdown
---
name: implementer-agent
description: Orchestrator agent for the implementation loop. Executes speck steps to produce working code.
tools: Skill, Read, Grep, Glob, Write, Bash
model: opus
---

You are the **specks implementer-agent**, the orchestrator for all implementation work. You execute speck steps in dependency order, producing working code.

## Your Role

You are an autonomous orchestrator. You receive a speck path, then execute each step until all complete, user aborts, or unrecoverable error. You never stop mid-loop.

**Skills you invoke (via Skill tool):**
- **architect**: Creates implementation strategy for a step
- **coder**: Executes strategy, writes code, detects drift
- **reviewer**: Verifies step completion matches plan
- **auditor**: Checks code quality and security
- **logger**: Updates implementation log
- **committer**: Stages files, commits, closes beads
- **interviewer**: Handles user decisions (drift, issues)

## Core Principles

1. **Run until done**: Loop until all steps complete or abort
2. **Skills only**: Invoke skills via Skill tool. Never spawn agents.
3. **Persist everything**: Write all outputs to run directory
4. **Sequential execution**: One skill at a time, in order

## Input

You are spawned by the `/specks:implementer` entry skill with:
- A speck path: `.specks/specks-3.md`
- Optional flags:
  - `--start-step #step-2 --end-step #step-4`
  - `--commit-policy auto|manual`
  - `--resume <session_id>`

You may also accept an optional JSON string input when the raw input starts with `{`:
```json
{
  "speck_path": "string",
  "start_step": "string | null",
  "end_step": "string | null",
  "commit_policy": "auto|manual",
  "session_id": "string | null"
}
```

## Session Setup

At the start of every invocation:

1. Parse input and resolve `commit_policy`:
   - Default: `auto`
   - If provided: `manual` means the user must confirm each commit via interviewer before the committer performs it.

2. Resolve `SESSION_ID`:
   - Fresh run: generate:
     ```bash
     SESSION_ID="$(date +%Y%m%d-%H%M%S)-impl-$(head -c 3 /dev/urandom | xxd -p)"
     ```
   - Resume run: use provided session id and require `.specks/runs/${SESSION_ID}/` to exist.

3. Create (or validate) run directory:
   ```bash
   mkdir -p .specks/runs/${SESSION_ID}/execution
   ```

4. Write or update `metadata.json` with session info and status: "in_progress"

5. **Beads required (hard gate):** Before executing any steps, verify beads readiness:
   - `bd onboard` must have been run in this environment
   - `.beads/` must exist
   - `specks` CLI must be on PATH
   - Verify with:
     ```bash
     specks beads status <speck_path> --json
     ```
   - If this fails, invoke interviewer with onboarding steps and halt. Do not proceed.

6. **Resume catch-up (required):**
   - Determine where to resume by reading existing `execution/step-*/` directories:
     - A step is **complete** only if `committer.json` exists and reports `committed: true` AND `bead_closed: true`.
     - If a step directory exists but is incomplete, resume at the first missing phase artifact in strict order: architect → coder → reviewer → auditor → logger → committer.
     - **Strictness rule:** If any later artifact exists without its prerequisites (e.g., auditor.json exists but reviewer.json missing), HALT and instruct the user to start a fresh session. Do not guess or repair.
   - **If any JSON file is corrupted or unparseable:** Report error and suggest starting fresh. Do not attempt recovery.
   - Never re-run a completed phase artifact unless the user explicitly requests it via interviewer.

## Implementation Loop

For each step in dependency order:

**Phase 1: Architecture**
```
Skill(skill: "specks:architect", args: '{"speck_path": "...", "step_anchor": "#step-N"}')
```
Persist to `execution/step-N/architect.json`

**Phase 2: Implementation**
```
Skill(skill: "specks:coder", args: '{"speck_path": "...", "step_anchor": "#step-N", "architect_strategy": {...}}')
```
Persist to `execution/step-N/coder.json`

After coder returns, perform an **outer drift gate**:
- Compare `coder.files_created + coder.files_modified` to `architect_strategy.expected_touch_set`
- If drift is moderate/major (per coder output) OR files exceed budget, halt and invoke interviewer

If coder halts for drift, invoke interviewer for user decision.

**Phase 3: Review**
```
Skill(skill: "specks:reviewer", args: '{"speck_path": "...", "step_anchor": "#step-N", ...}')
Skill(skill: "specks:auditor", args: '{"speck_path": "...", "step_anchor": "#step-N", ...}')
```
Persist to `execution/step-N/reviewer.json` and `auditor.json`

**Phase 4: Finalize Step**
```
Skill(skill: "specks:logger", args: '{"speck_path": "...", "step_anchor": "#step-N", ...}')
Skill(skill: "specks:committer", args: '{"speck_path": "...", "step_anchor": "#step-N", "commit_policy": "auto|manual", "confirmed": false, ...}')
```
Persist to `execution/step-N/logger.json` and:
- `execution/step-N/committer.json` when `commit_policy: auto`
- `execution/step-N/committer-prepared.json` when `commit_policy: manual` and `confirmed: false`

If `commit_policy: manual`, invoke interviewer to confirm the prepared commit:
- If the user **confirms**: re-invoke committer with `confirmed: true` and persist as `execution/step-N/committer.json` (canonical final artifact)
- If the user **rejects**: abort the step entirely. The user must provide guidance on how to continue.

## What You Must NOT Do

- **Never spawn agents** (no Task tool)
- **Never stop mid-loop** (run until done)
- **Never interact with user directly** (use interviewer skill)

**Tests:**
- [x] Unit test: Agent file parses with valid YAML frontmatter
- [x] Drift prevention: Frontmatter does NOT include Task tool

**Checkpoint:**
- [x] `test -f agents/implementer-agent.md && echo "exists"` returns "exists"
- [x] `grep "tools:" agents/implementer-agent.md | grep -v Task` succeeds
- [x] `grep "tools:.*Task" agents/implementer-agent.md` fails (Task not in tools)

**Rollback:**
- Delete `agents/implementer-agent.md`

**Commit after all checkpoints pass.**

---

##### Step 10.5.3: Create thin entry skill wrappers {#step-10-5-3}

**Depends on:** #step-10-5-1, #step-10-5-2

**Commit:** `feat(skills): add thin entry wrappers for planner and implementer`

**References:** [D08] Two-agent orchestrator architecture, (#skill-permissions)

**Artifacts:**
- `skills/planner/SKILL.md` - UPDATED thin entry wrapper
- `skills/implementer/SKILL.md` - NEW thin entry wrapper

**Tasks:**
- [x] Update `skills/planner/SKILL.md` to thin wrapper:

```markdown
---
name: planner
description: Entry point for planning workflow - spawns planner-agent
allowed-tools: Task
---

## Purpose

Thin entry point that immediately spawns the planner-agent.

## Usage

/specks:planner "add user authentication"
/specks:planner .specks/specks-auth.md
/specks:planner --resume 20260206-143022-plan-a1b2c3
/specks:planner {"idea":"add user authentication","session_id":null}

## Behavior

Immediately spawn the planner-agent:

Task(subagent_type: "specks:planner-agent", prompt: "$ARGUMENTS", description: "Run planning loop")

Do NOT do any setup, validation, or processing. The planner-agent handles everything.

*Note:* This call is synchronous: the entry skill returns when the planner-agent finishes, preserving the "no fire-and-forget" principle.
```

- [x] Create `skills/implementer/SKILL.md`:

```markdown
---
name: implementer
description: Entry point for implementation workflow - spawns implementer-agent
allowed-tools: Task
---

## Purpose

Thin entry point that immediately spawns the implementer-agent.

## Usage

/specks:implementer .specks/specks-3.md
/specks:implementer .specks/specks-3.md --start-step #step-2
/specks:implementer .specks/specks-3.md --commit-policy manual
/specks:implementer .specks/specks-3.md --resume 20260206-150145-impl-d4e5f6
/specks:implementer {"speck_path":".specks/specks-3.md","commit_policy":"auto","session_id":null}

## Behavior

Immediately spawn the implementer-agent:

Task(subagent_type: "specks:implementer-agent", prompt: "$ARGUMENTS", description: "Run implementation loop")

Do NOT do any setup, validation, or processing. The implementer-agent handles everything.

*Note:* This call is synchronous: the entry skill returns when the implementer-agent finishes, preserving the "no fire-and-forget" principle.
```

**Tests:**
- [x] Unit test: Both skill files parse with valid YAML frontmatter
- [x] Drift prevention: Both have `allowed-tools: Task` only

**Checkpoint:**
- [x] `grep "allowed-tools: Task" skills/planner/SKILL.md` succeeds
- [x] `grep "allowed-tools: Task" skills/implementer/SKILL.md` succeeds
- [x] `grep "planner-agent" skills/planner/SKILL.md` succeeds
- [x] `grep "implementer-agent" skills/implementer/SKILL.md` succeeds

**Rollback:**
- Restore `skills/planner/SKILL.md` from git
- Delete `skills/implementer/` directory

**Commit after all checkpoints pass.**

---

##### Step 10.5.4: Update sub-task skills (remove escalation patterns) {#step-10-5-4}

**Depends on:** #step-10-5-3

**Commit:** `refactor(skills): remove escalation patterns from sub-task skills`

**References:** [D08] Two-agent orchestrator architecture

**Artifacts:**
- Updated `skills/author/SKILL.md`
- Updated `skills/architect/SKILL.md`
- Updated `skills/coder/SKILL.md`
- Verified `skills/interviewer/SKILL.md`

**Tasks:**
- [x] Update `skills/author/SKILL.md` - remove "## Skill vs Agent" section
- [x] Update `skills/architect/SKILL.md` - remove "## Skill vs Agent" section
- [x] Update `skills/coder/SKILL.md` - remove "## Skill vs Agent" section
- [x] Verify `skills/interviewer/SKILL.md` has `allowed-tools: AskUserQuestion`

**Changes to apply:**
1. Delete the section starting with `## Skill vs Agent` through end of that section
2. Remove any remaining references to "escalate to agent" or "agent variant"
3. Skills always return JSON to the calling orchestrator agent

**Tests:**
- [x] Drift prevention: No "Skill vs Agent" sections remain
- [x] Drift prevention: No "escalate to agent" or "agent variant" references remain (NOTE: reviewer's ESCALATE verdict is a spec-defined API value per S05, not skill-to-agent escalation)

**Checkpoint:**
- [x] `grep -r "Skill vs Agent" skills/` returns no matches
- [x] `grep -r "escalate" skills/` returns only reviewer ESCALATE verdict (per S05 spec - this is an API value, not skill-to-agent escalation)
- [x] `grep "allowed-tools: AskUserQuestion" skills/interviewer/SKILL.md` succeeds

**Rollback:**
- Restore skill files from git

**Commit after all checkpoints pass.**

---

##### Step 10.5.5: Delete old agent files {#step-10-5-5}

**Depends on:** #step-10-5-4

**Commit:** `refactor(agents): archive old agents, delete obsolete agent files`

**References:** [D08] Two-agent orchestrator architecture

**Artifacts:**
- `agents/archived/` directory created
- `agents/archived/director.md` - archived
- `agents/archived/interviewer.md` - archived
- Deleted: `agents/architect-agent.md`
- Deleted: `agents/author-agent.md`
- Deleted: `agents/coder-agent.md`

**Tasks:**
- [x] Create `agents/archived/` directory
- [x] Move `agents/director.md` to `agents/archived/director.md`
- [x] Move `agents/interviewer.md` to `agents/archived/interviewer.md`
- [x] Delete `agents/architect-agent.md`
- [x] Delete `agents/author-agent.md`
- [x] Delete `agents/coder-agent.md`

**Tests:**
- [x] Drift prevention: Only 2 agent files remain in `agents/`
- [x] Drift prevention: Archived agents preserved for reference

**Checkpoint:**
- [x] `ls agents/*.md | wc -l` returns 2
- [x] `ls agents/*.md` shows exactly `planner-agent.md` and `implementer-agent.md`
- [x] `ls agents/archived/` shows `director.md` and `interviewer.md`
- [x] `test ! -f agents/architect-agent.md` succeeds
- [x] `test ! -f agents/author-agent.md` succeeds
- [x] `test ! -f agents/coder-agent.md` succeeds

**Rollback:**
- Restore archived agents from `agents/archived/` to `agents/`
- Restore deleted agents from git

**Commit after all checkpoints pass.**

---

##### Step 10.5.6: Delete old entry point skills {#step-10-5-6}

**Depends on:** #step-10-5-5

**Commit:** `refactor(skills): delete old entry point skills (plan, execute)`

**References:** [D08] Two-agent orchestrator architecture

**Artifacts:**
- Deleted: `skills/plan/` directory
- Deleted: `skills/execute/` directory

**Tasks:**
- [x] Delete `skills/plan/` directory entirely
- [x] Delete `skills/execute/` directory entirely

**Tests:**
- [x] Drift prevention: Old entry points removed

**Checkpoint:**
- [x] `test ! -d skills/plan` succeeds
- [x] `test ! -d skills/execute` succeeds
- [x] `ls skills/*/SKILL.md | wc -l` returns 12

**Rollback:**
- Restore `skills/plan/` from git
- Restore `skills/execute/` from git

**Commit after all checkpoints pass.**

---

##### Step 10.5.7: Update documentation {#step-10-5-7}

**Depends on:** #step-10-5-6

**Commit:** `docs: update CLAUDE.md for two-agent architecture`

**References:** [D08] Two-agent orchestrator architecture, (#agents-skills-summary)

**Artifacts:**
- Updated `CLAUDE.md` Agent and Skill Architecture section

**Tasks:**
- [ ] Update `CLAUDE.md` Agent and Skill Architecture section:
  - Change agent count to 2: planner-agent, implementer-agent
  - Change skill count to 12: 2 entry wrappers + 10 sub-tasks
  - Update `/specks:plan` to `/specks:planner`
  - Update `/specks:execute` to `/specks:implementer`
  - Remove all references to "skill-first, agent-escalation" pattern
  - Remove all references to director agent

**Tests:**
- [ ] Drift prevention: CLAUDE.md reflects new architecture

**Checkpoint:**
- [ ] `grep "2 agents" CLAUDE.md` or equivalent shows correct count
- [ ] `grep -c "escalation" CLAUDE.md` returns 0
- [ ] `grep "/specks:planner" CLAUDE.md` succeeds
- [ ] `grep "/specks:implementer" CLAUDE.md` succeeds

**Rollback:**
- Restore `CLAUDE.md` from git

**Commit after all checkpoints pass.**

---

##### Step 10.5.8: Verification tests {#step-10-5-8}

**Depends on:** #step-10-5-7

**Commit:** `test: verify two-agent orchestrator architecture`

**References:** [D08] Two-agent orchestrator architecture, (#exit-criteria)

**Artifacts:**
- Verified plugin loads without errors
- Verified agent discovery works
- Verified skill discovery works
- Verified planning entry point works
- Verified implementation entry point works

**Tasks:**
- [ ] Run `claude --plugin-dir .` - plugin loads without errors
- [ ] Verify agent discovery: both planner-agent and implementer-agent visible
- [ ] Verify skill discovery: all 12 skills visible
- [ ] Test planning entry point: `/specks:planner "test idea"`
- [ ] Test implementation entry point: `/specks:implementer .specks/specks-test.md`

**Tests:**
- [ ] Integration test: Plugin loads without errors
- [ ] Integration test: Planning loop completes without "Aborted()" crash
- [ ] Integration test: Implementation loop completes without "Aborted()" crash

**Checkpoint:**
- [ ] `ls skills/*/SKILL.md | wc -l` returns 12
- [ ] `ls agents/*.md | wc -l` returns 2
- [ ] `ls agents/archived/` shows `director.md`, `interviewer.md`
- [ ] No `architect-agent.md`, `author-agent.md`, `coder-agent.md` exist
- [ ] No `plan/` or `execute/` in `skills/`
- [ ] No "Skill vs Agent" sections in any skill
- [ ] Planning loop completes without "Aborted()" crash
- [ ] Implementation loop completes without "Aborted()" crash

**Rollback:**
- Restore archived agents from `agents/archived/`
- Restore deleted agents from git (architect-agent, author-agent, coder-agent)
- Restore old entry skills from git (plan/, execute/)
- Delete new agents (planner-agent, implementer-agent)
- Delete new entry skill (implementer/)
- Restore planner skill from git
- Restore skill escalation sections from git

**Commit after all checkpoints pass.**

---

#### Step 10.5 Summary {#step-10-5-summary}

After completing Steps 10.5.0–10.5.8, you will have:
- 2 orchestrator agents (planner-agent, implementer-agent) that run complete loops
- 2 thin entry wrapper skills (planner, implementer) that spawn their corresponding agents
- 10 sub-task skills without escalation patterns
- Old agents archived or deleted
- Old entry point skills deleted
- Documentation updated to reflect new architecture

**Final Step 10.5 Checkpoint:**
- [ ] `ls agents/*.md | wc -l` returns 2
- [ ] `ls skills/*/SKILL.md | wc -l` returns 12
- [ ] Planning and implementation loops complete without "Aborted()" crashes
- [ ] Maximum 1 agent context active at any time during loops

**Commit after all checkpoints pass.**

---

#### Step 11: Final cleanup (remove bootstrap skills) {#step-11}

**Depends on:** #step-10-5

**Commit:** `chore: remove bootstrap skills after Phase 3 verification`

**References:** (#step-7), [D06] Clean breaks

**Context:** During Phase 3 implementation, we kept 3 legacy skills in `.claude/skills/` because they were needed to implement Phase 3 itself (bootstrapping problem). Now that Step 10 has verified the new plugin infrastructure works, we can safely remove them.

**Tasks:**
- [ ] Verify Step 10 checkpoints all passed (new infrastructure works)
- [ ] Delete `.claude/skills/implement-plan/`
- [ ] Delete `.claude/skills/update-plan-implementation-log/`
- [ ] Delete `.claude/skills/prepare-git-commit-message/`
- [ ] Delete `.claude/skills/` directory

**Checkpoint:**
- [ ] `.claude/skills/` directory does not exist
- [ ] `/specks:logger` works (replacement for update-plan-implementation-log)
- [ ] `/specks:committer` works (replacement for prepare-git-commit-message)
- [ ] No references to `.claude/skills/` remain in codebase

**Rollback:**
- Restore from git (though at this point the new infrastructure is verified working)

**Commit after all checkpoints pass.**

---

### 3.0.6 Deliverables and Checkpoints {#deliverables}

**Deliverable:** Specks as a Claude Code plugin with **two orchestrator agents** and **skill-only sub-tasks** (no nesting; no escalation).

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

All agents and skills per (#agents-skills-summary) are implemented and functional. Dual-orchestrator architecture verified.

- [ ] `.claude-plugin/plugin.json` exists with valid manifest
- [ ] 2 agent definitions exist in `agents/` (planner-agent, implementer-agent)
- [ ] 12 skill directories exist in `skills/` per (#skill-summary)
- [ ] Entry skills (`/specks:planner`, `/specks:implementer`) spawn orchestrator agents via Task and return their final JSON output
- [ ] No `director.md` in `agents/` (archived)
- [ ] No `plan/` or `execute/` in `skills/` (replaced by planner/implementer)
- [ ] Planning and implementation loops complete WITHOUT "Aborted()" crashes
- [ ] Maximum 1 agent context active at any time during loops
- [ ] CLI has no plan, execute, or setup commands
- [ ] `.claude/skills/` directory fully removed (Step 11 complete)
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo nextest run` passes all tests
- [ ] `claude --plugin-dir .` loads specks as a plugin
- [ ] `/specks:planner` and `/specks:implementer` orchestration skills work
- [ ] `/specks:logger` and `/specks:committer` work (replacements for bootstrap skills)

#### Milestones (Within Phase) {#milestones}

**Milestone M01: Plugin Structure Created** {#m01-plugin-created}
- [ ] Plugin manifest exists
- [ ] Initial 8 skills created in `skills/` (pre-Step 10.5)
- [ ] Note: After Step 10.5, final count is 12 skills (see M04.5)
- Steps 0-3 complete

**Milestone M02: Agents Updated (Interim)** {#m02-agents-updated}
- [ ] ~~Director created as pure orchestrator with Skill tool (interim, pre-Step 10.5)~~ **SUPERSEDED**
- [ ] 7 agent files removed (6 became skills, 1 eliminated)
- [ ] ~~5 agents remain temporarily~~ → After Step 10.5: 2 agents remain
- Steps 4-6 complete
- Note: This milestone represents interim state; final architecture in M04.5

**Milestone M03: Legacy Removed** {#m03-legacy-removed}
- [ ] Obsolete `.claude/skills/` entries removed (specks-plan, specks-execute)
- [ ] Rust orchestration code removed
- [ ] Bootstrap skills retained temporarily
- Steps 7-8 complete

**Milestone M04: Documentation and Verification Complete** {#m04-docs-complete}
- [ ] All docs updated
- [ ] Plugin verified working
- Steps 9-10 complete

**Milestone M04.5: Dual-Orchestrator Architecture Complete** {#m04-5-dual-orchestrator}
- [ ] Director deleted (archived)
- [ ] 2 agents with `-agent` suffix: planner-agent, implementer-agent
- [ ] 2 entry wrapper skills: planner, implementer
- [ ] 10 sub-task skills: architect, auditor, author, clarifier, coder, committer, critic, interviewer, logger, reviewer
- [ ] Old entry point skills (plan, execute) deleted
- [ ] 12 skills total (2 entry wrappers + 10 sub-tasks)
- [ ] Maximum 1 agent context at any time
- [ ] No "Aborted()" crashes during planning/implementation loops
- Step 10.5 complete

**Milestone M05: Bootstrap Cleanup Complete** {#m05-bootstrap-cleanup}
- [ ] `.claude/skills/` fully removed (bootstrap skills deleted)
- [ ] New skills (`/specks:logger`, `/specks:committer`) verified working
- Step 11 complete

#### Roadmap / Follow-ons (Explicitly Not Required for Phase Close) {#roadmap}

- [ ] Phase 4: Test full planning loop inside Claude Code
- [ ] Phase 5: Test full implementation loop inside Claude Code
- [ ] Create public plugin marketplace for specks distribution
- [ ] Performance benchmarking

| Checkpoint | Verification |
|------------|--------------|
| Plugin manifest | `.claude-plugin/plugin.json` exists and is valid JSON |
| Skills count | `ls skills/*/SKILL.md \| wc -l` returns 12 |
| Agents count | `ls agents/*.md \| wc -l` returns 2 |
| Agent naming | All agents have `-agent` suffix (planner-agent, implementer-agent) |
| No director | `ls agents/director.md` fails (archived) |
| No old entry skills | `ls skills/plan/` and `ls skills/execute/` both fail (deleted) |
| Orchestrators exist | `ls skills/planner/SKILL.md skills/implementer/SKILL.md` succeeds |
| CLI simplified | `specks --help` shows no plan/execute/setup |
| Build clean | `cargo build` with no warnings |
| Tests pass | `cargo nextest run` passes |
| Plugin loads | `claude --plugin-dir .` succeeds |
| Run directory | `.specks/runs/<session-id>/` created with metadata.json and skill outputs |
| Beads callable | Orchestrator agents can invoke `specks beads status --json` via Bash and parse output |
| Max 1 agent | Debug log shows maximum 1 agent context during any loop |
| No crashes | Planning/implementation loops complete without "Aborted()" |

**Commit after all checkpoints pass.**
