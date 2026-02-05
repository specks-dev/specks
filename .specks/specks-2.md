## Phase 2.0: Onboarding and Ergonomics {#phase-onboarding-ergonomics}

**Purpose:** Enable real-world usage of specks by adding CLI commands for agent orchestration (`plan`, `execute`), providing binary distribution (homebrew, prebuilt releases), and creating comprehensive documentation for both users and contributors.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | TBD |
| Status | active |
| Target branch | main |
| Tracking issue/PR | N/A |
| Last updated | 2026-02-04 |

---

### Phase Overview {#phase-overview}

#### Context {#context}

Phase 1 established the foundational infrastructure for specks: a CLI with validation, listing, and status commands; a full parser and validation engine; beads integration for work tracking; and a complete ten-agent suite (director, planner, critic, architect, implementer, monitor, reviewer, auditor, logger, committer). However, specks has never been used for actual software development work.

The current experience has significant gaps. There is no `specks plan <idea>` or `specks execute <speck>` command; invoking agents requires manual Claude Code calls like `@specks-planner Create a plan for...`. The full orchestration loop (director coordinating agents) depends on Claude Code interpreting the director agent's markdown specification rather than the CLI driving the workflow. Additionally, there is no binary distribution; users must build from source.

#### Strategy {#strategy}

- Prioritize ergonomics for internal development first (using specks to build specks) since this provides immediate feedback
- Add `specks plan` command with an iterative refinement loop: interviewer gathers input, planner creates spec, critic reviews, interviewer presents results and gathers feedback, loop until approved
- Add `specks execute` command to invoke the director for step-by-step implementation
- Create a new interviewer agent that handles conversational interaction with users during planning
- Create GitHub Actions release workflow with prebuilt binaries for macOS (arm64, x86_64)
- Publish to Homebrew for easy macOS installation
- Add getting started guide and tutorials for external users
- Add contributor guide for internal development
- Defer MCP server and Linux distribution packaging to Phase 3

#### Stakeholders / Primary Customers {#stakeholders}

1. Internal developers using specks to develop specks (highest priority)
2. External developers evaluating specks for their own projects
3. New contributors wanting to help develop specks

#### Success Criteria (Measurable) {#success-criteria}

- `specks plan "add feature X"` enters iterative loop and produces a valid speck after user approval (run `specks validate` on output)
- `specks plan .specks/specks-existing.md` enters revision mode on existing speck
- `specks execute specks-test.md` runs full execution loop, completing at least one step with commit
- Inside Claude Code: `/specks-plan "add feature X"` enters iterative loop (same behavior as CLI)
- Inside Claude Code: `/specks-execute specks-test.md` runs full execution loop (same behavior as CLI)
- `brew install specks` installs working binary on macOS (test on fresh system)
- README has "Getting Started" section with working examples (manual verification)
- CONTRIBUTING.md exists with development setup instructions (manual verification)
- Time from git clone to running first command is under 5 minutes (timed test)

#### Scope {#scope}

1. New agent: `specks-interviewer` for conversational input gathering and feedback
2. CLI command: `specks plan` with iterative refinement loop
3. CLI command: `specks execute` for step-by-step implementation
4. Slash command skills for internal Claude Code use: `/specks-plan`, `/specks-execute`
5. Binary distribution: GitHub Releases with prebuilt macOS binaries
6. Homebrew formula for macOS installation
7. Getting started documentation for external users
8. Contributor guide for internal development
9. Updated README with installation options

#### Non-goals (Explicitly out of scope) {#non-goals}

- MCP server (deferred to Phase 3)
- Linux distribution packages (apt, rpm, etc.)
- Windows support
- GUI or TUI interface
- Plugin system for custom agents
- Automated PR creation by execute command
- `specks new` command (skeleton is a reference, not a template)

#### Dependencies / Prerequisites {#dependencies}

- Phase 1 complete (all agents defined, validation working, beads integration)
- GitHub repository configured for releases
- Homebrew tap repository created (for formula hosting)

#### Constraints {#constraints}

- Must work with Claude Code as the execution environment for agents
- Agent invocation uses Task tool within Claude Code
- Binary size should remain reasonable (target under 10MB)
- macOS builds require arm64 and x86_64 support

#### Assumptions {#assumptions}

- Users have Claude Code installed and configured
- Users have Anthropic API access for agent invocation
- Homebrew tap can be created and maintained
- GitHub Actions minutes are sufficient for release builds

---

### Open Questions {#open-questions}

#### [Q01] Agent Invocation Mechanism (DECIDED) {#q01-agent-invocation}

**Question:** How should `specks plan` and `specks execute` invoke agents?

**Why it matters:** The mechanism determines whether specks can run standalone or requires Claude Code.

**Options:**
- Direct API calls to Anthropic (standalone but complex)
- Shell out to `claude` CLI (depends on Claude Code installation)
- Generate agent prompts that user pastes into Claude Code (degraded UX)

**Plan to resolve:** Prototype shell-out approach in Step 1.

**Resolution:** DECIDED - Shell out to `claude` CLI. See [D02].

#### [Q02] Homebrew Tap Location (OPEN) {#q02-homebrew-tap}

**Question:** Should the Homebrew formula live in the main repo or a separate tap?

**Why it matters:** Affects maintenance overhead and installation UX.

**Options:**
- Same repo under `homebrew/` directory
- Separate `homebrew-specks` repository
- Submit to homebrew-core (requires popularity threshold)

**Plan to resolve:** Decide during Step 4 based on Homebrew best practices.

**Resolution:** OPEN

---

### Risks and Mitigations {#risks}

| Risk | Impact | Likelihood | Mitigation | Trigger to revisit |
|------|--------|------------|------------|--------------------|
| Claude CLI changes break integration | high | low | Version pin, integration tests | Claude Code major release |
| Homebrew formula rejected | medium | medium | Start with tap, migrate to core later | Formula PR feedback |
| Cross-compilation failures | medium | medium | Use GitHub Actions matrix, test locally | CI failures |
| Large binary size | low | low | Strip debug symbols, optimize for size | Binary exceeds 15MB |

