# Architecture Report for specks-4.md

**Purpose:** This report extracts and organizes the key specifications from specks-3.md to guide the creation of specks-4.md, which will define the correct Task-based orchestration model.

**Date:** 2026-02-07

---

## Executive Summary

**THE CRITICAL INSIGHT:**

| Mechanism | Behavior |
|-----------|----------|
| **Skill + Skill tool** | Prompt injection → takes over context → **NO RETURN** |
| **Agent + Task tool** | Spawns subagent → runs to completion → **RETURNS RESULT** |
| **Subagent spawning** | **Subagents CANNOT spawn other subagents** |

**THE CORRECT ARCHITECTURE:**

```
┌─────────────────────────────────────────────────────────────────┐
│                     ORCHESTRATOR SKILLS                          │
│         (run in main context, USE Task to spawn agents)          │
├─────────────────────────────────────────────────────────────────┤
│  /specks:planner          │  /specks:implementer                 │
│  - IS the orchestrator    │  - IS the orchestrator               │
│  - Uses Task tool         │  - Uses Task tool                    │
│  - Uses AskUserQuestion   │  - Uses AskUserQuestion              │
│  - Runs planning loop     │  - Runs implementation loop          │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ Task(subagent_type: "specks:X-agent")
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                        SUB-AGENTS                                │
│            (spawned via Task, run, RETURN result)                │
├─────────────────────────────────────────────────────────────────┤
│  Planning:                 │  Implementation:                    │
│  - clarifier-agent         │  - architect-agent                  │
│  - author-agent            │  - coder-agent                      │
│  - critic-agent            │  - reviewer-agent                   │
│                            │  - auditor-agent                    │
│                            │  - logger-agent                     │
│                            │  - committer-agent                  │
└─────────────────────────────────────────────────────────────────┘
```

**WHAT THIS MEANS:**

1. **planner** and **implementer** are SKILLS that contain the orchestration logic
2. They run in the MAIN CONTEXT (not as subagents)
3. They use Task tool to spawn sub-agents
4. Sub-agents run, complete, and RETURN results to the skill
5. The skill continues processing after each Task returns
6. **There are NO "orchestrator agents"** - the skills ARE the orchestrators

**OBSOLETE FILES TO DELETE:**
- `agents/planner-agent.md` - orchestration moves to the skill
- `agents/implementer-agent.md` - orchestration moves to the skill

**FINAL COUNT:**
- 2 orchestrator skills (planner, implementer)
- 9 sub-agents (clarifier, author, critic, architect, coder, reviewer, auditor, logger, committer)
- 0 orchestrator agents

---

## 1. Orchestrator Skill Specifications

The planner and implementer are SKILLS that contain the full orchestration logic.

### 1.1 planner Skill (Orchestrator)

**File:** `skills/planner/SKILL.md`

**Type:** Skill (runs in main context)

**Frontmatter:**
```yaml
---
name: planner
description: Orchestrates the planning workflow - spawns sub-agents via Task
disable-model-invocation: true
allowed-tools: Task, AskUserQuestion, Read, Grep, Glob, Write, Bash
---
```

**What it does:**
1. Receives idea or speck path as input
2. Creates session directory and metadata
3. Spawns clarifier-agent via Task → gets questions
4. Uses AskUserQuestion to get user answers (NO separate interviewer)
5. Spawns author-agent via Task → gets draft speck
6. Spawns critic-agent via Task → gets review
7. If issues: AskUserQuestion for user decision, loop back if needed
8. Persists all outputs to run directory
9. Returns final result

**Input formats:**
- String: `"add user authentication"` (idea)
- String: `.specks/specks-auth.md` (existing speck to revise)
- Flags: `--resume 20260206-143022-plan-a1b2c3`

### 1.2 implementer Skill (Orchestrator)

**File:** `skills/implementer/SKILL.md`

**Type:** Skill (runs in main context)

**Frontmatter:**
```yaml
---
name: implementer
description: Orchestrates the implementation workflow - spawns sub-agents via Task
disable-model-invocation: true
allowed-tools: Task, AskUserQuestion, Read, Grep, Glob, Write, Bash
---
```

