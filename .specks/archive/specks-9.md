## Phase D: Post-Loop Quality Gates (Auditor and Integrator Agents) {#phase-d}

**Purpose:** Restructure the implementer workflow by adding two post-loop agents (auditor and integrator), simplifying the committer, and moving PR creation into a new integrator agent with CI verification.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | kocienda |
| Status | draft |
| Target branch | main |
| Tracking issue/PR | TBD |
| Last updated | 2026-02-12 |
| Beads Root | `specks-99w` |

---

### Phase Overview {#phase-overview}

#### Context {#context}

The current implementer workflow (setup, architect, coder, reviewer, committer) has no post-loop quality gate. Per-step reviews catch individual step issues, but cross-step integration problems, deliverable verification, and CI failures are not detected until after the PR is created. The committer agent currently handles both commit and publish responsibilities, creating a dual-role agent that conflates step-level operations with session-level operations.

The planner workflow already has a critic agent that performs holistic review after the author finishes. The implementer needs an analogous quality gate: the auditor. Additionally, CI verification requires a dedicated agent that can push, create PRs, and wait for status checks — the integrator.

#### Strategy {#strategy}

- Add an auditor agent that runs after all steps complete, performing holistic review: spot-check steps, verify deliverables (#exit-criteria), check cross-step integration, and run fresh build/test/clippy/fmt as authoritative source of truth
- Add an integrator agent that takes over PR creation from the committer, pushes branches, creates PRs, and waits for CI via `gh pr checks`
- Simplify the committer by removing its publish mode entirely and adding a fixup mode for audit/integration fix commits
- Wire the auditor and integrator into the implementer SKILL.md orchestration loop as post-loop phases with retry support
- Reuse existing coder and committer agents for audit/integration fixes (they retain accumulated context)
- Keep all changes to agent definition files and the implementer skill file; no CLI or Rust code changes required

#### Stakeholders / Primary Customers {#stakeholders}

1. Specks users running `/specks:implementer` to execute specks
2. Specks developers maintaining the agent and skill definitions

#### Success Criteria (Measurable) {#success-criteria}

- `agents/auditor-agent.md` exists and defines the auditor agent with input/output contracts
- `agents/integrator-agent.md` exists and defines the integrator agent with input/output contracts
- `agents/committer-agent.md` has publish mode removed and fixup mode added
- `skills/implementer/SKILL.md` orchestration loop includes post-loop auditor and integrator phases with retry logic
- All agent files reference correct models (architect: opus, auditor: opus, integrator: sonnet, committer: sonnet)
- Retry limits (max 3) and escalation to user via AskUserQuestion are documented for both auditor and integrator loops

#### Scope {#scope}

1. Create `agents/auditor-agent.md` with full agent definition
2. Create `agents/integrator-agent.md` with full agent definition
3. Modify `agents/committer-agent.md` to remove publish mode and add fixup mode
4. Modify `agents/architect-agent.md` frontmatter to change model from sonnet to opus
5. Modify `skills/implementer/SKILL.md` to add post-loop phases and update orchestration diagram

#### Non-goals (Explicitly out of scope) {#non-goals}

- No changes to CLI Rust code (no new commands, no modifications to `specks step-commit` or `specks step-publish`)
- No changes to the planner workflow or planning agents
- No changes to the coder, reviewer, or setup agent definitions (beyond what the orchestrator passes them)
- No changes to the beads system or bead-mediated communication protocols
- No new configuration options or config file changes

#### Dependencies / Prerequisites {#dependencies}

- Current implementer skill with committer publish mode (the baseline we are modifying)
- Existing `specks step-publish` CLI command (the integrator will call it instead of the committer)
- `gh` CLI available for PR checks (`gh pr checks`)

#### Constraints {#constraints}

- Agent definition files are markdown with YAML frontmatter; changes must conform to the existing format
- The implementer skill is a pure orchestrator with only `Task` and `AskUserQuestion` tools
- Fixup commits must be outside the bead system (no bead tracking for polish commits)
- Auditor must use opus model for deep analysis; integrator must use sonnet model for CLI operations

#### Assumptions {#assumptions}

- The `specks step-publish` CLI command works correctly and does not need modification
- The `gh pr checks` command is available in the environment and respects branch protection rules
- Coder and committer agents can be resumed for audit/integration fixes without re-spawning
- Auto-compaction handles context growth from additional post-loop agent invocations

---

### 9.0 Design Decisions {#design-decisions}

#### [D01] Auditor uses opus model for deep analysis (DECIDED) {#d01-auditor-model}

**Decision:** The auditor agent uses the opus model, matching the critic agent's model choice.

**Rationale:**
- Auditor performs holistic review analogous to the critic in the planning workflow
- Deep analysis of cross-step integration and deliverable verification requires opus-level reasoning
- Spot-checking step implementations against the speck requires reading and reasoning over substantial code

**Implications:**
- Higher token cost per auditor invocation compared to sonnet agents
- Auditor agent frontmatter must specify `model: opus`

#### [D02] Integrator uses sonnet model for CLI operations (DECIDED) {#d02-integrator-model}

**Decision:** The integrator agent uses the sonnet model, matching the committer's model choice.

**Rationale:**
- Integrator performs straightforward CLI operations: push, create PR, check CI
- No deep code analysis required; the auditor has already verified quality
- Sonnet is sufficient for mapping inputs to CLI command invocations

**Implications:**
- Lower token cost for integrator operations
- Integrator agent frontmatter must specify `model: sonnet`

#### [D03] Fixup commits have no bead tracking (DECIDED) {#d03-fixup-no-beads}

**Decision:** Fixup commits produced during audit/integration loops are outside the bead system. They use `specks log prepend` and plain `git commit` (no `specks step-commit`, no bead close).

**Rationale:**
- Fixup commits are polish, not plan steps; they do not correspond to any step in the speck
- The bead system tracks plan execution progress; audit/integration fixes are post-plan quality gates
- Using `specks step-commit` would require a bead ID that does not exist for these operations

**Implications:**
- Committer fixup mode uses `specks log prepend` for log tracking and `git -C <worktree> commit` for the commit
- Conventional commit format with `fix` type (e.g., `fix(<scope>): <description>`)
- No bead close operation in fixup mode

#### [D04] Auditor scope is hybrid: spot-check plus deliverables plus fresh build (DECIDED) {#d04-auditor-scope}

**Decision:** The auditor performs a hybrid review: spot-check individual step implementations, verify deliverables from the speck's `#exit-criteria` section, check cross-step integration, and run a fresh build/test/clippy/fmt as the authoritative source of truth for project health.

**Rationale:**
- Per-step reviews may pass individually but miss cross-step integration issues
- The deliverables section (#exit-criteria) defines "done" for the phase; auditor verifies that contract
- A fresh build/test/clippy/fmt run catches accumulated issues that per-step checks might miss
- The auditor does not trust per-step bead notes for final project state; it runs its own verification

**Implications:**
- Auditor agent has Bash tool access to run build/test/clippy/fmt commands
- Auditor reads the speck's Deliverables section and checks each exit criterion
- Auditor grades issues P0-P3 for prioritized remediation

#### [D05] Retry limit is 3 with user escalation (DECIDED) {#d05-retry-limit}

**Decision:** Both the auditor and integrator retry loops have a maximum of 3 attempts. After 3 failed attempts, the orchestrator escalates to the user via `AskUserQuestion` with options to continue, fix manually, or abort.

**Rationale:**
- Unbounded retry loops risk infinite token consumption
- 3 retries balances giving agents enough attempts to fix issues against avoiding runaway loops
- User escalation provides a manual escape hatch for issues agents cannot resolve autonomously

**Implications:**
- Orchestrator tracks `auditor_attempts` and `integrator_attempts` counters
- Escalation prompt includes the specific issues and failure details for user context
- User can choose: continue (reset counter), fix manually and continue, or abort

#### [D06] Coder is reused for audit and integration fixes (DECIDED) {#d06-reuse-coder}

**Decision:** The existing coder agent (already persistent from the step loop) is resumed for audit and integration fix cycles. It is not re-spawned.

**Rationale:**
- The coder has accumulated context across all steps: files created, patterns established, build system knowledge
- Re-spawning would lose this accumulated context, leading to inconsistent fixes
- The persistent agent pattern already handles context growth via auto-compaction

**Implications:**
- Orchestrator resumes `coder_id` with audit/integration issues as the prompt
- The coder receives a different prompt shape (fix issues rather than implement step) but retains full project knowledge
- If the coder's context is exhausted, the orchestrator applies the same context exhaustion recovery as in the step loop

#### [D07] Integrator takes over PR creation from committer (DECIDED) {#d07-integrator-pr}

**Decision:** The committer's publish mode is removed entirely. The integrator agent handles branch push, PR creation (via `specks step-publish` or `gh pr create`), and CI verification (via `gh pr checks`).

**Rationale:**
- PR creation is logically a session-level operation, not a step-level operation
- Coupling PR creation with CI verification in a single agent allows the retry loop to handle CI failures naturally
- The committer becomes a simpler, single-responsibility agent focused on git commits

**Implications:**
- `agents/committer-agent.md` loses its Publish Mode section
- `agents/integrator-agent.md` inherits the publish logic plus CI check logic
- The implementer SKILL.md changes the post-loop sequence from "committer publish" to "integrator"
- Integrator's first invocation pushes and creates PR; subsequent invocations (retries) push fixup commits and re-check CI

#### [D08] Auditor returns PASS, REVISE, or ESCALATE (DECIDED) {#d08-auditor-recommendations}

**Decision:** The auditor returns one of three recommendations: PASS (all quality gates met), REVISE (fixable issues found, coder should address), or ESCALATE (critical issues requiring user intervention).

**Rationale:**
- Matches the recommendation pattern used by the reviewer agent
- PASS/REVISE/ESCALATE maps cleanly to the orchestrator's decision logic
- ESCALATE handles cases where issues cannot be resolved automatically (e.g., fundamental design problems)

**Implications:**
- Orchestrator dispatches to coder on REVISE, to user on ESCALATE
- PASS proceeds to integrator phase
- Issues are graded P0-P3 to help the coder prioritize fixes

#### [D09] Architect agent uses opus model (DECIDED) {#d09-architect-model}

**Decision:** The architect agent uses the opus model. The current `agents/architect-agent.md` incorrectly specifies `model: sonnet` and must be updated to `model: opus`.

**Rationale:**
- The architect performs deep read-only codebase analysis and strategy creation, which benefits from opus-level reasoning
- Consistency with Table T01 (#t01-agent-roster) which lists architect as opus
- The user confirmed the architect should be opus: "I thought the architect *was* opus. It should be!"

**Implications:**
- `agents/architect-agent.md` frontmatter must be updated from `model: sonnet` to `model: opus`
- Higher token cost for architect invocations
- This is a bug fix to an existing agent file, not a new design choice

---

### 9.1 Agent Contract Specifications {#agent-contracts}

#### Auditor Agent Contract {#auditor-contract}

**Spec S01: Auditor Input Contract** {#s01-auditor-input}

```json
{
  "worktree_path": "/abs/path/to/.specks-worktrees/specks__name-timestamp",
  "speck_path": ".specks/specks-N.md"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `worktree_path` | string | yes | Absolute path to the implementation worktree |
| `speck_path` | string | yes | Relative path to the speck file |

Resume prompt (re-audit after fixes):
```
Re-audit after coder fixes. Previous issues: <issues_json>.
```

**Spec S02: Auditor Output Contract** {#s02-auditor-output}

```json
{
  "build_results": {
    "build": {"command": "string", "exit_code": 0, "output_tail": "string"},
    "test": {"command": "string", "exit_code": 0, "output_tail": "string"},
    "clippy": {"command": "string", "exit_code": 0, "output_tail": "string"},
    "fmt_check": {"command": "string", "exit_code": 0, "output_tail": "string"}
  },
  "deliverable_checks": [
    {"criterion": "string", "status": "PASS|FAIL", "evidence": "string"}
  ],
  "cross_step_issues": [
    {"description": "string", "files": ["string"], "priority": "P0|P1|P2|P3"}
  ],
  "spot_check_findings": [
    {"step_anchor": "string", "description": "string", "priority": "P0|P1|P2|P3"}
  ],
  "issues": [
    {"description": "string", "priority": "P0|P1|P2|P3", "file": "string|null"}
  ],
  "recommendation": "PASS|REVISE|ESCALATE"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `build_results` | object | yes | Fresh build/test/clippy/fmt results |
| `deliverable_checks` | array | yes | Verification of each exit criterion from #exit-criteria |
| `cross_step_issues` | array | yes | Integration issues spanning multiple steps |
| `spot_check_findings` | array | yes | Issues found by spot-checking individual steps |
| `issues` | array | yes | All issues consolidated, graded P0-P3 |
| `recommendation` | enum | yes | PASS, REVISE, or ESCALATE |

#### Integrator Agent Contract {#integrator-contract}

**Spec S03: Integrator Input Contract** {#s03-integrator-input}

First invocation (push and create PR):
```json
{
  "operation": "publish",
  "worktree_path": "/abs/path/to/.specks-worktrees/specks__name-timestamp",
  "branch_name": "specks/name-timestamp",
  "base_branch": "main",
  "speck_title": "Phase D: Post-Loop Quality Gates",
  "speck_path": ".specks/specks-N.md",
  "repo": "owner/repo"
}
```

Resume prompt (re-check CI after fixup):
```
Fixup committed. Re-push and re-check CI. PR: <pr_url>.
```

**Spec S04: Integrator Output Contract** {#s04-integrator-output}

```json
{
  "pr_url": "string",
  "pr_number": 0,
  "branch_pushed": true,
  "ci_status": "pass|fail|pending|timeout",
  "ci_details": [
    {"check_name": "string", "status": "pass|fail|pending", "url": "string|null"}
  ],
  "recommendation": "PASS|REVISE|ESCALATE"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `pr_url` | string | yes | URL of the created or existing PR |
| `pr_number` | integer | yes | PR number |
| `branch_pushed` | boolean | yes | Whether branch was pushed successfully |
| `ci_status` | enum | yes | Overall CI status: pass, fail, pending, timeout |
| `ci_details` | array | yes | Individual check results |
| `recommendation` | enum | yes | PASS (CI green), REVISE (CI failure, fixable), ESCALATE (infra issue) |

#### Committer Fixup Mode Contract {#committer-fixup-contract}

**Spec S05: Committer Fixup Input** {#s05-committer-fixup-input}

```json
{
  "operation": "fixup",
  "worktree_path": "/abs/path/to/.specks-worktrees/specks__name-timestamp",
  "speck_path": ".specks/specks-N.md",
  "proposed_message": "fix(<scope>): <description>",
  "files_to_stage": ["path/to/file1.rs", "path/to/file2.rs"],
  "log_entry": {
    "summary": "Audit fix: <description>"
  }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `operation` | string | yes | Must be "fixup" |
| `worktree_path` | string | yes | Absolute path to the worktree |
| `speck_path` | string | yes | Relative path to the speck |
| `proposed_message` | string | yes | Conventional commit message with `fix` type |
| `files_to_stage` | array | yes | Files to stage for commit |
| `log_entry.summary` | string | yes | Summary for implementation log |

**Spec S06: Committer Fixup Output** {#s06-committer-fixup-output}

```json
{
  "operation": "fixup",
  "commit_hash": "abc1234",
  "commit_message": "fix(<scope>): <description>",
  "files_staged": ["path/to/file1.rs"],
  "log_updated": true,
  "aborted": false,
  "abort_reason": null
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `operation` | string | yes | "fixup" |
| `commit_hash` | string | yes | Short hash of the commit |
| `commit_message` | string | yes | The commit message used |
| `files_staged` | array | yes | Files that were staged |
| `log_updated` | boolean | yes | Whether implementation log was updated |
| `aborted` | boolean | yes | Whether the operation was aborted |
| `abort_reason` | string/null | yes | Reason for abort, if any |

---

### 9.2 Orchestration Flow Update {#orchestration-flow}

**Diagram Diag01: Updated Implementer Orchestration** {#diag01-orchestration}

```
  Task: implementer-setup-agent (FRESH spawn, one time)
       │
       └── ready (worktree_path, branch_name, base_branch, resolved_steps, bead_mapping)
              │
              ▼
       ┌─────────────────────────────────────────────────────────────────┐
       │              FOR EACH STEP in resolved_steps                    │
       │  ┌───────────────────────────────────────────────────────────┐  │
       │  │  architect → coder → [review loop] → committer(commit)   │  │
       │  └───────────────────────────────────────────────────────────┘  │
       └─────────────────────────────────────────────────────────────────┘
              │
              ▼
       ┌─────────────────────────────────────────────────────────────────┐
       │  POST-LOOP: AUDITOR PHASE                                      │
       │  ┌───────────────────────────────────────────────────────────┐  │
       │  │  SPAWN auditor-agent → auditor_id                         │  │
       │  │         │                                                 │  │
       │  │    PASS? ──► proceed to integrator                        │  │
       │  │         │                                                 │  │
       │  │    REVISE? ──► RESUME coder_id (fix issues)               │  │
       │  │                  ──► RESUME committer_id (fixup)          │  │
       │  │                  ──► RESUME auditor_id (re-audit)         │  │
       │  │                  ──► (max 3 rounds, then ESCALATE)        │  │
       │  │         │                                                 │  │
       │  │    ESCALATE? ──► AskUserQuestion (continue/abort)         │  │
       │  └───────────────────────────────────────────────────────────┘  │
       └─────────────────────────────────────────────────────────────────┘
              │
              ▼
       ┌─────────────────────────────────────────────────────────────────┐
       │  POST-LOOP: INTEGRATOR PHASE                                   │
       │  ┌───────────────────────────────────────────────────────────┐  │
       │  │  SPAWN integrator-agent → integrator_id                   │  │
       │  │    (push branch, create PR, wait for CI)                  │  │
       │  │         │                                                 │  │
       │  │    PASS? ──► implementation complete                      │  │
       │  │         │                                                 │  │
       │  │    REVISE? ──► RESUME coder_id (fix CI issues)            │  │
       │  │                  ──► RESUME committer_id (fixup)          │  │
       │  │                  ──► RESUME integrator_id (re-push/check) │  │
       │  │                  ──► (max 3 rounds, then ESCALATE)        │  │
       │  │         │                                                 │  │
       │  │    ESCALATE? ──► AskUserQuestion (continue/abort)         │  │
       │  └───────────────────────────────────────────────────────────┘  │
       └─────────────────────────────────────────────────────────────────┘
```

**Table T01: Updated Agent Roster** {#t01-agent-roster}

| Agent | Model | Tools | Spawned | Role |
|-------|-------|-------|---------|------|
| architect | opus | Bash, Read, Grep, Glob, WebFetch, WebSearch | Step 0 | Read-only strategy |
| coder | sonnet | Read, Grep, Glob, Write, Edit, Bash, WebFetch, WebSearch | Step 0 | Implement + fix |
| reviewer | sonnet | Bash, Read, Grep, Glob, Write, Edit | Step 0 | Per-step review |
| committer | sonnet | Bash | Step 0 | Commit + fixup |
| auditor | opus | Bash, Read, Grep, Glob | Post-loop | Holistic quality gate |
| integrator | sonnet | Bash | Post-loop | Push, PR, CI check |

**Table T02: Agent ID Tracking** {#t02-agent-ids}

| Variable | Set When | Used By |
|----------|----------|---------|
| `architect_id` | Step 0 spawn | Steps 1..N resume |
| `coder_id` | Step 0 spawn | Steps 1..N + review retries + audit fixes + CI fixes |
| `reviewer_id` | Step 0 spawn | Steps 1..N + re-reviews |
| `committer_id` | Step 0 spawn | Steps 1..N + audit fixup + CI fixup |
| `auditor_id` | Post-loop spawn | Audit retries resume |
| `integrator_id` | Post-loop spawn | CI retry resume |

---

### Open Questions (MUST RESOLVE OR EXPLICITLY DEFER) {#open-questions}

No open questions. All design decisions have been resolved through clarification.

---

### Risks and Mitigations {#risks}

| Risk | Impact | Likelihood | Mitigation | Trigger to revisit |
|------|--------|------------|------------|--------------------|
| Auditor context too large for opus | med | low | Auto-compaction handles; auditor reads only worktree, not full history | Auditor starts failing to complete |
| CI checks timeout or hang | med | med | Integrator has timeout handling; escalates to user after max retries | CI environment becomes unreliable |
| Coder context exhaustion during fix cycles | high | low | Same recovery pattern as step loop: spawn fresh coder with file list | Coder resumes fail with "Prompt too long" |

**Risk R01: Agent File Consistency** {#r01-agent-consistency}

- **Risk:** Modifying the committer and adding two new agents may introduce inconsistencies in input/output contracts, tool declarations, or model specifications across agent files.
- **Mitigation:** Each step includes verification checkpoints that cross-reference agent files against the orchestrator's Task call patterns.
- **Residual risk:** Manual review is still needed to verify that the prose descriptions in agent files align with the structured contracts.

---

### 9.3 Execution Steps {#execution-steps}

#### Step 0: Create auditor-agent.md {#step-0}

**Bead:** `specks-99w.1`

**Commit:** `feat(agents): add auditor-agent and fix architect model to opus`

**References:** [D01] Auditor uses opus model, [D04] Auditor scope is hybrid, [D08] Auditor returns PASS/REVISE/ESCALATE, [D09] Architect agent uses opus model, Spec S01, Spec S02, (#auditor-contract, #agent-contracts)

**Artifacts:**
- `agents/auditor-agent.md` (new file)
- `agents/architect-agent.md` (modified: model field)

**Tasks:**
- [ ] Update `agents/architect-agent.md` YAML frontmatter: change `model: sonnet` to `model: opus` per [D09]
- [ ] Create `agents/auditor-agent.md` with YAML frontmatter: `name: auditor-agent`, `description: Post-loop quality gate...`, `model: opus`, `permissionMode: dontAsk`, `tools: Bash, Read, Grep, Glob`
- [ ] Write the "Your Role" section describing the auditor as the implementer analog to the planner's critic
- [ ] Write the Persistent Agent Pattern section: initial spawn runs full audit, resume re-audits after fixes
- [ ] Write the Input Contract section matching Spec S01 (#s01-auditor-input): initial spawn receives worktree_path and speck_path; resume receives previous issues
- [ ] Write the Output Contract section matching Spec S02 (#s02-auditor-output): build_results, deliverable_checks, cross_step_issues, spot_check_findings, issues, recommendation
- [ ] Write the Implementation section with four phases: (1) run fresh build/test/clippy/fmt, (2) read speck #exit-criteria and verify each criterion, (3) spot-check step implementations, (4) check cross-step integration
- [ ] Write the Priority Grading section: P0 (build/test failure), P1 (deliverable not met), P2 (cross-step issue), P3 (code quality)
- [ ] Write the Recommendation Logic section: PASS if no P0/P1 issues and build/test green; REVISE if fixable P0/P1 issues; ESCALATE if fundamental design problems
- [ ] Write the Behavior Rules section including: Bash tool for build/test/clippy/fmt only, Read/Grep/Glob for code inspection, no file modifications
- [ ] Write JSON Validation Requirements section with minimal error response
- [ ] Write Error Handling section with error response format

**Tests:**
- [ ] Manual review: verify `agents/architect-agent.md` frontmatter has `model: opus`
- [ ] Manual review: verify `agents/auditor-agent.md` frontmatter has `model: opus` and `tools: Bash, Read, Grep, Glob`
- [ ] Manual review: verify input contract matches Spec S01
- [ ] Manual review: verify output contract matches Spec S02

**Checkpoint:**
- [ ] `agents/architect-agent.md` YAML frontmatter contains `model: opus` (not `model: sonnet`)
- [ ] File `agents/auditor-agent.md` exists
- [ ] `agents/auditor-agent.md` YAML frontmatter contains `model: opus`
- [ ] `agents/auditor-agent.md` contains `## Input Contract` and `## Output Contract` sections
- [ ] `agents/auditor-agent.md` contains `recommendation` field with `PASS|REVISE|ESCALATE` values

**Rollback:**
- Restore `agents/architect-agent.md` from git: `git checkout agents/architect-agent.md`
- Delete `agents/auditor-agent.md`

**Commit after all checkpoints pass.**

---

#### Step 1: Create integrator-agent.md {#step-1}

**Bead:** `specks-99w.2`

**Commit:** `feat(agents): add integrator-agent for PR creation and CI verification`

**References:** [D02] Integrator uses sonnet model, [D07] Integrator takes over PR creation, Spec S03, Spec S04, (#integrator-contract, #agent-contracts)

**Artifacts:**
- `agents/integrator-agent.md` (new file)

**Tasks:**
- [ ] Create `agents/integrator-agent.md` with YAML frontmatter: `name: integrator-agent`, `description: Push branch, create PR, verify CI...`, `model: sonnet`, `permissionMode: dontAsk`, `tools: Bash`
- [ ] Write the "Your Role" section describing the integrator as handling PR creation and CI verification
- [ ] Write the Persistent Agent Pattern section: initial spawn pushes and creates PR, resume re-pushes fixups and re-checks CI
- [ ] Write the Input Contract section matching Spec S03 (#s03-integrator-input): first invocation receives operation, worktree_path, branch_name, base_branch, speck_title, speck_path, repo; resume receives PR URL
- [ ] Write the Output Contract section matching Spec S04 (#s04-integrator-output): pr_url, pr_number, branch_pushed, ci_status, ci_details, recommendation
- [ ] Write the Implementation section with two modes: (1) first invocation calls `specks step-publish` then `gh pr checks --watch`, (2) resume pushes with `git -C <worktree> push` then `gh pr checks --watch`
- [ ] Write the CI Check Logic section: parse `gh pr checks` output, map to ci_status (pass/fail/pending/timeout), populate ci_details array
- [ ] Write the Recommendation Logic section: PASS if CI green; REVISE if CI failure with actionable error; ESCALATE if infrastructure issue or persistent failure
- [ ] Write the Behavior Rules section including: Bash tool only, no file reads or modifications
- [ ] Write JSON Validation Requirements section with minimal error response
- [ ] Write Error Handling section with error response format

**Tests:**
- [ ] Manual review: verify frontmatter has `model: sonnet` and `tools: Bash`
- [ ] Manual review: verify input contract matches Spec S03
- [ ] Manual review: verify output contract matches Spec S04

**Checkpoint:**
- [ ] File `agents/integrator-agent.md` exists
- [ ] YAML frontmatter contains `model: sonnet`
- [ ] File contains `specks step-publish` command reference
- [ ] File contains `gh pr checks` command reference

**Rollback:**
- Delete `agents/integrator-agent.md`

**Commit after all checkpoints pass.**

---

#### Step 2: Modify committer-agent.md (remove publish, add fixup) {#step-2}

**Depends on:** #step-0

**Bead:** `specks-99w.3`

**Commit:** `refactor(agents): replace committer publish mode with fixup mode`

**References:** [D03] Fixup commits have no bead tracking, [D07] Integrator takes over PR creation, Spec S05, Spec S06, (#committer-fixup-contract)

**Artifacts:**
- `agents/committer-agent.md` (modified)

**Tasks:**
- [ ] Remove the entire "Publish Mode" section from the Input Contract
- [ ] Remove the "Publish Mode" subsection from the Implementation section (the `specks step-publish` invocation)
- [ ] Remove the publish mode from the Output Contract description
- [ ] Add "Fixup Mode" to the Input Contract section matching Spec S05 (#s05-committer-fixup-input): operation "fixup", worktree_path, speck_path, proposed_message, files_to_stage, log_entry.summary
- [ ] Add "Fixup Mode" to the Implementation section: (1) run `specks log prepend --step audit-fix --speck <path> --summary <text>`, (2) run `git -C <worktree> add <files>`, (3) run `git -C <worktree> commit -m "<message>"`
- [ ] Add fixup mode to the Output Contract matching Spec S06 (#s06-committer-fixup-output): operation "fixup", commit_hash, commit_message, files_staged, log_updated, aborted, abort_reason
- [ ] Update the agent description in YAML frontmatter to reflect commit and fixup modes (no longer mentions publish)
- [ ] Update the "Your Role" section to describe commit mode and fixup mode (remove publish references)

**Tests:**
- [ ] Manual review: verify no remaining references to "publish" mode or `specks step-publish` in the file
- [ ] Manual review: verify fixup mode implementation uses `git commit` (not `specks step-commit`)
- [ ] Manual review: verify fixup mode has no bead-related operations

**Checkpoint:**
- [ ] File `agents/committer-agent.md` exists and has been modified
- [ ] No occurrences of `step-publish` in the file
- [ ] File contains `"operation": "fixup"` in the input contract
- [ ] Fixup implementation section uses `specks log prepend` and `git -C` commit

**Rollback:**
- Restore `agents/committer-agent.md` from git: `git checkout agents/committer-agent.md`

**Commit after all checkpoints pass.**

---

#### Step 3: Update implementer SKILL.md orchestration {#step-3}

**Depends on:** #step-0, #step-1, #step-2

**Bead:** `specks-99w.4`

**Commit:** `feat(skills): add auditor and integrator phases to implementer orchestration`

**References:** [D05] Retry limit is 3 with user escalation, [D06] Coder is reused for fixes, [D07] Integrator takes over PR creation, [D08] Auditor returns PASS/REVISE/ESCALATE, Diagram Diag01, Table T01, Table T02, Spec S01, Spec S02, Spec S03, Spec S04, Spec S05, Spec S06, (#orchestration-flow, #diag01-orchestration, #t01-agent-roster, #t02-agent-ids)

**Tasks:**
- [ ] See substeps 3.1-3.3 below for detailed task breakdown

**Tests:**
- [ ] See substeps and Step 3 Summary checkpoint

**Checkpoint:**
- [ ] See Step 3 Summary checkpoint

> This step is broken into three substeps with separate commits and checkpoints.
> Substep 3.1 updates the diagram and variables. Substep 3.2 writes the auditor and integrator orchestration sections. Substep 3.3 updates reference sections and progress reporting.

##### Step 3.1: Update orchestration diagram, variables, and remove publish mode {#step-3-1}

**Commit:** `refactor(skills): update orchestration diagram and agent ID tracking for auditor/integrator`

**References:** [D07] Integrator takes over PR creation, Diagram Diag01, Table T01, Table T02, (#orchestration-flow, #diag01-orchestration, #t01-agent-roster, #t02-agent-ids)

**Artifacts:**
- `skills/implementer/SKILL.md` (modified)

**Tasks:**
- [ ] Add `auditor_id = null` and `integrator_id = null` to the agent ID initialization block alongside architect_id, coder_id, reviewer_id, committer_id
- [ ] Add `auditor_attempts = 0` and `integrator_attempts = 0` counter initialization
- [ ] Update the Orchestration Loop ASCII diagram to match Diagram Diag01 (#diag01-orchestration): replace the post-loop "RESUME committer_id (publish mode)" with auditor phase and integrator phase
- [ ] Remove the committer-agent publish post-call message template from Progress Reporting
- [ ] Remove the section "4. Implementation Completion" that calls committer in publish mode (will be replaced in Step 3.2)

**Tests:**
- [ ] Manual review: verify orchestration diagram shows setup, [steps], auditor, integrator flow
- [ ] Manual review: verify `auditor_id` and `integrator_id` are declared
- [ ] Manual review: verify no remaining `"operation": "publish"` in committer Task calls

**Checkpoint:**
- [ ] File `skills/implementer/SKILL.md` has been modified
- [ ] File contains `auditor_id` and `integrator_id` variable declarations
- [ ] File contains `auditor_attempts` and `integrator_attempts` counter references
- [ ] No remaining `"operation": "publish"` in committer Task calls

**Rollback:**
- Restore `skills/implementer/SKILL.md` from git: `git checkout skills/implementer/SKILL.md`

**Commit after all checkpoints pass.**

---

##### Step 3.2: Add auditor and integrator orchestration phases {#step-3-2}

**Depends on:** #step-3-1

**Commit:** `feat(skills): add auditor and integrator phases with retry loops`

**References:** [D05] Retry limit is 3 with user escalation, [D06] Coder is reused for fixes, [D08] Auditor returns PASS/REVISE/ESCALATE, Spec S01, Spec S02, Spec S03, Spec S04, Spec S05, Spec S06, (#orchestration-flow, #auditor-contract, #integrator-contract, #committer-fixup-contract)

**Artifacts:**
- `skills/implementer/SKILL.md` (modified)

**Tasks:**
- [ ] Add section "4. Auditor Phase" after the step loop: spawn auditor-agent fresh with worktree_path and speck_path, parse JSON output per Spec S02, handle PASS/REVISE/ESCALATE
- [ ] Write the auditor REVISE loop: resume coder_id with issues, resume committer_id with fixup operation (Spec S05), resume auditor_id for re-audit; increment auditor_attempts; escalate after 3
- [ ] Write the auditor ESCALATE handler: AskUserQuestion with issues and options (continue/fix manually/abort)
- [ ] Add section "5. Integrator Phase" after auditor phase: spawn integrator-agent fresh with publish payload per Spec S03, parse JSON output per Spec S04, handle PASS/REVISE/ESCALATE
- [ ] Write the integrator REVISE loop: resume coder_id with CI failure details, resume committer_id with fixup operation (Spec S05), resume integrator_id for re-push/re-check; increment integrator_attempts; escalate after 3
- [ ] Write the integrator ESCALATE handler: AskUserQuestion with CI details and options
- [ ] Add section "6. Implementation Completion": output PR URL from integrator, output session end message

**Tests:**
- [ ] Manual review: verify auditor REVISE loop resumes coder_id, committer_id (fixup), and auditor_id in sequence
- [ ] Manual review: verify integrator REVISE loop resumes coder_id, committer_id (fixup), and integrator_id in sequence
- [ ] Manual review: verify max 3 retries for both auditor and integrator loops
- [ ] Manual review: verify escalation prompts include specific issues for user context

**Checkpoint:**
- [ ] File contains "4. Auditor Phase" and "5. Integrator Phase" and "6. Implementation Completion" section headers
- [ ] File contains `AskUserQuestion` escalation for both auditor and integrator max retries
- [ ] Auditor phase references Spec S02 output fields (build_results, deliverable_checks, recommendation)
- [ ] Integrator phase references Spec S04 output fields (pr_url, ci_status, recommendation)

**Rollback:**
- Restore `skills/implementer/SKILL.md` from git: `git checkout skills/implementer/SKILL.md`

**Commit after all checkpoints pass.**

---

##### Step 3.3: Update reference sections, progress reporting, and validation {#step-3-3}

**Depends on:** #step-3-2

**Commit:** `docs(skills): update reference tables, progress templates, and validation for new agents`

**References:** [D03] Fixup commits have no bead tracking, Table T01, Table T02, Spec S02, Spec S04, Spec S06, (#t01-agent-roster, #t02-agent-ids)

**Artifacts:**
- `skills/implementer/SKILL.md` (modified)

**Tasks:**
- [ ] Update the "Reference: Persistent Agent Pattern" table to include auditor and integrator rows matching Table T01 (#t01-agent-roster)
- [ ] Add progress reporting template for auditor-agent post-call message (recommendation, build status, issue count by priority)
- [ ] Add progress reporting template for integrator-agent post-call message (PR URL, CI status, check details)
- [ ] Add progress reporting template for committer-agent fixup post-call message (commit hash, message, files staged)
- [ ] Update the "Reference: Beads Integration" section to note that fixup commits do not close beads
- [ ] Add JSON Validation sections for auditor output (Spec S02 fields) and integrator output (Spec S04 fields)

**Tests:**
- [ ] Manual review: verify Persistent Agent Pattern table has 6 rows (architect, coder, reviewer, committer, auditor, integrator)
- [ ] Manual review: verify all three new progress reporting templates are present
- [ ] Manual review: verify Beads Integration section mentions fixup commits

**Checkpoint:**
- [ ] "Reference: Persistent Agent Pattern" table includes auditor and integrator rows
- [ ] Progress Reporting section contains `auditor-agent` and `integrator-agent` templates
- [ ] Progress Reporting section contains committer fixup template
- [ ] "Reference: Beads Integration" section mentions fixup commits

**Rollback:**
- Restore `skills/implementer/SKILL.md` from git: `git checkout skills/implementer/SKILL.md`

**Commit after all checkpoints pass.**

---

#### Step 3 Summary {#step-3-summary}

**Depends on:** #step-3-1, #step-3-2, #step-3-3

**Bead:** `specks-99w.5`

**Commit:** `N/A (aggregate checkpoint only)`

**References:** [D05] Retry limit is 3 with user escalation, [D07] Integrator takes over PR creation, Diagram Diag01, Table T01, (#orchestration-flow, #t01-agent-roster)

After completing Steps 3.1-3.3, you will have:
- Updated orchestration diagram showing setup, [per-step loop], auditor phase, integrator phase
- Agent ID tracking for all 6 agents including auditor_id and integrator_id
- Auditor and integrator orchestration sections with retry loops (max 3) and user escalation
- Updated reference tables, progress reporting templates, and validation for all new agents
- Beads integration documentation updated for fixup commits

**Tasks:**
- [ ] Verify all substeps completed successfully

**Tests:**
- [ ] Manual review: end-to-end read of `skills/implementer/SKILL.md` for coherence

**Checkpoint:**
- [ ] `skills/implementer/SKILL.md` contains complete orchestration for setup, step loop, auditor phase, integrator phase, and completion
- [ ] No remaining references to committer publish mode anywhere in the file

---

### 9.4 Deliverables and Checkpoints {#deliverables}

**Deliverable:** Post-loop quality gates (auditor and integrator agents) integrated into the implementer workflow, with the committer simplified to commit and fixup modes only.

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

- [ ] `agents/architect-agent.md` has `model: opus` in frontmatter (updated from sonnet)
- [ ] `agents/auditor-agent.md` exists with opus model, Bash/Read/Grep/Glob tools, and PASS/REVISE/ESCALATE output contract
- [ ] `agents/integrator-agent.md` exists with sonnet model, Bash tool, and PASS/REVISE/ESCALATE output contract using `specks step-publish` and `gh pr checks`
- [ ] `agents/committer-agent.md` has no publish mode and has fixup mode using `specks log prepend` + `git commit`
- [ ] `skills/implementer/SKILL.md` orchestration includes post-loop auditor phase with max 3 retry loop
- [ ] `skills/implementer/SKILL.md` orchestration includes post-loop integrator phase with max 3 retry loop
- [ ] Both retry loops escalate to user via AskUserQuestion after max retries
- [ ] Coder and committer agents are reused (not re-spawned) for audit and integration fixes

**Acceptance tests:**
- [ ] Manual review: all four modified/created files are internally consistent (contracts match across agent files and orchestrator)
- [ ] Manual review: no orphaned references to committer publish mode in any file

#### Roadmap / Follow-ons (Explicitly Not Required for Phase Close) {#roadmap}

- [ ] Update CLAUDE.md agent roster and command documentation to reflect auditor/integrator agents and committer changes (committer no longer handles "push, create PR")
- [ ] Add automated tests that validate agent contract JSON schemas
- [ ] Add integration tests that simulate the auditor/integrator retry loops
- [ ] Consider adding a `--skip-audit` flag to the implementer for small changes
- [ ] Consider caching CI check results to avoid redundant `gh pr checks` calls

| Checkpoint | Verification |
|------------|--------------|
| Agent files exist | `ls agents/auditor-agent.md agents/integrator-agent.md` |
| Committer has no publish | `grep -c 'step-publish' agents/committer-agent.md` returns 0 |
| Orchestrator has auditor | `grep -c 'auditor_id' skills/implementer/SKILL.md` returns nonzero |
| Orchestrator has integrator | `grep -c 'integrator_id' skills/implementer/SKILL.md` returns nonzero |

**Commit after all checkpoints pass.**