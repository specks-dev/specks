## Phase 1.0: Consolidate Implementer Loop Agents {#phase-consolidate-agents}

**Purpose:** Slim down the implementer loop from 6 agents to 4 by merging logger into committer and auditor into reviewer, reducing orchestration complexity without losing functionality.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | specks |
| Status | draft |
| Target branch | main |
| Tracking issue/PR | TBD |
| Last updated | 2026-02-08 |
| Beads Root | `specks-eng` |

---

### Phase Overview {#phase-overview}

#### Context {#context}

The current implementer loop uses 6 agents: architect, coder, reviewer, auditor, logger, and committer. This creates orchestration overhead with multiple handoffs between tightly-coupled agents. The logger-agent runs immediately before committer-agent and its sole purpose is to update the implementation log. The auditor-agent runs immediately after reviewer-agent and performs quality checks that are conceptually part of the review process.

By consolidating these agents, we reduce the number of Task tool invocations per step, simplify the implementer SKILL.md orchestration logic, and reduce the number of agent output files per step.

#### Strategy {#strategy}

- Merge logger-agent INTO committer-agent: The committer takes on all logging responsibilities
- Merge auditor-agent INTO reviewer-agent: The reviewer takes on all auditing responsibilities
- Extend output contracts to include fields from both source agents
- Merge tool sets (reviewer gains Edit, committer gains Edit)
- Update implementer SKILL.md orchestration loop to reflect 4-agent architecture
- Update CLAUDE.md documentation to reflect new agent count and responsibilities
- Delete the logger-agent.md and auditor-agent.md files after consolidation

#### Stakeholders / Primary Customers {#stakeholders}

1. Implementer skill (orchestrator)
2. Developers using specks for implementation

#### Success Criteria (Measurable) {#success-criteria}

> Make these falsifiable. Avoid "works well".

- Implementer loop executes with 4 agents per step instead of 6 (count Task invocations in SKILL.md loop)
- All logging functionality preserved (implementation log entries still created with same format)
- All auditing functionality preserved (code quality checks still performed, issues still categorized by severity)
- Agent file count reduced from 6 to 4 implementation agents (Glob "agents/*-agent.md")
- Combined retry budget: max 3 attempts for all review+audit issues

#### Scope {#scope}

1. Merge logger-agent responsibilities into committer-agent
2. Merge auditor-agent responsibilities into reviewer-agent
3. Update reviewer-agent output contract with audit categories and issues
4. Update committer-agent to log before committing
5. Update implementer SKILL.md orchestration diagram and loop
6. Update CLAUDE.md agent tables
7. Delete obsolete agent files

#### Non-goals (Explicitly out of scope) {#non-goals}

- Changing the architect or coder agents
- Modifying the planning agents (clarifier, author, critic)
- Changing the speck format or skeleton
- Adding new functionality beyond consolidation

#### Dependencies / Prerequisites {#dependencies}

- Existing reviewer-agent.md, auditor-agent.md, logger-agent.md, committer-agent.md files
- Existing implementer SKILL.md with 6-agent loop

#### Constraints {#constraints}

- Must preserve all existing functionality (no feature regression)
- Combined retry budget: max 3 attempts total for review+audit issues
- Sequential flow: review/audit first, then log, then commit

#### Assumptions {#assumptions}

- Tool merging is safe (reviewer gaining Edit does not cause conflicts)
- Output contract extensions are additive (no breaking changes to consumers)
- The implementer skill can adapt to fewer agent invocations without issues

---

### 1.0.0 Design Decisions {#design-decisions}

#### [D01] Merge logger INTO committer (DECIDED) {#d01-merge-logger}

**Decision:** The committer-agent absorbs all logger-agent responsibilities. Logger-agent.md is deleted.

**Rationale:**
- Logger runs immediately before committer with no intervening agents
- Logging is conceptually part of the "finalize step" workflow
- Reduces one Task invocation per step
- The implementation log update is atomic with the commit (both staged together)

**Implications:**
- Committer gains Edit tool (for updating implementation log)
- Committer input contract adds logger fields (summary, files_changed)
- Committer performs log update BEFORE git operations
- Committer output contract adds logging success fields