**What it does:**
1. Receives speck path as input
2. Creates session directory and metadata
3. For each step in speck:
   a. Spawns architect-agent via Task → gets strategy
   b. Spawns coder-agent via Task → gets implementation + drift assessment
   c. If drift: AskUserQuestion for user decision
   d. Spawns reviewer-agent via Task → gets plan adherence check
   e. Spawns auditor-agent via Task → gets quality check
   f. If issues: handle based on severity
   g. Spawns logger-agent via Task → updates log
   h. Spawns committer-agent via Task → commits and closes bead
4. Persists all outputs to run directory
5. Returns final result

**Input formats:**
- String: `.specks/specks-3.md` (speck to implement)
- Flags: `--start-step #step-2`, `--end-step #step-4`, `--commit-policy auto|manual`, `--resume <session_id>`

---

## 2. Sub-Agent Specifications

These are AGENTS spawned via Task tool. They run to completion and RETURN results.

### 2.1 Planning Sub-Agents

#### clarifier-agent

**File:** `agents/clarifier-agent.md`

**Purpose:** Analyze an idea or critic feedback to generate clarifying questions.

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

**Special Behavior:**
- Reads codebase to understand existing patterns
- Limits to 3-5 questions maximum
- If critic feedback present, focuses questions on addressing those issues
- If idea is clear and unambiguous, returns empty questions array

---

#### author-agent

**File:** `agents/author-agent.md`

**Purpose:** Create and revise speck documents following skeleton format.

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

**Special Behavior:**
- MUST read `.specks/specks-skeleton.md` before writing any content
- Skeleton compliance is mandatory (P0)
- Self-validates against skeleton before returning
- Writes to `.specks/specks-{name}.md`

---

#### critic-agent

**File:** `agents/critic-agent.md`

**Purpose:** Review a speck for skeleton compliance, quality, and implementability.

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

**Special Behavior:**
- Skeleton compliance is a HARD GATE
- If `skeleton_compliant == false`, recommendation MUST be REJECT
- P0 issues always block approval
- Must verify all sections, anchors, step formats, and references

---

### 2.2 Implementation Sub-Agents

#### architect-agent

**File:** `agents/architect-agent.md`

**Purpose:** Create implementation strategies for speck steps.

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

**Special Behavior:**
- Read-only analysis, never writes files
- `expected_touch_set` is critical for drift detection by coder
- If `revision_feedback` provided, adjusts strategy accordingly

---

#### coder-agent

**File:** `agents/coder-agent.md`

**Purpose:** Execute architect strategies with drift detection.

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

**Drift Detection Contract:**

Proximity Scoring:
| Category | Description | Budget Impact |
|----------|-------------|---------------|
| Green | File in expected_touch_set | No impact |
| Yellow | Adjacent directory (sibling, parent, child) | +1 to budget |
| Red | Unrelated subsystem | +2 to budget |

File Type Modifiers:
- Test files (`*_test.rs`, `tests/`) -> +2 leeway
- Config files (`Cargo.toml`, `*.toml`) -> +1 leeway
- Documentation (`*.md`) -> +1 leeway
- Core logic in unexpected areas -> no leeway

Thresholds:
| Severity | Condition | Action |
|----------|-----------|--------|
| none | All files in expected set | Continue |
| minor | 1-2 yellow touches | Continue (note in output) |
| moderate | 3-4 yellow OR 1 red | HALT |
| major | 5+ yellow OR 2+ red | HALT |

Self-Halt Behavior:
1. Stop further implementation work immediately
2. Return with `success: false` and `halted_for_drift: true`
3. Include full drift_assessment for orchestrator

**MANDATORY:** `drift_assessment` must ALWAYS be present, even when `drift_severity: "none"`.

---

#### reviewer-agent

**File:** `agents/reviewer-agent.md`

**Purpose:** Verify a completed step matches the plan specification.

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

**Special Behavior:**
- APPROVE: All tasks complete, artifacts exist, drift is none or minor
- REVISE: Missing tasks or artifacts that coder can fix
- ESCALATE: Conceptual issues requiring user input or plan changes
- `drift_notes` flags minor drift (1-2 yellow touches) for visibility

---

#### auditor-agent