**Risk R01: Claude CLI Dependency** {#r01-claude-cli}

- **Risk:** Specks depends on Claude Code's `claude` CLI which may change without notice
- **Mitigation:**
  - Document minimum Claude Code version
  - Add integration tests that verify CLI behavior
  - Provide fallback instructions for manual invocation
- **Residual risk:** Major Claude Code changes could temporarily break specks

---

### 2.0.0 Design Decisions {#design-decisions}

#### [D01] CLI-First Agent Invocation (DECIDED) {#d01-cli-first}

**Decision:** Add `specks plan` and `specks execute` as first-class CLI commands that orchestrate agents.

**Rationale:**
- Users expect CLI tools to do the work, not just validate
- Manual agent invocation via `@specks-planner` is error-prone
- CLI commands can enforce proper workflows and error handling

**Implications:**
- CLI must detect Claude Code environment
- Commands must handle agent output parsing
- Need graceful fallback when Claude Code unavailable

---

#### [D02] Shell Out to Claude CLI (DECIDED) {#d02-shell-claude}

**Decision:** Agent invocation shells out to the `claude` CLI rather than direct API calls.

**Rationale:**
- Claude Code provides the Task tool infrastructure agents need
- Direct API would require reimplementing agent execution
- Keeps specks lightweight; Claude Code handles LLM complexity
- `claude --print` flag enables capturing agent output

**Implications:**
- Claude Code must be installed for plan/execute to work
- Specks can still validate/list/status without Claude Code
- Error messages must guide users to install Claude Code

---

#### [D03] Iterative Planning Loop (DECIDED) {#d03-iterative-loop}

**Decision:** `specks plan` implements an iterative refinement loop with the interviewer agent bookending the process.

**Rationale:**
- Planning is inherently iterative; first drafts rarely capture all requirements
- User feedback is essential for producing implementable plans
- The interviewer agent provides consistent, conversational UX
- Loop continues until explicit user approval, ensuring user ownership

**Implications:**
- Interviewer agent must be created with input-gathering and feedback-presenting capabilities
- Loop must have clear exit conditions (approve, abort)
- Loop runs until user says "ready" (no arbitrary iteration limit)
- Each iteration produces a draft that passes validation

**Flow:**
```
specks plan [idea OR existing-speck-path]
         |
    interviewer (gather initial input OR revision feedback)
         |
    planner (create or revise speck)
         |
    critic (review for quality and compliance)
         |
    interviewer (present result, ask: "ready or revise?")
         |
    user says ready? --> speck status = active, done
    user has feedback? --> loop back to planner with feedback
```

---

#### [D04] Interviewer Agent for User Interaction (DECIDED) {#d04-interviewer-agent}

**Decision:** Create a dedicated interviewer agent that handles all conversational interaction with users during planning, using a proactive punch list approach.

**Rationale:**
- Separates concerns: planner focuses on planning, interviewer on UX
- Consistent interaction patterns across planning workflow
- Proactive behavior ensures critical issues are surfaced, not buried
- Punch list provides visibility into what is resolved vs still open
- Enables revision mode where users can re-enter planning for any speck

**Proactive Behavior:**
- After critic review, analyze feedback and highlight the most important issues
- Maintain a "punch list" of open items/concerns throughout the planning session
- Present punch list each iteration so user can see what is resolved vs still open
- Prioritize items the interviewer thinks need the most attention

**Flexible Behavior:**
- Respond to whatever the user wants to focus on (user can override priorities)
- Accept user feedback on any aspect of the plan
- Always track what the interviewer thinks is still unresolved, even if user has not addressed it

**Punch List Mechanics:**
- Items can come from: critic feedback, interviewer's own analysis, user concerns
- Items get checked off when addressed to interviewer's satisfaction
- Interviewer presents: "Here is what I think still needs attention: [list]. What would you like to focus on?"
- User can say "looks good" or "ready" to exit, or address items, or raise new concerns

**Implications:**
- New agent definition: `agents/specks-interviewer.md`
- Agent must have AskUserQuestion tool for interactive dialogue
- Must handle both "fresh idea" and "existing speck revision" modes
- Must present critic feedback in user-friendly format with punch list
- Must track and present open items across iterations

---

#### [D05] Prebuilt Binaries via GitHub Releases (DECIDED) {#d05-github-releases}

**Decision:** Distribute prebuilt binaries via GitHub Releases for macOS arm64 and x86_64.

**Rationale:**
- Eliminates need for users to install Rust toolchain
- GitHub Actions provides free CI for releases
- Standard distribution method for Rust CLI tools

**Implications:**
- Need release workflow in .github/workflows/
- Version tagging triggers release builds
- Checksums included for verification

---

#### [D06] Homebrew Tap for Installation (DECIDED) {#d06-homebrew-tap}

**Decision:** Create a Homebrew tap for easy macOS installation.

**Rationale:**
- `brew install` is the expected installation method on macOS
- Tap allows rapid iteration without homebrew-core review
- Can migrate to homebrew-core once established

**Implications:**
- Need separate repository or subdirectory for tap
- Formula must update with each release
- Document tap installation in README

---

#### [D07] Documentation Structure (DECIDED) {#d07-documentation}

**Decision:** Documentation lives in docs/ directory with README linking to guides.

**Rationale:**
- Single README becomes too long
- Separate docs allow focused tutorials
- Follows common open source patterns

**Implications:**
- Create docs/getting-started.md
- Create docs/tutorials/ directory
- Create CONTRIBUTING.md at repo root
- Update README with links

---

#### [D08] Plan Command Workflow (DECIDED) {#d08-plan-workflow}

**Decision:** `specks plan` orchestrates an iterative loop: interviewer -> planner -> critic -> interviewer, until user approves.

**Rationale:**
- Matches natural planning workflow with feedback loops
- User remains in control of when plan is "ready"
- Critic ensures quality gate before execution
- Supports both new ideas and revision of existing specks

**Implications:**
- Command accepts idea string OR existing speck path
- Creates speck file in .specks/ directory
- Loops until user explicitly approves (no arbitrary iteration limit)
- Sets speck status to "active" on approval
- User can abort at any time

---

#### [D09] Execute Command Workflow (DECIDED) {#d09-execute-workflow}

**Decision:** `specks execute` implements the director's S10 execution protocol via CLI.

**Rationale:**
- Director agent spec (S10) defines complete execution loop
- CLI makes this accessible without manual orchestration
- Enables automated and semi-automated execution

**Implications:**
- Supports `--start-step` and `--end-step` for partial execution
- Supports `--commit-policy` (manual/auto)
- Creates run directory with artifacts
- Returns structured status on completion

---

#### [D10] Dual Invocation Paths: CLI and Claude Code Internal (DECIDED) {#d10-dual-invocation}

**Decision:** Support both external CLI invocation (`specks plan`, `specks execute`) and internal Claude Code invocation via slash commands (`/specks-plan`, `/specks-execute`).

**Rationale:**
- Users often work inside Claude Code sessions already; shelling back out to CLI is awkward
- The CLI approach (shelling out to `claude`) works well from terminal but is indirect from inside Claude Code
- Slash commands provide native Claude Code UX when already in a session
- Both paths should produce identical outcomes for consistency
- Flexibility: users choose the invocation path that fits their workflow

**Internal Claude Code Usage:**
- `/specks-plan "add feature X"` - starts iterative planning loop inside current session
- `/specks-plan .specks/specks-existing.md` - revision mode on existing speck
- `/specks-execute .specks/specks-1.md` - runs execution loop
- `@specks-director` with explicit mode parameter also works for direct agent invocation

**Implications:**
- Create slash command skills in `.claude/skills/` for `specks-plan` and `specks-execute`
- Skills invoke the director agent with appropriate mode and parameters
- Director agent handles both modes (already designed this way)
- Documentation must cover both invocation methods
- Same iterative loop, same agents, same outcomes regardless of entry point
- Skills have access to AskUserQuestion tool for interactive dialogue

---

### Deep Dives {#deep-dives}

#### Agent Invocation Architecture {#agent-invocation-arch}

**Concept C01: CLI to Agent Bridge** {#c01-cli-agent-bridge}

The `specks plan` and `specks execute` commands bridge the CLI to Claude Code's agent infrastructure:

**Path 1: External CLI (terminal workflow)**

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  specks plan    │────>│  claude CLI      │────>│  specks-        │
│  specks execute │     │  (Task tool)     │     │  interviewer/   │
└─────────────────┘     └──────────────────┘     │  planner/       │
         │                      │                │  director       │
         │                      │                └─────────────────┘
         v                      v                        │
   Arguments &            Agent prompt &            Agent output &
   Options                Context files             Artifacts
```

**Path 2: Internal Claude Code (session workflow)**

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  /specks-plan   │────>│  Skill invokes   │────>│  specks-        │
│  /specks-execute│     │  Task tool       │     │  director       │
└─────────────────┘     │  directly        │     │  (mode=plan or  │
         │              └──────────────────┘     │   mode=execute) │
         │                      │                └─────────────────┘
         v                      v                        │
   Slash command        Director orchestrates      Agent output &
   arguments            interviewer/planner/etc    Artifacts
```

Both paths converge on the same agent suite and produce identical outcomes.

**Invocation Pattern:**

```bash
# specks plan internally runs iterative loop:

# 1. Invoke interviewer to gather input
claude --print --allowedTools "Read,Grep,Glob,Bash,AskUserQuestion" \
  --systemPrompt "$(cat agents/specks-interviewer.md)" \
  "Gather requirements for: <user's idea or existing speck>"

# 2. Invoke planner with requirements
claude --print --allowedTools "Read,Grep,Glob,Bash,Write,Edit,AskUserQuestion" \
  --systemPrompt "$(cat agents/specks-planner.md)" \
  "Create/revise speck based on: <interviewer output>"

# 3. Invoke critic to review
claude --print --allowedTools "Read,Grep,Glob,Bash" \
  --systemPrompt "$(cat agents/specks-critic.md)" \
  "Review speck: <path>"

# 4. Invoke interviewer to present results
claude --print --allowedTools "Read,Grep,Glob,Bash,AskUserQuestion" \
  --systemPrompt "$(cat agents/specks-interviewer.md)" \
  "Present results and ask: ready or revise?"

# 5. Loop back to step 2 if user has feedback

# specks execute internally runs:
claude --print --allowedTools "Task,Read,Grep,Glob,Bash,Write,Edit" \
  --systemPrompt "$(cat agents/specks-director.md)" \
  "Execute speck: <path> mode=execute ..."
```

**Error Handling:**

| Scenario | Detection | Response |
|----------|-----------|----------|
| Claude CLI not installed | which claude fails | Print install instructions, exit 6 |
| Claude CLI times out | Exit code 124 | Retry once, then report timeout |
| Agent produces invalid speck | specks validate fails | Return to planner with validation errors |
| Agent requests user input | AskUserQuestion | Relay to terminal, return response |
| User aborts loop | Explicit abort | Save draft, exit cleanly |

---

#### Iterative Planning Loop {#iterative-planning-loop}

**Concept C02: Planning Loop State Machine** {#c02-planning-loop}

The planning loop is a state machine with clear transitions:

```
                    ┌─────────────────────────────────────────────┐
                    │                                             │
                    v                                             │
┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌──────────┴──┐
│   START     │──>│ INTERVIEWER │──>│  PLANNER    │──>│   CRITIC    │
│ (idea or    │   │ (gather)    │   │ (create/    │   │ (review)    │
│  speck path)│   └─────────────┘   │  revise)    │   └─────────────┘
└─────────────┘           ^         └─────────────┘          │
                          │                                   │
                          │         ┌─────────────┐          │
                          │         │ INTERVIEWER │<─────────┘
                          │         │ (present)   │
                          │         └─────────────┘
                          │                │
                          │    ┌───────────┴───────────┐
                          │    │                       │
                          │    v                       v
                    ┌──────────────┐           ┌──────────────┐
                    │   REVISE     │           │   APPROVED   │
                    │ (user has    │           │ (status =    │
                    │  feedback)   │           │  active)     │
                    └──────────────┘           └──────────────┘
```

**States:**
1. **START**: Receive idea string or existing speck path
2. **INTERVIEWER (gather)**: Collect requirements, context, constraints
3. **PLANNER**: Create new speck or revise existing one
4. **CRITIC**: Review for quality, compliance, implementability
5. **INTERVIEWER (present)**: Show results with punch list of open items, ask "ready or revise?"
6. **REVISE**: User provides feedback, loop to PLANNER (loop continues until user says ready)
7. **APPROVED**: User accepts, set status to active, exit

**Revision Mode:**
When given an existing speck path instead of an idea string:
- Interviewer presents current speck state
- Asks what user wants to change
- Proceeds through normal loop with revision context

---

#### Release Workflow Architecture {#release-workflow-arch}

**Concept C03: Automated Release Pipeline** {#c03-release-pipeline}

Release workflow triggered by version tags:

```
┌──────────────┐    ┌───────────────┐    ┌─────────────────┐
│  git tag     │───>│  GitHub       │───>│  Build Matrix   │
│  v0.2.0      │    │  Actions      │    │  - macos-arm64  │
└──────────────┘    └───────────────┘    │  - macos-x86_64 │
                                          └─────────────────┘
                                                   │
                                                   v
                           ┌─────────────────────────────────────┐
                           │  GitHub Release                     │
                           │  - specks-v0.2.0-macos-arm64.tar.gz │
                           │  - specks-v0.2.0-macos-x86_64.tar.gz│
                           │  - checksums.txt                    │
                           └─────────────────────────────────────┘
```

**Homebrew Formula Update:**

```ruby
class Specks < Formula
  desc "From ideas to implementation via multi-agent orchestration"
  homepage "https://github.com/yourusername/specks"
  version "0.2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/.../specks-v0.2.0-macos-arm64.tar.gz"
      sha256 "..."
    else
      url "https://github.com/.../specks-v0.2.0-macos-x86_64.tar.gz"
      sha256 "..."
    end
  end

  def install
    bin.install "specks"
  end
end
```

---

### 2.0.1 Specification {#specification}

#### 2.0.1.1 Inputs and Outputs {#inputs-outputs}

**Inputs:**

- `specks plan`: Idea string (argument) OR existing speck path, optional context files
- `specks execute`: Speck file path, execution options

**Outputs:**

- `specks plan`: Path to created/revised speck file, validation status, approval status
- `specks execute`: Run status (success/failure/partial), run directory path

**Key invariants:**

- Created specks always pass `specks validate` (or loop continues)
- Approved specks have status = "active"
- Execute always creates run directory even on failure

#### 2.0.1.2 Terminology {#terminology}

- **Agent Invocation**: Executing a specks agent via the claude CLI
- **Run Directory**: UUID-named directory under .specks/runs/ containing execution artifacts
- **Touch Set**: Files expected to be created/modified by a step (from architect)
- **Checkpoint Mode**: When to pause for user confirmation (step/milestone/continuous)
- **Planning Loop**: Iterative cycle of interviewer -> planner -> critic -> interviewer until approval
- **Revision Mode**: Entering the planning loop with an existing speck to modify

#### 2.0.1.3 Supported Features {#supported-features}

- **Supported**:
  - Plan creation from text idea via iterative loop
  - Plan revision from existing speck path
  - Execution of single steps
  - Execution of step ranges (--start-step, --end-step)
  - Manual and auto commit policies
  - Run directory persistence
  - JSON output for all commands

- **Explicitly not supported**:
  - Parallel step execution
  - Remote agent invocation (non-local Claude Code)
  - Execution resumption across sessions (must restart)
  - Custom agent definitions (uses built-in agents only)
  - `specks new` scaffolding command (skeleton is reference, not template)

- **Behavior when unsupported is encountered**:
  - Parallel step requests: Execute sequentially with warning
  - Missing Claude CLI: Exit with code 6 and install instructions

#### 2.0.1.4 Command Specifications {#command-specs}

**Spec S01: specks plan Command** {#s01-plan-command}

```
specks plan [OPTIONS] [INPUT]

Arguments:
  [INPUT]  Either an idea string OR path to existing speck for revision

Options:
  --name <NAME>        Name for the speck file (default: auto-generated)
  --context <FILE>     Additional context files to include (repeatable)
  --timeout <SECS>     Timeout per agent invocation (default: 300)
  --json               Output result as JSON
  --quiet              Suppress progress messages

Exit codes:
  0  Success - speck created/revised and approved
  1  General error
  3  Validation error - speck created but still invalid
  5  User aborted planning loop
  6  Claude CLI not installed

Behavior:
  If INPUT is a file path (exists and ends in .md): Enter revision mode
  If INPUT is a string: Create new speck from idea
  If INPUT is omitted: Prompt user for idea interactively
  Loop continues until user says "ready" (no arbitrary iteration limit)
```

**Spec S02: specks execute Command** {#s02-execute-command}

```
specks execute [OPTIONS] <SPECK>

Arguments:
  <SPECK>  Path to speck file to execute

Options:
  --start-step <ANCHOR>   Step anchor to start from (default: first ready)
  --end-step <ANCHOR>     Step anchor to stop after (default: all)
  --commit-policy <POLICY> manual or auto (default: manual)
  --checkpoint-mode <MODE> step, milestone, or continuous (default: step)
  --dry-run              Show what would be executed without doing it
  --timeout <SECS>       Timeout per step (default: 600)
  --json                 Output result as JSON
  --quiet                Suppress progress messages

Exit codes:
  0  Success - all requested steps completed
  1  General error
  3  Step failed validation/review
  4  Execution halted by monitor
  6  Claude CLI not installed
  9  Not initialized
```

#### 2.0.1.5 Error Model {#error-model}

**New Error Codes:**

| Code | Severity | Message | Exit Code |
|------|----------|---------|-----------|
| E019 | error | Claude CLI not installed | 6 |
| E020 | error | Agent invocation failed: {reason} | 1 |
| E021 | error | Agent timeout after {secs} seconds | 1 |
| E022 | error | Monitor halted execution: {reason} | 4 |
| E023 | warning | Created speck has validation warnings | 0 |
| E024 | info | User aborted planning loop | 5 |
| E025 | error | Skills not found in share directory: {path} | 7 |

#### 2.0.1.6 JSON Output Schema {#json-schema}

**Plan Command Response:**

```json
{
  "schema_version": "1",
  "command": "plan",
  "status": "ok",
  "data": {
    "speck_path": ".specks/specks-feature.md",
    "speck_name": "feature",
    "mode": "new" | "revision",
    "iterations": 3,
    "validation": {
      "errors": 0,
      "warnings": 2
    },
    "critic_approved": true,
    "user_approved": true
  },
  "issues": []
}
```

**Execute Command Response:**

```json
{
  "schema_version": "1",
  "command": "execute",
  "status": "ok",
  "data": {
    "speck_path": ".specks/specks-1.md",
    "run_id": "abc123-...",
    "run_directory": ".specks/runs/abc123-.../",
    "steps_completed": ["#step-0", "#step-1"],
    "steps_remaining": ["#step-2"],
    "commits_created": 2,
    "outcome": "success"
  },
  "issues": []
}
```

**Setup Command Response:**

```json
{
  "schema_version": "1",
  "command": "setup",
  "status": "ok",
  "data": {
    "subcommand": "claude",
    "action": "install" | "check",
    "share_dir": "/opt/homebrew/share/specks",
    "skills_installed": [
      {
        "name": "specks-plan",
        "path": ".claude/skills/specks-plan/SKILL.md",
        "status": "installed" | "updated" | "unchanged" | "missing"
      },
      {
        "name": "specks-execute",
        "path": ".claude/skills/specks-execute/SKILL.md",
        "status": "installed" | "updated" | "unchanged" | "missing"
      }
    ]
  },
  "issues": []
}
```

---

### 2.0.2 Symbol Inventory {#symbol-inventory}

#### 2.0.2.1 New files {#new-files}

| File | Purpose |
|------|---------|
| `agents/specks-interviewer.md` | Interviewer agent definition |
| `crates/specks/src/commands/plan.rs` | Plan command implementation |
| `crates/specks/src/commands/execute.rs` | Execute command implementation |
| `crates/specks/src/commands/setup.rs` | Setup subcommand implementation |
| `crates/specks/src/agent.rs` | Agent invocation via claude CLI |
| `crates/specks/src/planning_loop.rs` | Iterative planning loop state machine |
| `crates/specks/src/share.rs` | Share directory discovery and skill installation |
| `.claude/skills/specks-plan/SKILL.md` | Slash command skill for planning inside Claude Code |
| `.claude/skills/specks-execute/SKILL.md` | Slash command skill for execution inside Claude Code |
| `.github/workflows/release.yml` | Release workflow for binaries |
| `homebrew/specks.rb` | Homebrew formula template |
| `docs/getting-started.md` | Getting started guide |
| `docs/tutorials/first-speck.md` | Tutorial: Create your first speck |
| `docs/tutorials/execute-plan.md` | Tutorial: Execute a plan |
| `CONTRIBUTING.md` | Contributor guide |

#### 2.0.2.2 Symbols to add / modify {#symbols}

| Symbol | Kind | Location | Notes |
|--------|------|----------|-------|
| `Commands::Plan` | variant | `cli.rs` | New subcommand |
| `Commands::Execute` | variant | `cli.rs` | New subcommand |
| `Commands::Setup` | variant | `cli.rs` | New subcommand with nested SetupCommands |
| `SetupCommands` | enum | `cli.rs` | Nested subcommands for setup |
| `run_plan()` | fn | `commands/plan.rs` | Plan command entry point |
| `run_execute()` | fn | `commands/execute.rs` | Execute command entry point |
| `run_setup()` | fn | `commands/setup.rs` | Setup command entry point |
| `AgentRunner` | struct | `agent.rs` | Manages agent invocation |
| `AgentResult` | struct | `agent.rs` | Agent invocation result |
| `invoke_agent()` | fn | `agent.rs` | Shell out to claude CLI |
| `PlanningLoop` | struct | `planning_loop.rs` | State machine for iterative planning |
| `LoopState` | enum | `planning_loop.rs` | Planning loop states |
| `LoopOutcome` | enum | `planning_loop.rs` | Approved, Aborted |
| `find_share_dir()` | fn | `share.rs` | Discover share directory location |
| `get_skills_dir()` | fn | `share.rs` | Get skills directory path |
| `copy_skill_to_project()` | fn | `share.rs` | Install skill to project |
| `verify_skill_installation()` | fn | `share.rs` | Check skill installation status |
| `PlanData` | struct | `output.rs` | JSON response data for plan |
| `ExecuteData` | struct | `output.rs` | JSON response data for execute |
| `SetupData` | struct | `output.rs` | JSON response data for setup |
| `SkillsNotFound` | variant | `error.rs` | E025 error for missing skills |

---

### 2.0.3 Documentation Plan {#documentation-plan}

- [ ] Create agents/specks-interviewer.md with full agent definition
- [ ] Create docs/ directory structure
- [ ] Write docs/getting-started.md with installation and first steps
- [ ] Write docs/tutorials/first-speck.md walkthrough
- [ ] Write docs/tutorials/execute-plan.md walkthrough
- [ ] Write CONTRIBUTING.md with development setup
- [ ] Update README.md with new installation options (brew, binary)
- [ ] Update README.md with plan/execute command documentation
- [ ] Add troubleshooting section for common issues
- [ ] Document Claude Code dependency and setup
- [ ] Document iterative planning workflow in README
- [ ] Document dual invocation paths: CLI (`specks plan/execute`) vs Claude Code internal (`/specks-plan`, `/specks-execute`)
- [ ] Add "Using Specks Inside Claude Code" section to getting-started.md
- [ ] Document when to use each invocation path (terminal workflow vs Claude Code session workflow)

---

### 2.0.4 Test Plan Concepts {#test-plan-concepts}

| Category | Purpose | When to use |
|----------|---------|-------------|
| **Unit** | Test command parsing, argument validation, loop state machine | CLI argument tests, state transitions |
| **Integration** | Test commands with mock claude CLI, full loop simulation | Full command workflows |
| **Golden** | Test JSON output format stability | Output schema changes |
| **E2E** | Test real agent invocation (slow, optional) | Pre-release validation |

#### Test Fixtures {#test-fixtures}

**Mock Claude CLI:**

Create `tests/bin/claude-mock` that simulates claude CLI responses:

```bash
#!/bin/bash
# Simulates claude CLI for testing
# Reads expected output from SPECKS_CLAUDE_MOCK_OUTPUT env var
echo "$SPECKS_CLAUDE_MOCK_OUTPUT"
```

**Fixture Directory Structure:**

```
tests/fixtures/
├── plans/                    # Test plan inputs
│   ├── simple-idea.txt       # Simple idea text
│   └── complex-idea.txt      # Multi-paragraph idea
├── expected-outputs/         # Expected command outputs
│   ├── plan-success.json
│   └── execute-partial.json
├── mock-responses/           # Mock claude CLI responses
│   ├── interviewer-gather.md # Interviewer gathering input
│   ├── interviewer-present.md# Interviewer presenting results
│   ├── planner-success.md    # Valid speck from planner
│   ├── planner-invalid.md    # Invalid speck (tests error handling)
│   └── critic-approve.json   # Critic approval response
└── existing-specks/          # For revision mode testing
    └── specks-revision-test.md
```

---

### 2.0.5 Execution Steps {#execution-steps}

#### Step 0: Create Interviewer Agent Definition {#step-0}

**Commit:** `feat(agents): add specks-interviewer agent for user interaction`

**References:** [D04] Interviewer agent for user interaction, [D03] Iterative planning loop, (#c02-planning-loop, #agent-invocation-arch)

**Artifacts:**
- `agents/specks-interviewer.md` - Interviewer agent definition

**Tasks:**
- [x] Create `agents/specks-interviewer.md` with YAML frontmatter
- [x] Define agent role: gather requirements, present results, collect feedback
- [x] Specify tools: Read, Grep, Glob, Bash, AskUserQuestion
- [x] Document input modes: fresh idea vs revision of existing speck
- [x] Document output format for planner handoff
- [x] Document proactive punch list behavior:
  - [x] After critic review, analyze feedback and highlight most important issues
  - [x] Maintain punch list of open items/concerns throughout planning session
  - [x] Present punch list each iteration showing resolved vs open items
  - [x] Prioritize items the interviewer thinks need most attention
- [x] Document flexible behavior:
  - [x] Respond to whatever user wants to focus on (user can override priorities)
  - [x] Accept user feedback on any aspect of the plan
  - [x] Track what interviewer thinks is unresolved even if user has not addressed it
- [x] Document punch list mechanics:
  - [x] Items come from: critic feedback, interviewer analysis, user concerns
  - [x] Items checked off when addressed to interviewer satisfaction
  - [x] Present: "Here is what I think still needs attention: [list]. What would you like to focus on?"
  - [x] User says "looks good" or "ready" to exit, or addresses items, or raises new concerns
- [x] Add decision tree for "ready or revise?" interaction

**Tests:**
- [ ] Manual test: agent definition is valid markdown with proper frontmatter
- [ ] Manual test: agent can be invoked via claude CLI with mock prompts
- [ ] Manual test: agent presents punch list format correctly

**Checkpoint:**
- [x] `agents/specks-interviewer.md` exists with proper structure
- [x] Agent definition follows same patterns as other agents
- [x] YAML frontmatter includes name, description, tools, model
- [x] Punch list behavior is clearly documented in agent definition

**Rollback:**
- Remove agents/specks-interviewer.md

**Commit after all checkpoints pass.**

---

#### Step 1: Agent Invocation Infrastructure {#step-1}

**Depends on:** #step-0

**Commit:** `feat(core): add agent invocation infrastructure via claude CLI`

**References:** [D02] Shell out to Claude CLI, Concept C01, (#c01-cli-agent-bridge, #error-model)

**Artifacts:**
- `crates/specks/src/agent.rs` - Agent invocation module
- Error codes E019, E020, E021 in specks-core

**Tasks:**
- [x] Create agent.rs module with AgentRunner struct
- [x] Implement `check_claude_cli()` to verify claude is installed
- [x] Implement `invoke_agent()` to shell out with proper arguments
- [x] Parse agent output and capture artifacts
- [x] Handle timeout with configurable duration
- [x] Add E019 (Claude CLI not installed) to error.rs
- [x] Add E020 (Agent invocation failed) to error.rs
- [x] Add E021 (Agent timeout) to error.rs
- [x] Create tests/bin/claude-mock for testing

**Tests:**
- [x] Unit test: check_claude_cli returns appropriate result
- [x] Unit test: invoke_agent constructs correct command line
- [x] Unit test: timeout handling works correctly
- [x] Integration test: mock claude CLI produces expected output

**Checkpoint:**
- [x] `cargo build` succeeds
- [x] `cargo test` passes (new tests)
- [x] Agent invocation with mock returns expected result
- [x] E019 error displays install instructions

**Rollback:**
- Revert commit, remove agent.rs

**Commit after all checkpoints pass.**

---

#### Step 2: Implement specks plan Command {#step-2}

**Depends on:** #step-1

**Commit:** `feat(cli): add specks plan command with iterative refinement loop`

**References:** [D01] CLI-first agent invocation, [D03] Iterative planning loop, [D08] Plan command workflow, Spec S01, Concept C02, (#s01-plan-command, #c02-planning-loop, #iterative-planning-loop)

**Artifacts:**
- `crates/specks/src/commands/plan.rs` - Plan command implementation
- `crates/specks/src/planning_loop.rs` - Loop state machine
- Updated cli.rs with Plan variant
- PlanData in output.rs
- Error code E024 (user aborted)

**Tasks:**
- [x] Add `Commands::Plan` variant to cli.rs with all options
- [x] Create planning_loop.rs with LoopState enum and PlanningLoop struct
- [x] Implement state transitions: Start -> InterviewerGather -> Planner -> Critic -> InterviewerPresent -> (Revise | Approved)
- [x] Implement `run_plan()` in commands/plan.rs
- [x] Detect input type: idea string vs existing speck path
- [x] Invoke interviewer agent for initial input gathering
- [x] Invoke planner agent with interviewer output
- [x] Run `specks validate` on created speck
- [x] Invoke critic agent to review speck
- [x] Invoke interviewer to present results with punch list and ask "ready or revise?"
- [x] Handle user feedback and loop back to planner (loop runs until user says ready)
- [x] Handle abort/exit cleanly
- [x] Set speck status to "active" on approval
- [x] Add E024 error code (user aborted)
- [x] Add PlanData struct to output.rs
- [x] Update commands/mod.rs exports

**Tests:**
- [x] Unit test: plan command parses arguments correctly
- [x] Unit test: input type detection (idea vs speck path)
- [x] Unit test: loop state machine transitions correctly
- [x] Integration test: plan with mock completes single iteration
- [x] Integration test: plan loops on revision feedback
- [x] Integration test: plan handles abort cleanly
- [x] Golden test: JSON output matches schema

**Checkpoint:**
- [x] `cargo build` succeeds
- [x] `cargo test` passes (new tests)
- [x] `specks plan "test idea"` with mock produces speck file
- [x] Created speck passes `specks validate`
- [x] Loop terminates on user approval or abort

**Rollback:**
- Revert commit, remove plan.rs and planning_loop.rs

**Commit after all checkpoints pass.**

---

#### Step 3: Create Slash Command Skills for Claude Code Internal Use {#step-3}

**Depends on:** #step-2

**Commit:** `feat(skills): add /specks-plan and /specks-execute slash commands for Claude Code internal use`

**References:** [D10] Dual invocation paths, [D03] Iterative planning loop, [D09] Execute command workflow, (#d10-dual-invocation, #c02-planning-loop)

**Artifacts:**
- `.claude/skills/specks-plan/SKILL.md` - Planning slash command skill
- `.claude/skills/specks-execute/SKILL.md` - Execution slash command skill

**Tasks:**
- [x] Create `.claude/skills/specks-plan/` directory
- [x] Create `SKILL.md` with YAML frontmatter (name, description, argument-hint)
- [x] Document skill invocation: `/specks-plan "idea"` or `/specks-plan path/to/speck.md`
- [x] Skill invokes director agent with mode=plan
- [x] Skill has access to AskUserQuestion tool for interactive dialogue
- [x] Document input modes: fresh idea vs revision of existing speck
- [x] Document the iterative loop behavior inside Claude Code
- [x] Create `.claude/skills/specks-execute/` directory
- [x] Create `SKILL.md` with YAML frontmatter
- [x] Document skill invocation: `/specks-execute path/to/speck.md [options]`
- [x] Skill invokes director agent with mode=execute
- [x] Document supported options (start-step, end-step, commit-policy, checkpoint-mode)
- [x] Document run directory creation and artifact collection
- [x] Ensure both skills reference the director agent definition

**Tests:**
- [x] Manual test: `/specks-plan "test idea"` enters iterative loop inside Claude Code
- [x] Manual test: `/specks-plan .specks/specks-existing.md` enters revision mode
- [x] Manual test: `/specks-execute` with test speck creates run directory
- [x] Manual test: Interactive dialogue works via AskUserQuestion tool

**Checkpoint:**
- [x] `.claude/skills/specks-plan/SKILL.md` exists with proper structure
- [x] `.claude/skills/specks-execute/SKILL.md` exists with proper structure
- [x] Skills follow same patterns as existing skills (implement-plan, etc.)
- [x] YAML frontmatter includes name, description, argument-hint

**Rollback:**
- Remove `.claude/skills/specks-plan/` and `.claude/skills/specks-execute/` directories

**Commit after all checkpoints pass.**

---

#### Step 3.5: Package Claude Code Skills for Distribution {#step-3-5}

**Depends on:** #step-3

**Commit:** `feat(dist): package skills for distribution and add setup command`

**References:** [D10] Dual invocation paths, [D05] Prebuilt binaries via GitHub Releases, [D06] Homebrew tap for installation, (#d10-dual-invocation, #c03-release-pipeline, #new-files)

**Artifacts:**
- `crates/specks/src/share.rs` - Share directory discovery and skill installation
- `crates/specks/src/commands/setup.rs` - Setup subcommand implementation
- Updated `commands/init.rs` - Skill installation during init
- Updated cli.rs with Setup variant
- Updated release workflow to include skills in tarball
- Updated homebrew formula to install share files
- Error code E025 (Skills not found in share directory)

**Context:**

The skills created in Step 3 (`.claude/skills/specks-plan/SKILL.md` and `.claude/skills/specks-execute/SKILL.md`) exist only in the specks repository itself. When users install specks via homebrew or binary download, they need these skills installed into their own projects to use `/specks-plan` and `/specks-execute` inside Claude Code sessions.

**Distribution Model:**

Skills are distributed as separate files alongside the binary (not embedded in the binary). This allows skills to be updated independently of the binary.

1. **Release tarball structure:**
   ```
   specks-v0.x.x-macos-arm64/
   ├── bin/specks           # The binary
   └── share/specks/
       └── skills/
           ├── specks-plan/SKILL.md
           └── specks-execute/SKILL.md
   ```

2. **Homebrew installation locations:**
   - Binary: `/opt/homebrew/bin/specks` (ARM) or `/usr/local/bin/specks` (x86_64)
   - Skills: `/opt/homebrew/share/specks/skills/` (ARM) or `/usr/local/share/specks/skills/`

3. **Share directory discovery order:**
   - Environment variable: `SPECKS_SHARE_DIR`
   - Relative to binary: `../share/specks/` (works for both homebrew and tarball extraction)
   - Standard locations: `/opt/homebrew/share/specks/`, `/usr/local/share/specks/`
   - Development fallback: `./` (when running from source with skills in repo)

**Tasks:**

- [x] Create `crates/specks/src/share.rs` module for share directory operations:
  - [x] Implement `find_share_dir()` to discover the share directory using the discovery order above
  - [x] Implement `get_skills_dir()` to return `{share_dir}/skills/`
  - [x] Implement `list_available_skills()` to enumerate skills in share directory
  - [x] Implement `copy_skill_to_project(skill_name, project_dir)` to install a skill
  - [x] Implement `verify_skill_installation(skill_name, project_dir)` to check if skill is installed and up-to-date
  - [x] Add checksum/version comparison to detect when installed skill differs from source
- [x] Add E025 error code to `error.rs`:
  - [x] `SkillsNotFound { share_dir: String }` - Skills directory not found in share location
  - [x] Exit code: 7
- [x] Create `crates/specks/src/commands/setup.rs` with subcommand structure:
  - [x] `specks setup claude` - Install Claude Code skills to project
  - [x] `specks setup claude --check` - Verify skill installation status without installing
  - [x] `specks setup claude --force` - Overwrite existing skills even if up-to-date
  - [x] Return JSON output with installed skills list when `--json` flag is set
- [x] Add `Commands::Setup` variant to `cli.rs`:
  - [x] Nested subcommand: `SetupCommands::Claude { check: bool, force: bool }`
  - [x] Long help explaining what skills are and why they are needed
- [x] Update `commands/init.rs` to install skills:
  - [x] After creating `.specks/` directory, call skill installation
  - [x] Create `.claude/skills/specks-plan/` and `.claude/skills/specks-execute/` directories
  - [x] Copy SKILL.md files from share directory to project
  - [x] Add `.claude/skills/` creation to `files_created` output
  - [x] Make skill installation optional (warn but continue if share dir not found)
  - [x] Add skills to output message: "Created: .claude/skills/specks-plan/SKILL.md"
- [x] Update `.github/workflows/release.yml` (Step 5 artifact):
  - [x] Add step to copy `.claude/skills/` to `share/specks/skills/` in build directory
  - [x] Update tarball creation to include `share/` directory
  - [x] Verify tarball structure includes both `bin/` and `share/`
- [x] Update `homebrew/specks.rb` formula (Step 6 artifact):
  - [x] Add `share.install "share/specks" => "specks"` to install share files
  - [x] Skills end up at `#{HOMEBREW_PREFIX}/share/specks/skills/`
- [x] Update commands/mod.rs to export setup module
- [x] Add SetupData struct to output.rs for JSON response

**Tests:**

- [x] Unit test: `find_share_dir()` returns correct path when SPECKS_SHARE_DIR is set
- [x] Unit test: `find_share_dir()` falls back to relative path when env var not set
- [x] Unit test: `find_share_dir()` returns None when no share directory exists
- [x] Unit test: `copy_skill_to_project()` creates correct directory structure
- [x] Unit test: `copy_skill_to_project()` preserves file contents exactly
- [x] Unit test: `verify_skill_installation()` detects missing skills
- [x] Unit test: `verify_skill_installation()` detects outdated skills
- [x] Unit test: setup command parses arguments correctly
- [x] Unit test: setup --check returns correct status without modifying files
- [x] Integration test: `specks init` creates `.claude/skills/` when share dir exists
- [x] Integration test: `specks init` succeeds with warning when share dir missing
- [x] Integration test: `specks setup claude` installs skills to empty project
- [x] Integration test: `specks setup claude` is idempotent (safe to re-run)
- [x] Integration test: `specks setup claude --check` reports installed/missing status
- [x] Integration test: `specks setup claude --force` overwrites existing skills
- [x] Golden test: JSON output for setup command matches schema

**Checkpoint:**

- [x] `cargo build` succeeds
- [x] `cargo test` passes (new and existing tests)
- [x] `specks setup claude --help` shows correct usage
- [x] `specks setup claude --check` reports skills missing (before installation)
- [x] `specks setup claude` creates `.claude/skills/specks-plan/SKILL.md`
- [x] `specks setup claude` creates `.claude/skills/specks-execute/SKILL.md`
- [x] `specks setup claude --check` reports skills installed (after installation)
- [x] `specks init` in new project creates both `.specks/` and `.claude/skills/`
- [x] Installed SKILL.md files are identical to source files
- [x] Running `specks setup claude` twice is idempotent (no errors, no changes second time)
- [x] Release tarball includes `share/specks/skills/` directory with both skills
- [x] Homebrew formula installs skills to share directory

**Rollback:**

- Revert commit
- Remove `share.rs` and `commands/setup.rs`
- Remove skill installation code from `commands/init.rs`
- Revert changes to release workflow and homebrew formula

**Commit after all checkpoints pass.**

---

#### Step 4: Implement specks execute Command {#step-4}

**Depends on:** #step-2

**Commit:** `feat(cli): add specks execute command for agent-driven execution`

**References:** [D01] CLI-first agent invocation, [D09] Execute command workflow, Spec S02, (#s02-execute-command, #agent-invocation-arch)

**Artifacts:**
- `crates/specks/src/commands/execute.rs` - Execute command implementation
- E022 (Monitor halt) in specks-core
- ExecuteData in output.rs

**Tasks:**
- [ ] Add `Commands::Execute` variant to cli.rs with all options
- [ ] Implement `run_execute()` in commands/execute.rs
- [ ] Validate speck exists and passes validation
- [ ] Verify speck status is "active"
- [ ] Verify beads root exists (or run sync)
- [ ] Create run directory with UUID
- [ ] Construct director agent prompt with speck and options
- [ ] Invoke director agent via AgentRunner
- [ ] Monitor for halt signals from .specks/runs/{uuid}/.halt
- [ ] Collect run artifacts (architect-plan.md, etc.)
- [ ] Implement --dry-run to show execution plan
- [ ] Implement --start-step and --end-step filtering
- [ ] Implement --commit-policy and --checkpoint-mode
- [ ] Add E022 (Monitor halted execution) to error.rs
- [ ] Add ExecuteData struct to output.rs
- [ ] Update commands/mod.rs exports

**Tests:**
- [ ] Unit test: execute command parses arguments correctly
- [ ] Unit test: step filtering with --start-step and --end-step
- [ ] Integration test: execute with mock completes step
- [ ] Integration test: dry-run shows plan without executing
- [ ] Integration test: halt signal stops execution
- [ ] Golden test: JSON output matches schema

**Checkpoint:**
- [ ] `cargo build` succeeds
- [ ] `cargo test` passes (new tests)
- [ ] `specks execute .specks/specks-test.md --dry-run` shows execution plan
- [ ] Run directory created with expected structure

**Rollback:**
- Revert commit, remove execute.rs

**Commit after all checkpoints pass.**

---

#### Step 5: GitHub Releases Workflow {#step-5}

**Depends on:** #step-4

**Commit:** `ci: add release workflow for prebuilt macOS binaries`

**References:** [D05] Prebuilt binaries via GitHub Releases, Concept C03, (#c03-release-pipeline, #new-files)

**Artifacts:**
- `.github/workflows/release.yml` - Release workflow
- Binary artifacts on GitHub Releases

**Tasks:**
- [ ] Create .github/workflows/release.yml
- [ ] Configure trigger on version tags (v*)
- [ ] Add macOS arm64 build job
- [ ] Add macOS x86_64 build job
- [ ] Strip debug symbols for smaller binaries
- [ ] Create checksums.txt with SHA256 hashes
- [ ] Upload artifacts to GitHub Release
- [ ] Test workflow with draft release

**Tests:**
- [ ] Manual test: workflow triggers on tag push
- [ ] Manual test: binaries download and run on macOS arm64
- [ ] Manual test: binaries download and run on macOS x86_64
- [ ] Manual test: checksums match downloaded files

**Checkpoint:**
- [ ] Push test tag triggers workflow
- [ ] Workflow completes without errors
- [ ] Release contains both binaries and checksums
- [ ] Downloaded binary runs `specks --version`

**Rollback:**
- Delete release workflow, delete test tag and release

**Commit after all checkpoints pass.**

---

#### Step 6: Homebrew Formula {#step-6}

**Depends on:** #step-5

**Commit:** `docs: add Homebrew formula for macOS installation`

**References:** [D06] Homebrew tap for installation, (#q02-homebrew-tap, #new-files)

**Artifacts:**
- `homebrew/specks.rb` - Homebrew formula template
- Documentation for tap setup

**Tasks:**
- [ ] Create homebrew/ directory
- [ ] Create specks.rb formula template
- [ ] Configure formula to download from GitHub Releases
- [ ] Handle arm64 and x86_64 architecture selection
- [ ] Add installation instructions to README
- [ ] Document tap creation process
- [ ] Test formula installation locally

**Tests:**
- [ ] Manual test: formula installs from local tap
- [ ] Manual test: installed binary runs correctly
- [ ] Manual test: formula works on both architectures

**Checkpoint:**
- [ ] `brew install --build-from-source homebrew/specks.rb` works locally
- [ ] README documents Homebrew installation
- [ ] Tap setup instructions are complete

**Rollback:**
- Remove homebrew/ directory, revert README changes

**Commit after all checkpoints pass.**

---

#### Step 7: Getting Started Documentation {#step-7}

**Depends on:** #step-6

**Commit:** `docs: add getting started guide and tutorials`

**References:** [D07] Documentation structure, (#documentation-plan, #new-files)

**Artifacts:**
- `docs/getting-started.md` - Getting started guide
- `docs/tutorials/first-speck.md` - First speck tutorial
- `docs/tutorials/execute-plan.md` - Execute tutorial
- Updated README.md

**Tasks:**
- [ ] Create docs/ directory structure
- [ ] Write getting-started.md covering installation and first steps
- [ ] Document iterative planning workflow in getting-started.md
- [ ] Write first-speck.md tutorial walking through planning loop
- [ ] Write execute-plan.md tutorial for execution workflow
- [ ] Update README.md with links to docs
- [ ] Update README.md with new command documentation
- [ ] Add troubleshooting section for common issues
- [ ] Review and edit all documentation for clarity

**Tests:**
- [ ] Manual test: follow getting-started.md on fresh system
- [ ] Manual test: tutorials complete without errors
- [ ] Manual test: all links in README work

**Checkpoint:**
- [ ] docs/ directory contains all planned files
- [ ] README links to all documentation
- [ ] Time from clone to first command under 5 minutes

**Rollback:**
- Remove docs/ directory, revert README changes

**Commit after all checkpoints pass.**

---

#### Step 8: Contributor Guide {#step-8}

**Depends on:** #step-7

**Commit:** `docs: add contributor guide and development setup`

**References:** [D07] Documentation structure, (#documentation-plan, #new-files)

**Artifacts:**
- `CONTRIBUTING.md` - Contributor guide
- Updated development documentation

**Tasks:**
- [ ] Create CONTRIBUTING.md at repo root
- [ ] Document development environment setup
- [ ] Document code structure and conventions
- [ ] Document testing requirements and patterns
- [ ] Document PR and review process
- [ ] Document agent development guidelines
- [ ] Add section on using specks to develop specks
- [ ] Link from README.md

**Tests:**
- [ ] Manual test: follow setup instructions on fresh system
- [ ] Manual test: all documented commands work
- [ ] Manual test: test suite runs as documented

**Checkpoint:**
- [ ] CONTRIBUTING.md exists and is comprehensive
- [ ] Development setup instructions work
- [ ] README links to CONTRIBUTING.md

**Rollback:**
- Remove CONTRIBUTING.md, revert README changes

**Commit after all checkpoints pass.**

---

#### Step 9: End-to-End Validation {#step-9}

**Depends on:** #step-8

**Commit:** `test: validate end-to-end workflow with real agent invocation`

**References:** [D01] CLI-first agent invocation, [D03] Iterative planning loop, [D10] Dual invocation paths, (#success-criteria, #exit-criteria, #d10-dual-invocation)

**Artifacts:**
- End-to-end test documentation
- Validated workflow recordings

**Tasks:**
- [ ] Test `specks plan "add a simple feature"` with real planner and interviewer
- [ ] Verify planning loop completes with user interaction
- [ ] Verify created speck passes validation
- [ ] Test `specks plan .specks/specks-existing.md` for revision mode
- [ ] Test `specks execute` on simple test speck
- [ ] Verify run directory contains expected artifacts
- [ ] Test `/specks-plan "add a simple feature"` inside Claude Code session
- [ ] Verify `/specks-plan` iterative loop works with AskUserQuestion
- [ ] Test `/specks-execute` inside Claude Code session
- [ ] Verify both invocation paths produce equivalent outcomes
- [ ] Test homebrew installation on clean macOS system
- [ ] Verify homebrew installation includes skills in share directory
- [ ] Test `specks init` installs skills from homebrew share directory
- [ ] Test `specks setup claude` on existing project without skills
- [ ] Document any issues found and fixes applied
- [ ] Update README if workflow differs from documentation

**Tests:**
- [ ] E2E test: plan with new idea -> approve -> execute workflow completes
- [ ] E2E test: plan with existing speck enters revision mode
- [ ] E2E test: `/specks-plan` inside Claude Code produces valid speck
- [ ] E2E test: `/specks-execute` inside Claude Code completes step
- [ ] E2E test: `specks setup claude` installs skills from homebrew share
- [ ] E2E test: homebrew installation works
- [ ] E2E test: all CLI commands work as documented

**Checkpoint:**
- [ ] `specks plan "test"` produces valid speck with real agents
- [ ] `/specks-plan "test"` produces valid speck inside Claude Code
- [ ] Planning loop responds to user feedback correctly (both paths)
- [ ] `specks execute` completes at least one step with commit
- [ ] `/specks-execute` completes at least one step inside Claude Code
- [ ] `brew install specks` works on fresh macOS system
- [ ] All success criteria met

**Rollback:**
- Document issues, defer to patch release

**Commit after all checkpoints pass.**

---

### 2.0.6 Deliverables and Checkpoints {#deliverables}

**Deliverable:** Specks CLI with `plan` and `execute` commands that invoke agents through an iterative refinement loop, distributed via Homebrew and GitHub Releases, with comprehensive documentation for users and contributors.

#### Phase Exit Criteria {#exit-criteria}

- [ ] `agents/specks-interviewer.md` exists with proper agent definition
- [ ] `specks plan "<idea>"` invokes interviewer, planner, critic in loop
- [ ] `specks plan <existing-speck>` enters revision mode
- [ ] `specks execute <speck>` invokes director and completes step
- [ ] `/specks-plan` slash command works inside Claude Code sessions
- [ ] `/specks-execute` slash command works inside Claude Code sessions
- [ ] `specks setup claude` installs skills to project
- [ ] `specks init` installs skills as part of initialization
- [ ] GitHub Releases contains macOS binaries with skills in share/
- [ ] Homebrew formula installs working binary and skills
- [ ] docs/getting-started.md exists and is accurate
- [ ] CONTRIBUTING.md exists and is accurate
- [ ] README documents all new commands and both invocation paths

**Acceptance tests:**
- [ ] Integration test: plan command with mock agents
- [ ] Integration test: execute command with mock agent
- [ ] E2E test: full iterative planning workflow with real agents
- [ ] E2E test: revision mode on existing speck
- [ ] E2E test: `/specks-plan` inside Claude Code produces valid speck
- [ ] E2E test: `/specks-execute` inside Claude Code completes step
- [ ] Manual test: homebrew installation on clean system

#### Milestones {#milestones}

**Milestone M01: Agent Infrastructure Complete** {#m01-agent-infra}
- [ ] specks-interviewer agent defined
- [ ] Agent invocation infrastructure implemented

**Milestone M02: CLI and Skills Complete** {#m02-cli-complete}
- [ ] specks plan implemented with iterative loop
- [ ] specks execute implemented and tested
- [ ] /specks-plan slash command skill created
- [ ] /specks-execute slash command skill created
- [ ] specks setup claude command installs skills
- [ ] specks init installs skills automatically

**Milestone M03: Distribution Ready** {#m03-distribution-ready}
- [ ] GitHub Releases workflow produces binaries with skills
- [ ] Homebrew formula installs working binary and skills
- [ ] Skills discoverable via share directory

**Milestone M04: Documentation Complete** {#m04-docs-complete}
- [ ] Getting started guide written
- [ ] Tutorials written
- [ ] Contributor guide written
- [ ] Both invocation paths documented

#### Roadmap {#roadmap}

- [ ] Phase 3: MCP server for IDE integration
- [ ] Phase 3: Linux distribution packages
- [ ] Phase 3: Windows support
- [ ] Phase 4: Custom agent definitions
- [ ] Phase 4: Parallel step execution

| Checkpoint | Verification |
|------------|--------------|
| Interviewer agent defined | `agents/specks-interviewer.md` exists |
| CLI commands work | `cargo test` passes |
| Slash command skills work | `/specks-plan` and `/specks-execute` invocable in Claude Code |
| Skills distribution works | `specks setup claude` installs skills from share dir |
| Binaries build | Release workflow succeeds with skills in tarball |
| Docs complete | Manual review |
| E2E validated | Real agent test passes (both paths) |

**Commit after all checkpoints pass.**
