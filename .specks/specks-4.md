## Phase 4.0: Task-Based Orchestration Architecture {#phase-4}

**Purpose:** Restructure specks as a Claude Code plugin with the correct Task-based orchestration model: 2 orchestrator SKILLS that spawn 9 sub-AGENTS via Task tool.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | specks |
| Status | draft |
| Target branch | main |
| Tracking issue/PR | N/A |
| Last updated | 2026-02-07 |
| Beads Root | |

---

### Phase Overview {#phase-overview}

#### Context {#context}

The specks plugin architecture was incorrectly designed based on a misunderstanding of Claude Code's Skill and Task tool mechanics. The current implementation uses "orchestrator agents" that attempt to call skills via the Skill tool. This fails because:

1. Skills invoked via Skill tool perform prompt injection and take over context - they do not return
2. Subagents cannot spawn other subagents
3. The Skill tool is for "fire-and-forget" context takeover, not for sub-task invocation

The correct architecture uses orchestrator SKILLS (which run in the main context and can use Task tool) that spawn sub-AGENTS (which run to completion and return results).

#### Strategy {#strategy}

- Clean up test files and obsolete orchestrator agents first (low risk, clears confusion)
- Convert planning sub-agents before implementation sub-agents (simpler, fewer tools)
- Convert read-only agents before write-capable agents (lower risk)
- Update orchestrator skills incrementally with tested logic
- Test after each phase to verify Task-based spawning works correctly
- Delete old skill files only after agent conversions are verified

#### Stakeholders / Primary Customers {#stakeholders}

1. Specks users invoking `/specks:planner` and `/specks:implementer`
2. Specks developers maintaining the plugin architecture

#### Success Criteria (Measurable) {#success-criteria}

- `/specks:planner "test idea"` spawns clarifier-agent, author-agent, critic-agent via Task and returns results (verify via run directory artifacts)
- `/specks:implementer .specks/specks-test.md` spawns all implementation agents via Task and completes a step (verify via commit and bead closure)
- No context takeover issues - orchestrator maintains control throughout the loop
- All 9 sub-agents exist as agent files with proper YAML frontmatter
- Planner and implementer skills contain full orchestration logic
- All obsolete files deleted (0 skill directories for converted agents)

#### Scope {#scope}

1. Delete obsolete orchestrator agent files (planner-agent.md, implementer-agent.md)
2. Delete test files (test-counter-agent.md, test-decider-agent.md, test skill directories)
3. Create 9 new agent files with specifications from architecture report
4. Modify 2 orchestrator skills (planner, implementer) to contain full orchestration logic
5. Delete skill directories for agents that were converted
6. Delete legacy `.claude/skills/` directory
7. Delete interviewer skill (eliminated - orchestrators use AskUserQuestion directly)

#### Non-goals (Explicitly out of scope) {#non-goals}

- Changes to the Rust CLI codebase
- Changes to beads integration logic
- New agent capabilities not specified in the architecture report
- Performance optimization of agent invocations

#### Dependencies / Prerequisites {#dependencies}

- Architecture report at `.specks/specks-4-architecture-report.md` (complete)
- Understanding of Task vs Skill mechanics (documented in architecture report)
- All current files exist as verified by glob

#### Constraints {#constraints}

- Maximum ONE agent context active at any time
- Sub-agents cannot spawn other sub-agents
- Skills cannot return values when invoked via Skill tool

#### Assumptions {#assumptions}

- Task tool invocations return results to the calling skill
- Agent files with proper YAML frontmatter are recognized by Claude Code
- The plugin-dir mechanism loads both agents/ and skills/ directories

---

### Open Questions (MUST RESOLVE OR EXPLICITLY DEFER) {#open-questions}

No open questions. All architecture decisions were resolved in the architecture report.

---

### Risks and Mitigations {#risks}

| Risk | Impact | Likelihood | Mitigation | Trigger to revisit |
|------|--------|------------|------------|--------------------|
| Task tool behavior differs from documented | high | low | Test incrementally with simple agents first | First agent fails to return |
| Agent frontmatter format incorrect | medium | low | Verify format against Claude Code documentation | Plugin fails to load agents |
| Orchestrator skill logic too large | medium | medium | Use clear sections and helper comments | Skill exceeds context limits |
| Beads not installed or unavailable | high | low | Check beads availability in Step 0 before any implementation work | `specks beads status` fails |
| Task tool returns wrapped/transformed JSON | medium | low | Validate in Step 1 with simple agent spawn test | JSON parsing fails in orchestrator |
| Concurrent session conflicts | medium | low | Optimistic isolation with metadata scan (see D06) | Session artifacts corrupted |
| Skill auto-triggered unexpectedly | low | low | Use `disable-model-invocation: true` in orchestrator skills | Skill runs without explicit invocation |

