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

#### [Q02] Homebrew Tap Location (DECIDED) {#q02-homebrew-tap}

**Question:** Should the Homebrew formula live in the main repo or a separate tap?

**Why it matters:** Affects maintenance overhead and installation UX.

**Options:**
- Same repo with standard `Formula/` directory (Homebrew tap convention)
- Separate `homebrew-specks` repository
- Submit to homebrew-core (requires popularity threshold)

**Plan to resolve:** Decide during Step 4 based on Homebrew best practices.

**Resolution:** DECIDED - Same repo with standard `Formula/` directory layout. Users install via:
```bash
brew tap specks-dev/specks https://github.com/specks-dev/specks && brew install specks
```
CI automatically updates the formula after each release, so the maintainer only needs to push a tag.

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

**Decision:** Create a Homebrew tap in the main repo with fully automated formula updates.

**Rationale:**
- `brew install` is the expected installation method on macOS
- Tap allows rapid iteration without homebrew-core review
- Same-repo tap simplifies maintenance (one repo, not two)
- CI automation means maintainer only pushes a tag to release

**Implications:**
- Formula lives at `Formula/specks.rb` (standard Homebrew tap layout)
- Release workflow automatically updates formula with new version and checksums
- `scripts/update-homebrew-formula.sh` handles formula updates (no manual sed)
- Users install via: `brew tap specks-dev/specks https://github.com/specks-dev/specks && brew install specks`
- Can migrate to homebrew-core once established

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

Fully automated release workflow triggered by version tags. Maintainer only runs:
```bash
git tag v0.2.0
git push origin v0.2.0
```

CI handles everything else:

```
┌──────────────┐    ┌───────────────┐    ┌───────────────────────┐
│  git tag     │───>│  GitHub       │───>│  Build Matrix         │
│  v0.2.0      │    │  Actions      │    │  - macos-14 (arm64)   │
└──────────────┘    └───────────────┘    │  - macos-13 (x86_64)  │
                                          └───────────────────────┘
                                                   │
                                                   v
                           ┌─────────────────────────────────────┐
                           │  GitHub Release                     │
                           │  - specks-0.2.0-macos-arm64.tar.gz  │
                           │  - specks-0.2.0-macos-x86_64.tar.gz │
                           │  - checksums.txt                    │
                           └─────────────────────────────────────┘
                                                   │
                                                   v
                           ┌─────────────────────────────────────┐
                           │  Update Formula (automatic)         │
                           │  1. scripts/update-homebrew-        │
                           │     formula.sh v0.2.0 <sha> <sha>   │
                           │  2. git commit Formula/specks.rb    │
                           │  3. git push to main                │
                           └─────────────────────────────────────┘
```

**Tarball Structure (no wrapper directory):**
```
specks-0.2.0-macos-arm64.tar.gz
├── bin/specks
└── share/specks/skills/
    ├── specks-plan/SKILL.md
    └── specks-execute/SKILL.md
```

**User Installation:**
```bash
brew tap specks-dev/specks https://github.com/specks-dev/specks && brew install specks
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
| `scripts/update-homebrew-formula.sh` | Script to update formula version and checksums |
| `Formula/specks.rb` | Homebrew formula (standard tap layout) |
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

1. **Release tarball structure** (no wrapper directory - root contains bin/ and share/ directly):
   ```
   specks-0.x.0-macos-arm64.tar.gz
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
- [x] Update `homebrew/specks.rb` formula (moved to `Formula/specks.rb` in Step 5):
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
- [x] Add `Commands::Execute` variant to cli.rs with all options
- [x] Implement `run_execute()` in commands/execute.rs
- [x] Validate speck exists and passes validation
- [x] Verify speck status is "active"
- [x] Verify beads root exists (or run sync)
- [x] Create run directory with UUID
- [x] Construct director agent prompt with speck and options
- [x] Invoke director agent via AgentRunner
- [x] Monitor for halt signals from .specks/runs/{uuid}/.halt
- [x] Collect run artifacts (architect-plan.md, etc.)
- [x] Implement --dry-run to show execution plan
- [x] Implement --start-step and --end-step filtering
- [x] Implement --commit-policy and --checkpoint-mode
- [x] Add E022 (Monitor halted execution) to error.rs
- [x] Add ExecuteData struct to output.rs
- [x] Update commands/mod.rs exports

**Tests:**
- [x] Unit test: execute command parses arguments correctly
- [x] Unit test: step filtering with --start-step and --end-step
- [x] Integration test: execute with mock completes step
- [x] Integration test: dry-run shows plan without executing
- [x] Integration test: halt signal stops execution
- [x] Golden test: JSON output matches schema

**Checkpoint:**
- [x] `cargo build` succeeds
- [x] `cargo test` passes (new tests)
- [x] `specks execute .specks/specks-test.md --dry-run` shows execution plan
- [x] Run directory created with expected structure

**Rollback:**
- Revert commit, remove execute.rs

**Commit after all checkpoints pass.**

---

#### Step 5: GitHub Releases Workflow {#step-5}

**Depends on:** #step-4

**Commit:** `ci: add fully automated release workflow with formula updates`

