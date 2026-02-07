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

This phase defines **3 agents and 12 skills** (dual-orchestrator architecture). Two top-level skills (`planner` and `implementer`) run the planning and implementation loops synchronously. No director agent. Sub-tasks run as skills for simple work; agent variants exist for complex tasks.

**Architecture principle:** Skill-first, agent-escalation. Orchestrators try lightweight skills first, escalate to agent variants when tasks need more power. Maximum ONE agent context active at any time.

**Terminology:** A `task` refers to either a skill or an agent. Sub-tasks are the workers that orchestrators invoke.

#### Agents {#agent-summary}

Agents have `-agent` suffix to distinguish from skill counterparts.

| Agent | Remit |
|-------|-------|
| **architect-agent** | Full agentic power for complex implementation strategy creation. Used when skill variant is insufficient. |
| **author-agent** | Full agentic power for complex speck creation/revision. Used when skill variant is insufficient. |
| **coder-agent** | Full agentic power for complex implementation with drift detection. Used when skill variant is insufficient. |

**Agent count:** 3 agent files total (architect-agent, author-agent, coder-agent).

#### Skills {#skill-summary}

**Orchestrators (entry points):**

| Skill | Spec | Remit |
|-------|------|-------|
| **planner** | S01 | ORCHESTRATOR. Entry point `/specks:planner`. Runs planning loop synchronously. Invokes sub-tasks one at a time. |
| **implementer** | S02 | ORCHESTRATOR. Entry point `/specks:implementer`. Runs implementation loop synchronously. Invokes sub-tasks one at a time. |

**Sub-tasks (skill-only):**

| Skill | Spec | Remit |
|-------|------|-------|
| **clarifier** | S03 | Analyzes idea or critic feedback. Returns clarifying questions with options. JSON output. |
| **critic** | S04 | Reviews speck for skeleton compliance, completeness, implementability. Returns APPROVE/REVISE/REJECT. JSON output. |
| **reviewer** | S05 | Verifies completed step matches plan. Checks tasks, tests, artifacts. Returns APPROVE/REVISE/ESCALATE. JSON output. |
| **auditor** | S06 | Checks code quality, error handling, security. Returns severity-ranked issues. JSON output. |
| **logger** | S07 | Updates `.specks/specks-implementation-log.md` with completed work. JSON output. |
| **committer** | S08 | Finalizes step: stages files, commits changes, closes associated bead. JSON output. |
| **interviewer** | S09 | Single point of user interaction. Presents questions/feedback via AskUserQuestion. Returns structured decisions. |

**Sub-tasks (skill+agent pairs):**