**Risk R01: Incomplete Agent Conversion** {#r01-incomplete-conversion}

- **Risk:** Some agent specifications may be incomplete or miss edge cases
- **Mitigation:** Use JSON schemas from architecture report verbatim; test each agent after creation
- **Residual risk:** Edge cases may surface during real usage

---

### 4.0.0 Design Decisions {#design-decisions}

#### [D01] Skills are Orchestrators, Agents are Sub-tasks (DECIDED) {#d01-skill-orchestrator}

**Decision:** The planner and implementer are SKILLS that contain orchestration logic and spawn sub-agents via Task tool.

**Rationale:**
- Skills run in main context and can use Task tool to spawn subagents
- Task tool returns results to the caller, enabling orchestration loops
- Skill tool performs prompt injection and does not return

**Implications:**
- Orchestrator logic lives in SKILL.md files, not agent files
- Sub-tasks must be agents (invoked via Task) not skills
- No "orchestrator agents" - that concept is obsolete

---

#### [D02] Nine Sub-Agents with Specific Tool Sets (DECIDED) {#d02-nine-agents}

**Decision:** Create exactly 9 sub-agents with the tools specified in the architecture report.

**Rationale:**
- Each agent has a focused purpose and minimal tool set
- Read-only agents (clarifier, critic, architect, reviewer, auditor) have Read, Grep, Glob only
- Write-capable agents (author, coder, logger, committer) have additional Write/Edit/Bash

**Implications:**
- Agent files must have accurate `tools` frontmatter (allowlist model)
- Agent files must have `description` field (required by Claude Code)
- Tool restrictions are enforced by Claude Code (confirmed: `tools` is an allowlist, not advisory)

---

#### [D03] Interviewer Eliminated (DECIDED) {#d03-no-interviewer}

**Decision:** There is no interviewer agent. Orchestrator skills use AskUserQuestion directly.

**Rationale:**
- AskUserQuestion is a tool available to skills running in main context
- Spawning an agent just to ask a question adds latency and complexity
- User interaction belongs in the orchestrator, not delegated to sub-agents

**Implications:**
- Delete `skills/interviewer/SKILL.md`
- Do not create an interviewer-agent

---

#### [D04] JSON Input/Output Contracts (DECIDED) {#d04-json-contracts}

**Decision:** All sub-agents receive JSON input and return JSON output as specified in the architecture report.

**Rationale:**
- Structured contracts enable reliable parsing
- JSON schemas are documented in architecture report sections 2.1 and 2.2
- Orchestrator can validate responses before proceeding

**Implications:**
- Agent prompts must specify input/output JSON schemas
- Orchestrator skills must parse JSON responses

---

#### [D05] Incremental Testing Strategy (DECIDED) {#d05-incremental-testing}

**Decision:** Convert and test agents in the order specified in the architecture report section 7.

**Rationale:**
- Start with simple read-only agents (clarifier) to verify Task mechanics
- Build up to write-capable agents (coder) after basics are confirmed
- Full integration test after all agents converted

**Implications:**
- Execution steps follow this ordering
- Each step has verification checkpoints

---

#### [D06] Concurrent Session Handling via Optimistic Isolation (DECIDED) {#d06-concurrent-sessions}

**Decision:** Each session writes to its own directory (natural isolation). Concurrent sessions on the same speck are detected by scanning `metadata.json` files, not prevented by lock files.

**Rationale:**
- Each session has a unique session ID with timestamp + UUID suffix
- Different sessions write to different directories, no file-level conflict
- Optimistic concurrency: conflicts are detected, not prevented
- No lock files means no stale lock problem from crashed sessions

**Detection mechanism:**
1. Before starting, scan `.specks/runs/*/metadata.json` for entries where `status == "in_progress"` AND `speck_path` matches the current speck
2. If an active session is found with `last_updated_at` less than 1 hour old, warn the user via AskUserQuestion: "Another session is active on this speck. Continue anyway?"
3. If the active session's `last_updated_at` is older than 1 hour, treat it as abandoned — report it to the user as stale and continue
4. If no active sessions found, proceed normally

**Implications:**
- Orchestrator skills scan metadata files before starting (read-only check, no file creation)
- Conflicts on shared files (e.g., implementation log) are resolved at commit time via standard git mechanisms
- Abandoned sessions leave no cleanup burden — their metadata naturally ages out
- No lock files, no stale lock recovery, no cleanup-on-crash logic

---

#### [D07] Run Directory Cleanup Policy (DECIDED) {#d07-runs-cleanup}

**Decision:** Run directories are retained for 30 days. A `specks runs gc` command (future work) will clean up old sessions.

**Rationale:**
- Session artifacts are valuable for debugging and auditing
- Automatic cleanup prevents unbounded growth
- 30 days provides reasonable retention for post-mortems

**Implications:**
- Run directories include creation timestamp in session ID
- Future `specks runs gc --older-than 30d` command
- For now, manual cleanup: `rm -rf .specks/runs/<session-id>`
- `.specks/runs/` added to `.gitignore`

---

#### [D08] Error Handling for Task and JSON Failures (DECIDED) {#d08-error-handling}

**Decision:** If Task tool fails or returns unparseable JSON, orchestrator logs raw output to error file and halts with descriptive message.

**Rationale:**
- Silent failures are worse than halting
- Raw output helps debug agent issues
- User can review and retry or fix

**Implications:**
- Orchestrators wrap Task invocations in error handling
- On failure: write to `<session>/error.json` with raw output
- On failure: halt with "Agent [name] failed: [reason]"
- Do not retry automatically - user must intervene

---

### 4.0.1 File Inventory {#file-inventory}

#### Files to Delete {#files-to-delete}

**Table T01: Obsolete Agent Files** {#t01-obsolete-agents}

| File | Reason |
|------|--------|
| `agents/planner-agent.md` | Orchestration moves to planner SKILL |
| `agents/implementer-agent.md` | Orchestration moves to implementer SKILL |
| `agents/test-counter-agent.md` | Test file from experimentation |
| `agents/test-decider-agent.md` | Test file from experimentation |

**Table T02: Skill Directories to Delete** {#t02-skill-dirs}

| Directory | Reason |
|-----------|--------|
| `skills/interviewer/` | Eliminated - AskUserQuestion used directly |
| `skills/clarifier/` | Converted to agent |
| `skills/author/` | Converted to agent |
| `skills/critic/` | Converted to agent |
| `skills/architect/` | Converted to agent |
| `skills/coder/` | Converted to agent |
| `skills/reviewer/` | Converted to agent |
| `skills/auditor/` | Converted to agent |
| `skills/logger/` | Converted to agent |
| `skills/committer/` | Converted to agent |
| `skills/test-counter/` | Test directory |
| `skills/test-decider/` | Test directory |
| `skills/test-loop-orchestrator/` | Test directory |

**Table T03: Legacy Directories to Delete** {#t03-legacy-dirs}

| Directory | Reason |
|-----------|--------|
| `.claude/skills/implement-plan/` | Legacy skill |
| `.claude/skills/prepare-git-commit-message/` | Legacy skill |
| `.claude/skills/update-plan-implementation-log/` | Legacy skill |
| `.claude/skills/` | Empty after above deletions |
| `.claude/agents/code-architect.md` | Legacy agent (not part of specks plugin) |
| `.claude/agents/code-planner.md` | Legacy agent (not part of specks plugin) |
| `.claude/agents/` | Empty after above deletions |

#### Files to Create {#files-to-create}

**Table T04: New Agent Files** {#t04-new-agents}

| File | Purpose |
|------|---------|
| `agents/clarifier-agent.md` | Analyze ideas, generate clarifying questions |
| `agents/author-agent.md` | Create and revise speck documents |
| `agents/critic-agent.md` | Review specks for quality and compliance |
| `agents/architect-agent.md` | Create implementation strategies |
| `agents/coder-agent.md` | Execute strategies with drift detection |
| `agents/reviewer-agent.md` | Verify step matches plan |
| `agents/auditor-agent.md` | Check code quality and security |
| `agents/logger-agent.md` | Update implementation log |
| `agents/committer-agent.md` | Stage, commit, close beads |

#### Files to Modify {#files-to-modify}

**Table T05: Orchestrator Skills to Modify** {#t05-orchestrator-mods}

| File | Change |
|------|--------|
| `skills/planner/SKILL.md` | Add complete planning orchestration logic |
| `skills/implementer/SKILL.md` | Add complete implementation orchestration logic |

#### Files to Keep {#files-to-keep}

**Table T06: Archived Files** {#t06-archived}

| File | Reason |
|------|--------|
| `agents/archived/director.md` | Historical reference |
| `agents/archived/interviewer.md` | Historical reference |

---

### 4.0.2 Agent Specifications {#agent-specifications}

The following specifications are extracted verbatim from the architecture report. Each agent file must implement these contracts exactly.

**Required YAML frontmatter fields** (per Claude Code documentation):
- `name`: Agent identifier (e.g., `clarifier-agent`)
- `description`: When Claude should delegate to this agent (required by Claude Code; use the purpose text from each spec below)
- `tools`: Allowlist of tools the agent can use (enforced by Claude Code)

#### Spec S01: clarifier-agent {#s01-clarifier}

**Tools:** Read, Grep, Glob

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

**Behavior:** Read codebase to understand patterns. Limit to 3-5 questions maximum. If critic feedback present, focus on those issues. If idea is clear, return empty questions array.

---

#### Spec S02: author-agent {#s02-author}

**Tools:** Read, Grep, Glob, Write, Edit

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

**Output JSON:**
```json
{
  "speck_path": ".specks/specks-N.md",
  "created": true,
  "sections_written": ["plan-metadata", "phase-overview", "design-decisions", "execution-steps", "deliverables"],
  "skeleton_compliance": {
    "read_skeleton": true,
    "has_explicit_anchors": true,
    "has_required_sections": true,
    "steps_have_references": true
  },
  "validation_status": "valid | warnings | errors"
}
```

**Behavior:** MUST read `.specks/specks-skeleton.md` before writing. Skeleton compliance is mandatory. Self-validate before returning.

---

#### Spec S03: critic-agent {#s03-critic}

**Tools:** Read, Grep, Glob

**Input JSON:**
```json
{
  "speck_path": "string",
  "skeleton_path": ".specks/specks-skeleton.md"
}
```

**Output JSON:**
```json
{
  "skeleton_compliant": true,
  "skeleton_check": {
    "has_required_sections": true,
    "has_explicit_anchors": true,
    "steps_properly_formatted": true,
    "references_valid": true,
    "decisions_formatted": true,
    "violations": []
  },
  "areas": {
    "completeness": "PASS|WARN|FAIL",
    "implementability": "PASS|WARN|FAIL",
    "sequencing": "PASS|WARN|FAIL"
  },
  "issues": [
    {
      "priority": "P0|HIGH|MEDIUM|LOW",
      "category": "skeleton|completeness|implementability|sequencing",
      "description": "string"
    }
  ],
  "recommendation": "APPROVE|REVISE|REJECT"
}
```

**Behavior:** Skeleton compliance is a HARD GATE. If not compliant, recommendation MUST be REJECT. P0 issues always block approval.

---

#### Spec S04: architect-agent {#s04-architect}

**Tools:** Read, Grep, Glob (read-only)

**Input JSON:**
```json
{
  "speck_path": "string",
  "step_anchor": "string",
  "revision_feedback": "string | null"
}
```

**Output JSON:**
```json
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
```

**Behavior:** Read-only analysis, never writes files. `expected_touch_set` is critical for drift detection. Adjust strategy if `revision_feedback` provided.

---

#### Spec S05: coder-agent {#s05-coder}

**Tools:** Read, Grep, Glob, Write, Edit, Bash

**Input JSON:**
```json
{
  "speck_path": ".specks/specks-N.md",
  "step_anchor": "#step-N",
  "architect_strategy": {
    "approach": "string",
    "expected_touch_set": ["string"],
    "implementation_steps": [{"order": 1, "description": "string", "files": ["string"]}],
    "test_plan": "string"
  },
  "session_id": "20260207-143022-impl-abc123"
}
```

**Output JSON:**
```json
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
    "drift_budget": {
      "yellow_used": 1,
      "yellow_max": 4,
      "red_used": 0,
      "red_max": 2
    },
    "qualitative_assessment": "All changes within expected scope"
  }
}
```

**Drift Detection:** Green = in expected set. Yellow = adjacent (+1 budget). Red = unrelated (+2 budget). Moderate (3-4 yellow OR 1 red) or Major (5+ yellow OR 2+ red) = HALT. `drift_assessment` MUST always be present.

**Test Execution:** Run tests via `cargo nextest run`. Exit code 0 = passed, non-zero = failed. Set `tests_run: true` and `tests_passed` based on result. If test command not applicable to project, set `tests_run: false`.

---

#### Spec S06: reviewer-agent {#s06-reviewer}

**Tools:** Read, Grep, Glob

**Input JSON:**
```json
{
  "speck_path": "string",
  "step_anchor": "string",
  "coder_output": {
    "files_created": ["string"],
    "files_modified": ["string"],
    "tests_passed": true,
    "drift_assessment": { ... }
  }
}
```

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

**Behavior:** APPROVE = all complete, minor or no drift. REVISE = missing tasks coder can fix. ESCALATE = conceptual issues requiring user input.

---

#### Spec S07: auditor-agent {#s07-auditor}

**Tools:** Read, Grep, Glob

**Input JSON:**
```json
{
  "speck_path": "string",
  "step_anchor": "string",
  "files_to_audit": ["string"],
  "drift_assessment": { ... }
}
```

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

**Behavior:** APPROVE = all PASS or minor warns. FIX_REQUIRED = major issues coder can fix. MAJOR_REVISION = critical or design problems.

---

#### Spec S08: logger-agent {#s08-logger}

**Tools:** Read, Grep, Glob, Edit

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

**Behavior:** Prepend entries to `.specks/specks-implementation-log.md`. Runs AFTER reviewer and auditor approve, BEFORE committer. The orchestrator MUST include the log file path in committer's `files_to_stage` so the log entry is committed atomically with the code changes. If the commit fails, the on-disk log entry is uncommitted but harmless — the next successful commit will include it.

---

#### Spec S09: committer-agent {#s09-committer}

**Tools:** Read, Grep, Glob, Bash

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

**Edge Cases:** Manual + not confirmed = stage only. Missing bead_id = error. Bead already closed = warning. Bead not found = HALT. Commit succeeds but bead close fails = HALT.

**Log file inclusion:** The orchestrator adds `.specks/specks-implementation-log.md` to `files_to_stage` so that the logger's entry is committed atomically with the code changes.

---

### 4.0.3 Orchestrator Skill Specifications {#orchestrator-specifications}

#### Spec S10: planner Skill (Orchestrator) {#s10-planner}

**File:** `skills/planner/SKILL.md`

**Frontmatter:**
```yaml
---
name: planner
description: Orchestrates the planning workflow - spawns sub-agents via Task
disable-model-invocation: true
allowed-tools: Task, AskUserQuestion, Read, Grep, Glob, Write, Bash
---
```

**Orchestration Flow:**
1. Parse input (idea text, speck path, or resume flag)
2. Scan for active sessions on same speck (see D06); warn user if found
3. Create session directory: `.specks/runs/<session-id>/planning/`
4. Write `metadata.json` with `status: "in_progress"`
5. Spawn clarifier-agent via Task -> get questions
6. Use AskUserQuestion DIRECTLY if questions exist
7. Spawn author-agent via Task -> get draft speck
8. Spawn critic-agent via Task -> get review
9. If REJECT/REVISE: AskUserQuestion for user decision, loop if needed
10. If APPROVE: update `metadata.json` with `status: "completed"`, return success

**Task Invocation Example:**
```
Task(
  subagent_type: "specks:clarifier-agent",
  prompt: '{"idea": "add user authentication", "speck_path": null, "critic_feedback": null}',
  description: "Analyze idea and generate clarifying questions"
)
```

The prompt parameter contains the JSON input. The Task returns the agent's JSON output as a string. Parse with standard JSON parsing.

**AskUserQuestion Interface:**
```
AskUserQuestion(
  questions: [
    {
      question: "Which authentication method should we use?",
      header: "Auth method",
      options: [
        { label: "JWT tokens (Recommended)", description: "Stateless, scalable" },
        { label: "Session cookies", description: "Traditional, simpler" }
      ],
      multiSelect: false
    }
  ]
)
```

Returns user selection. Use to present clarifier questions, critic issues, drift decisions.

**Error Handling:**
- If Task fails: write raw output to `<session>/error.json`, halt with message (see D08)
- If JSON parse fails: write raw response to error file, halt
- Update `metadata.json` with `status: "failed"` before halting

---

#### Spec S11: implementer Skill (Orchestrator) {#s11-implementer}

**File:** `skills/implementer/SKILL.md`

**Frontmatter:**
```yaml
---
name: implementer
description: Orchestrates the implementation workflow - spawns sub-agents via Task
disable-model-invocation: true
allowed-tools: Task, AskUserQuestion, Read, Grep, Glob, Write, Bash
---
```

**Orchestration Flow (per step):**
1. Parse input (speck path, options, or resume flag)
2. Check beads availability: `specks beads status` (fail fast if unavailable)
3. Scan for active sessions on same speck (see D06); warn user if found
4. Write `metadata.json` with `status: "in_progress"`
5. Create session directory: `.specks/runs/<session-id>/execution/step-N/`
6. Spawn architect-agent via Task -> get strategy
7. Spawn coder-agent via Task -> get implementation + drift
8. If drift exceeds threshold: AskUserQuestion DIRECTLY
9. Spawn reviewer-agent via Task -> get plan check
10. Spawn auditor-agent via Task -> get quality check
11. If issues: re-spawn coder or escalate via AskUserQuestion
12. If clean: spawn logger-agent -> updates log file; then spawn committer-agent with log file included in `files_to_stage`
13. Proceed to next step; update `metadata.json` with `status: "completed"` when all steps complete

**Error Handling:**
- If beads unavailable: halt immediately with "Beads not installed. Run `bd init` first."
- If Task fails: write raw output to `<session>/error.json`, halt with message (see D08)
- If JSON parse fails: write raw response to error file, halt
- Update `metadata.json` with `status: "failed"` before halting

---

### 4.0.4 Test Plan Concepts {#test-plan-concepts}

#### Test Categories {#test-categories}

| Category | Purpose | When to use |
|----------|---------|-------------|
| **Smoke** | Verify agent spawns and returns | After each agent file created |
| **Contract** | Verify JSON input/output matches spec | After each agent tested |
| **Integration** | Verify full loop works | After all agents in a phase |
| **E2E** | Create speck and implement it | Final verification |

#### Verification Checkpoints {#verification-checkpoints}

After each agent conversion:
- [ ] Agent file has valid YAML frontmatter
- [ ] Agent returns JSON to orchestrator skill
- [ ] Orchestrator skill can parse the returned JSON
- [ ] Run directory artifacts are created correctly
- [ ] No context takeover issues

---

### 4.0.5 Execution Steps {#execution-steps}

#### Step 0: Delete Test and Obsolete Files, Verify Prerequisites {#step-0}

**Commit:** `chore: delete test files and obsolete orchestrator agents`

**References:** [D01] Skills are Orchestrators, Table T01 (#t01-obsolete-agents), Table T02 (#t02-skill-dirs), Table T03 (#t03-legacy-dirs)

**Artifacts:**
- Deleted: `agents/planner-agent.md`, `agents/implementer-agent.md`
- Deleted: `agents/test-counter-agent.md`, `agents/test-decider-agent.md`
- Deleted: `skills/test-counter/`, `skills/test-decider/`, `skills/test-loop-orchestrator/`
- ~~Deleted: `.claude/skills/` directory~~ **DEFERRED** (bootstrap: code-architect/code-planner agents needed to run this plan)
- ~~Deleted: `.claude/agents/` directory~~ **DEFERRED** (bootstrap: implement-plan and related skills needed to run this plan)
- Verified: beads is available
- Verified: `.specks/runs/` directory exists with `.gitignore` entry

**Tasks:**
- [x] Verify beads availability: `specks beads status` returns valid JSON (fail fast)
- [x] Create `.specks/runs/` directory if not exists
- [x] Verify `.specks/runs/` is in `.gitignore` (already present)
- [x] Delete `agents/planner-agent.md`
- [x] Delete `agents/implementer-agent.md`
- [x] Delete `agents/test-counter-agent.md`
- [x] Delete `agents/test-decider-agent.md`
- [x] Delete `skills/test-counter/` directory
- [x] Delete `skills/test-decider/` directory
- [x] Delete `skills/test-loop-orchestrator/` directory
- [ ] ~~Delete `.claude/skills/` directory and all contents~~ **DEFERRED to post-Phase 4** (bootstrap: needed to run this plan)
- [ ] ~~Delete `.claude/agents/` directory and all contents~~ **DEFERRED to post-Phase 4** (bootstrap: needed to run this plan)

**Tests:**
- [x] Prerequisite test: `specks beads status` returns JSON without error
- [x] Integration test: `ls agents/` shows only `archived/` directory
- [x] Integration test: `ls skills/` shows no test-* directories

**Checkpoint:**
- [x] `specks beads status` succeeds
- [x] `.specks/runs/` directory exists
- [x] `.gitignore` contains `.specks/runs/`
- [x] `ls agents/*.md` returns no results (only archived/ exists)
- [x] `ls skills/test-*` returns no results
- [ ] ~~`ls .claude/skills/` returns "No such file or directory"~~ **DEFERRED** (bootstrap)
- [ ] ~~`ls .claude/agents/` returns "No such file or directory"~~ **DEFERRED** (bootstrap)

**Rollback:**
- `git checkout HEAD -- agents/ skills/ .claude/ .gitignore`
- `rm -rf .specks/runs/`

**Commit after all checkpoints pass.**

---

#### Step 1: Create Planning Sub-Agents {#step-1}

**Depends on:** #step-0

**Commit:** `feat(agents): add clarifier, author, critic agents for planning loop`

**References:** [D02] Nine Sub-Agents, [D04] JSON Contracts, Spec S01 (#s01-clarifier), Spec S02 (#s02-author), Spec S03 (#s03-critic)

**Artifacts:**
- `agents/clarifier-agent.md` - with full specification from Spec S01
- `agents/author-agent.md` - with full specification from Spec S02
- `agents/critic-agent.md` - with full specification from Spec S03

**Tasks:**
- [x] Create `agents/clarifier-agent.md` with frontmatter: `name: clarifier-agent`, `description: <purpose>`, `tools: Read, Grep, Glob`
- [x] Include input/output JSON schemas from Spec S01
- [x] Include behavior notes (3-5 questions max, critic feedback handling)
- [x] Create `agents/author-agent.md` with frontmatter: `name: author-agent`, `description: <purpose>`, `tools: Read, Grep, Glob, Write, Edit`
- [x] Include input/output JSON schemas from Spec S02
- [x] Include skeleton compliance requirements
- [x] Create `agents/critic-agent.md` with frontmatter: `name: critic-agent`, `description: <purpose>`, `tools: Read, Grep, Glob`
- [x] Include input/output JSON schemas from Spec S03
- [x] Include hard gate requirements (skeleton compliance)

**Tests:**
- [x] Smoke test: Each agent file has valid YAML frontmatter (parseable)
- [x] Contract test: JSON schemas match architecture report exactly

**Checkpoint:**
- [x] `ls agents/*.md` shows clarifier-agent.md, author-agent.md, critic-agent.md
- [x] Each file has `---` delimited frontmatter with name and tools fields
- [x] Task spawn test: Invoke `Task(subagent_type: "specks:clarifier-agent", prompt: '{"idea": "test", "speck_path": null, "critic_feedback": null}', description: "test spawn")` and verify JSON response returns

**Rollback:**
- `rm agents/clarifier-agent.md agents/author-agent.md agents/critic-agent.md`

**Commit after all checkpoints pass.**

---

#### Step 2: Create Implementation Sub-Agents {#step-2}

**Depends on:** #step-1

> This step is large. It creates 6 agent files for the implementation loop.

##### Step 2.1: Create Architect and Coder Agents {#step-2-1}

**Commit:** `feat(agents): add architect and coder agents`

**References:** [D02] Nine Sub-Agents, Spec S04 (#s04-architect), Spec S05 (#s05-coder)

**Artifacts:**
- `agents/architect-agent.md` - with full specification from Spec S04
- `agents/coder-agent.md` - with full specification from Spec S05

**Tasks:**
- [x] Create `agents/architect-agent.md` with frontmatter: `name: architect-agent`, `description: <purpose>`, `tools: Read, Grep, Glob`
- [x] Include input/output JSON schemas from Spec S04
- [x] Include expected_touch_set requirements
- [x] Create `agents/coder-agent.md` with frontmatter: `name: coder-agent`, `description: <purpose>`, `tools: Read, Grep, Glob, Write, Edit, Bash`
- [x] Include input/output JSON schemas from Spec S05
- [x] Include complete drift detection contract (proximity scoring, thresholds, self-halt)

**Tests:**
- [x] Smoke test: Each agent file has valid YAML frontmatter
- [x] Contract test: Coder drift_assessment schema matches spec exactly

**Checkpoint:**
- [x] `ls agents/architect-agent.md agents/coder-agent.md` both exist

**Rollback:**
- `rm agents/architect-agent.md agents/coder-agent.md`

**Commit after all checkpoints pass.**

---

##### Step 2.2: Create Reviewer and Auditor Agents {#step-2-2}

**Commit:** `feat(agents): add reviewer and auditor agents`

**References:** [D02] Nine Sub-Agents, Spec S06 (#s06-reviewer), Spec S07 (#s07-auditor)

**Artifacts:**
- `agents/reviewer-agent.md` - with full specification from Spec S06
- `agents/auditor-agent.md` - with full specification from Spec S07

**Tasks:**
- [x] Create `agents/reviewer-agent.md` with frontmatter: `name: reviewer-agent`, `description: <purpose>`, `tools: Read, Grep, Glob`
- [x] Include input/output JSON schemas from Spec S06
- [x] Include APPROVE/REVISE/ESCALATE decision criteria
- [x] Create `agents/auditor-agent.md` with frontmatter: `name: auditor-agent`, `description: <purpose>`, `tools: Read, Grep, Glob`
- [x] Include input/output JSON schemas from Spec S07
- [x] Include APPROVE/FIX_REQUIRED/MAJOR_REVISION criteria

**Tests:**
- [x] Smoke test: Each agent file has valid YAML frontmatter

**Checkpoint:**
- [x] `ls agents/reviewer-agent.md agents/auditor-agent.md` both exist

**Rollback:**
- `rm agents/reviewer-agent.md agents/auditor-agent.md`

**Commit after all checkpoints pass.**

---

##### Step 2.3: Create Logger and Committer Agents {#step-2-3}

**Commit:** `feat(agents): add logger and committer agents`

**References:** [D02] Nine Sub-Agents, Spec S08 (#s08-logger), Spec S09 (#s09-committer)

**Artifacts:**
- `agents/logger-agent.md` - with full specification from Spec S08
- `agents/committer-agent.md` - with full specification from Spec S09

**Tasks:**
- [x] Create `agents/logger-agent.md` with frontmatter: `name: logger-agent`, `description: <purpose>`, `tools: Read, Grep, Glob, Edit`
- [x] Include input/output JSON schemas from Spec S08
- [x] Include log entry format requirements
- [x] Create `agents/committer-agent.md` with frontmatter: `name: committer-agent`, `description: <purpose>`, `tools: Read, Grep, Glob, Bash`
- [x] Include input/output JSON schemas from Spec S09
- [x] Include edge case handling table

**Tests:**
- [x] Smoke test: Each agent file has valid YAML frontmatter

**Checkpoint:**
- [x] `ls agents/logger-agent.md agents/committer-agent.md` both exist

**Rollback:**
- `rm agents/logger-agent.md agents/committer-agent.md`

**Commit after all checkpoints pass.**

---

#### Step 2 Summary {#step-2-summary}

After completing Steps 2.1-2.3, you will have:
- 6 implementation sub-agent files (architect, coder, reviewer, auditor, logger, committer)
- All agents with proper frontmatter and JSON schemas
- Complete drift detection contract in coder-agent

**Final Step 2 Checkpoint:**
- [x] `ls agents/*-agent.md | wc -l` returns 9 (3 planning + 6 implementation)

---

#### Step 3: Update Planner Skill with Orchestration Logic {#step-3}

**Depends on:** #step-1

**Commit:** `feat(skills): add full orchestration logic to planner skill`

**References:** [D01] Skills are Orchestrators, [D03] No Interviewer, Spec S10 (#s10-planner), (#orchestrator-specifications)

**Artifacts:**
- Modified `skills/planner/SKILL.md` with complete orchestration logic

**Tasks:**
- [ ] Update frontmatter to include all required tools: Task, AskUserQuestion, Read, Grep, Glob, Write, Bash
- [ ] Add `disable-model-invocation: true` to frontmatter
- [ ] Write orchestration flow: conflict detection scan, session creation, clarifier spawn, user questions, author spawn, critic spawn, loop logic
- [ ] Include input format handling (idea string, speck path, resume flag)
- [ ] Include session ID generation logic
- [ ] Include active session conflict detection (see D06)
- [ ] Include `metadata.json` lifecycle (create with `in_progress`, update to `completed` or `failed`)
- [ ] Include run directory structure creation
- [ ] Include JSON persistence pattern using Write tool
- [ ] Include AskUserQuestion usage for user interaction
- [ ] Include loop/retry logic for critic REVISE recommendations

**Tests:**
- [ ] Smoke test: Skill file has valid frontmatter with all tools
- [ ] Contract test: Orchestration flow matches Spec S10

**Checkpoint:**
- [ ] `skills/planner/SKILL.md` contains `allowed-tools: Task, AskUserQuestion, Read, Grep, Glob, Write, Bash`
- [ ] File contains session directory creation logic
- [ ] File contains Task spawning for all three planning agents

**Rollback:**
- `git checkout HEAD -- skills/planner/SKILL.md`

**Commit after all checkpoints pass.**

---

#### Step 4: Update Implementer Skill with Orchestration Logic {#step-4}

**Depends on:** #step-2

**Commit:** `feat(skills): add full orchestration logic to implementer skill`

**References:** [D01] Skills are Orchestrators, Spec S11 (#s11-implementer), (#orchestrator-specifications)

**Artifacts:**
- Modified `skills/implementer/SKILL.md` with complete orchestration logic

**Tasks:**
- [ ] Update frontmatter to include all required tools: Task, AskUserQuestion, Read, Grep, Glob, Write, Bash
- [ ] Add `disable-model-invocation: true` to frontmatter
- [ ] Write orchestration flow for each step: conflict detection scan, architect, coder, drift check, reviewer, auditor, logger, committer
- [ ] Include input format handling (speck path, start/end step, commit policy, resume)
- [ ] Include session ID generation logic
- [ ] Include active session conflict detection (see D06)
- [ ] Include `metadata.json` lifecycle (create with `in_progress`, update to `completed` or `failed`)
- [ ] Include run directory structure creation per step
- [ ] Include drift threshold evaluation and AskUserQuestion for drift decisions
- [ ] Include retry logic for REVISE/FIX_REQUIRED recommendations
- [ ] Include beads integration via `specks beads` commands
- [ ] Include logger/committer coordination: add log file to committer's `files_to_stage`

**Tests:**
- [ ] Smoke test: Skill file has valid frontmatter with all tools
- [ ] Contract test: Orchestration flow matches Spec S11

**Checkpoint:**
- [ ] `skills/implementer/SKILL.md` contains `allowed-tools: Task, AskUserQuestion, Read, Grep, Glob, Write, Bash`
- [ ] File contains per-step directory creation logic
- [ ] File contains Task spawning for all six implementation agents

**Rollback:**
- `git checkout HEAD -- skills/implementer/SKILL.md`

**Commit after all checkpoints pass.**

---

#### Step 5a: Delete Planning Skill Directories {#step-5a}

**Depends on:** #step-3

**Commit:** `chore: delete planning skill directories converted to agents`

**References:** [D03] No Interviewer, Table T02 (#t02-skill-dirs)

**Artifacts:**
- Deleted: `skills/interviewer/` (eliminated - AskUserQuestion used directly)
- Deleted: `skills/clarifier/`, `skills/author/`, `skills/critic/` (converted to agents)

**Tasks:**
- [ ] Delete `skills/interviewer/` directory
- [ ] Delete `skills/clarifier/` directory
- [ ] Delete `skills/author/` directory
- [ ] Delete `skills/critic/` directory

**Checkpoint:**
- [ ] `ls skills/interviewer/` returns "No such file or directory"
- [ ] `ls skills/clarifier/` returns "No such file or directory"
- [ ] `ls skills/author/` returns "No such file or directory"
- [ ] `ls skills/critic/` returns "No such file or directory"

**Rollback:**
- `git checkout HEAD -- skills/interviewer/ skills/clarifier/ skills/author/ skills/critic/`

**Commit after all checkpoints pass.**

---

#### Step 5b: Delete Implementation Skill Directories {#step-5b}

**Depends on:** #step-4

**Commit:** `chore: delete implementation skill directories converted to agents`

**References:** Table T02 (#t02-skill-dirs)

**Artifacts:**
- Deleted: `skills/architect/`, `skills/coder/`, `skills/reviewer/`, `skills/auditor/`, `skills/logger/`, `skills/committer/` (converted to agents)

**Tasks:**
- [ ] Delete `skills/architect/` directory
- [ ] Delete `skills/coder/` directory
- [ ] Delete `skills/reviewer/` directory
- [ ] Delete `skills/auditor/` directory
- [ ] Delete `skills/logger/` directory
- [ ] Delete `skills/committer/` directory

**Tests:**
- [ ] Integration test: Only planner/ and implementer/ skill directories remain

**Checkpoint:**
- [ ] `ls skills/` shows only `planner/` and `implementer/`

**Rollback:**
- `git checkout HEAD -- skills/architect/ skills/coder/ skills/reviewer/ skills/auditor/ skills/logger/ skills/committer/`

**Commit after all checkpoints pass.**

---

#### Step 6: Verification Gate - Planning Loop {#step-6}

**Depends on:** #step-3, #step-5a

> **This is a verification gate, not a commit step.** No commit is produced. The test artifacts are ephemeral and cleaned up after verification. This step must pass before Step 8 can proceed.

**References:** [D05] Incremental Testing, Spec S10 (#s10-planner), (#verification-checkpoints)

**Artifacts (ephemeral):**
- Test speck created via `/specks:planner`
- Run directory with planning artifacts

**Tasks:**
- [ ] Invoke `/specks:planner "add a hello world command"`
- [ ] Verify clarifier-agent spawns and returns questions JSON
- [ ] Answer clarifying questions via AskUserQuestion
- [ ] Verify author-agent spawns and creates speck file
- [ ] Verify critic-agent spawns and returns review JSON
- [ ] Verify run directory contains: `001-clarifier.json`, `002-user-answers.json`, `003-author.json`, `004-critic.json`
- [ ] Verify no context takeover issues (planner skill maintains control)

**Tests:**
- [ ] E2E test: Full planning loop completes
- [ ] Contract test: All JSON artifacts match schemas

**Checkpoint:**
- [ ] A new speck file exists in `.specks/`
- [ ] Run directory exists at `.specks/runs/<session-id>/planning/`
- [ ] All numbered JSON files exist in run directory

**Cleanup after verification:**
- Keep test speck for use in Step 7
- Delete run directory: `rm -rf .specks/runs/<session-id>`

---

#### Step 7: Verification Gate - Implementation Loop {#step-7}

**Depends on:** #step-4, #step-5b, #step-6

> **This is a verification gate, not a commit step.** The implementer itself produces a commit (via committer-agent) as part of its normal operation — that commit is the implementation artifact, not a step-7-specific commit. This step validates that the full implementation loop works correctly.

**References:** [D05] Incremental Testing, Spec S11 (#s11-implementer), (#verification-checkpoints)

**Artifacts (produced by implementer):**
- Completed step from test speck (committed by committer-agent)
- Run directory with execution artifacts
- Closed bead

**Tasks:**
- [ ] Use the test speck from Step 6 (or create a simple single-step test speck)
- [ ] Invoke `/specks:implementer <speck-path>`
- [ ] Verify architect-agent spawns and returns strategy JSON
- [ ] Verify coder-agent spawns and returns implementation + drift JSON
- [ ] Verify reviewer-agent spawns and returns review JSON
- [ ] Verify auditor-agent spawns and returns audit JSON
- [ ] Verify logger-agent spawns and updates implementation log
- [ ] Verify committer-agent spawns, creates commit with log file included, and closes bead
- [ ] Verify run directory contains all step artifacts

**Tests:**
- [ ] E2E test: Full implementation step completes
- [ ] Contract test: All JSON artifacts match schemas
- [ ] Integration test: Bead is closed after step

**Checkpoint:**
- [ ] Run directory exists at `.specks/runs/<session-id>/execution/step-0/`
- [ ] All agent output files exist (architect.json, coder.json, reviewer.json, auditor.json, logger.json, committer.json)
- [ ] Git commit exists with proper message and includes implementation log
- [ ] Bead is closed (verify via `specks beads status`)

**Cleanup after verification:**
- Delete test speck: `rm .specks/specks-hello-world.md` (or whatever was created)
- `git revert HEAD` to undo the test implementation commit
- Delete run directories from Steps 6 and 7: `rm -rf .specks/runs/<session-id>`

---

#### Step 8: Update CLAUDE.md Documentation {#step-8}

**Depends on:** #step-5a, #step-5b, #step-6, #step-7

**Commit:** `docs: update CLAUDE.md for correct Task-based architecture`

**References:** [D01] Skills are Orchestrators, [D02] Nine Sub-Agents, (#design-decisions)

**Artifacts:**
- Updated `CLAUDE.md` with correct architecture description

**Tasks:**
- [ ] Update "Agent and Skill Architecture" section header and intro
- [ ] Remove "Agents (2)" section referencing planner-agent and implementer-agent
- [ ] Update "Skills (12)" section to "Orchestrator Skills (2)" with only planner and implementer
- [ ] Add new "Sub-Agents (9)" section listing all 9 agents with their roles
- [ ] Remove "Entry wrappers (2)" subsection (planner/implementer are now full orchestrators)
- [ ] Remove "Sub-tasks (10)" subsection (these are now agents, not skills)
- [ ] Update any text that says "orchestrator agents" to "orchestrator skills"
- [ ] Update any text referencing 12 skills to reflect correct count
- [ ] Verify grep for "orchestrator agent" returns no matches

**Tests:**
- [ ] Documentation test: `grep -i "orchestrator agent" CLAUDE.md` returns no matches
- [ ] Documentation test: `grep -i "planner-agent" CLAUDE.md` returns no matches (as file reference)
- [ ] Documentation test: `grep -i "implementer-agent" CLAUDE.md` returns no matches (as file reference)

**Checkpoint:**
- [ ] `CLAUDE.md` accurately describes 2 orchestrator skills + 9 sub-agents architecture
- [ ] No mention of "orchestrator agents" or "orchestrator agent"
- [ ] No mention of planner-agent.md or implementer-agent.md as files

**Rollback:**
- `git checkout HEAD -- CLAUDE.md`

**Commit after all checkpoints pass.**

---

### 4.0.6 Deliverables and Checkpoints {#deliverables}

**Deliverable:** Specks plugin with correct Task-based orchestration: 2 orchestrator skills spawning 9 sub-agents via Task tool.

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

- [ ] Beads verified available (`specks beads status` succeeds)
- [ ] `.specks/runs/` directory exists and is in `.gitignore`
- [ ] 9 agent files exist in `agents/` with valid YAML frontmatter (verify: `ls agents/*-agent.md | wc -l` = 9)
- [ ] 2 orchestrator skill files exist with full logic (verify: planner and implementer contain Task spawning code)
- [ ] 0 obsolete files remain (verify: no planner-agent.md, implementer-agent.md, test files, converted skill dirs, .claude/agents/, .claude/skills/)
- [ ] `/specks:planner` creates a speck through the full loop (verify: run directory artifacts)
- [ ] `/specks:implementer` completes a step through the full loop (verify: commit and bead closure)
- [ ] CLAUDE.md accurately documents the architecture (no "orchestrator agent" references)

**Acceptance tests:**
- [ ] E2E test: `/specks:planner "add hello command"` -> speck created
- [ ] E2E test: `/specks:implementer .specks/specks-test.md` -> step completed, bead closed

#### Milestones (Within Phase) {#milestones}

**Milestone M01: Cleanup and Prerequisites Complete** {#m01-cleanup}
- [ ] All obsolete and test files deleted (Step 0)
- [ ] Beads verified available
- [ ] `.specks/runs/` infrastructure created

**Milestone M02: All Agents Created** {#m02-agents-created}
- [ ] 9 agent files exist with specifications (Steps 1-2)
- [ ] Task spawn test passed (clarifier-agent returns JSON)

**Milestone M03: Orchestrators Updated** {#m03-orchestrators-updated}
- [ ] Both planner and implementer skills have full logic (Steps 3-4)

**Milestone M04: Old Skills Deleted** {#m04-skills-deleted}
- [ ] Planning skill directories deleted (Step 5a)
- [ ] Implementation skill directories deleted (Step 5b)
- [ ] Only planner/ and implementer/ remain

**Milestone M05: Integration Verified** {#m05-integration-verified}
- [ ] Planning loop works end-to-end (Step 6)
- [ ] Implementation loop works end-to-end (Step 7)

#### Roadmap / Follow-ons (Explicitly Not Required for Phase Close) {#roadmap}

- [ ] `specks runs gc --older-than 30d` command for run directory cleanup (see D07)
- [ ] Performance optimization of agent spawning
- [ ] Resume functionality testing
- [ ] Multi-step speck execution testing
- [ ] Error recovery and edge case handling

| Checkpoint | Verification |
|------------|--------------|
| Beads available | `specks beads status` succeeds |
| Runs infrastructure | `.specks/runs/` exists, in `.gitignore` |
| All agents exist | `ls agents/*-agent.md \| wc -l` = 9 |
| No obsolete files | `ls agents/planner-agent.md` returns error |
| No legacy dirs | `ls .claude/agents/` and `ls .claude/skills/` return error |
| Skills have logic | `grep "Task" skills/*/SKILL.md` returns matches |
| Planning works | `/specks:planner` completes without error |
| Implementation works | `/specks:implementer` completes step |
| Docs accurate | `grep -i "orchestrator agent" CLAUDE.md` returns nothing |

**Commit after all checkpoints pass.**