**References:** [D05] Prebuilt binaries via GitHub Releases, [D06] Homebrew tap for installation, [Q02] Homebrew tap location, Concept C03, (#c03-release-pipeline, #new-files)

**Artifacts:**
- `.github/workflows/release.yml` - Release workflow (enhance existing)
- `scripts/update-homebrew-formula.sh` - Formula update script
- Binary artifacts on GitHub Releases

**Context:**

The release process must be fully automated. The maintainer's workflow is:
```bash
git tag v0.2.0
git push origin v0.2.0
```

CI handles everything else:
1. Build binaries on native runners (macos-14 for arm64, macos-13 for x86_64)
2. Create tarballs with `bin/` and `share/specks/skills/` at root (no wrapper directory)
3. Calculate SHA256 checksums
4. Create GitHub Release with artifacts
5. Update `Formula/specks.rb` with new version and checksums
6. Commit and push the formula update to main

**Operational Details:**

1. **Version normalization**: Script accepts tag format `v0.2.0` and strips the `v` prefix internally. Workflow passes `${{ github.ref_name }}` directly.

2. **Checksum extraction**: The build jobs create `.sha256` files alongside tarballs. The formula update job downloads these artifacts and extracts checksums:
   ```bash
   ARM64_SHA=$(awk '{print $1}' specks-*-macos-arm64.tar.gz.sha256)
   X86_64_SHA=$(awk '{print $1}' specks-*-macos-x86_64.tar.gz.sha256)
   ```

3. **Formula update job sequence**:
   ```bash
   # Set git identity for CI
   git config user.name "github-actions[bot]"
   git config user.email "github-actions[bot]@users.noreply.github.com"

   # Sync to latest main (FF-only to avoid merge commits)
   git fetch origin main
   git checkout main
   git reset --hard origin/main

   # Update formula
   ./scripts/update-homebrew-formula.sh "$TAG" "$ARM64_SHA" "$X86_64_SHA"

   # Commit only if there are changes
   git add Formula/specks.rb
   git diff --cached --quiet || git commit -m "Update formula to $VERSION"
   git push origin main
   ```

4. **Avoiding empty commits**: The `git diff --cached --quiet ||` pattern ensures commit only happens if there are staged changes.

5. **Concurrency control**: Add `concurrency: { group: formula-update, cancel-in-progress: false }` to the formula update job. This serializes formula updates if two tags are pushed close together.

6. **Gate to real releases only**: Formula update job runs only for release tags (not test/RC tags):
   ```yaml
   if: github.repository == 'specks-dev/specks' && !contains(github.ref, '-')
   ```
   This matches `v0.2.0` but skips `v0.0.1-test` or `v0.2.0-rc1`.

7. **Permissions**: The workflow already has `permissions: contents: write`. This allows the `GITHUB_TOKEN` to push to `main` (assuming main is unprotected).

**Tasks:**
- [x] Move formula from `homebrew/specks.rb` to `Formula/specks.rb` (standard tap layout)
- [x] Update formula URLs to use `specks-dev/specks` consistently
- [x] Enhance `.github/workflows/release.yml` (created in Step 3.5):
  - [x] Pin runners: `macos-14` for arm64, `macos-13` for x86_64 (native builds)
  - [x] Fix tarball structure: root contains `bin/` and `share/` directly (no wrapper dir)
  - [x] Add `update-formula` job that runs after release is published
  - [x] Gate job to real releases only: `if: github.repository == 'specks-dev/specks' && !contains(github.ref, '-')`
  - [x] Download checksum artifacts from build jobs, extract SHA values
  - [x] Set git identity (`user.name`, `user.email`) for CI commits
  - [x] Sync to main with `git reset --hard origin/main` (avoid merge commits)
  - [x] Add concurrency group to serialize formula updates
- [x] Create `scripts/update-homebrew-formula.sh`:
  - [x] Accept tag (e.g., `v0.2.0`), arm64 checksum, and x86_64 checksum as arguments
  - [x] Strip `v` prefix from tag to get version number
  - [x] Update version string in formula
  - [x] Update both SHA256 checksums in formula
  - [x] Exit 0 with no changes if formula already has correct values (idempotent)
  - [x] Use proper file manipulation (not sed one-liners)
- [x] Update `Formula/specks.rb`:
  - [x] Ensure checksums are on clearly marked lines for script to update
  - [x] Add comments identifying arm64 vs x86_64 checksums
  - [x] Verify `install` block works with new tarball structure
- [x] Delete `homebrew/` directory (replaced by `Formula/`)
- [ ] Test end-to-end release flow with a test tag (v0.0.1-test or similar)

**Tests:**
- [ ] Manual test: push test tag (e.g., `v0.0.1-test`) triggers build but skips formula update
- [ ] Manual test: push release tag (e.g., `v0.1.0`) triggers full workflow including formula update
- [ ] Manual test: binaries build on correct native runners
- [ ] Manual test: tarballs have correct structure (no wrapper directory)
- [ ] Manual test: formula is automatically updated with correct checksums
- [ ] Manual test: updated formula commit appears on main branch
- [ ] Manual test: `brew install` works after release completes

**Checkpoint:**
- [ ] Push test tag (e.g., `v0.0.1-test`) builds release but does NOT update formula
- [ ] Push release tag (e.g., `v0.1.0`) triggers full workflow
- [ ] Release contains both architecture binaries and checksums.txt
- [ ] Formula update commit appears on main with correct version/checksums
- [ ] Downloaded binary runs `specks --version`
- [ ] `brew tap specks-dev/specks https://github.com/specks-dev/specks && brew install specks` works

**Rollback:**
- Delete test tag and release (formula unchanged since test tags skip formula update)
- For release tags: delete tag/release, revert formula commit if needed

**Commit after all checkpoints pass.**

---

#### Step 6: Homebrew Installation Documentation {#step-6}

**Depends on:** #step-5

**Commit:** `docs: add Homebrew installation instructions to README`

**References:** [D06] Homebrew tap for installation, [Q02] Homebrew tap location, (#q02-homebrew-tap)

**Artifacts:**
- Updated README.md with installation instructions

**Context:**

The formula (`Formula/specks.rb`) and release workflow were created in Steps 3.5 and 5. This step documents the installation experience for users.

**Tasks:**
- [x] Add "Installation" section to README with:
  - [x] Homebrew installation (tap + install commands)
  - [x] Direct binary download option (from GitHub Releases)
  - [x] Building from source option
- [x] Add post-install setup instructions (`specks init`, `specks setup claude`)
- [x] Verify formula caveats message is helpful

**Tests:**
- [ ] Manual test: follow README instructions on fresh system
- [ ] Manual test: `specks --version` works after brew install
- [ ] Manual test: `specks setup claude` works after brew install

**Checkpoint:**
- [x] README has clear installation section
- [x] All three installation methods documented
- [x] Post-install steps are clear

**Rollback:**
- Revert README changes

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
- [x] Create docs/ directory structure
- [x] Write getting-started.md covering installation and first steps
- [x] Document iterative planning workflow in getting-started.md
- [x] Write first-speck.md tutorial walking through planning loop
- [x] Write execute-plan.md tutorial for execution workflow
- [x] Update README.md with links to docs
- [x] Update README.md with new command documentation
- [x] Add troubleshooting section for common issues
- [x] Review and edit all documentation for clarity

**Tests:**
- [ ] Manual test: follow getting-started.md on fresh system
- [ ] Manual test: tutorials complete without errors
- [ ] Manual test: all links in README work

**Checkpoint:**
- [x] docs/ directory contains all planned files
- [x] README links to all documentation
- [ ] Time from clone to first command under 5 minutes

**Rollback:**
- Remove docs/ directory, revert README changes

**Commit after all checkpoints pass.**

---

### Step 8: Onboarding Infrastructure {#step-8}

**Purpose:** Fix the critical gap in onboarding where fresh projects cannot use `specks plan` because agents are not discoverable. This step ensures that after installation, users can immediately create a new project and start planning.

**Context:** Steps 0-7 built features that don't work for the basic greenfield use case. A user who installs specks and runs `specks plan "idea"` in an empty project gets an error because agents are only looked for in the project's local `agents/` directory, which doesn't exist. This step fixes agent discovery and distribution to enable proper onboarding.

---

#### Step 8.1: Agent Distribution and Discovery {#step-8-1}

**Depends on:** #step-7

**Commit:** `feat(core): add agent distribution and multi-location discovery`

**References:** [D02] Shell out to Claude CLI, (#c01-cli-agent-bridge, #agent-invocation-arch)

**Problem Statement:**

Currently, when a user creates a fresh project:
```bash
mkdir py-calc && cd py-calc
specks init
specks plan "create a python command-line calculator"
# ERROR: Agent invocation failed: Failed to read agent definition at py-calc/agents/specks-interviewer.md
```

The failure occurs because `get_agent_path()` in `agent.rs` only checks `project_root/agents/` with no fallback to the installation's share directory.

**Design Decision: Agents Are Part of the Installation, Not Per-Project**

| Asset | Global Location | Per-Project | Rationale |
|-------|-----------------|-------------|-----------|
| Binary | `/opt/homebrew/bin/specks` | Never | It's the tool |
| Agents | `/opt/homebrew/share/specks/agents/` | Optional per-agent override | Part of the tool, not project data |
| Skills | Source in share | `.claude/skills/` | Claude Code needs them local (or global) |
| Skeleton | Embedded | `.specks/specks-skeleton.md` | User edits it |
| Specks | Never | `.specks/specks-*.md` | Project-specific plans |

**Design Decision: Per-Agent Resolution (Not Directory Selection)**

Agent resolution is per-agent, not per-directory. If a project has `agents/specks-planner.md` but no other agents, the planner comes from the project and the other 10 agents fall back to share.

Rationale:
- Users who customize one agent shouldn't have to copy and maintain all 11
- Least surprise: adding a custom agent doesn't break other agents
- Matches how config layering works in most tools

**Design Decision: Env Var = Share Root**

`SPECKS_SHARE_DIR` points to the share root (e.g., `/opt/homebrew/share/specks`). Code appends subdirectories:
- Skills: `${SPECKS_SHARE_DIR}/skills/`
- Agents: `${SPECKS_SHARE_DIR}/agents/`

This is consistent with how `share.rs` already works for skills.

**Per-Agent Resolution Order** (for each agent by name):
1. `project_root/agents/{agent_name}.md` (if file exists)
2. `{share_dir}/agents/{agent_name}.md` where `share_dir` comes from existing `find_share_dir()` in `share.rs`
3. Development fallback: `{specks_repo}/agents/{agent_name}.md` (only when specks workspace detected via Cargo.toml)

**Important**: Reuse `find_share_dir()` as the single source of truth for share directory discovery. Do NOT re-encode env var / binary-relative / standard location logic in `agent.rs`—that would drift from `share.rs` over time.

**Required Agents by Command:**
- `specks plan`: `specks-interviewer`, `specks-planner`, `specks-critic`
- `specks execute`: `specks-director`, `specks-architect`, `specks-implementer`, `specks-monitor`, `specks-reviewer`, `specks-auditor`, `specks-committer`, `specks-logger`

**Artifacts:**
- Updated `crates/specks/src/agent.rs` - Per-agent resolution via `resolve_agent_path()`
- Updated `crates/specks/src/cli.rs` - Add `--verbose` flag to plan and execute commands
- New error code E026 (RequiredAgentsMissing) in `crates/specks-core/src/error.rs`
- Updated `crates/specks/src/planning_loop.rs` - Remove redundant preflight checks (commands own preflight)
- Updated `crates/specks/src/commands/plan.rs` - Preflight + verbose agent paths
- Updated `crates/specks/src/commands/execute.rs` - Preflight for execute-required agents
- Updated `crates/specks/src/commands/init.rs` - Show agent discovery status
- Updated `.github/workflows/release.yml` - Include agents in tarball
- Updated `Formula/specks.rb` - Install agents to share directory

**Tasks:**

Per-Agent Resolution:
- [x] Implement `resolve_agent_path(agent_name, project_root)` returning `Option<PathBuf>`:
  - Check `project_root/agents/{agent_name}.md` (if file exists, return it)
  - Call `find_share_dir()` and check `{share_dir}/agents/{agent_name}.md`
  - Check dev fallback via `is_specks_workspace()`
  - Return `None` if not found anywhere
- [x] Implement `is_specks_workspace(path)` to detect dev mode (Cargo.toml with specks workspace + agents/ dir present)
- [x] Update `get_agent_path()` to call `resolve_agent_path()`
- [x] Add E026 `RequiredAgentsMissing` error: lists missing agent names + searched paths
- [x] Exit code 8 for missing agents (preflight error, before any invocation)

CLI Flag Plumbing:
- [x] Add `--verbose` flag to `specks plan` in `cli.rs`
- [x] Add `--verbose` flag to `specks execute` in `cli.rs`
- [x] Pass verbose flag through to commands

Agent Version Compatibility:
- [ ] Add `specks-version` field to agent frontmatter schema
- [ ] Parse agent frontmatter when loading (extract version if present): best-effort parse of the initial YAML frontmatter block only; on parse failure, warn once and continue
- [ ] Implement best-effort version check (warn only, don't fail)
- [ ] If `specks-version` field missing, skip check (back-compat)
- [ ] Show version mismatch warning once per run (summary), not per agent

Preflight Verification (both plan and execute):
- [x] Add `verify_required_agents(command, project_root)` that checks all required agents resolve
- [x] Call preflight in `plan.rs` before starting planning loop
- [x] Call preflight in `execute.rs` before starting execution
- [x] On failure: "Missing required agents for 'specks plan': specks-interviewer, specks-critic. Searched: [paths]"
- [x] With `--verbose`: show resolved path for each agent found
- [x] Preflight ownership: commands (`plan.rs`, `execute.rs`) are the single place that enforce required-agent checks; `planning_loop.rs` assumes preflight already ran and does not re-check

Init Command Updates:
- [x] Update `specks init` output to show agent resolution summary
- [x] List agents found and their source (project/share/dev)
- [x] Warn if any required agents are missing

Distribution Updates:
- [x] Update release workflow to copy `agents/*.md` to `share/specks/agents/`
- [x] Update tarball structure to include `share/specks/agents/`
- [x] Update homebrew formula to install agents to share directory
- [ ] Verify agents are installed after `brew install specks`

**Tests:**

Unit tests (in `agent.rs`):
- [x] `resolve_agent_path()` finds agent in project when present
- [x] `resolve_agent_path()` falls back to share when not in project
- [x] `resolve_agent_path()` returns None when agent not found anywhere
- [x] Partial override works (project has 1 agent, share has rest)
- [x] `is_specks_workspace()` returns true in specks repo
- [x] `is_specks_workspace()` returns false in random project
- [ ] Version compatibility check warns on mismatch, doesn't fail
- [ ] Missing `specks-version` field doesn't cause error

Integration tests (preflight verification only—no actual agent invocation):
- [x] `verify_required_agents("plan", ...)` succeeds when agents in share dir
- [x] `verify_required_agents("plan", ...)` fails with E026 when agents missing
- [x] `verify_required_agents("execute", ...)` checks execute-required agents
- [x] Partial override: project has 1 agent, share has rest, preflight passes
- [x] `specks init` shows agent resolution summary (use temp dir with agents copied)

**Test Strategy Note**: Integration tests verify preflight logic only, not full planning loop. Full end-to-end tests (actual `specks plan` producing a speck) are covered in Step 8.3 and Step 10 where `claude-mock` or real invocation is used.

**Test Determinism Note (env vars):** `find_share_dir()` reads `SPECKS_SHARE_DIR`. Rust tests are parallel by default, so env-var-dependent tests must be made deterministic (e.g., run those tests serially, or structure test helpers so env var is set/cleared in a single-threaded context). Avoid any tests that mutate or delete system-managed paths like `/opt/homebrew/...`.

**Checkpoint:**
- [x] `cargo build` succeeds
- [x] `cargo nextest run` passes (all tests)
- [x] Release tarball includes `share/specks/agents/` with all 11 agents
- [ ] `brew install specks` puts agents in `/opt/homebrew/share/specks/agents/`
- [ ] Fresh project preflight passes:
  ```bash
  mkdir py-calc && cd py-calc
  specks init
  specks plan --verbose "test"  # Shows agent paths, preflight passes
  ```
- [x] Missing agents gives clear error:
  ```bash
  mkdir empty && cd empty
  specks init
  EMPTY_SHARE="$(mktemp -d)"
  SPECKS_SHARE_DIR="$EMPTY_SHARE" specks plan "test"
  # ERROR E026: Missing required agents for 'specks plan': specks-interviewer, ...
  # Searched: ./agents/, $EMPTY_SHARE/agents/ (and any other discovery paths)
  ```
- [x] Development mode works (specks workspace detected, agents from repo)
- [ ] Partial override works:
  ```bash
  mkdir test-override && cd test-override
  specks init
  mkdir agents
  cp /opt/homebrew/share/specks/agents/specks-planner.md agents/
  specks plan --verbose "test"
  # Shows: specks-planner from ./agents/, others from share
  ```
- [x] `specks plan --verbose` shows resolved path for each agent
- [x] `specks execute --verbose` shows resolved path for each agent
- [ ] Version mismatch warning appears once (summary), not per-agent

**Rollback:**
- Revert commits, remove agent discovery changes
- Agents remain in share directory (no harm)

**Commit after all checkpoints pass.**

---

#### Step 8.2: Global Skills Installation Option {#step-8-2}

**Depends on:** #step-8-1

**Commit:** `feat(cli): add --global option to install skills to ~/.claude/skills/`

**References:** [D10] Dual invocation paths, (#d10-dual-invocation)

**Context:**

Currently, skills are installed per-project to `.claude/skills/`. This means every project needs its own copy. For users who work on many projects, a global installation to `~/.claude/skills/` reduces repetition.

**Artifacts:**
- Updated `crates/specks/src/share.rs` - Global skill installation
- Updated `crates/specks/src/commands/setup.rs` - `--global` flag
- Updated CLI help text

**Tasks:**
- [x] Add `install_skills_globally()` function to `share.rs`
- [x] Detect home directory using `dirs::home_dir()`
- [x] Create `~/.claude/skills/` if it doesn't exist
- [x] Add `--global` flag to `specks setup claude`
- [x] Update help text to explain global vs per-project
- [x] Add `--global` to `specks setup claude --check` for verification

**Command Interface:**
```bash
specks setup claude           # Install to .claude/skills/ (per-project)
specks setup claude --global  # Install to ~/.claude/skills/ (global)
specks setup claude --check --global  # Check global installation
```

**Tests:**
- [x] Unit test: `install_skills_globally()` creates correct directory structure
- [x] Unit test: Global installation doesn't affect per-project installation
- [x] Integration test: `specks setup claude --global` installs to home directory
- [x] Integration test: `--check --global` reports correct status

**Checkpoint:**
- [x] `specks setup claude --global` installs to `~/.claude/skills/`
- [ ] `/specks-plan` works from any directory after global install
- [x] Per-project install still works as before
- [x] `--check --global` reports installation status correctly

**Rollback:**
- Revert commits, remove `--global` flag

**Commit after all checkpoints pass.**

---

#### Step 8.3: Interaction Adapter System {#step-8-3}

**Depends on:** #step-8-2

**Purpose:** Enable mode-specific, excellent user interaction by abstracting interaction patterns behind an adapter trait. CLI mode uses polished terminal prompts; Claude Code mode delegates interaction to the interviewer agent.

**Context:**

The `specks plan` CLI command hangs because agents try to use `AskUserQuestion` tool, but in `--print` mode the user cannot interact. The root cause is that agent-driven interaction does not work outside Claude Code. The solution is an `InteractionAdapter` trait that abstracts interaction patterns, with mode-specific implementations.

**Design Decisions:**

- **[D15] Specks Owns CLI Interaction** {#d15-cli-interaction}: In CLI mode, specks itself gathers user input using `inquire` crate and passes responses to agents as context. The interviewer agent runs without `AskUserQuestion` tool.

- **[D16] Agents Own Claude Code Interaction** {#d16-cc-interaction}: In Claude Code mode (via skills), the interviewer agent uses `AskUserQuestion` natively. No Rust adapter is needed; the interviewer IS the adapter.

- **[D17] Graceful Non-TTY Fallback** {#d17-non-tty}: When stdin is not a TTY (CI, pipes), use sensible defaults or fail fast with clear message rather than hanging.

**Dependencies to add:**
```toml
inquire = "0.7"      # Interactive prompts
indicatif = "0.17"   # Progress spinners
owo-colors = "4"     # Colored output
ctrlc = "3.4"        # Ctrl+C handling
```

---

##### Step 8.3.1: Core Interaction Adapter Trait {#step-8-3-1}

**Depends on:** #step-8-2

**Commit:** `feat(core): add InteractionAdapter trait for mode-agnostic user interaction`

**References:** [D15] CLI interaction, [D16] CC interaction, (#d15-cli-interaction, #d16-cc-interaction)

**Artifacts:**
- `crates/specks-core/src/interaction.rs` - InteractionAdapter trait and types
- Updated `crates/specks-core/src/lib.rs` - Export interaction module

**Tasks:**
- [ ] Add dependencies to `crates/specks-core/Cargo.toml`: `inquire`, `indicatif`, `owo-colors`
- [ ] Create `interaction.rs` module with `InteractionAdapter` trait
- [ ] Define trait methods:
  - `ask_text(&self, prompt: &str, default: Option<&str>) -> Result<String>`
  - `ask_select(&self, prompt: &str, options: &[&str]) -> Result<usize>`
  - `ask_confirm(&self, prompt: &str, default: bool) -> Result<bool>`
  - `ask_multi_select(&self, prompt: &str, options: &[&str]) -> Result<Vec<usize>>`
  - `start_progress(&self, message: &str) -> ProgressHandle`
  - `end_progress(&self, handle: ProgressHandle, success: bool)`
  - `print_info(&self, message: &str)`
  - `print_warning(&self, message: &str)`
  - `print_error(&self, message: &str)`
  - `print_success(&self, message: &str)`
- [ ] Define `ProgressHandle` type for tracking spinners
- [ ] Define `InteractionError` enum with variants for cancellation, timeout, non-tty
- [ ] Export trait and types from lib.rs

**Tests:**
- [ ] Unit test: trait is object-safe (can use `dyn InteractionAdapter`)
- [ ] Unit test: error types implement std::error::Error

**Checkpoint:**
- [ ] `cargo build` succeeds
- [ ] `cargo nextest run` passes
- [ ] Trait compiles and is usable as trait object

**Rollback:**
- Remove interaction.rs, revert Cargo.toml changes

**Commit after all checkpoints pass.**

---

##### Step 8.3.2: CLI Adapter Implementation {#step-8-3-2}

**Depends on:** #step-8-3-1

**Commit:** `feat(cli): implement CliAdapter with inquire for interactive prompts`

**References:** [D15] CLI interaction, [D17] Non-TTY fallback, (#d15-cli-interaction, #d17-non-tty)

**Artifacts:**
- `crates/specks/src/interaction/mod.rs` - CLI interaction module
- `crates/specks/src/interaction/cli_adapter.rs` - CliAdapter implementation
- Updated `crates/specks/Cargo.toml` - Add ctrlc dependency

**Tasks:**
- [ ] Add `ctrlc = "3.4"` to `crates/specks/Cargo.toml`
- [ ] Create `interaction/` module directory
- [ ] Implement `CliAdapter` struct with TTY detection
- [ ] Implement `ask_text` using `inquire::Text`
- [ ] Implement `ask_select` using `inquire::Select`
- [ ] Implement `ask_confirm` using `inquire::Confirm`
- [ ] Implement `ask_multi_select` using `inquire::MultiSelect`
- [ ] Implement `start_progress` using `indicatif::ProgressBar::new_spinner()`
- [ ] Implement `end_progress` with success/failure styling
- [ ] Implement `print_*` methods using `owo-colors` for consistent styling
- [ ] Add TTY check: if not TTY, return `InteractionError::NonTty` or use defaults
- [ ] Set up Ctrl+C handler with `ctrlc` crate for graceful cancellation
- [ ] Handle Ctrl+C during prompts: return `InteractionError::Cancelled`

**Color Scheme:**
- Info: default/white
- Warning: yellow
- Error: red bold
- Success: green

**Tests:**
- [ ] Unit test: `CliAdapter::new()` detects TTY correctly
- [ ] Unit test: non-TTY mode returns appropriate errors
- [ ] Integration test: manual verification of prompt styling (document in test comments)

**Checkpoint:**
- [ ] `cargo build` succeeds
- [ ] `cargo nextest run` passes
- [ ] Manual test: `CliAdapter` prompts work in terminal
- [ ] Manual test: Ctrl+C cancels gracefully

**Rollback:**
- Remove interaction module, revert Cargo.toml

**Commit after all checkpoints pass.**

---

##### Step 8.3.3: Planning Loop Adapter Integration {#step-8-3-3}

**Depends on:** #step-8-3-2

**Commit:** `feat(plan): integrate InteractionAdapter into planning loop`

**References:** [D15] CLI interaction, (#d15-cli-interaction, #planning-loop)

**Artifacts:**
- Updated `crates/specks/src/commands/plan.rs` - Use adapter for all user interaction
- Updated `crates/specks-core/src/planning_loop.rs` - Accept adapter parameter

**Tasks:**
- [ ] Update `PlanningLoop` to accept `Box<dyn InteractionAdapter>` parameter
- [ ] Replace direct stdin reads with `adapter.ask_text()`
- [ ] Replace yes/no prompts with `adapter.ask_confirm()`
- [ ] Add progress spinners for agent invocation:
  - "Interviewer gathering requirements..."
  - "Planner creating speck..."
  - "Critic reviewing speck..."
- [ ] Use `adapter.print_*` for status messages
- [ ] Update `plan` command to create `CliAdapter` and pass to loop
- [ ] Handle `InteractionError::Cancelled` - clean exit with message
- [ ] Handle `InteractionError::NonTty` - fail fast with helpful message

**Interaction Flow (CLI mode):**
```
1. specks plan "add feature X"
2. CLI creates CliAdapter
3. PlanningLoop uses adapter for all prompts
4. Interviewer agent runs WITHOUT AskUserQuestion tool
5. CLI gathers input via adapter, passes to interviewer as context
6. Loop continues until user approves or cancels
```

**Tests:**
- [ ] Integration test: planning loop with mock adapter
- [ ] Integration test: cancellation handling

**Checkpoint:**
- [ ] `cargo build` succeeds
- [ ] `cargo nextest run` passes
- [ ] `specks plan "test"` uses interactive prompts (manual test)
- [ ] Ctrl+C during planning exits gracefully

**Rollback:**
- Revert changes to plan.rs and planning_loop.rs

**Commit after all checkpoints pass.**

---

##### Step 8.3.4: Mode-Aware Agent Configuration {#step-8-3-4}

**Depends on:** #step-8-3-3

**Commit:** `feat(agents): add mode-aware tool configuration for interviewer`

**References:** [D15] CLI interaction, [D16] CC interaction, (#d15-cli-interaction, #d16-cc-interaction)

**Artifacts:**
- Updated `crates/specks-core/src/agent.rs` - Mode-aware config generation
- Updated interviewer agent documentation

**Tasks:**
- [ ] Add `InteractionMode` enum: `Cli`, `ClaudeCode`
- [ ] Add `interviewer_config(mode: InteractionMode)` function
- [ ] CLI mode config: Remove `AskUserQuestion` from tool list
- [ ] Claude Code mode config: Include `AskUserQuestion` in tool list
- [ ] Update agent loading to accept mode parameter
- [ ] Document the mode difference in interviewer agent header

**Agent Tool Sets:**
```
CLI mode:       Read, Grep, Glob, Bash, Task
Claude Code:    Read, Grep, Glob, Bash, Task, AskUserQuestion
```

**Tests:**
- [ ] Unit test: `interviewer_config(Cli)` excludes AskUserQuestion
- [ ] Unit test: `interviewer_config(ClaudeCode)` includes AskUserQuestion

**Checkpoint:**
- [ ] `cargo build` succeeds
- [ ] `cargo nextest run` passes
- [ ] Interviewer agent in CLI mode does not attempt AskUserQuestion

**Rollback:**
- Revert agent.rs changes

**Commit after all checkpoints pass.**

---

##### Step 8.3.5: Non-Interactive Mode Support {#step-8-3-5}

**Depends on:** #step-8-3-4

**Commit:** `feat(cli): add non-interactive mode with --yes flag and CI detection`

**References:** [D17] Non-TTY fallback, (#d17-non-tty)

**Artifacts:**
- `crates/specks/src/interaction/non_interactive.rs` - NonInteractiveAdapter
- Updated CLI argument parsing

**Tasks:**
- [ ] Create `NonInteractiveAdapter` implementing `InteractionAdapter`
- [ ] `ask_confirm` returns `true` (accept defaults)
- [ ] `ask_text` returns default or empty string
- [ ] `ask_select` returns first option
- [ ] Progress methods are no-ops or minimal output
- [ ] Add `--yes` / `-y` flag to bypass confirmations
- [ ] Detect CI environment: `CI=true`, `GITHUB_ACTIONS`, `JENKINS_URL`
- [ ] Auto-select non-interactive mode when:
  - `--yes` flag provided
  - CI environment detected
  - stdin is not a TTY
- [ ] Print warning when using non-interactive defaults

**Tests:**
- [ ] Unit test: CI detection works for common CI systems
- [ ] Unit test: NonInteractiveAdapter returns expected defaults
- [ ] Integration test: `specks plan --yes` runs without prompts

**Checkpoint:**
- [ ] `cargo build` succeeds
- [ ] `cargo nextest run` passes
- [ ] `CI=true specks plan "test"` does not hang
- [ ] `specks plan --yes "test"` accepts defaults

**Rollback:**
- Remove non_interactive.rs, revert CLI changes

**Commit after all checkpoints pass.**

---

##### Step 8.3.6: Claude Code Path Verification {#step-8-3-6}

**Depends on:** #step-8-3-5

**Commit:** `test: verify Claude Code interaction path works correctly`

**References:** [D16] CC interaction, (#d16-cc-interaction)

**Artifacts:**
- Updated skill SKILL.md files with interaction mode documentation
- Test script for Claude Code path

**Tasks:**
- [ ] Verify `/specks-plan` skill works in Claude Code
- [ ] Verify interviewer agent can use AskUserQuestion in CC mode
- [ ] Document interaction mode in skill SKILL.md files
- [ ] Create manual test script for Claude Code path
- [ ] Verify director coordination works with Task tool

**Test Script (manual):**
```
1. Open Claude Code in test project
2. Run: /specks-plan "add a greeting command"
3. Verify: Interviewer asks questions via AskUserQuestion
4. Verify: Planning loop completes
5. Verify: Speck is created
```

**Checkpoint:**
- [ ] `/specks-plan` works in Claude Code (manual test)
- [ ] Interviewer uses AskUserQuestion correctly
- [ ] No adapter code is invoked in Claude Code path

**Rollback:**
- N/A (verification step)

**Commit after all checkpoints pass.**

---

##### Step 8.3.7: Interaction Polish and Documentation {#step-8-3-7}

**Depends on:** #step-8-3-6

**Commit:** `docs: add interaction adapter documentation and polish UX`

**References:** (#d15-cli-interaction, #d16-cc-interaction, #d17-non-tty)

**Artifacts:**
- `docs/architecture/interaction-adapter.md` - Architecture documentation
- Updated `docs/getting-started.md` - Note about interaction modes
- Updated README

**Tasks:**
- [ ] Create architecture documentation for interaction adapter
- [ ] Document the two modes and when each is used
- [ ] Add troubleshooting section for non-TTY issues
- [ ] Polish spinner messages for consistency
- [ ] Add helpful error messages for common issues:
  - "Not a terminal - use --yes for non-interactive mode"
  - "Cancelled by user"
- [ ] Verify color output respects NO_COLOR environment variable
- [ ] Test on macOS Terminal, iTerm2, VS Code terminal

**Checkpoint:**
- [ ] Documentation is complete and accurate
- [ ] Error messages are helpful
- [ ] Colors work correctly in tested terminals
- [ ] NO_COLOR is respected

**Rollback:**
- Revert documentation changes

**Commit after all checkpoints pass.**

---

**Step 8.3 Completion Checkpoint:**
- [ ] All substeps completed
- [ ] `specks plan "test"` works interactively in CLI
- [ ] `specks plan --yes "test"` works non-interactively
- [ ] `/specks-plan` works in Claude Code
- [ ] Documentation complete
- [ ] No regressions in existing functionality

---

#### Step 8.4: Greenfield Project Test (py-calc) {#step-8-4}

**Depends on:** #step-8-3

**Commit:** `test: validate greenfield onboarding with py-calc example`

**References:** (#step-8-1, #step-8-2, #step-8-3)

**Context:**

This step validates the complete onboarding experience by creating a real project from scratch. The `py-calc` example becomes the canonical "first project" for documentation.

**Test Scenario:**

```bash
# 1. Fresh installation
brew tap specks-dev/specks https://github.com/specks-dev/specks
brew install specks

# 2. Verify installation
specks --version
ls /opt/homebrew/share/specks/agents/  # 11 agents
ls /opt/homebrew/share/specks/skills/  # 2 skills

# 3. Create greenfield project
mkdir py-calc
cd py-calc

# 4. Initialize specks
specks init
# Output shows:
#   Created: .specks/
#   Created: .claude/skills/specks-plan/
#   Created: .claude/skills/specks-execute/
#   Agents available from: /opt/homebrew/share/specks/agents/ (11 agents)

# 5. Run planning (THE CRITICAL TEST)
specks plan "create a python command-line calculator that supports +, -, *, /"
# SUCCESS: Planning loop starts, interviewer asks questions

# 6. Alternative: Claude Code internal
# Open Claude Code in py-calc directory
/specks-plan "create a python command-line calculator"
# SUCCESS: Same planning loop works
```

**Tasks:**
- [x] Document the complete test scenario
- [ ] Run test on fresh macOS system (or VM)
- [x] Capture output for documentation
- [x] Identify and fix any issues discovered
- [x] Create `docs/tutorials/py-calc-example.md` with walkthrough
- [x] Update `docs/getting-started.md` to use py-calc example

**Checkpoint:**
- [ ] Test scenario passes completely on fresh system
- [ ] Both CLI and Claude Code paths work
- [x] Output is clear and helpful
- [x] Documentation captures the actual experience

**Rollback:**
- Document issues, create follow-up tasks

**Commit after all checkpoints pass.**

---

#### Step 8.5: Living On - Using Specks to Develop Specks {#step-8-5}

**Depends on:** #step-8-4

**Purpose:** Enable comfortable self-development where the specks team uses `specks plan` and `specks execute` to add features to specks itself, with appropriate safety mechanisms and clear workflows.

**Context:**

This is the ultimate ergonomics test. If the specks team cannot comfortably use specks for internal development, external users will not have a good experience either. Investigation shows that development mode already works (agents found in `project_root/agents/`, skills found in local `.claude/skills/`), but visibility and safety mechanisms are missing.

**Design Decisions:**

- **[D11] Development Mode is Implicit** {#d11-implicit-dev-mode}: Dev mode is detected automatically based on workspace structure, not via a `--dev` flag. Rationale: reduces cognitive load, more accurate, matches cargo/rustc behavior.

- **[D12] Agent Reload is Manual** {#d12-manual-reload}: Agents are loaded once per invocation. To pick up changes, restart the command. Rationale: hot-reload adds complexity; agent changes should be deliberate.

- **[D13] Infrastructure Warnings are Advisory** {#d13-advisory-warning}: Warnings about self-modification are advisory, not blocking. User can proceed with `--yes`. Rationale: developers know what they're doing; blocking would make development painful.

- **[D14] Touch Set Includes File Classification** {#d14-touch-set-classification}: The architect's expected_touch_set should classify files as "infrastructure" vs "application" to enable targeted warnings.

**Infrastructure File Patterns:**
- `agents/*.md` - Agent definitions
- `.claude/skills/**/SKILL.md` - Skill definitions
- `crates/specks/src/*.rs` - CLI implementation
- `crates/specks-core/src/*.rs` - Core library

---

##### Step 8.5.1: Development Mode Detection {#step-8-5-1}

**Depends on:** #step-8-4

**Commit:** `feat(core): add development mode detection for specks workspace`

**References:** [D11] Development mode is implicit, (#d11-implicit-dev-mode)

**Artifacts:**
- `crates/specks/src/dev_mode.rs` - Development mode detection module
- Updated `agent.rs` - Use dev_mode for agent discovery
- Updated `share.rs` - Use dev_mode for skill discovery

**Detection Heuristics:**
1. Check if `Cargo.toml` exists in project root with specks workspace package
2. Check if `agents/` directory contains `specks-*.md` files matching expected set
3. Check if `.claude/skills/specks-plan/` exists locally

**Tasks:**
- [ ] Create `dev_mode.rs` module with `is_development_mode()` function
- [ ] Implement detection: check for specks workspace Cargo.toml
- [ ] Implement detection: check for local agents matching expected set
- [ ] Add `--verbose` output showing development mode status
- [ ] Update `agent.rs` to log agent discovery path in verbose mode
- [ ] Update `share.rs` to log skill discovery path in verbose mode
- [ ] Add development mode indicator to JSON output

**Tests:**
- [ ] Unit test: `is_development_mode()` returns true in specks repo
- [ ] Unit test: `is_development_mode()` returns false in fresh project
- [ ] Integration test: verbose output shows agent/skill paths

**Checkpoint:**
- [ ] `cargo build` succeeds
- [ ] `cargo nextest run` passes
- [ ] `specks plan --verbose "test"` shows development mode indicator
- [ ] `specks plan --verbose "test"` shows agent source path

**Rollback:**
- Revert commit, remove dev_mode.rs

**Commit after all checkpoints pass.**

---

##### Step 8.5.2: Agent Version Tracking {#step-8-5-2}

**Depends on:** #step-8-5-1

**Commit:** `feat(agents): add version tracking and modification detection`

**References:** [D12] Manual reload, (#d12-manual-reload)

**Artifacts:**
- Updated agent frontmatter with `specks-version` and `last-modified`
- Updated `agent.rs` - Content hashing and modification detection
- Agent modification warning system

**Agent Frontmatter Addition:**
```yaml
---
name: specks-planner
description: Creates structured plans
tools: Read, Grep, Glob, Bash, Write, Edit
model: opus
specks-version: 0.2.0
last-modified: 2026-02-05
---
```

**Tasks:**
- [ ] Add `specks-version: 0.2.0` to all agent frontmatter
- [ ] Add `last-modified: YYYY-MM-DD` to all agent frontmatter
- [ ] Implement agent content hashing at load time
- [ ] Store hash in AgentConfig struct
- [ ] Before invocation, check if file hash has changed
- [ ] If changed, warn user: "Agent X has been modified since loading"
- [ ] Add `--reload-agents` flag to force re-read of agent files

**Tests:**
- [ ] Unit test: Agent frontmatter parsing includes version fields
- [ ] Unit test: Content hash changes when file is modified
- [ ] Integration test: Modification warning displayed when agent changes

**Checkpoint:**
- [ ] All 11 agents have `specks-version` and `last-modified` in frontmatter
- [ ] `cargo nextest run` passes
- [ ] Modifying an agent mid-execution produces warning

**Rollback:**
- Revert commit, restore original agent files

**Commit after all checkpoints pass.**

---

##### Step 8.5.3: Infrastructure Modification Warnings {#step-8-5-3}

**Depends on:** #step-8-5-2

**Commit:** `feat(execute): add infrastructure modification warnings`

**References:** [D13] Advisory warning, [D14] Touch set classification, (#d13-advisory-warning, #d14-touch-set-classification)

**Artifacts:**
- Updated `execute.rs` - Infrastructure file detection
- Updated architect prompt - Request file classification
- Warning display and confirmation system

**Warning Example:**
```
WARNING: This step modifies specks infrastructure:
  - agents/specks-planner.md (agent definition)
  - crates/specks/src/agent.rs (agent invocation code)

These changes may affect the currently running specks process.
Recommendation: Complete this step, then restart specks for subsequent steps.

Continue? [y/N]
```

**Tasks:**
- [ ] Define infrastructure file patterns in constants
- [ ] Implement `classify_touch_set()` to identify infrastructure files
- [ ] Add classification field to architect expected_touch_set
- [ ] Before step execution, check if touch set includes infrastructure
- [ ] Display warning with affected files and recommendation
- [ ] Add `--yes` flag to skip confirmation
- [ ] After infrastructure modification, display rebuild recommendation

**Tests:**
- [ ] Unit test: Infrastructure pattern matching is correct
- [ ] Unit test: Classification correctly identifies specks files
- [ ] Integration test: Warning displayed before modifying agent
- [ ] Integration test: `--yes` flag skips confirmation

**Checkpoint:**
- [ ] `cargo nextest run` passes
- [ ] Executing a step that modifies `agent.rs` shows warning
- [ ] Warning includes rebuild recommendation
- [ ] `--yes` flag works correctly

**Rollback:**
- Revert commit

**Commit after all checkpoints pass.**

---

##### Step 8.5.4: Self-Development Documentation {#step-8-5-4}

**Depends on:** #step-8-5-3

**Commit:** `docs: add self-development workflow documentation`

**Artifacts:**
- `docs/self-development.md` - Complete self-development guide
- Updated `CONTRIBUTING.md` - Link to self-development guide
- Updated `CLAUDE.md` - Self-development section

**Self-Development Workflow:**
```bash
# 1. Create a speck for the change
specks plan "add --verbose flag to agent invocation"

# 2. Review the generated speck
specks validate .specks/specks-verbose-flag.md

# 3. Execute the speck (builds the feature)
specks execute .specks/specks-verbose-flag.md

# 4. Rebuild specks with the changes
cargo build

# 5. Verify the feature works by using it
specks plan --verbose "verify verbose flag works"
```

**Tasks:**
- [ ] Create `docs/self-development.md` with:
  - [ ] Overview: "Using specks to develop specks"
  - [ ] Prerequisites: Development environment setup
  - [ ] Workflow: Plan -> Execute -> Rebuild -> Verify
  - [ ] Safety: Infrastructure modification warnings
  - [ ] Troubleshooting: Common issues and solutions
  - [ ] Best practices: When to use specks vs manual changes
- [ ] Document the modify -> rebuild -> restart cycle
- [ ] Add examples of self-development specks
- [ ] Update CONTRIBUTING.md with link
- [ ] Add self-development section to CLAUDE.md

**Tests:**
- [ ] Manual test: Follow documented workflow for simple change
- [ ] Manual test: Workflow handles infrastructure modification correctly

**Checkpoint:**
- [ ] `docs/self-development.md` exists and is comprehensive
- [ ] All links in documentation work
- [ ] Example workflow can be followed successfully

**Rollback:**
- Remove documentation files

**Commit after all checkpoints pass.**

---

##### Step 8.5.5: Self-Development Integration Test {#step-8-5-5}

**Depends on:** #step-8-5-4

**Commit:** `test: add self-development integration test`

**Artifacts:**
- `tests/integration/self_dev_test.rs` - Self-development integration test
- Test fixtures for self-development scenario

**Tasks:**
- [ ] Create integration test that:
  - [ ] Runs `specks plan "add test command flag"` in specks repo
  - [ ] Verifies speck is created and valid
  - [ ] Verifies development mode is detected
  - [ ] Verifies agent paths are from local repo
- [ ] Create test fixture: simple self-development speck
- [ ] Add to CI workflow

**Tests:**
- [ ] Integration test: Self-dev workflow produces valid speck
- [ ] Integration test: Development mode detected in specks repo
- [ ] Integration test: Infrastructure warnings work correctly

**Checkpoint:**
- [ ] `cargo nextest run` passes
- [ ] Integration test passes in CI
- [ ] Test verifies development mode detection

**Rollback:**
- Remove test files

**Commit after all checkpoints pass.**

---

#### Step 9: Contributor Guide {#step-9}

**Depends on:** #step-8-5-5

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

#### Step 10: End-to-End Validation {#step-10}

**Depends on:** #step-9

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
- [ ] Verify homebrew installation includes agents and skills in share directory
- [ ] Test `specks init` shows agent discovery status
- [ ] Test `specks setup claude --global` installs to home directory
- [ ] Document any issues found and fixes applied
- [ ] Update README if workflow differs from documentation

**Tests:**
- [ ] E2E test: plan with new idea -> approve -> execute workflow completes
- [ ] E2E test: plan with existing speck enters revision mode
- [ ] E2E test: `/specks-plan` inside Claude Code produces valid speck
- [ ] E2E test: `/specks-execute` inside Claude Code completes step
- [ ] E2E test: `specks setup claude` installs skills from homebrew share
- [ ] E2E test: `specks setup claude --global` installs to ~/.claude/skills/
- [ ] E2E test: homebrew installation works with agents
- [ ] E2E test: all CLI commands work as documented

**Checkpoint:**
- [ ] `specks plan "test"` produces valid speck with real agents
- [ ] `/specks-plan "test"` produces valid speck inside Claude Code
- [ ] Planning loop responds to user feedback correctly (both paths)
- [ ] `specks execute` completes at least one step with commit
- [ ] `/specks-execute` completes at least one step inside Claude Code
- [ ] `brew install specks` works on fresh macOS system
- [ ] Agents discovered from share directory
- [ ] All success criteria met

**Rollback:**
- Document issues, defer to patch release

**Commit after all checkpoints pass.**

---

### 2.0.6 Deliverables and Checkpoints {#deliverables}

**Deliverable:** Specks CLI with `plan` and `execute` commands that invoke agents through an iterative refinement loop, distributed via Homebrew and GitHub Releases, with comprehensive documentation for users and contributors.

#### Phase Exit Criteria {#exit-criteria}

- [ ] `agents/specks-interviewer.md` exists with proper agent definition
- [ ] Agents distributed in share directory and discoverable at runtime
- [ ] `specks plan "<idea>"` invokes interviewer, planner, critic in loop
- [ ] `specks plan <existing-speck>` enters revision mode
- [ ] `specks execute <speck>` invokes director and completes step
- [ ] `/specks-plan` slash command works inside Claude Code sessions
- [ ] `/specks-execute` slash command works inside Claude Code sessions
- [ ] `specks setup claude` installs skills to project
- [ ] `specks setup claude --global` installs skills to ~/.claude/skills/
- [ ] `specks init` installs skills as part of initialization
- [ ] GitHub Releases contains macOS binaries with agents and skills in share/
- [ ] Homebrew formula installs working binary, agents, and skills
- [ ] Greenfield project test passes (py-calc scenario)
- [ ] Development mode detection works in specks repo
- [ ] Agent version tracking with modification warnings
- [ ] Infrastructure modification warnings functional
- [ ] Self-development workflow documented
- [ ] docs/getting-started.md exists and is accurate
- [ ] docs/self-development.md exists and is accurate
- [ ] CONTRIBUTING.md exists and is accurate
- [ ] README documents all new commands and both invocation paths

**Acceptance tests:**
- [ ] Integration test: plan command with mock agents
- [ ] Integration test: execute command with mock agent
- [ ] Integration test: agent discovery from share directory
- [ ] Integration test: development mode detection
- [ ] Integration test: infrastructure modification warnings
- [ ] E2E test: full iterative planning workflow with real agents
- [ ] E2E test: revision mode on existing speck
- [ ] E2E test: `/specks-plan` inside Claude Code produces valid speck
- [ ] E2E test: `/specks-execute` inside Claude Code completes step
- [ ] E2E test: greenfield py-calc project creation
- [ ] E2E test: self-development workflow in specks repo
- [ ] Manual test: homebrew installation on clean system

#### Milestones {#milestones}

**Milestone M01: Agent Infrastructure Complete** {#m01-agent-infra}
- [ ] specks-interviewer agent defined
- [ ] Agent invocation infrastructure implemented
- [ ] Agent discovery with multi-location fallback

**Milestone M02: CLI and Skills Complete** {#m02-cli-complete}
- [ ] specks plan implemented with iterative loop
- [ ] specks execute implemented and tested
- [ ] /specks-plan slash command skill created
- [ ] /specks-execute slash command skill created
- [ ] specks setup claude command installs skills
- [ ] specks setup claude --global option works
- [ ] specks init installs skills automatically

**Milestone M03: Distribution Ready** {#m03-distribution-ready}
- [ ] GitHub Releases workflow produces binaries with agents and skills
- [ ] Homebrew formula installs working binary, agents, and skills
- [ ] Agents discoverable via share directory
- [ ] Skills discoverable via share directory

**Milestone M04: Onboarding Complete** {#m04-onboarding-complete}
- [ ] Greenfield project test passes (py-calc)
- [ ] Both CLI and Claude Code paths work from fresh install
- [ ] Clear error messages when agents not found

**Milestone M05: Living On Complete** {#m05-living-on-complete}
- [ ] Development mode detection works in specks repo
- [ ] Agent version tracking and modification warnings functional
- [ ] Infrastructure modification warnings work correctly
- [ ] Self-development workflow documented
- [ ] Integration tests for self-development pass

**Milestone M06: Documentation Complete** {#m06-docs-complete}
- [ ] Getting started guide written with py-calc example
- [ ] Self-development guide written
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
| Agent discovery works | Agents found from share directory in fresh project |
| CLI commands work | `cargo test` passes |
| Slash command skills work | `/specks-plan` and `/specks-execute` invocable in Claude Code |
| Skills distribution works | `specks setup claude` installs skills from share dir |
| Global skills work | `specks setup claude --global` installs to ~/.claude/skills/ |
| Binaries build | Release workflow succeeds with agents and skills in tarball |
| Greenfield test passes | py-calc scenario completes successfully |
| Development mode works | `specks plan --verbose` shows agent source in specks repo |
| Infrastructure warnings work | Warning displayed before modifying agent files |
| Living On workflow works | Can use specks to plan/execute changes to specks |
| Docs complete | Manual review |
| E2E validated | Real agent test passes (both paths) |

**Commit after all checkpoints pass.**