| Skill | Spec | Agent Variant | Remit |
|-------|------|---------------|-------|
| **architect** | S10 | architect-agent | Creates implementation strategies for steps. Returns JSON with expected touch set. Read-only. |
| **author** | S11 | author-agent | Creates and revises speck documents. Writes structured markdown to `.specks/`. |
| **coder** | S12 | coder-agent | Executes architect strategies with drift detection. Writes code, runs tests. |

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
- Namespacing: `/specks:planner`, `specks:coder-agent`, etc.
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
- `agents/` directory at repo root contains 3 agents per (#agent-summary)
- `claude --plugin-dir .` loads specks as a plugin with all skills/agents available
- `/specks:planner` and `/specks:implementer` orchestration skills work
- Skill-first, agent-escalation pattern demonstrated
- Maximum 1 agent context active at any time
- `specks plan` and `specks execute` CLI commands removed
- `specks setup claude` command removed
- `.claude/skills/` directory removed (replaced by `skills/`)
- `cargo build` succeeds with no warnings
- `cargo nextest run` passes all tests

#### Scope {#scope}

1. Create `.claude-plugin/plugin.json` manifest
2. Create `skills/` directory with 12 skills:
   - 2 orchestrators: `planner`, `implementer`
   - 7 skill-only sub-tasks: auditor, clarifier, committer, critic, interviewer, logger, reviewer
   - 3 skill+agent pairs (skill side): architect, author, coder
3. Create/rename agent definitions at `agents/` (3 agents with `-agent` suffix: architect-agent, author-agent, coder-agent)
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

**Resolution:** DECIDED - use `specks`. Skills become `/specks:planner`, `/specks:implementer`, etc. Agents become `specks:coder-agent`, etc.

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
- After D08: 3 agents remain with `-agent` suffix: architect-agent, author-agent, coder-agent
- Coder-agent includes self-monitoring for drift detection (see #smart-drift)
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

**Decision:** Replace director agent with two top-level orchestration skills: `planner` (planning loop) and `implementer` (implementation loop). No director. No nested agents. Skills run inline. Agent variants exist for complex sub-tasks.

**Rationale:**
- The original multi-agent design created 11+ nested agent contexts during execution
- Claude Code terminal rendering cannot handle this many concurrent contexts
- Crashes with "Aborted()" message due to rendering overload ("High write ratio: 100% writes")
- Director was just a router—orchestration logic collapses into the two entry points
- Both planning and implementation loops are straightforward; no need for agentic complexity

**Architecture:**
```
Entry Points (orchestration skills, run inline):
  /specks:planner     → runs planning loop synchronously
  /specks:implementer → runs implementation loop synchronously

Sub-tasks:
  Skill-only:                   Skill + Agent pairs:
  ───────────────               ─────────────────────
  auditor                       architect / architect-agent
  clarifier                     author / author-agent
  committer                     coder / coder-agent
  critic
  interviewer (TBD)
  logger
  reviewer
```

**Naming:**
- Current `planner` agent → renamed to `author` (skill + agent)
- Current `implementer` agent → renamed to `coder` (skill + agent)
- All agents have `-agent` suffix to distinguish from skill counterparts
- `planner` and `implementer` become the new orchestration skill names

**Skill-first, agent-escalation:**
- Orchestrators invoke skill variants by default for simple tasks
- Escalate to agent variants when tasks are complex or skill variant struggles
- Orchestrators monitor progress and can retry with more "agent muscle" if needed
- One-task-at-a-time constraint: orchestrators invoke sub-tasks sequentially

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
| Agent | `agents/coder-agent.md` | `name: coder-agent` | N/A | `Task(subagent_type: "specks:coder-agent")` |

**Agent naming rule:** All agents have `-agent` suffix to distinguish from skill counterparts.
- `architect-agent`, `author-agent`, `coder-agent` (required)

**Namespacing rule (applies to BOTH skills AND agents):**
- Plugin name `specks` provides the namespace prefix automatically
- Skill folder `skills/planner/` becomes `/specks:planner`
- Agent file `agents/coder-agent.md` becomes `specks:coder-agent`
- Both Skill and Task tools use the colon-namespaced format

**Consistency rule:** Always use fully-qualified namespaced names in code:
- `Skill(skill: "specks:clarifier")` not `Skill(skill: "clarifier")`
- `Task(subagent_type: "specks:coder-agent")` not `Task(subagent_type: "coder-agent")`

**Syntax verification:** The exact Skill tool invocation syntax (`Skill(skill: "specks:clarifier")`) is verified in Step 10. If the actual syntax differs, update this section and all references before proceeding past Step 10. Step 10 is the hard gate for syntax correctness.

**Plugin directory layout:**

```
specks/                           # Plugin root (repo root)
├── .claude-plugin/
│   └── plugin.json              # Plugin manifest
├── skills/                       # Skills (auto-discovered)
│   ├── planner/                 # ORCHESTRATOR - planning loop
│   │   └── SKILL.md
│   ├── implementer/             # ORCHESTRATOR - implementation loop
│   │   └── SKILL.md
│   ├── architect/               # Sub-task (has agent pair)
│   │   └── SKILL.md
│   ├── author/                  # Sub-task (has agent pair)
│   │   └── SKILL.md
│   ├── coder/                   # Sub-task (has agent pair)
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
│   ├── architect-agent.md       # Agent pair for architect skill
│   ├── author-agent.md          # Agent pair for author skill
│   ├── coder-agent.md           # Agent pair for coder skill
│   └── (interviewer is skill-only, no agent needed)
├── agents/archived/              # Old/replaced agent files
│   └── director.md              # Archived after Step 10.5
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

These flowcharts define the orchestration logic for the planner and implementer skills. All steps reference these flows.

**Legend:**
- `[AGENT]` = spawned via Task tool (isolated context) - used for escalation
- `(SKILL)` = invoked via Skill tool (inline, JSON output) - default for sub-tasks
- `{USER}` = interaction via interviewer skill (AskUserQuestion)

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
│  │ (PLANNER) orchestration skill receives input, runs INLINE            │   │
│  │                                                                       │   │
│  │ 1. Invoke (CLARIFIER) skill                                          │   │
│  │    → Returns: analysis{}, questions[], assumptions[]                  │   │
│  │                                                                       │   │
│  │ 2. IF questions exist:                                                │   │
│  │    → Invoke (INTERVIEWER) skill/agent with questions                  │   │
│  │    → Interviewer uses AskUserQuestion                                 │   │
│  │    → Returns: user_answers{}                                          │   │
│  │                                                                       │   │
│  │ 3. Invoke (AUTHOR) skill with:  ← TRY SKILL FIRST                     │   │
│  │    - Original idea/speck                                              │   │
│  │    - User answers (if any)                                            │   │
│  │    - Clarifier assumptions                                            │   │
│  │    → Returns: draft speck path                                        │   │
│  │                                                                       │   │
│  │    IF task is complex → ESCALATE to [AUTHOR-AGENT]                    │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│       │                                                                      │
│       ▼                                                                      │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ (PLANNER) review loop                                                 │   │
│  │                                                                       │   │
│  │ 4. Invoke (CRITIC) skill with draft speck                             │   │
│  │    → Returns: skeleton_compliant, areas{}, issues[], recommendation   │   │
│  │                                                                       │   │
│  │ 5. IF recommendation == REJECT or REVISE:                             │   │
│  │    → Invoke (INTERVIEWER) skill/agent with critic issues              │   │
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
- PLANNER is an orchestration skill, runs inline (no agent context)
- Skill-first pattern: try (AUTHOR) skill, escalate to [AUTHOR-AGENT] if complex
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
│  │ (IMPLEMENTER) orchestration skill receives speck, runs INLINE        │   │
│  │                                                                       │   │
│  │ FOR EACH step in speck.execution_steps:                               │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│       │                                                                      │
│       ▼                                                                      │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ STEP 1: Get Implementation Strategy                                   │   │
│  │                                                                       │   │
│  │ Invoke (ARCHITECT) skill with step details  ← TRY SKILL FIRST         │   │
│  │ → Returns: strategy, expected_touch_set[], test_plan                  │   │
│  │                                                                       │   │
│  │ IF strategy is complex → ESCALATE to [ARCHITECT-AGENT]                │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│       │                                                                      │
│       ▼                                                                      │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ STEP 2: Implementation (with Self-Monitoring)                         │   │
│  │                                                                       │   │
│  │ Invoke (CODER) skill with architect strategy  ← TRY SKILL FIRST       │   │
│  │ → Coder reads strategy, writes code, runs tests                       │   │
│  │ → Coder self-monitors against expected_touch_set (see #smart-drift)   │   │
│  │ → Returns: success/failure + drift_assessment                         │   │
│  │                                                                       │   │
│  │ IF task is complex → ESCALATE to [CODER-AGENT]                        │   │
│  │                                                                       │   │
│  │ IF coder.halted_for_drift:                                            │   │
│  │   → Invoke (INTERVIEWER) skill/agent with drift details               │   │
│  │   → User decides: continue anyway? back to architect? abort?          │   │
│  │   → IMPLEMENTER acts on user decision                                 │   │
│  │                                                                       │   │
│  │ IF coder.success == false (non-drift failure):                        │   │
│  │   → Handle error, may retry or escalate to [CODER-AGENT]              │   │
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
│  │   ├─ Minor quality issues → Re-invoke (CODER) or [CODER-AGENT]        │   │
│  │   ├─ Design issues → Back to (ARCHITECT) or [ARCHITECT-AGENT]         │   │
│  │   └─ Conceptual issues → Invoke (INTERVIEWER), may need re-planning   │   │
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
- IMPLEMENTER is an orchestration skill, runs inline (no agent context)
- Skill-first pattern: try (ARCHITECT)/(CODER) skills, escalate to agents if complex
- One sub-task at a time (sequential invocation, no parallelism)
- Maximum 1 agent context at any moment
- Coder includes **self-monitoring** for drift detection (see #smart-drift)
- ALL escalation decisions go through interviewer for user input
- Logger and committer are invoked after each successful step

---

#### Tool Invocation Summary {#flow-tools}

**Orchestrators (entry points, run inline):**

| Component | Type | User Invocation | Purpose |
|-----------|------|-----------------|---------|
| **planner** | Skill | `/specks:planner "idea"` | Orchestrates planning loop |
| **implementer** | Skill | `/specks:implementer path.md` | Orchestrates implementation loop |

**Sub-tasks with skill+agent pairs:**

| Skill | Agent | Purpose |
|-------|-------|---------|
| `Skill(skill: "specks:architect")` | `Task(subagent_type: "specks:architect-agent")` | Implementation strategies |
| `Skill(skill: "specks:author")` | `Task(subagent_type: "specks:author-agent")` | Creates/revises speck documents |
| `Skill(skill: "specks:coder")` | `Task(subagent_type: "specks:coder-agent")` | Executes strategies with drift detection |

**Skill-only sub-tasks:**

| Component | Invocation | Purpose |
|-----------|------------|---------|
| **auditor** | `Skill(skill: "specks:auditor")` | Code quality/security checks |
| **clarifier** | `Skill(skill: "specks:clarifier")` | Generates clarifying questions |
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
| **planner orchestrator** | Write, Bash | Creates run directory, writes metadata.json and all sub-task outputs for planning phase |
| **implementer orchestrator** | Write, Bash | Creates run directory, writes metadata.json and all sub-task outputs for execution phase |
| **author** | Write | Writes draft speck to `.specks/` (not runs/) |
| **architect** | Read | Reads speck; orchestrator persists strategy to runs/ |
| **All sub-tasks** | Varies | Return JSON to orchestrator; orchestrator persists to runs/ |

**Key design:** Sub-tasks return JSON to their orchestrator. The orchestrator (planner or implementer) persists all outputs to the runs directory. This keeps sub-tasks focused and orchestrators as single source of truth for audit trail.

#### Session ID Format {#session-id}

Format: `YYYYMMDD-HHMMSS-<mode>-<short-uuid>`

Examples:
- `20260206-143022-plan-a1b2c3`
- `20260206-150145-execute-d4e5f6`

**Generation method:** The orchestrator generates session ID at start via Bash using `/dev/urandom` (portable across macOS and Linux):

```bash
# Generate session ID (MODE is "plan" or "impl")
SESSION_ID="$(date +%Y%m%d-%H%M%S)-${MODE}-$(head -c 3 /dev/urandom | xxd -p)"
```

*Note: Uses `/dev/urandom` instead of `uuidgen` for portability. `date +%N` not used because it's unsupported on macOS.*

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

Orchestrators write JSON to the runs directory using the **Write tool** (not Bash). This avoids all escaping issues and is the natural tool for file creation.

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
| Task | Escalating to agent variants when needed |

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

**Orchestrators** (entry points, run the loops):

| Skill | allowed-tools | Reason |
|-------|---------------|--------|
| **planner** | Skill, Task, Read, Grep, Glob, Write, Bash | Orchestrates planning loop, invokes sub-tasks, persists to runs/ |
| **implementer** | Skill, Task, Read, Grep, Glob, Write, Bash | Orchestrates implementation loop, invokes sub-tasks, persists to runs/ |

**Sub-tasks with skill+agent pairs:**

| Skill | allowed-tools | Agent variant |
|-------|---------------|---------------|
| **architect** | Read, Grep, Glob | architect-agent (full agentic power) |
| **author** | Read, Grep, Glob, Write, Edit | author-agent (full agentic power) |
| **coder** | Read, Grep, Glob, Write, Edit, Bash | coder-agent (includes drift detection) |

**Skill-only sub-tasks:**

| Skill | allowed-tools | Reason |
|-------|---------------|--------|
| **auditor** | Read, Grep, Glob | Reads code for quality |
| **clarifier** | Read, Grep, Glob | Reads codebase for context |
| **committer** | Read, Grep, Glob, Bash | `git add`, `git commit`, `specks beads close` |
| **critic** | Read, Grep, Glob | Reads speck for review |
| **interviewer** | AskUserQuestion | Presents questions to user |
| **logger** | Read, Grep, Glob, Edit | Updates implementation log |
| **reviewer** | Read, Grep, Glob | Checks plan artifacts |

**Baseline:** All skills get `Read, Grep, Glob` for codebase access. Additional tools added only where needed.

#### Skill Tool Invocation Contract {#skill-invocation-contract}

**Invocation:** Orchestrators invoke sub-tasks via the Skill tool (or Task tool for agent escalation) using the fully-qualified name and a JSON payload.

**Skill invocation:**
```
Skill(skill: "specks:clarifier", args: '{"idea": "...", "speck_path": null}')
```

**Agent escalation:**
```
Task(subagent_type: "specks:author-agent", prompt: "Create speck for: ...")
```

##### How Skills Work (Critical Context) {#how-skills-work}

**Skills work through prompt injection, not function calls.** This has important implications for orchestration:

| Mechanism | How It Works | Output Capture |
|-----------|--------------|----------------|
| **Skill tool** | Injects skill content into Claude's prompt as instructions | Output flows through conversation context, visible to Claude's reasoning |
| **Task tool** | Spawns isolated subagent with separate context | Agent returns structured output when complete |

**Key insight:** Skills don't have return values. When Claude invokes a skill, the skill's SKILL.md content becomes part of Claude's instructions, and Claude executes within that expanded context.

**Implications for orchestration:**
1. **Skills don't need `Skill` in allowed-tools** - Claude decides which skills to use based on context and skill descriptions
2. **Output capture is implicit** - Sub-task skill output appears in the conversation, and the orchestrator (which is also a skill running in Claude's context) can parse it
3. **JSON output convention** - Sub-tasks output JSON-only so Claude can parse it as it continues reasoning
4. **For complex I/O needs** - Use Task tool (agents) which have explicit input/output boundaries

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
- If the second run fails, escalate: try agent variant if available, or invoke interviewer
- Persist the raw JSON in the run directory for audit

**Escalation triggers:**
- Sub-task returns error or invalid JSON
- Task complexity exceeds skill capability (heuristic: 3+ files, deep codebase exploration)
- Reviewer/critic reports issues that skill variant cannot address

#### planner Skill (Orchestrator) {#skill-planner}

**Spec S01: planner** {#s01-planner}

**Flow:** See (#flow-planning) for complete orchestration.

```yaml
---
name: planner
description: Orchestrates the planning loop from idea to approved speck
disable-model-invocation: true
allowed-tools: Skill, Task, Read, Grep, Glob, Write, Bash
---
```

**Behavior:**
1. Accepts: idea text OR path to existing speck
2. **Setup phase** (BEFORE any sub-task invocation):
   ```bash
   MODE="plan"
   SESSION_ID="$(date +%Y%m%d-%H%M%S)-${MODE}-$(head -c 3 /dev/urandom | xxd -p)"
   mkdir -p .specks/runs/${SESSION_ID}/planning
   ```
   Then write `metadata.json` using Write tool.
3. Executes Planning Phase Flow (#flow-planning) directly:
   - Invoke `Skill(specks:clarifier)` → persist to `001-clarifier.json`
   - If questions: invoke `Skill(specks:interviewer)` → persist to `002-interviewer.json`
   - Invoke `Skill(specks:author)` → persist to `003-author.json`
     - Escalate to `Task(specks:author-agent)` if complex
   - Invoke `Skill(specks:critic)` → persist to `004-critic.json`
   - Loop if REVISE (increment counter), exit if APPROVE
4. **Finalize phase**: Update `metadata.json` with `status: "completed"` and `completed_at`
5. Returns: path to approved speck

**State management:**
- Orchestrator persists all sub-task outputs to runs directory using Write tool
- File naming: `NNN-<subtask>.json` where NNN is zero-padded counter
- Each sub-task receives relevant context as JSON input
- One sub-task at a time (sequential invocation)

**Escalation triggers:**
- Author skill fails validation twice → escalate to author-agent
- Critic issues require significant restructuring → escalate to author-agent

---

#### implementer Skill (Orchestrator) {#skill-implementer}

**Spec S02: implementer** {#s02-implementer}

**Flow:** See (#flow-implementation) for complete orchestration.

```yaml
---
name: implementer
description: Orchestrates the implementation loop from speck to completed code
disable-model-invocation: true
allowed-tools: Skill, Task, Read, Grep, Glob, Write, Bash
---
```

**Behavior:**
1. Accepts: path to approved speck
2. **Setup phase** (BEFORE any sub-task invocation):
   ```bash
   MODE="impl"
   SESSION_ID="$(date +%Y%m%d-%H%M%S)-${MODE}-$(head -c 3 /dev/urandom | xxd -p)"
   mkdir -p .specks/runs/${SESSION_ID}/execution
   ```
   Then write `metadata.json` using Write tool.
3. **For each step** in speck (iterate in order, respecting dependencies):
   - Create step subdirectory: `mkdir -p .specks/runs/${SESSION_ID}/execution/step-N`
   - Invoke `Skill(specks:architect)` → persist to `step-N/architect.json`
     - Escalate to `Task(specks:architect-agent)` if strategy is complex
   - Invoke `Skill(specks:coder)` → persist to `step-N/coder.json`
     - Escalate to `Task(specks:coder-agent)` if implementation is complex or drift detected
   - Invoke `Skill(specks:reviewer)` → persist to `step-N/reviewer.json`
   - Invoke `Skill(specks:auditor)` → persist to `step-N/auditor.json`
   - Handle issues: retry coder, go back to architect, or invoke interviewer
   - Invoke `Skill(specks:logger)` → persist to `step-N/logger.json`
   - Invoke `Skill(specks:committer)` → persist to `step-N/committer.json`
4. **Finalize phase**: Write `execution/summary.json`, update `metadata.json` with `status: "completed"`
5. Returns: completion status, log of changes

**State management:**
- Orchestrator persists all sub-task outputs to runs directory per step using Write tool
- Step directories: `execution/step-0/`, `execution/step-1/`, etc.
- Each sub-task receives relevant context as JSON input
- One sub-task at a time (sequential invocation)

**Escalation triggers:**
- Architect skill struggles with complex codebase → escalate to architect-agent
- Coder skill fails tests or detects drift → escalate to coder-agent
- Coder skill touches 3+ unexpected files → escalate to coder-agent

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
  "bead_id": "string | null",
  "warnings": ["string"]
}
```
- `warnings`: Non-fatal issues encountered (e.g., bead already closed, bead close failed but commit succeeded)

**Edge Case Handling:**

| Scenario | Behavior | Output |
|----------|----------|--------|
| Bead already closed | Commit proceeds, report warning | `bead_closed: true, warnings: ["Bead already closed"]` |
| Bead ID not found | Commit proceeds, report warning | `bead_closed: false, warnings: ["Bead not found: <id>"]` |
| Commit succeeds, bead close fails | Report partial success | `committed: true, bead_closed: false, warnings: ["Bead close failed: <reason>"]` |
| No bead_id provided | Commit only, skip bead close | `bead_closed: false, bead_id: null` |
| Bead sync out of date | Not detectable by committer | Orchestrator responsibility to sync before implementation |

**Principle:** The commit is the primary deliverable. Bead operations are secondary. If bead close fails, the committer still reports success for the commit but includes a warning about the bead failure.

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
*Note: Read-only skill. For complex strategies requiring deep exploration, orchestrator escalates to architect-agent.*

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
*Note: For complex speck creation or major restructuring, orchestrator escalates to author-agent.*

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
*Note: Includes drift detection. For complex implementations or when drift is detected, orchestrator escalates to coder-agent.*

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

Orchestrators follow a **skill-first, agent-escalation** pattern. This section consolidates the escalation decision logic.

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
| **committer** | `speck_path`, `step_anchor`, `proposed_message`, `files_to_stage`, `bead_id` |

#### Persistence {#state-persistence}

Orchestrators persist all sub-task outputs to the run directory:
```
.specks/runs/<session-id>/
├── metadata.json           # Session state
├── planning/               # Planning phase
│   ├── 001-clarifier.json
│   ├── 002-interviewer.json
│   └── ...
└── implementation/         # Implementation phase (per step)
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
   - If `committer.json` exists: step complete, move to next step
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
- Corrupted metadata.json: Report error, cannot resume

---

#### coder-agent (Agent with Skill Variant) {#coder-agent-note}

**Note:** Under the dual-orchestrator architecture (D08), implementation work is handled by:
- `coder` **skill** - lightweight variant for simple implementations
- `coder-agent` **agent** - full agentic power for complex implementations, includes self-monitoring for drift detection

The orchestrator (`implementer` skill) tries the `coder` skill first and escalates to `coder-agent` when needed. See (#coder-agent-contract) for the input/output contract and drift detection heuristics.

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

These agents become skills per (#agents-skills-summary). Note: Under D08, the old `implementer` agent is renamed to `coder-agent` (with a `coder` skill variant). Monitor is eliminated entirely (no skill replacement).

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
| architect-agent | Read, Grep, Glob, Bash |
| author-agent | Read, Grep, Glob, Bash, Write, Edit |
| coder-agent | Read, Grep, Glob, Bash, Write, Edit |

*Note: director is deleted. Interviewer is now a skill (AskUserQuestion works from skills). Planner → author-agent. Implementer → coder-agent.*

#### Architect Agent Output Contract {#architect-output}

The architect-agent returns JSON to the orchestrator, which persists it to the runs directory and passes it to the coder (skill or agent).

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

#### Coder Agent Contract {#coder-agent-contract}

The coder-agent executes architect strategies. It includes **self-monitoring** for drift detection: after each implementation sub-step, the coder checks its own changes against the expected_touch_set and halts if drift thresholds are exceeded.

*Note: The coder skill is a lightweight variant for simple implementations. The coder-agent is used for complex implementations or when the skill detects drift. Both share the same input/output contracts.*

**Coder Agent Definition:**
```yaml
---
name: coder-agent
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

#### Step 10.5: Dual-Orchestrator Architecture {#step-10-5}

**Depends on:** #step-0

**Commit:** `refactor: dual-orchestrator architecture with skill-first sub-tasks`

**References:** [D08] Dual-orchestrator architecture, (#agents-skills-summary), (#flow-planning), (#flow-implementation)

**Context:** The current architecture creates 11+ nested agent contexts (director spawns architect, implementer, interviewer as agents), causing Claude Code to crash with "Aborted()" due to terminal rendering overload. This step replaces the director with two orchestration skills and implements skill-first, agent-escalation for sub-tasks.

**Architecture Change:**

Before (BROKEN - 11+ contexts):
```
/specks:plan or /specks:execute (skill)
  └── director (agent)
        ├── planner (agent)
        ├── architect (agent) x N steps
        ├── implementer (agent) x N steps
        └── interviewer (agent) x ~3 per step
```

After (STABLE - 0-1 agent contexts):
```
/specks:planner (orchestration skill, runs inline)
  ├── Skill(specks:clarifier)
  ├── Skill(specks:author)      ← simple tasks
  │   OR Task(specks:author-agent) ← complex tasks
  ├── Skill(specks:critic)
  └── Skill(specks:interviewer)

/specks:implementer (orchestration skill, runs inline)
  ├── Skill(specks:architect)   ← simple strategies
  │   OR Task(specks:architect-agent) ← complex strategies
  ├── Skill(specks:coder)       ← simple implementations
  │   OR Task(specks:coder-agent) ← complex implementations
  ├── Skill(specks:reviewer)
  ├── Skill(specks:auditor)
  ├── Skill(specks:logger)
  └── Skill(specks:committer)
```

**Naming Changes:**
- `planner` agent → `author` (skill) + `author-agent` (agent)
- `implementer` agent → `coder` (skill) + `coder-agent` (agent)
- `architect` agent → `architect` (skill) + `architect-agent` (agent)
- `planner` → NEW orchestration skill (planning loop)
- `implementer` → NEW orchestration skill (implementation loop)
- `plan` and `execute` entry point skills → DELETED (replaced by planner/implementer)
- `director` agent → DELETED (orchestration absorbed into planner/implementer)

**Key Principles:**
1. **Skill-first:** Orchestrators invoke skill variants by default
2. **Agent-escalation:** Escalate to agent variant when task is complex or skill struggles
3. **One-at-a-time:** Orchestrators invoke sub-tasks sequentially (no parallelism)
4. **Max 1 agent:** At most one agent context active at any time
5. **Direct invocation:** Any sub-task can be invoked directly via `/specks:<name>`

**Artifacts:**
- `skills/planner/SKILL.md` - NEW orchestration skill (planning loop)
- `skills/implementer/SKILL.md` - NEW orchestration skill (implementation loop)
- `skills/architect/SKILL.md` - NEW skill variant
- `skills/author/SKILL.md` - NEW skill variant (renamed from planner)
- `skills/coder/SKILL.md` - NEW skill variant (renamed from implementer)
- `agents/architect-agent.md` - RENAMED from architect.md
- `agents/author-agent.md` - RENAMED from planner.md
- `agents/coder-agent.md` - RENAMED from implementer.md
- `agents/archived/` - Old agent files (director.md, interviewer.md) and deleted entry skills

**Tasks:**

##### 10.5.1: Verify skill tool invocation capabilities {#step-10-5-1}

**STATUS: VERIFIED** (2026-02-07)

All three capability tests passed:

| Test | Result | Implication |
|------|--------|-------------|
| Skill→Skill | ✓ SUCCESS | Orchestrators CAN invoke sub-task skills |
| Skill→Agent | ✓ SUCCESS | Orchestrators CAN escalate to agents |
| AskUserQuestion | ✓ SUCCESS | Interviewer is skill-only (no agent fallback) |

- [x] Created test skills: `test-skill-invoke`, `test-task-invoke`, `test-ask`
- [x] Ran `claude --plugin-dir .` and invoked all three tests
- [x] All tests passed - architecture is technically feasible
- [x] Deleted test skills after verification

**Verified Syntax (AUTHORITATIVE):**

| Tool | Syntax | Example |
|------|--------|---------|
| Skill invocation | `Skill(skill: "<namespace>:<name>", args: <JSON>)` | `Skill(skill: "specks:clarifier", args: '{"idea": "..."}')`  |
| Agent escalation | `Task(subagent_type: "<namespace>:<name>-agent", prompt: "...")` | `Task(subagent_type: "specks:author-agent", prompt: "Create speck for: ...")` |

**Skill argument passing:**
- The `args` parameter accepts JSON strings
- Skills receive the JSON args as a string via `$ARGUMENTS` (i.e., the `args` payload is exposed to the skill content as `$ARGUMENTS`)
- If skill doesn't use `$ARGUMENTS`, Claude Code appends `ARGUMENTS: <input>` to skill content

##### 10.5.2: Create interviewer skill (CRITICAL PATH) {#step-10-5-2}

**Priority:** FIRST - Interviewer is on critical path for all user interaction.

**AskUserQuestion WORKS from skill context** (verified in 10.5.1)

**Tasks:**
- [x] Create `skills/interviewer/SKILL.md`
- [x] Verify skill can be invoked by orchestrators (AskUserQuestion verified in 10.5.1)

**Full SKILL.md content:**

```markdown
---
name: interviewer
description: Single point of user interaction for orchestration workflows
allowed-tools: AskUserQuestion
---

## Purpose

Handle ALL user interaction for specks orchestration. Receive questions, issues, or decisions from orchestrators and present them to the user via AskUserQuestion.

## Input

You receive JSON input via $ARGUMENTS:

\`\`\`json
{
  "context": "clarifier | critic | drift | review",
  "speck_path": ".specks/specks-N.md",
  "step_anchor": "#step-N | null",
  "payload": { ... }
}
\`\`\`

### Payload by context:

| Context | Payload | What to present |
|---------|---------|-----------------|
| `clarifier` | `{questions: [...], assumptions: [...]}` | Clarifying questions before planning |
| `critic` | `{issues: [...], recommendation: "..."}` | Critic feedback on draft speck |
| `drift` | `{drift_assessment: {...}, files_touched: [...]}` | Coder halted due to drift |
| `review` | `{issues: [...], source: "reviewer|auditor"}` | Issues from review phase |

## Behavior

1. Parse the input JSON from $ARGUMENTS
2. Format the payload into clear, actionable questions
3. Use AskUserQuestion to present to user
4. Capture user's decision
5. Return structured JSON output

## Output

Return JSON-only (no prose, no fences):

\`\`\`json
{
  "context": "clarifier | critic | drift | review",
  "decision": "continue | halt | revise",
  "user_answers": { ... },
  "notes": "string | null"
}
\`\`\`
```

**Verification:**
- [x] Invoke `/specks:interviewer` directly with test JSON
- [x] Verify AskUserQuestion presents options correctly
- [x] Verify JSON output is returned

---

##### 10.5.3: Create architect skill+agent (simplest pair, read-only) {#step-10-5-3}

**Priority:** SECOND - Simplest skill+agent pair, read-only operations only.

**Tasks:**
- [ ] Rename `agents/architect.md` → `agents/architect-agent.md`
- [ ] Update agent frontmatter: `name: architect-agent`
- [ ] Create `skills/architect/SKILL.md`

**Full SKILL.md content:**

```markdown
---
name: architect
description: Creates implementation strategies for speck steps
allowed-tools: Read, Grep, Glob
---

## Purpose

Create implementation strategies for speck steps. Analyze the step requirements, explore the codebase, and produce a strategy with expected file changes.

## Input

You receive JSON input via $ARGUMENTS:

\`\`\`json
{
  "speck_path": ".specks/specks-N.md",
  "step_anchor": "#step-N",
  "revision_feedback": "string | null"
}
\`\`\`

## Behavior

1. Read the speck and locate the specified step
2. Analyze what the step requires (tasks, artifacts, tests)
3. Explore the codebase to understand current state
4. Design an implementation approach
5. Identify ALL files that will be created or modified
6. Return structured strategy JSON

## Output

Return JSON-only (no prose, no fences):

\`\`\`json
{
  "step_anchor": "#step-N",
  "approach": "High-level description of implementation approach",
  "expected_touch_set": ["path/to/file1.rs", "path/to/file2.rs"],
  "implementation_steps": [
    {"order": 1, "description": "Create X", "files": ["path/to/file.rs"]},
    {"order": 2, "description": "Update Y", "files": ["path/to/other.rs"]}
  ],
  "test_plan": "How to verify the implementation works",
  "risks": ["Potential issue 1", "Potential issue 2"]
}
\`\`\`

## Skill vs Agent

- **Skill (this):** Straightforward strategies, clear requirements, < 10 files
- **Agent (architect-agent):** Complex codebase exploration, ambiguous requirements, 10+ files
```

**Agent frontmatter update:**
```yaml
---
name: architect-agent
description: Full agentic power for complex implementation strategy creation
tools: Read, Grep, Glob, Bash
model: inherit
---
```

**Verification:**
- [ ] Invoke `/specks:architect` directly with test step
- [ ] Verify JSON output contains expected_touch_set
- [ ] Invoke architect-agent via Task tool

---

##### 10.5.4: Create author skill+agent {#step-10-5-4}

**Tasks:**
- [ ] Rename `agents/planner.md` → `agents/author-agent.md`
- [ ] Update agent frontmatter: `name: author-agent`
- [ ] Create `skills/author/SKILL.md`

**Full SKILL.md content:**

```markdown
---
name: author
description: Creates and revises speck documents following skeleton format
allowed-tools: Read, Grep, Glob, Write, Edit
---

## Purpose

Create and revise speck documents. Follow the skeleton format strictly. Handle simple edits inline; complex restructuring should escalate to author-agent.

## Input

You receive JSON input via $ARGUMENTS:

\`\`\`json
{
  "idea": "string | null",
  "speck_path": "string | null",
  "user_answers": { ... },
  "clarifier_assumptions": ["string"],
  "critic_feedback": { ... } | null
}
\`\`\`

## Behavior

1. If `idea` provided: Create new speck from scratch
2. If `speck_path` provided: Revise existing speck
3. Apply user_answers and assumptions
4. If critic_feedback provided: Address the issues
5. Write speck to `.specks/specks-{name}.md`
6. Validate against skeleton

## Output

Return JSON-only (no prose, no fences):

\`\`\`json
{
  "speck_path": ".specks/specks-N.md",
  "created": true,
  "sections_written": ["phase-overview", "execution-steps", "..."],
  "validation_status": "valid | warnings | errors"
}
\`\`\`

## Skill vs Agent

- **Skill (this):** Simple edits, section updates, validation fixes
- **Agent (author-agent):** Full speck creation from scratch, major restructuring
```

**Agent frontmatter update:**
```yaml
---
name: author-agent
description: Full agentic power for complex speck creation and revision
tools: Read, Grep, Glob, Bash, Write, Edit
model: inherit
---
```

**Verification:**
- [ ] Invoke `/specks:author` with simple revision task
- [ ] Verify speck is written correctly
- [ ] Invoke author-agent via Task tool for complex creation

---

##### 10.5.5: Create coder skill+agent (most complex, drift detection) {#step-10-5-5}

**Priority:** Most complex sub-task. Includes drift detection.

**Tasks:**
- [ ] Rename `agents/implementer.md` → `agents/coder-agent.md`
- [ ] Update agent frontmatter: `name: coder-agent`
- [ ] Create `skills/coder/SKILL.md`
- [ ] Preserve ALL drift detection logic from (#smart-drift)

**Full SKILL.md content:**

```markdown
---
name: coder
description: Executes architect strategies with drift detection
allowed-tools: Read, Grep, Glob, Write, Edit, Bash
---

## Purpose

Execute architect strategies. Write code, run tests, detect drift. For simple implementations, complete inline. For complex work or drift, return and let orchestrator escalate.

## Input

You receive JSON input via $ARGUMENTS:

\`\`\`json
{
  "speck_path": ".specks/specks-N.md",
  "step_anchor": "#step-N",
  "architect_strategy": {
    "approach": "...",
    "expected_touch_set": ["file1.rs", "file2.rs"],
    "implementation_steps": [...],
    "test_plan": "..."
  },
  "session_id": "20260207-143022-impl-abc123"
}
\`\`\`

## Behavior

1. Read the architect strategy
2. Execute each implementation step in order
3. After each step, perform drift detection (see below)
4. Run tests per test_plan
5. If drift exceeds threshold, HALT and return
6. Return results

## Drift Detection (#smart-drift)

After each file write, check if file is in expected_touch_set:

| Category | Description | Budget Impact |
|----------|-------------|---------------|
| **Green** | File in expected_touch_set | No impact |
| **Yellow** | Adjacent directory (sibling, parent, child) | +1 to budget |
| **Red** | Unrelated subsystem | +2 to budget |

**Thresholds:**
- `none`: All files in expected set
- `minor`: 1-2 yellow (continue, note in output)
- `moderate`: 3-4 yellow OR 1 red → HALT
- `major`: 5+ yellow OR 2+ red → HALT

## Output

Return JSON-only (no prose, no fences):

\`\`\`json
{
  "success": true,
  "halted_for_drift": false,
  "files_created": ["path/to/new.rs"],
  "files_modified": ["path/to/existing.rs"],
  "tests_run": true,
  "tests_passed": true,
  "drift_assessment": {
    "drift_severity": "none | minor | moderate | major",
    "expected_files": ["file1.rs", "file2.rs"],
    "actual_changes": ["file1.rs", "file3.rs"],
    "unexpected_changes": [
      {"file": "file3.rs", "category": "yellow", "reason": "Adjacent to expected"}
    ],
    "drift_budget": {"yellow_used": 1, "yellow_max": 4, "red_used": 0, "red_max": 2}
  }
}
\`\`\`

## Skill vs Agent

- **Skill (this):** Simple implementations, 1-2 files, tests pass
- **Agent (coder-agent):** Complex multi-file changes, deep debugging, drift recovery
```

**Agent frontmatter update:**
```yaml
---
name: coder-agent
description: Execute architect strategies with self-monitoring. Full agentic power for complex implementations.
tools: Read, Grep, Glob, Write, Edit, Bash
model: inherit
---
```

**Verification:**
- [ ] Invoke `/specks:coder` with simple implementation
- [ ] Verify drift_assessment is always present in output
- [ ] Test drift detection by intentionally touching unexpected file
- [ ] Invoke coder-agent via Task tool

---

##### 10.5.6: Create planner orchestration skill {#step-10-5-6}

**Tasks:**
- [ ] Create `skills/planner/SKILL.md`
- [ ] Absorb planning loop logic from `agents/director.md`
- [ ] Implement skill-first, agent-escalation pattern

**Full SKILL.md content:**

```markdown
---
name: planner
description: Orchestrates the planning loop from idea to approved speck
disable-model-invocation: true
allowed-tools: Skill, Task, Read, Grep, Glob, Write, Bash
---

## Purpose

ORCHESTRATOR. Entry point `/specks:planner`. Runs the planning loop from idea to approved speck.

## Input

User invokes with idea or speck path:
- `/specks:planner "add user authentication"`
- `/specks:planner .specks/specks-auth.md`

## Planning Loop

### Setup Phase

\`\`\`bash
MODE="plan"
SESSION_ID="$(date +%Y%m%d-%H%M%S)-${MODE}-$(head -c 3 /dev/urandom | xxd -p)"
mkdir -p .specks/runs/${SESSION_ID}/planning
\`\`\`

Write metadata.json with Write tool.

### State Reconstruction (CRITICAL)

**Skills are stateless.** You must reconstruct state from the run directory:

1. **Counter:** Before each persist, count existing files in `planning/` to determine next number
2. **Previous outputs:** Read persisted JSON files when building context for subsequent sub-tasks
3. **Session ID:** Store in metadata.json and re-read if needed
4. **Retry tracking:** If a sub-task fails, write `NNN-<subtask>-error.json`. Count error files for that sub-task to determine retry attempts.

**File listing pattern (for counter):**
\`\`\`bash
# Get next counter for planning phase
NEXT_NUM=$(printf "%03d" $(($(ls .specks/runs/${SESSION_ID}/planning/*.json 2>/dev/null | wc -l) + 1)))
# Result: "001", "002", etc.
\`\`\`

### Main Loop

For each sub-task:
1. Invoke the skill/agent
2. Parse the JSON output
3. Determine next counter by listing existing files
4. Persist output to `NNN-<subtask>.json`
5. Use persisted outputs to build context for subsequent sub-tasks

**Sub-task sequence:**

1. **Clarify**: `Skill(skill: "specks:clarifier", args: '{"idea": "...", "speck_path": null}')`
   - Persist output to `NNN-clarifier.json`

2. **If questions exist**: `Skill(skill: "specks:interviewer", args: '{"context": "clarifier", ...}')`
   - Persist output to `NNN-interviewer.json`

3. **Author**: `Skill(skill: "specks:author", args: '{"idea": "...", "user_answers": {...}}')`
   - If complex (new speck from scratch): `Task(subagent_type: "specks:author-agent", prompt: "...")`
   - Persist output to `NNN-author.json`

4. **Critic**: `Skill(skill: "specks:critic", args: '{"speck_path": "..."}')`
   - Persist output to `NNN-critic.json`

5. **If REVISE**: Loop back to step 3 with critic feedback
   **If REJECT**: `Skill(skill: "specks:interviewer")` to get user decision
   **If APPROVE**: Continue to finalize

### Finalize

Update metadata.json with `status: "completed"`, return speck path.

## Constraints

- ONE sub-task at a time (sequential invocation)
- Maximum 1 agent context active at any time
- Persist ALL sub-task outputs to run directory

## Error Handling

**Malformed JSON from sub-task:**
1. Log the malformed output to run directory as `NNN-<subtask>-error.json`
2. Retry the same sub-task once
3. If retry also fails: escalate to agent variant (if available)
4. If agent also fails: invoke interviewer to get user decision (abort/retry/skip)

**Sub-task returns error object:**
1. Check if error is recoverable (e.g., validation failure)
2. If recoverable: retry with adjusted input
3. If not recoverable: escalate to agent or interviewer

**Agent escalation also fails:**
1. Invoke interviewer with full context (skill error, agent error)
2. Present options: abort planning, retry from beginning, continue with partial result
3. Honor user decision

**Example error recovery flow:**
1. Invoke `Skill(specks:author, args: '{"idea": "..."}')`
2. Output is malformed: `{speck_path: ...` (missing quotes, invalid JSON)
3. Persist to `003-author-error.json`
4. Retry: `Skill(specks:author, args: '{"idea": "..."}')`
5. If retry succeeds: persist to `004-author.json`, continue
6. If retry fails: escalate to `Task(subagent_type: "specks:author-agent", prompt: "...")`
7. If agent fails: invoke interviewer for user decision
```

**Verification:**
- [ ] Invoke `/specks:planner "test idea"`
- [ ] Verify run directory created at `.specks/runs/<session-id>/`
- [ ] Verify sub-task outputs persisted as JSON
- [ ] Verify skill-first pattern (author skill tried before agent)

---

##### 10.5.7: Create implementer orchestration skill {#step-10-5-7}

**Tasks:**
- [ ] Create `skills/implementer/SKILL.md`
- [ ] Absorb implementation loop logic from `agents/director.md`
- [ ] Implement skill-first, agent-escalation pattern

**Full SKILL.md content:**

```markdown
---
name: implementer
description: Orchestrates the implementation loop from speck to completed code
disable-model-invocation: true
allowed-tools: Skill, Task, Read, Grep, Glob, Write, Bash
---

## Purpose

ORCHESTRATOR. Entry point `/specks:implementer`. Runs the implementation loop for each step in a speck.

## Input

User invokes with speck path:
- `/specks:implementer .specks/specks-3.md`

## Implementation Loop

### Setup Phase

\`\`\`bash
MODE="impl"
SESSION_ID="$(date +%Y%m%d-%H%M%S)-${MODE}-$(head -c 3 /dev/urandom | xxd -p)"
mkdir -p .specks/runs/${SESSION_ID}/execution
\`\`\`

Write metadata.json with Write tool.

### State Reconstruction (CRITICAL)

**Skills are stateless.** You must reconstruct state from the run directory:

1. **Current step:** Read metadata.json to determine which step to work on
2. **Previous outputs:** Read persisted JSON files (architect.json, coder.json) when building context
3. **Step completion:** Check which step-N directories exist and have committer.json (indicates complete)
4. **Dependencies:** Parse speck to understand step dependencies before iterating
5. **Retry tracking:** If a sub-task fails, write `step-N/<subtask>-error.json`. Count error files to determine retry attempts.

**Dependency resolution:**
Before starting a step, check its `**Depends on:**` line. If dependencies reference incomplete steps (no `committer.json` in their step-N directory), skip this step and try the next. If all remaining steps are blocked, report to user.

### Per-Step Loop

For each step in speck (respecting dependencies):

1. Check if step already complete (step-N/committer.json exists) - skip if so
2. Create step directory: `mkdir -p .specks/runs/${SESSION_ID}/execution/step-N`
3. Update metadata.json with `current_step: "#step-N"`

**Sub-task sequence:**

1. **Architect**: `Skill(skill: "specks:architect", args: '{"speck_path": "...", "step_anchor": "#step-N"}')`
   - If complex (10+ files): `Task(subagent_type: "specks:architect-agent", prompt: "...")`
   - Persist to `step-N/architect.json`

2. **Coder**: `Skill(skill: "specks:coder", args: '{"architect_strategy": {...}, ...}')`
   - Read architect.json to get strategy
   - If complex or drift: `Task(subagent_type: "specks:coder-agent", prompt: "...")`
   - Persist to `step-N/coder.json`
   - If `halted_for_drift`: invoke interviewer for decision

3. **Reviewer**: `Skill(skill: "specks:reviewer", args: '{"step_anchor": "#step-N", ...}')`
   - Read coder.json to get implementation output
   - Persist to `step-N/reviewer.json`

4. **Auditor**: `Skill(skill: "specks:auditor", args: '{"files_to_audit": [...], ...}')`
   - Read coder.json to get files changed
   - Persist to `step-N/auditor.json`

5. **If issues**: Handle per escalation protocol
   - Minor quality: retry coder
   - Design issues: back to architect
   - Conceptual: invoke interviewer

6. **Logger**: `Skill(skill: "specks:logger", args: '{"step_anchor": "#step-N", ...}')`
   - Persist to `step-N/logger.json`

7. **Committer**: `Skill(skill: "specks:committer", args: '{"files_to_stage": [...], "bead_id": "...", ...}')`
   - First, retrieve bead_id (see below)
   - Persist to `step-N/committer.json`

**Bead ID retrieval (before committer):**
\`\`\`bash
# Get bead_id for this step (if beads enabled)
BEADS_STATUS=$(specks beads status ${SPECK_PATH} --json 2>/dev/null)
if [ $? -eq 0 ]; then
  BEAD_ID=$(echo $BEADS_STATUS | jq -r '.data.files[0].steps[] | select(.anchor == "'${STEP_ANCHOR}'") | .bead_id')
else
  BEAD_ID=""  # Beads not enabled or error
fi
\`\`\`
Pass `bead_id: null` if beads not enabled or step has no linked bead.

### Finalize

Write `execution/summary.json`, update metadata.json with `status: "completed"`.

## Constraints

- ONE sub-task at a time (sequential invocation)
- Maximum 1 agent context active at any time
- Persist ALL sub-task outputs to run directory

## Error Handling

**Malformed JSON from sub-task:**
1. Log the malformed output to `step-N/<subtask>-error.json`
2. Retry the same sub-task once
3. If retry also fails: escalate to agent variant (if available)
4. If agent also fails: invoke interviewer to get user decision

**Coder halts for drift:**
1. This is NOT an error—it's expected behavior per drift detection
2. Invoke interviewer with drift details
3. User decides: continue anyway, back to architect, abort step

**Test failures:**
1. If tests fail ≤3 times: retry coder with test failure context
2. If tests fail >3 times: escalate to coder-agent
3. If agent also fails: invoke interviewer

**Agent escalation also fails:**
1. Invoke interviewer with full context
2. Present options: abort step, skip step, retry from architect, abort entire run
3. Honor user decision

**Example error recovery flow:**
1. Invoke `Skill(specks:coder, args: '{"architect_strategy": {...}}')`
2. Coder returns `halted_for_drift: true` with `drift_severity: "moderate"`
3. Persist to `step-0/coder.json` (includes drift_assessment)
4. Invoke `Skill(specks:interviewer, args: '{"context": "drift", "payload": {...}}')`
5. User decides "back to architect"
6. Re-invoke `Skill(specks:architect, args: '{"revision_feedback": "Reduce scope..."}')`
7. Continue with new strategy
```

**Verification:**
- [ ] Invoke `/specks:implementer .specks/specks-test.md` on test speck
- [ ] Verify step directories created
- [ ] Verify sub-task outputs persisted as JSON
- [ ] Verify skill-first pattern (architect/coder skills tried before agents)
- [ ] Verify drift detection halts and escalates correctly

---

##### 10.5.8: Archive director and delete old skills {#step-10-5-8}

**Tasks:**
- [ ] Create `agents/archived/` directory
- [ ] Move `agents/director.md` → `agents/archived/director.md`
- [ ] Move `agents/interviewer.md` → `agents/archived/interviewer.md` (clean break; interviewer is skill-only)
- [ ] Delete `skills/plan/` directory
- [ ] Delete `skills/execute/` directory

**Verification:**
- [ ] `ls agents/*.md` shows 3 files (architect-agent, author-agent, coder-agent)
- [ ] `ls agents/archived/` shows director.md
- [ ] `ls skills/plan/` fails (deleted)
- [ ] `ls skills/execute/` fails (deleted)

##### 10.5.9: Update documentation {#step-10-5-9}

- [ ] Update `CLAUDE.md`:
  - Change agent count to 3 (architect-agent, author-agent, coder-agent)
  - Update skill count to 12 (10 sub-tasks + 2 orchestrators)
  - Document dual-orchestrator architecture
  - Update `/specks:plan` → `/specks:planner`, `/specks:execute` → `/specks:implementer`
- [ ] Update (#agents-skills-summary) in this speck (already done, verify)
- [ ] Update (#flow-planning) to show planner orchestration skill
- [ ] Update (#flow-implementation) to show implementer orchestration skill
- [ ] Update (#flow-tools) table to reflect new skill/agent names

**Verification Tests:**

- [ ] Run `claude --plugin-dir .` - plugin loads without errors
- [ ] Invoke `/specks:planner "test idea"` - planning loop completes without crash
- [ ] Invoke `/specks:implementer` on a test speck - implementation loop completes without crash
- [ ] Check debug log: verify maximum 1 agent context at any time
- [ ] Verify skill-first pattern: orchestrator tries skill, can escalate to agent
- [ ] Verify any sub-task can be invoked directly: `/specks:author`, `/specks:coder`, etc.
- [ ] Verify drift detection still works in coder skill

**Checkpoint:**

- [ ] `ls skills/*/SKILL.md | wc -l` returns 12:
  - 2 orchestrators: planner, implementer
  - 7 skill-only sub-tasks: auditor, clarifier, committer, critic, interviewer, logger, reviewer
  - 3 skill+agent pairs (skill side): architect, author, coder
  - (plan and execute are DELETED, not counted)
  - (Total: 2 + 7 + 3 = 12)
- [ ] `ls agents/*.md | wc -l` returns 3:
  - architect-agent, author-agent, coder-agent
- [ ] No `director.md` in `agents/` (archived)
- [ ] No `plan/` or `execute/` in `skills/` (deleted)
- [ ] Planning loop completes without "Aborted()" crash
- [ ] Execution loop completes without "Aborted()" crash
- [ ] Orchestrators demonstrate skill-first, agent-escalation pattern

**Rollback:**

- Restore `agents/director.md` from `agents/archived/`
- Restore `skills/plan/` and `skills/execute/` from git
- Rename agents back (remove `-agent` suffix)
- Delete new orchestration skills (planner, implementer as orchestrators)
- Delete new sub-task skills (architect, author, coder)

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

**Deliverable:** Specks as a Claude Code plugin with dual-orchestrator architecture and skill-first sub-tasks.

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

All agents and skills per (#agents-skills-summary) are implemented and functional. Dual-orchestrator architecture verified.

- [ ] `.claude-plugin/plugin.json` exists with valid manifest
- [ ] 3 agent definitions exist in `agents/` (architect-agent, author-agent, coder-agent)
- [ ] 12 skill directories exist in `skills/` per (#skill-summary)
- [ ] Orchestrators (`planner`, `implementer`) use skill-first, agent-escalation pattern
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
- [ ] ~~5 agents remain temporarily~~ → After Step 10.5: 3 agents remain
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
- [ ] 3 agents with `-agent` suffix: architect-agent, author-agent, coder-agent
- [ ] 2 orchestration skills: planner, implementer
- [ ] 4 new sub-task skills: architect, author, coder, interviewer
- [ ] Old entry point skills (plan, execute) deleted
- [ ] 12 skills total (2 orchestrators + 10 sub-tasks)
- [ ] Skill-first, agent-escalation pattern demonstrated
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
| Agents count | `ls agents/*.md \| wc -l` returns 3 |
| Agent naming | All agents have `-agent` suffix (architect-agent, author-agent, coder-agent) |
| No director | `ls agents/director.md` fails (archived) |
| No old entry skills | `ls skills/plan/` and `ls skills/execute/` both fail (deleted) |
| Orchestrators exist | `ls skills/planner/SKILL.md skills/implementer/SKILL.md` succeeds |
| CLI simplified | `specks --help` shows no plan/execute/setup |
| Build clean | `cargo build` with no warnings |
| Tests pass | `cargo nextest run` passes |
| Plugin loads | `claude --plugin-dir .` succeeds |
| Run directory | `.specks/runs/<session-id>/` created with metadata.json and skill outputs |
| Beads callable | Orchestrators can invoke `specks beads status --json` via Bash and parse output |
| Max 1 agent | Debug log shows maximum 1 agent context during any loop |
| No crashes | Planning/implementation loops complete without "Aborted()" |

**Commit after all checkpoints pass.**