**File:** `agents/auditor-agent.md`

**Purpose:** Check code quality, performance, and security of recent changes.

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

**Special Behavior:**
- APPROVE: All categories PASS, or minor WARNs with no critical/major issues
- FIX_REQUIRED: Major issues that coder can fix quickly
- MAJOR_REVISION: Critical issues or fundamental design problems
- `drift_notes` raises concerns about unexpected changes if drift occurred

---

#### logger-agent

**File:** `agents/logger-agent.md`

**Purpose:** Update the implementation log with completed work.

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

**Special Behavior:**
- Prepends entries to `.specks/specks-implementation-log.md`
- Entry format: `## [speck-file.md] Step N: Title | COMPLETE | YYYY-MM-DD`
- Runs AFTER reviewer and auditor approve
- Runs BEFORE committer

---

#### committer-agent

**File:** `agents/committer-agent.md`

**Purpose:** Finalize a completed step: stage files, commit changes, close bead.

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

**Edge Case Handling:**
| Scenario | Behavior | Output |
|----------|----------|--------|
| `commit_policy: manual` and `confirmed: false` | Stage only; do not commit or close bead | `committed: false, bead_closed: false` |
| `commit_policy: manual` and user rejects | Abort step | `committed: false, aborted: true` |
| `bead_id` missing/null | Treat as error; do not commit | Return JSON error |
| Bead already closed | Commit proceeds, report warning | `bead_closed: true, warnings: ["Bead already closed"]` |
| Bead ID not found | HALT immediately | `committed: false, aborted: true` |
| Commit succeeds, bead close fails | HALT immediately | `committed: true, bead_closed: false, aborted: true` |

**Principle:** Beads are required. If bead close fails, return `aborted: true`.

---

## 3. Orchestration Flows

### 3.1 Planning Phase Flow

```
User invokes /specks:planner "idea" or /specks:planner path/to/speck.md

  +---------+
  |  INPUT  |  idea text OR existing speck path
  +----+----+
       |
       v
  +---------------------------------------------------------------+
  | PLANNER SKILL runs in main context                            |
  | (This IS the orchestrator - NOT a separate agent)             |
  |                                                               |
  | 1. Create session directory, generate session ID              |
  |    mkdir -p .specks/runs/<session-id>/planning                |
  |                                                               |
  | 2. Spawn CLARIFIER-AGENT via Task                             |
  |    Task(subagent_type: "specks:clarifier-agent",              |
  |         prompt: <input-json>)                                 |
  |    -> Agent runs, returns: analysis{}, questions[],           |
  |                            assumptions[]                      |
  |    -> Skill persists output to 001-clarifier.json             |
  |                                                               |
  | 3. IF questions exist:                                        |
  |    -> Use AskUserQuestion DIRECTLY (not via agent)            |
  |    -> Skill persists answers to 002-user-answers.json         |
  |                                                               |
  | 4. Spawn AUTHOR-AGENT via Task with:                          |
  |    - Original idea/speck                                      |
  |    - User answers (if any)                                    |
  |    - Clarifier assumptions                                    |
  |    -> Agent runs, writes speck, returns: speck_path           |
  |    -> Skill persists output to 003-author.json                |
  |                                                               |
  | 5. Spawn CRITIC-AGENT via Task with draft speck               |
  |    -> Agent runs, returns: skeleton_compliant, areas{},       |
  |                            issues[], recommendation           |
  |    -> Skill persists output to 004-critic.json                |
  |                                                               |
  | 6. IF recommendation == REJECT or REVISE:                     |
  |    -> Use AskUserQuestion DIRECTLY with critic issues         |
  |    -> Present issues, get user decision:                      |
  |       revise? accept anyway? abort?                           |
  |                                                               |
  |    IF user says revise:                                       |
  |    -> Go back to step 4 with critic feedback                  |
  |                                                               |
  | 7. IF recommendation == APPROVE (or user accepts):            |
  |    -> Update metadata.json with completed status              |
  |    -> Planning complete, ready for execution                  |
  +---------------------------------------------------------------+

  OUTPUT: Approved speck at .specks/specks-{name}.md
```