#### [D02] Merge auditor INTO reviewer (DECIDED) {#d02-merge-auditor}

**Decision:** The reviewer-agent absorbs all auditor-agent responsibilities. Auditor-agent.md is deleted.

**Rationale:**
- Auditor runs immediately after reviewer with tight coupling
- Audit checks are conceptually part of the "verify implementation" workflow
- Reduces one Task invocation per step
- Combined retry loop is simpler than separate loops

**Implications:**
- Reviewer gains explicit auditing checklist (lint, formatting, duplication, idioms, performance, Big-O)
- Reviewer retains plan-conformance-checking role
- Reviewer output contract adds audit categories (structure, error_handling, security)
- Reviewer output contract adds audit issues array with severity levels

#### [D03] Review/audit before log (DECIDED) {#d03-log-timing}

**Decision:** The consolidated committer performs logging AFTER all review/audit checks pass, but BEFORE git commit operations.

**Rationale:**
- User answer confirms: "Review/audit first, then log (sequential: all checks pass -> update log -> return)"
- Ensures log entries are only created for successfully reviewed code
- Log update failure is treated as needs_reconcile scenario

**Implications:**
- Committer input includes review/audit results
- Log is updated only when committer is invoked (i.e., after reviewer APPROVE)
- Log update failure does not block commit but sets needs_reconcile flag

#### [D04] Combined retry budget (DECIDED) {#d04-retry-budget}

**Decision:** Max 3 attempts total for all review+audit issues within the consolidated reviewer.

**Rationale:**
- User answer confirms: "Combined retry budget: max 3 attempts total for all review+audit issues"
- Simpler than tracking separate budgets for review and audit
- Forces escalation rather than infinite loops

**Implications:**
- Implementer tracks single `reviewer_attempts` counter
- Counter increments for any REVISE/FIX_REQUIRED recommendation
- After 3 attempts, escalate to user regardless of issue type

#### [D05] Keep existing agent file names (DECIDED) {#d05-file-names}

**Decision:** The consolidated agents keep their existing names: reviewer-agent.md and committer-agent.md.

**Rationale:**
- Clarifier assumption: "Keep existing agent file names with expanded responsibilities"
- Minimizes changes to orchestrator spawn commands
- Names remain accurate (reviewer still reviews, committer still commits)

**Implications:**
- Only file deletions needed (logger-agent.md, auditor-agent.md)
- No renames of surviving agents

---

### 1.0.1 Consolidated Reviewer-Agent Specification {#reviewer-spec}

#### Input Contract {#reviewer-input}

The consolidated reviewer receives the same input as before, plus audit context:

**Spec S01: Consolidated Reviewer Input** {#s01-reviewer-input}

```json
{
  "worktree_path": "/abs/path/to/.specks-worktrees/...",
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

No new input fields required - audit uses same files from coder_output.

#### Output Contract {#reviewer-output}

**Spec S02: Consolidated Reviewer Output** {#s02-reviewer-output}

```json
{
  "tasks_complete": true,
  "tests_match_plan": true,
  "artifacts_produced": true,
  "audit_categories": {
    "structure": "PASS|WARN|FAIL",
    "error_handling": "PASS|WARN|FAIL",
    "security": "PASS|WARN|FAIL"
  },
  "issues": [
    {
      "type": "missing_task|test_gap|artifact_missing|drift|conceptual|audit_structure|audit_error|audit_security",
      "severity": "critical|major|minor",
      "file": "string | null",
      "description": "string"
    }
  ],
  "drift_notes": "string | null",
  "recommendation": "APPROVE|REVISE|ESCALATE"
}
```

| Field | Description |
|-------|-------------|
| `audit_categories` | NEW: Audit category ratings (from auditor) |
| `issues[].type` | EXTENDED: Now includes audit types (audit_structure, audit_error, audit_security) |
| `issues[].severity` | NEW: Severity level (critical, major, minor) |
| `issues[].file` | NEW: File where issue found (null for plan issues) |

#### Recommendation Mapping {#reviewer-recommendations}

**Table T01: Consolidated Reviewer Recommendations** {#t01-reviewer-recs}

| Condition | Recommendation | Old Agent |
|-----------|----------------|-----------|
| All tasks complete, all audits PASS | APPROVE | reviewer + auditor |
| Missing tasks/artifacts, fixable | REVISE | reviewer |
| Audit issues, fixable | REVISE | auditor (was FIX_REQUIRED) |
| Critical audit issues | ESCALATE | auditor (was MAJOR_REVISION) |
| Conceptual/design issues | ESCALATE | reviewer |

Note: `FIX_REQUIRED` and `MAJOR_REVISION` are absorbed into `REVISE` and `ESCALATE`.

#### Auditing Checklist {#audit-checklist}

**List L01: Explicit Audit Responsibilities** {#l01-audit-checklist}

The consolidated reviewer checks for:

1. **Lint failures** - Code passes project linters (clippy, eslint, etc.)
2. **Formatting errors** - Code follows project formatting standards
3. **Code duplication** - No unnecessary copy-paste code
4. **Unidiomatic code** - Follows language/framework conventions
5. **Performance regressions** - No obvious performance issues
6. **Bad Big-O algorithms** - No inefficient algorithms where better alternatives exist
7. **Error handling** - Proper error propagation, no swallowed errors
8. **Security issues** - No injection risks, secrets exposure, etc.

---

### 1.0.2 Consolidated Committer-Agent Specification {#committer-spec}

#### Input Contract {#committer-input}

The consolidated committer receives the same input as before, plus logger fields:

**Spec S03: Consolidated Committer Input (Commit Mode)** {#s03-committer-input}

```json
{
  "operation": "commit",
  "worktree_path": "/abs/path/to/.specks-worktrees/...",
  "speck_path": "string",
  "step_anchor": "string",
  "proposed_message": "string",
  "files_to_stage": ["string"],
  "commit_policy": "auto|manual",
  "confirmed": false,
  "bead_id": "string | null",
  "close_reason": "string | null",
  "log_entry": {
    "summary": "string",
    "tasks_completed": [{"task": "string", "status": "Done"}],
    "tests_run": ["string"],
    "checkpoints_verified": ["string"]
  }
}
```

| Field | Description |
|-------|-------------|
| `log_entry` | NEW: Logger information for implementation log update |
| `log_entry.summary` | Brief summary of what was implemented |
| `log_entry.tasks_completed` | Task completion status for log entry |
| `log_entry.tests_run` | Tests that were run |
| `log_entry.checkpoints_verified` | Checkpoints that passed |

Publish mode input is unchanged.

#### Output Contract {#committer-output}

**Spec S04: Consolidated Committer Output (Commit Mode)** {#s04-committer-output}

```json
{
  "operation": "commit",
  "log_updated": true,
  "log_entry_added": {
    "step": "string",
    "timestamp": "string",
    "summary": "string"
  },
  "commit_message": "string",
  "files_staged": ["string"],
  "committed": true,
  "commit_hash": "string | null",
  "bead_closed": true,
  "bead_id": "string | null",
  "needs_reconcile": false,
  "aborted": false,
  "reason": "string | null",
  "warnings": ["string"]
}
```

| Field | Description |
|-------|-------------|
| `log_updated` | NEW: True if implementation log was updated |
| `log_entry_added` | NEW: Details of the log entry (from logger output) |

#### Workflow Sequence {#committer-workflow}

**Spec S05: Consolidated Committer Workflow** {#s05-committer-workflow}

1. Read the speck file and locate step anchor (for log entry title)
2. Read current implementation log header
3. Generate and prepend log entry using Edit tool
4. Stage all files (including implementation log)
5. Create git commit
6. Close bead
7. Return combined result

If log update fails:
- Set `log_updated: false`
- Set `needs_reconcile: true`
- Include warning but continue with commit
- User answer: "Treat as 'needs_reconcile' scenario"

---

### 1.0.3 Updated Implementer Loop {#implementer-loop}

#### Orchestration Diagram {#loop-diagram}

**Diagram Diag01: Consolidated Implementer Loop** {#diag01-loop}

```
  implementer-setup-agent (one-shot)
       │
       └── status: "ready"
              │
              ▼
       ┌─────────────────────────────────────────────────────────────┐
       │              FOR EACH STEP in resolved_steps                │
       │  ┌───────────────────────────────────────────────────────┐  │
       │  │                                                       │  │
       │  │  read bead_id ──► architect-agent ──► coder-agent     │  │
       │  │  from session      (with worktree)   (with worktree)  │  │
       │  │                           ┌───────────────┘           │  │
       │  │                           ▼                           │  │
       │  │                    Drift Check                        │  │
       │  │                    (AskUserQuestion if moderate/major)│  │
       │  │                           │                           │  │
       │  │           ┌───────────────┼───────────────┐           │  │
       │  │           ▼               ▼               ▼           │  │
       │  │        Continue        Revise          Abort          │  │
       │  │           │          (loop back)      (halt)          │  │
       │  │           ▼                                           │  │
       │  │  ┌─────────────────────────────────────────────┐      │  │
       │  │  │    REVIEW+AUDIT LOOP (max 3 attempts)       │      │  │
       │  │  │  reviewer-agent ──► REVISE? ──► coder-agent │      │  │
       │  │  │  (includes audit)                           │      │  │
       │  │  │         │                                   │      │  │
       │  │  │         ▼                                   │      │  │
       │  │  │      APPROVE                                │      │  │
       │  │  └─────────────────────────────────────────────┘      │  │
       │  │           │                                           │  │
       │  │           ▼                                           │  │
       │  │  committer-agent (commit mode)                        │  │
       │  │  ├─► update implementation log                        │  │
       │  │  ├─► stage files (incl. log)                          │  │
       │  │  ├─► commit + close bead                              │  │
       │  │  └─► collect step summary                             │  │
       │  │                                                       │  │
       │  └───────────────────────────────────────────────────────┘  │
       │                           │                                 │
       │                           ▼                                 │
       │                    Next step or done                        │
       └─────────────────────────────────────────────────────────────┘
              │
              ▼
       committer-agent (publish mode)
```

#### Agent Count Change {#agent-count}

**Table T02: Implementation Agent Changes** {#t02-agent-changes}

| Before | After | Change |
|--------|-------|--------|
| architect-agent | architect-agent | No change |
| coder-agent | coder-agent | No change |
| reviewer-agent | reviewer-agent | Gains audit responsibilities |
| auditor-agent | (deleted) | Merged into reviewer |
| logger-agent | (deleted) | Merged into committer |
| committer-agent | committer-agent | Gains logging responsibilities |
| **6 agents** | **4 agents** | **-2 agents** |

#### Retry Budget {#retry-budget}

**Spec S06: Combined Retry Budget** {#s06-retry-budget}

- Single counter: `reviewer_attempts`
- Increments on any REVISE recommendation (plan issues OR audit issues)
- Max 3 attempts before escalation to user
- Replaces separate `reviewer_attempts` and `auditor_attempts` counters

---

### 1.0.4 Tool Set Changes {#tool-changes}

**Table T03: Tool Set Merges** {#t03-tool-changes}

| Agent | Before | After |
|-------|--------|-------|
| reviewer-agent | Read, Grep, Glob | Read, Grep, Glob, Edit |
| committer-agent | Read, Grep, Glob, Bash | Read, Grep, Glob, Bash, Edit |

Note: Reviewer gains Edit for potential future use (not required for current audit checks). Committer gains Edit for updating implementation log.

---

### 1.0.5 Execution Steps {#execution-steps}

#### Step 0: Consolidate Reviewer-Agent {#step-0}

**Bead:** `specks-eng.1`

**Commit:** `refactor(agents): consolidate auditor into reviewer-agent`

**References:** [D02] Merge auditor INTO reviewer, Spec S01, Spec S02, Table T01, List L01, (#reviewer-spec, #audit-checklist)

**Artifacts:**
- Modified `agents/reviewer-agent.md` with expanded responsibilities

**Tasks:**
- [ ] Add audit_categories to output contract
- [ ] Add severity field to issues array
- [ ] Add file field to issues array
- [ ] Add audit issue types (audit_structure, audit_error, audit_security)
- [ ] Add auditing checklist section with all 8 explicit responsibilities
- [ ] Update recommendation criteria to include audit conditions
- [ ] Map FIX_REQUIRED to REVISE, MAJOR_REVISION to ESCALATE
- [ ] Add Edit tool to frontmatter

**Tests:**
- [ ] Unit test: Verify YAML frontmatter is valid
- [ ] Integration test: Agent file parses correctly

**Checkpoint:**
- [ ] `Grep "audit_categories" agents/reviewer-agent.md` finds output contract
- [ ] `Grep "Edit" agents/reviewer-agent.md` finds tool in frontmatter
- [ ] `Grep "Lint failures" agents/reviewer-agent.md` finds audit checklist

**Rollback:**
- Restore `agents/reviewer-agent.md` from git

**Commit after all checkpoints pass.**

---

#### Step 1: Consolidate Committer-Agent {#step-1}

**Depends on:** #step-0

**Bead:** `specks-eng.2`

**Commit:** `refactor(agents): consolidate logger into committer-agent`

**References:** [D01] Merge logger INTO committer, [D03] Review/audit before log, Spec S03, Spec S04, Spec S05, (#committer-spec, #committer-workflow)

**Artifacts:**
- Modified `agents/committer-agent.md` with logging responsibilities

**Tasks:**
- [ ] Add log_entry to commit mode input contract
- [ ] Add log_updated and log_entry_added to output contract
- [ ] Add Edit tool to frontmatter
- [ ] Add workflow section explaining: read speck -> read log -> prepend entry -> stage -> commit -> close bead
- [ ] Add log entry format section (copy from logger-agent.md)
- [ ] Add log failure handling (needs_reconcile scenario)

**Tests:**
- [ ] Unit test: Verify YAML frontmatter is valid
- [ ] Integration test: Agent file parses correctly

**Checkpoint:**
- [ ] `Grep "log_entry" agents/committer-agent.md` finds input contract
- [ ] `Grep "log_updated" agents/committer-agent.md` finds output contract
- [ ] `Grep "Edit" agents/committer-agent.md` finds tool in frontmatter

**Rollback:**
- Restore `agents/committer-agent.md` from git

**Commit after all checkpoints pass.**

---

#### Step 2: Update Implementer SKILL.md {#step-2}

**Depends on:** #step-1

**Bead:** `specks-eng.3`

**Commit:** `refactor(skills): update implementer loop for 4-agent architecture`

**References:** [D04] Combined retry budget, Spec S06, Diagram Diag01, Table T02, (#implementer-loop, #loop-diagram, #retry-budget)

**Artifacts:**
- Modified `skills/implementer/SKILL.md` with consolidated loop

**Tasks:**
- [ ] Update orchestration diagram to show 4 agents
- [ ] Remove auditor-agent spawn section (4f)
- [ ] Remove logger-agent spawn section (4g)
- [ ] Merge audit retry logic into reviewer retry logic
- [ ] Update committer input to include log_entry fields
- [ ] Update worktree structure section (fewer agent output files)
- [ ] Update "Execute This Sequence" summary to list 4 agents
- [ ] Change `auditor_attempts` to single `reviewer_attempts` counter
- [ ] Update max attempts from 2+3 to just 3 total

**Tests:**
- [ ] Unit test: Verify YAML frontmatter is valid
- [ ] Integration test: SKILL.md parses correctly

**Checkpoint:**
- [ ] `Grep "architect-agent" skills/implementer/SKILL.md` finds spawn
- [ ] `Grep "coder-agent" skills/implementer/SKILL.md` finds spawn
- [ ] `Grep "reviewer-agent" skills/implementer/SKILL.md` finds spawn
- [ ] `Grep "committer-agent" skills/implementer/SKILL.md` finds spawn
- [ ] `Grep "auditor-agent" skills/implementer/SKILL.md` returns no results
- [ ] `Grep "logger-agent" skills/implementer/SKILL.md` returns no results
- [ ] `Grep "max 3" skills/implementer/SKILL.md` finds retry budget

**Rollback:**
- Restore `skills/implementer/SKILL.md` from git

**Commit after all checkpoints pass.**

---

#### Step 3: Update CLAUDE.md Documentation {#step-3}

**Depends on:** #step-2

**Bead:** `specks-eng.4`

**Commit:** `docs: update CLAUDE.md for 4-agent implementer architecture`

**References:** Table T02, Table T03, (#agent-count, #tool-changes)

**Artifacts:**
- Modified `CLAUDE.md` with updated agent tables

**Tasks:**
- [ ] Update "Sub-Agents (9)" to "Sub-Agents (7)" in section header
- [ ] Remove auditor-agent row from implementation agents table
- [ ] Remove logger-agent row from implementation agents table
- [ ] Update reviewer-agent description to include audit responsibilities
- [ ] Update committer-agent description to include logging responsibilities
- [ ] Update reviewer-agent tools to include Edit
- [ ] Update committer-agent tools to include Edit
- [ ] Update implementer orchestration description (6 agents -> 4 agents)

**Tests:**
- [ ] Integration test: CLAUDE.md is valid markdown

**Checkpoint:**
- [ ] `Grep "Sub-Agents (7)" CLAUDE.md` finds updated count
- [ ] `Grep "auditor-agent" CLAUDE.md` returns no results
- [ ] `Grep "logger-agent" CLAUDE.md` returns no results
- [ ] `Grep "reviewer-agent.*audit" CLAUDE.md` finds updated description
- [ ] `Grep "committer-agent.*log" CLAUDE.md` finds updated description

**Rollback:**
- Restore `CLAUDE.md` from git

**Commit after all checkpoints pass.**

---

#### Step 4: Delete Obsolete Agent Files {#step-4}

**Depends on:** #step-3

**Bead:** `specks-eng.5`

**Commit:** `refactor(agents): delete obsolete auditor-agent and logger-agent files`

**References:** [D01] Merge logger INTO committer, [D02] Merge auditor INTO reviewer, [D05] Keep existing agent file names

**Artifacts:**
- Deleted `agents/auditor-agent.md`
- Deleted `agents/logger-agent.md`

**Tasks:**
- [ ] Delete `agents/auditor-agent.md`
- [ ] Delete `agents/logger-agent.md`
- [ ] Verify remaining agent files: architect, coder, reviewer, committer (implementation) + clarifier, author, critic (planning) + implementer-setup

**Tests:**
- [ ] Integration test: `Glob "agents/*-agent.md"` returns expected 8 files

**Checkpoint:**
- [ ] `ls agents/auditor-agent.md` fails (file not found)
- [ ] `ls agents/logger-agent.md` fails (file not found)
- [ ] `Glob "agents/*-agent.md"` returns 8 files (3 planning + 4 implementation + 1 setup)

**Rollback:**
- Restore deleted files from git

**Commit after all checkpoints pass.**

---

### 1.0.6 Deliverables and Checkpoints {#deliverables}

**Deliverable:** Implementer loop consolidated from 6 agents to 4 agents with all functionality preserved.

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

- [ ] Reviewer-agent includes all audit responsibilities (List L01)
- [ ] Committer-agent includes all logging responsibilities
- [ ] Implementer SKILL.md shows 4-agent loop
- [ ] CLAUDE.md documents 7 sub-agents total (3 planning + 4 implementation)
- [ ] auditor-agent.md and logger-agent.md are deleted
- [ ] Combined retry budget of 3 attempts documented

**Acceptance tests:**
- [ ] Integration test: `Glob "agents/*-agent.md"` returns 8 files
- [ ] Integration test: `Grep "audit_categories" agents/reviewer-agent.md` succeeds
- [ ] Integration test: `Grep "log_entry" agents/committer-agent.md` succeeds
- [ ] Integration test: `Grep "max 3" skills/implementer/SKILL.md` succeeds

#### Roadmap / Follow-ons (Explicitly Not Required for Phase Close) {#roadmap}

- [ ] Consider further consolidation of architect and coder if warranted
- [ ] Add telemetry to measure actual retry counts post-consolidation
- [ ] Evaluate if combined agent is too large (context limits)

| Checkpoint | Verification |
|------------|--------------|
| 4-agent loop | `Grep` for agent spawns in SKILL.md |
| Audit in reviewer | `Grep "audit_categories"` in reviewer-agent.md |
| Log in committer | `Grep "log_entry"` in committer-agent.md |
| Files deleted | `ls` for deleted files fails |

**Commit after all checkpoints pass.**