**Key Points:**
- PLANNER SKILL is the orchestrator (NOT planner-agent)
- Skill runs in main context, spawns sub-agents via Task
- User interaction via AskUserQuestion DIRECTLY in the skill
- No agent nesting (skill → agent, never agent → agent)
- Loop continues until critic approves OR user accepts

---

### 3.2 Implementation Phase Flow

```
User invokes /specks:implementer path/to/speck.md

  +---------------------------------------------------------------+
  | IMPLEMENTER SKILL runs in main context                        |
  | (This IS the orchestrator - NOT a separate agent)             |
  +---------------------------------------------------------------+
       |
       v
  +---------------------------------------------------------------+
  | STEP 1: Get Implementation Strategy                           |
  |                                                               |
  | Spawn ARCHITECT-AGENT via Task with step details              |
  | -> Agent runs, returns: strategy, expected_touch_set[],       |
  |                         test_plan                             |
  | -> Skill persists to execution/step-N/architect.json          |
  +---------------------------------------------------------------+
       |
       v
  +---------------------------------------------------------------+
  | STEP 2: Implementation (with Self-Monitoring)                 |
  |                                                               |
  | Spawn CODER-AGENT via Task with architect strategy            |
  | -> Agent reads strategy, writes code, runs tests              |
  | -> Agent self-monitors against expected_touch_set             |
  | -> Agent returns: success/failure + drift_assessment          |
  | -> Skill persists to execution/step-N/coder.json              |
  |                                                               |
  | Skill performs additional drift gate:                         |
  | - Compare coder.files_* to architect.expected_touch_set       |
  | - If drift exceeds thresholds:                                |
  |   -> AskUserQuestion DIRECTLY with drift details              |
  |   -> User decides: continue anyway? back to architect? abort? |
  |   -> Skill acts on user decision                              |
  +---------------------------------------------------------------+
       |
       v
  +---------------------------------------------------------------+
  | STEP 3: Review + Audit                                        |
  |                                                               |
  | SEQUENTIAL (one at a time, NOT parallel):                     |
  |                                                               |
  | Spawn REVIEWER-AGENT via Task                                 |
  | -> Checks plan adherence, tasks completed, tests match plan   |
  | -> Returns: APPROVE|REVISE|ESCALATE                           |
  | -> Skill persists to execution/step-N/reviewer.json           |
  |                                                               |
  | Spawn AUDITOR-AGENT via Task                                  |
  | -> Checks code quality, security, performance                 |
  | -> Returns: APPROVE|FIX_REQUIRED|MAJOR_REVISION               |
  | -> Skill persists to execution/step-N/auditor.json            |
  |                                                               |
  | Skill evaluates both reports                                  |
  +---------------------------------------------------------------+
       |
       v
  +---------------------------------------------------------------+
  | STEP 4: Resolution                                            |
  |                                                               |
  | IF issues found:                                              |
  |   -> Minor quality issues: Re-spawn CODER-AGENT               |
  |   -> Design issues: Back to ARCHITECT-AGENT                   |
  |   -> Conceptual issues: AskUserQuestion DIRECTLY              |
  |                                                               |
  | IF both reports clean:                                        |
  |   1. Spawn LOGGER-AGENT -> Updates implementation log         |
  |      -> Skill persists to execution/step-N/logger.json        |
  |   2. Spawn COMMITTER-AGENT -> Commits changes, closes bead    |
  |      -> Skill persists to execution/step-N/committer.json     |
  |   3. Mark step complete in metadata.json                      |
  |   4. Proceed to next step                                     |
  +---------------------------------------------------------------+
       |
       v
  NEXT STEP (loop back to STEP 1)

  WHEN all steps complete:
  -> Update metadata.json with completed status
  -> Report success to user
```

**Key Points:**
- IMPLEMENTER SKILL is the orchestrator (NOT implementer-agent)
- Skill runs in main context, spawns sub-agents via Task
- One sub-agent at a time (sequential invocation)
- ALL user interaction via AskUserQuestion DIRECTLY in the skill
- Coder includes self-monitoring for drift detection
- Logger and committer are spawned after each successful step

---

### 3.3 Tool Invocation Summary

**Orchestrator skills (run in main context):**

| Component | Type | User Invocation | Tools |
|-----------|------|-----------------|-------|
| planner | Skill | `/specks:planner <args>` | Task, AskUserQuestion, Read, Grep, Glob, Write, Bash |
| implementer | Skill | `/specks:implementer <args>` | Task, AskUserQuestion, Read, Grep, Glob, Write, Bash |

**Sub-agents (spawned by orchestrator skills via Task):**

| Component | Invocation | Tools |
|-----------|------------|-------|
| clarifier-agent | `Task(subagent_type: "specks:clarifier-agent")` | Read, Grep, Glob |
| author-agent | `Task(subagent_type: "specks:author-agent")` | Read, Grep, Glob, Write, Edit |
| critic-agent | `Task(subagent_type: "specks:critic-agent")` | Read, Grep, Glob |
| architect-agent | `Task(subagent_type: "specks:architect-agent")` | Read, Grep, Glob |
| coder-agent | `Task(subagent_type: "specks:coder-agent")` | Read, Grep, Glob, Write, Edit, Bash |
| reviewer-agent | `Task(subagent_type: "specks:reviewer-agent")` | Read, Grep, Glob |
| auditor-agent | `Task(subagent_type: "specks:auditor-agent")` | Read, Grep, Glob |
| logger-agent | `Task(subagent_type: "specks:logger-agent")` | Read, Grep, Glob, Edit |
| committer-agent | `Task(subagent_type: "specks:committer-agent")` | Read, Grep, Glob, Bash |

**User interaction:** Orchestrator skills call `AskUserQuestion` DIRECTLY (not via agent).

---

## 4. Contracts and Schemas

### 4.1 Beads Integration Contract

**Invocation method:** Agents call `specks beads ... --json` via Bash, NOT `bd` directly.

```bash
# Correct
specks beads status specks-3.md --json

# Wrong - bypasses config
bd show bd-123 --json
```

**Discovery chain (implemented in CLI):**
1. `SPECKS_BD_PATH` environment variable (highest priority)
2. `config.specks.beads.bd_path` from `.specks/config.toml`
3. Default `"bd"` (expects `bd` on PATH)

**JSON envelope schema:**
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
  "file": ".specks/specks-3.md",
  "line": 42,
  "anchor": "#step-0"
}
```

**Beads Commands:**
- `specks beads status [file] --json` - Returns completion status
- `specks beads sync <file> --json` - Creates/updates beads
- `specks beads pull [file] --json` - Updates checkboxes from beads
- `specks beads link <file> <step-anchor> <bead-id>` - Links bead to step
- `specks beads close <bead-id> [--reason <reason>] --json` - Closes a bead

**Error Handling:**
- `E005` / `BeadsNotInstalled` - `bd` binary not found
- `E009` / `NotInitialized` - `.specks` directory not found
- `E013` / `BeadsNotInitialized` - `.beads` directory not found
- `E016` / `BeadsCommand(msg)` - command failed with stderr

**Agent error recovery:**
1. If `specks beads` fails with "not installed", skill informs user via AskUserQuestion
2. Do not retry beads operations without user intervention
3. Beads is required: halt until beads is ready
4. Treat non-JSON or invalid JSON output as an error and halt

---

### 4.2 Run Directory Structure

```
.specks/runs/<session-id>/
├── metadata.json              # Session info, start time, mode, speck path
├── planning/                  # Planning phase artifacts
│   ├── 001-clarifier.json     # Clarifying questions generated
│   ├── 002-user-answers.json  # User answers received
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

---

### 4.3 Session ID Format

Format: `YYYYMMDD-HHMMSS-<mode>-<short-uuid>`

Examples:
- `20260206-143022-plan-a1b2c3`
- `20260206-150145-impl-d4e5f6`

**Generation method:**
```bash
SHORT_UUID="$(
  uuidgen 2>/dev/null \
    | tr -d '-' \
    | tr '[:upper:]' '[:lower:]' \
    | cut -c1-6 \
  || hexdump -n 3 -e '3/1 "%02x"' /dev/urandom
)"
SESSION_ID="$(date +%Y%m%d-%H%M%S)-${MODE}-${SHORT_UUID}"
```

---

### 4.4 Metadata Schema

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

---

### 4.5 JSON Persistence Pattern

Orchestrator skills write JSON to the runs directory using the **Write tool** (not Bash).

```
Write(file_path: ".specks/runs/20260206-143022-plan-a1b2c3/metadata.json", content: <json-string>)
```

**Why Write, not Bash:**
- Write tool handles content exactly as provided - no escaping needed
- More reliable than heredocs or echo for JSON with special characters
- Bash reserved for: `mkdir -p` (directory creation), session ID generation

---

### 4.6 State Reconstruction

Skills are stateless prompts. Orchestrator skills must reconstruct state by reading from the run directory.

**Reconstruction mechanism:**
1. **Session ID discovery:** Check if session ID was passed as input (resume case). If not, generate a new one.
2. **Counter determination:** List existing files in relevant directory and compute next counter.
3. **Previous output retrieval:** Read relevant JSON files when building context for subsequent sub-agents.
4. **Current step tracking:** Read `metadata.json` to determine `current_step`. Update after each step completes.

---

### 4.7 Resume Logic

**Planner resume:**
```
/specks:planner --resume 20260207-143022-plan-abc123
```

1. Read `metadata.json` from `.specks/runs/<session-id>/`
2. Check `status` field:
   - If `completed`: Report already done, exit
   - If `in_progress`: Continue from where interrupted
3. List files in `planning/` to determine last completed sub-agent
4. Determine next sub-agent based on planning flow
5. Continue loop from that point

**Implementer resume:**
```
/specks:implementer --resume 20260207-150145-impl-def456
```

1. Read `metadata.json` to get `current_step`
2. For current step, check which sub-agent outputs exist in `execution/step-N/`:
   - If `committer.json` exists AND reports `committed: true` AND `bead_closed: true`: step complete, move to next
   - If `coder.json` exists but no `reviewer.json`: resume at reviewer
   - etc.
3. Continue loop from determined point

**Failure modes:**
- Session directory doesn't exist: Report error, suggest starting fresh
- Session marked `failed`: Report what failed, ask whether to retry or abort
- Corrupted state: Report error, suggest starting fresh. Do not attempt recovery.
- Out-of-order artifacts: Report error, suggest starting fresh. Do not attempt recovery.

---

## 5. Files to Clean Up

### 5.1 Agent Files to DELETE (Obsolete Orchestrator Agents)

| File | Reason |
|------|--------|
| `agents/planner-agent.md` | **OBSOLETE** - orchestration moves to planner SKILL |
| `agents/implementer-agent.md` | **OBSOLETE** - orchestration moves to implementer SKILL |

### 5.2 Skill Files to Convert to Agents

| Current Skill | New Agent |
|---------------|-----------|
| `skills/clarifier/SKILL.md` | `agents/clarifier-agent.md` |
| `skills/author/SKILL.md` | `agents/author-agent.md` |
| `skills/critic/SKILL.md` | `agents/critic-agent.md` |
| `skills/architect/SKILL.md` | `agents/architect-agent.md` |
| `skills/coder/SKILL.md` | `agents/coder-agent.md` |
| `skills/reviewer/SKILL.md` | `agents/reviewer-agent.md` |
| `skills/auditor/SKILL.md` | `agents/auditor-agent.md` |
| `skills/logger/SKILL.md` | `agents/logger-agent.md` |
| `skills/committer/SKILL.md` | `agents/committer-agent.md` |

### 5.3 Skill Files to Delete

| Skill | Reason |
|-------|--------|
| `skills/interviewer/SKILL.md` | **ELIMINATED** - orchestrator skills use AskUserQuestion directly |

### 5.4 Skills to Keep (Orchestrators)

| Skill | Purpose |
|-------|---------|
| `skills/planner/SKILL.md` | **IS the orchestrator** - contains planning loop logic |
| `skills/implementer/SKILL.md` | **IS the orchestrator** - contains implementation loop logic |

### 5.5 Archived Agent Files (Keep in Archive)

These are already in `agents/archived/` and should remain there:
- `agents/archived/director.md`
- `agents/archived/interviewer.md`

### 5.6 Legacy Skill Directories to Delete

```
.claude/skills/implement-plan/
.claude/skills/prepare-git-commit-message/
.claude/skills/update-plan-implementation-log/
```

Then delete `.claude/skills/` directory entirely.

### 5.7 Test Files to Delete

These were created during experimentation:
- `agents/test-counter-agent.md`
- `agents/test-decider-agent.md`
- `skills/test-counter/`
- `skills/test-decider/`
- `skills/test-loop-orchestrator/`

---

## 6. What's Obsolete in specks-3.md

### 6.1 Completely Wrong Architecture

The entire specks-3.md architecture is based on:
- Entry skills → spawn orchestrator agents → orchestrator agents call skills via Skill tool

This is **WRONG** because:
1. Skills invoked via Skill tool do not return - they take over context
2. Even if they did, subagents cannot spawn other subagents

### 6.2 Obsolete Sections

- **D02 (Director is pure orchestrator)**: Director eliminated entirely
- **D07 (Skill invocation via Skill tool)**: Sub-tasks use Task, not Skill
- **D08 (Dual-orchestrator architecture)**: Referenced "orchestrator agents" which don't exist
- **Section 3.0.2.1 Escalation Guidelines**: Entire section obsolete
- **All flowcharts showing Skill tool invocation**: Must be rewritten for Task

### 6.3 Confused Terminology

The following terms in specks-3.md are misleading:
- "planner-agent" - This should not exist; planner is a skill
- "implementer-agent" - This should not exist; implementer is a skill
- "orchestrator agent" - There are no orchestrator agents; skills are the orchestrators

---

## 7. Incremental Testing Strategy

### 7.1 Phase 1: Already Complete ✓

We already verified that:
- Task tool returns results to the calling skill
- Skills can use AskUserQuestion directly
- The orchestration loop works with Task-based sub-agents

Test evidence: `/specks:test-loop-orchestrator` worked correctly.

### 7.2 Phase 2: Convert One Planning Sub-Agent

**Order:**
1. **clarifier-agent** - Simplest, read-only, returns structured questions

**Test:**
```
/specks:planner "add a hello world command"
```

**Verify:**
- [ ] clarifier-agent spawns via Task
- [ ] clarifier-agent returns JSON to planner skill
- [ ] planner skill continues and presents questions via AskUserQuestion

### 7.3 Phase 3: Complete Planning Loop

**Order:**
1. **author-agent** - Creates specks, uses Write tool
2. **critic-agent** - Reviews specks, read-only

**Test:** Full planning loop with all three agents

**Verify:**
- [ ] Each agent returns JSON to planner skill
- [ ] Planner skill loops if critic says REVISE
- [ ] Planner skill uses AskUserQuestion for user decisions
- [ ] Run directory artifacts created correctly

### 7.4 Phase 4: Convert One Implementation Sub-Agent

**Order:**
1. **architect-agent** - Read-only, returns strategy

**Test:**
```
/specks:implementer .specks/specks-test.md
```

**Verify:**
- [ ] architect-agent spawns via Task
- [ ] architect-agent returns strategy JSON
- [ ] implementer skill continues to next step

### 7.5 Phase 5: Complete Implementation Loop

**Order:**
1. **coder-agent** - The heavy lifter, writes code
2. **reviewer-agent** - Checks plan adherence
3. **auditor-agent** - Checks code quality
4. **logger-agent** - Updates log
5. **committer-agent** - Does commits

**Test:** Execute a simple single-step speck end-to-end.

**Verify:**
- [ ] Each agent returns JSON to implementer skill
- [ ] Drift detection works
- [ ] AskUserQuestion works for drift decisions
- [ ] Logger updates implementation log
- [ ] Committer creates commit and closes bead

### 7.6 Phase 6: Full Integration

**Tests:**
1. Create a new speck via `/specks:planner`
2. Execute that speck via `/specks:implementer`
3. Verify beads integration works
4. Test resume functionality
5. Test multi-step speck execution

### 7.7 Verification Checkpoints

After each agent conversion, verify:
- [ ] Agent file has valid YAML frontmatter
- [ ] Agent returns JSON to orchestrator skill
- [ ] Orchestrator skill can parse the returned JSON
- [ ] Run directory artifacts are created correctly
- [ ] No context takeover issues

---

## 8. Summary of Changes for specks-4.md

### 8.1 The Correct Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                          USER                                    │
│                            │                                     │
│                            ▼                                     │
│              /specks:planner  OR  /specks:implementer            │
│                            │                                     │
│                            ▼                                     │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │              ORCHESTRATOR SKILL                              ││
│  │         (runs in main context, NOT as subagent)              ││
│  │                                                              ││
│  │  - Uses Task tool to spawn sub-agents                        ││
│  │  - Uses AskUserQuestion DIRECTLY for user interaction        ││
│  │  - Persists outputs to run directory                         ││
│  │  - Manages orchestration loop                                ││
│  └─────────────────────────────────────────────────────────────┘│
│                            │                                     │
│                            │ Task(subagent_type: "specks:X-agent")
│                            ▼                                     │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                     SUB-AGENT                                ││
│  │           (spawned via Task, runs, RETURNS)                  ││
│  │                                                              ││
│  │  - Receives input JSON                                       ││
│  │  - Does focused work                                         ││
│  │  - Returns output JSON                                       ││
│  │  - Control returns to orchestrator skill                     ││
│  └─────────────────────────────────────────────────────────────┘│
│                            │                                     │
│                            ▼                                     │
│              Orchestrator skill continues...                     │
└─────────────────────────────────────────────────────────────────┘
```

### 8.2 File Count Summary

| Type | Count | Files |
|------|-------|-------|
| Orchestrator Skills | 2 | planner, implementer |
| Sub-Agents | 9 | clarifier, author, critic, architect, coder, reviewer, auditor, logger, committer |
| Orchestrator Agents | 0 | **NONE** (obsolete concept) |
| Interviewer | 0 | **ELIMINATED** (orchestrators use AskUserQuestion directly) |

### 8.3 The Key Principle

**WHY THIS WORKS:**

| Mechanism | Behavior | Result |
|-----------|----------|--------|
| Skill + Skill tool | Prompt injection | Takes over context, NO RETURN |
| Agent + Task tool | Spawns subagent | Runs to completion, RETURNS result |
| Subagent + Task tool | **BLOCKED** | Subagents CANNOT spawn other subagents |

**THEREFORE:**
- Orchestrators must be SKILLS (not agents) so they can use Task tool
- Sub-tasks must be AGENTS so they can be spawned and return results
- Skills run in main context and can spawn as many agents as needed (sequentially)

---

## Appendix A: Files to Create/Modify/Delete

### CREATE (9 new agent files)

```
agents/clarifier-agent.md
agents/author-agent.md
agents/critic-agent.md
agents/architect-agent.md
agents/coder-agent.md
agents/reviewer-agent.md
agents/auditor-agent.md
agents/logger-agent.md
agents/committer-agent.md
```

### MODIFY (2 orchestrator skills - add full orchestration logic)

```
skills/planner/SKILL.md      # Add complete planning loop
skills/implementer/SKILL.md  # Add complete implementation loop
```

### DELETE (obsolete files)

```
agents/planner-agent.md          # Orchestration moves to skill
agents/implementer-agent.md      # Orchestration moves to skill
agents/test-counter-agent.md     # Test file
agents/test-decider-agent.md     # Test file
skills/interviewer/SKILL.md      # Eliminated
skills/clarifier/SKILL.md        # Converted to agent
skills/author/SKILL.md           # Converted to agent
skills/critic/SKILL.md           # Converted to agent
skills/architect/SKILL.md        # Converted to agent
skills/coder/SKILL.md            # Converted to agent
skills/reviewer/SKILL.md         # Converted to agent
skills/auditor/SKILL.md          # Converted to agent
skills/logger/SKILL.md           # Converted to agent
skills/committer/SKILL.md        # Converted to agent
skills/test-counter/             # Test directory
skills/test-decider/             # Test directory
skills/test-loop-orchestrator/   # Test directory
.claude/skills/                  # Legacy directory
```

### KEEP (archived files)

```
agents/archived/director.md      # Historical reference
agents/archived/interviewer.md   # Historical reference
```
