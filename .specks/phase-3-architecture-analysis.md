# Phase 3 Architecture Analysis: Simplified Claude Code-First Design

**Date:** 2026-02-06
**Purpose:** Analyze current state and recommend a simplified Phase 3 architecture that prioritizes running inside Claude Code over CLI orchestration.

---

## Important Context

**This is a brand new library with ZERO external users.** There is no need for:
- Deprecation warnings
- Legacy API shims
- Migration guides for external consumers
- Backward compatibility

All changes should be **clean breaks**. Remove code that is no longer needed. Do not leave deprecated paths lying around.

---

## Executive Summary

The current architecture has a fundamental problem: **two competing orchestration layers**.

1. The Rust CLI (`planning_loop/mod.rs`) orchestrates agents by shelling out to `claude` CLI
2. The `specks-director` agent is defined but never invoked - it was designed to orchestrate agents via the Task tool

This creates:
- 6+ minute execution times due to separate `claude` process spawning per agent
- Duplicated logic between Rust code and agent definitions
- A director agent that exists but does nothing

**Recommendation:** Embrace a **Claude Code-first** architecture where:
- Planning and execution are orchestrated by the director agent running inside Claude Code
- The director is a **pure orchestrator** - it only coordinates, never does work itself
- All work is delegated to sub-agents (via Task) or skills (via Skill)
- The CLI becomes a thin utility for init/validate/list/status/setup

---

## Part 1: Agent vs Skill Analysis

### Current Agent Inventory

| Agent | Model | Tools | Current Role |
|-------|-------|-------|--------------|
| specks-director | opus | Task, Read, Grep, Glob, Bash, Write, Edit | NEVER CALLED - orchestrator exists on paper only |
| specks-clarifier | sonnet | Read, Grep, Glob, Bash | Generates questions (called by Rust loop) |
| specks-planner | opus | Read, Grep, Glob, Bash, Write, Edit, AskUserQuestion | Creates plans (called by Rust loop) |
| specks-critic | sonnet | Read, Grep, Glob, Bash | Reviews plans (called by Rust loop) |
| specks-interviewer | opus | Read, Grep, Glob, Bash, AskUserQuestion | Presents to user in Claude Code mode (called by Rust loop) |
| specks-architect | opus | Read, Grep, Glob, Bash | Creates implementation strategies |
| specks-implementer | opus | Task, Read, Grep, Glob, Bash, Write, Edit, Skill | Writes code, invokes skills |
| specks-monitor | haiku | Read, Grep, Glob, Bash | Watches for drift |
| specks-reviewer | sonnet | Read, Grep, Glob, Bash | Checks plan adherence |
| specks-auditor | sonnet | Read, Grep, Glob, Bash | Checks code quality |
| specks-logger | haiku | Read, Grep, Glob, Bash, Edit, Skill | Updates implementation log |
| specks-committer | haiku | Read, Grep, Glob, Bash, Write, Skill | Prepares commits |

### Design Principle: Director is a Pure Orchestrator

The director agent should **only orchestrate**. It should never:
- Write files directly
- Handle user interaction directly
- Perform git operations directly
- Do any "work" itself

All work is delegated to sub-agents (via Task tool) or skills (via Skill tool).

This keeps the director simple, predictable, and focused on coordination.

### Analysis by Role

#### 1. specks-director (Orchestrator)

**Current:** Full agent definition, never invoked. Rust `planning_loop` does orchestration instead.

**Recommendation:** **KEEP AS AGENT - pure orchestrator**

The director coordinates everything via Task and Skill tools only.

**Rationale:**
- Hub-and-spoke is the right architecture
- Director needs Task tool to spawn sub-agents
- Director needs Skill tool to invoke skills
- Director needs full context persistence across the workflow
- Running director INSIDE Claude Code eliminates process spawn overhead

**Tools:** Task, Skill, Read, Grep, Glob, Bash (for `specks validate`, `git status` checks)

**NOT in director's tools:** Write, Edit, AskUserQuestion (these belong to specialists)

---

#### 2. specks-clarifier (Question Generation)

**Current:** Sonnet agent spawned by Rust code in every iteration.

**Recommendation:** **CONVERT TO SKILL: `/specks-clarify`**

**Rationale:**
- Read-only analysis (no Write/Edit tools)
- Produces structured JSON output (questions)
- No user interaction (interviewer handles that)
- Simple enough to be inline prompt expansion
- Running as separate agent adds 30-60 seconds per iteration

---

#### 3. specks-planner (Plan Creation)

**Current:** Opus agent that writes speck files.

**Recommendation:** **KEEP AS AGENT**

**Rationale:**
- Needs Write/Edit tools to create/modify speck files
- Complex task requiring Opus-level reasoning
- Benefits from separate context to focus on planning
- Long-running task that benefits from Task tool isolation

---

#### 4. specks-critic (Plan Review)

**Current:** Sonnet agent that reviews plans.

**Recommendation:** **CONVERT TO SKILL: `/specks-critique`**

**Rationale:**
- Read-only analysis (no Write/Edit)
- Produces structured feedback
- Relatively straightforward checklist evaluation
- Could be Sonnet-optimized prompt

---

#### 5. specks-interviewer (User Interaction)

**Current:** Opus agent for Claude Code mode; CLI uses `inquire` prompts instead.

**Recommendation:** **KEEP AS AGENT - handles all user interaction**

**Rationale:**
- Director should be a pure orchestrator - no direct user interaction
- Interviewer uses AskUserQuestion to present questions and collect answers
- Keeps user interaction logic separate from orchestration logic
- Interviewer formats clarifier/critic output for user presentation

**Role:**
- Receives questions/data from director (via Task prompt)
- Presents to user via AskUserQuestion
- Returns user's decisions to director

---

#### 6. specks-architect (Implementation Strategy)

**Current:** Opus agent that creates `architect-plan.md`.

**Recommendation:** **KEEP AS AGENT**

**Rationale:**
- Complex reasoning task requiring Opus
- Produces detailed implementation strategy
- Benefits from focused context
- Long output that benefits from Task isolation

---

#### 7. specks-implementer (Code Writing)

**Current:** Opus agent that invokes `/implement-plan` skill.

**Recommendation:** **KEEP AS AGENT (primary workhorse)**

**Rationale:**
- Needs Write/Edit tools
- Long-running task
- Complex code writing requiring Opus
- Already uses Skill tool for `/implement-plan`

---

#### 8. specks-monitor (Drift Detection)

**Current:** Haiku agent that runs parallel to implementer.

**Recommendation:** **CONVERT TO SKILL: `/specks-check-drift`**

**Rationale:**
- Read-only analysis
- Simple comparison logic (touch set vs actual changes)
- Haiku model suggests simple task
- Director can invoke skill between steps

---

#### 9. specks-reviewer (Plan Adherence)

**Current:** Sonnet agent that checks implementation matches plan.

**Recommendation:** **CONVERT TO SKILL: `/specks-review-step`**

**Rationale:**
- Read-only verification
- Checklist-based evaluation
- Sonnet-appropriate complexity
- Structured output (pass/fail with issues)

---

#### 10. specks-auditor (Code Quality)

**Current:** Sonnet agent that checks code quality.

**Recommendation:** **CONVERT TO SKILL: `/specks-audit-code`**

**Rationale:**
- Read-only analysis
- Standard code review patterns
- Sonnet-appropriate
- Can run in parallel with reviewer (both skills)

---

#### 11. specks-logger (Implementation Log)

**Current:** Haiku agent that invokes `/update-specks-implementation-log` skill.

**Recommendation:** **CONVERT TO SKILL: `/specks-logger`**

Rename and keep the existing `/update-specks-implementation-log` skill as `/specks-logger` for naming consistency.

**Rationale:**
- Director invokes skill directly
- Consistent naming with other specks skills

---

#### 12. specks-committer (Git Commits)

**Current:** Haiku agent that invokes `/prepare-git-commit-message` skill.

**Recommendation:** **CONVERT TO SKILL: `/specks-committer`**

Rename and keep the existing `/prepare-git-commit-message` skill as `/specks-committer` for naming consistency.

**Rationale:**
- Director invokes skill directly
- Git operations stay out of director (pure orchestration)
- Skill handles message preparation
- Director delegates actual commit execution to this skill

---

### Summary: Agent vs Skill Decisions

| Role | Decision | Rationale |
|------|----------|-----------|
| director | **AGENT** | Hub orchestrator, needs Task/Skill tools |
| clarifier | **SKILL** | Read-only, structured output |
| planner | **AGENT** | Complex writing task, needs Opus |
| critic | **SKILL** | Read-only checklist evaluation |
| interviewer | **AGENT** | Handles all user interaction for director |
| architect | **AGENT** | Complex strategy creation |
| implementer | **AGENT** | Primary workhorse, code writing |
| monitor | **SKILL** | Read-only drift checking |
| reviewer | **SKILL** | Read-only verification |
| auditor | **SKILL** | Read-only code review |
| logger | **SKILL** | Log writing (renamed from update-specks-implementation-log) |
| committer | **SKILL** | Commit preparation (renamed from prepare-git-commit-message) |

**Final Count:**
- **5 Agents:** director, planner, interviewer, architect, implementer
- **7 Skills:** specks-clarify, specks-critique, specks-check-drift, specks-review-step, specks-audit-code, specks-logger, specks-committer
- **1 Skill (keep):** implement-plan

---

## Part 2: CLI Simplification

### Current CLI Commands

| Command | Status | Action |
|---------|--------|--------|
| `specks init` | Working | KEEP |
| `specks validate` | Working | KEEP |
| `specks list` | Working | KEEP |
| `specks status` | Working | KEEP |
| `specks plan` | 6+ minute, competing orchestration | **REMOVE** |
| `specks execute` | Untested, competing orchestration | **REMOVE** |
| `specks setup` | Working | KEEP |
| `specks beads` | Working | KEEP |

### Recommended Changes

**Keep as-is (Maintenance Commands):**
```
specks init          # Initialize project
specks validate      # Validate specks
specks list          # List specks
specks status        # Show progress
specks setup claude  # Install skills
specks beads sync    # Sync with beads
```

These are fast, self-contained operations that don't need LLM orchestration.

**Remove entirely:**
```
specks plan          # Use /specks-plan inside Claude Code instead
specks execute       # Use /specks-execute inside Claude Code instead
```

No deprecation warnings. No thin launchers. Just remove them. Users run `/specks-plan` and `/specks-execute` inside Claude Code.

### Rust Code to Remove

```
crates/specks/src/commands/plan.rs       # Remove entirely
crates/specks/src/commands/execute.rs    # Remove entirely
crates/specks/src/planning_loop/         # Remove entire module
crates/specks/src/streaming.rs           # Remove - not needed inside Claude Code
crates/specks/src/interaction/           # Remove - skills use AskUserQuestion
```

Clean break. No deprecated code paths.

---

## Part 3: Claude Code-First Architecture

### Core Insight

Running inside Claude Code means:
- **ONE session** - no process spawning overhead
- **Shared context** - director sees everything
- **Task tool works** - sub-agents spawn efficiently
- **Skill tool works** - skills run inline
- **AskUserQuestion works** - interviewer handles user interaction

This is fundamentally different from CLI shelling out to `claude` for each agent.

### New Architecture Diagram

```
User runs: /specks-plan "idea"
                |
                v
┌─────────────────────────────────────────────────────────────────┐
│                    specks-director (Opus)                        │
│  - PURE ORCHESTRATOR                                             │
│  - Only uses Task and Skill tools                                │
│  - Never writes files, never talks to user directly              │
└─────────────────────────────────────────────────────────────────┘
                |
    ┌───────────┼───────────────────────────────────┐
    |           |                                   |
    v           v                                   v
┌────────┐ ┌────────────┐                    ┌────────────────┐
│ Skill: │ │ Task:      │                    │ Task:          │
│clarify │ │ planner    │                    │ interviewer    │
└────────┘ │ (Opus)     │                    │ (Opus)         │
    |      └────────────┘                    │ AskUserQuestion│
    v           |                            └────────────────┘
  JSON          v                                   |
 questions   speck.md                               v
    |           |                            user decisions
    └───────────┴───────────────────────────────────┘
                |
                v
          ┌────────┐
          │ Skill: │
          │critique│
          └────────┘
                |
                v
          feedback → loop or approve
```

### Planning Loop (Inside Claude Code)

```
/specks-plan "idea"
     |
     v
Director starts (pure orchestrator)
     |
     v
┌─────────────────────────────────────────────────┐
│ ITERATION LOOP (runs until user approves)       │
│                                                 │
│  1. Skill: /specks-clarify                      │
│     - Analyzes idea/feedback                    │
│     - Returns JSON questions                    │
│                                                 │
│  2. Task: specks-interviewer                    │
│     - Receives questions from director          │
│     - Presents via AskUserQuestion              │
│     - Returns user answers                      │
│                                                 │
│  3. Director enriches requirements              │
│     - Combines idea + answers                   │
│                                                 │
│  4. Task: specks-planner                        │
│     - Receives enriched requirements            │
│     - Creates/revises speck file                │
│     - Returns speck path                        │
│                                                 │
│  5. Skill: /specks-critique                     │
│     - Analyzes speck                            │
│     - Returns structured feedback               │
│                                                 │
│  6. Task: specks-interviewer                    │
│     - Presents results + punch list             │
│     - Asks: approve/revise/abort                │
│     - Returns user decision                     │
│                                                 │
│  If revise: loop to step 1 with feedback        │
│  If approve: set status to active, exit         │
└─────────────────────────────────────────────────┘
```

### Execution Loop (Inside Claude Code)

```
/specks-execute .specks/specks-1.md
     |
     v
Director starts (pure orchestrator)
     |
     v
Bash: specks validate (verify speck is valid)
     |
     v
┌─────────────────────────────────────────────────┐
│ FOR EACH STEP (in dependency order)             │
│                                                 │
│  1. Task: specks-architect                      │
│     - Creates implementation strategy           │
│     - Returns architect-plan.md path            │
│                                                 │
│  2. Task: specks-implementer                    │
│     - Uses Skill: /implement-plan               │
│     - Writes code, runs tests                   │
│     - Checks task boxes                         │
│                                                 │
│  3. Skill: /specks-check-drift                  │
│     - Compares git status vs expected_touch_set │
│     - Returns drift assessment                  │
│     - If major drift: director halts            │
│                                                 │
│  4. Skill: /specks-review-step                  │
│     - Checks plan adherence                     │
│     - Returns pass/fail                         │
│                                                 │
│  5. Skill: /specks-audit-code                   │
│     - Checks code quality                       │
│     - Returns findings                          │
│                                                 │
│  6. Director: synthesize reports                │
│     - If issues: route to appropriate fix       │
│     - If clean: proceed                         │
│                                                 │
│  7. Skill: /specks-logger                       │
│     - Document completed work                   │
│                                                 │
│  8. Skill: /specks-committer                    │
│     - Create commit message                     │
│     - Execute git add + git commit              │
│                                                 │
│  9. Task: specks-interviewer (if manual policy) │
│     - Ask user to confirm commit                │
│     - Return decision                           │
│                                                 │
│  10. Mark step complete, next step              │
└─────────────────────────────────────────────────┘
```

---

## Part 4: Specific Recommendations

### 4.1 Agent Definitions to Update

#### specks-director.md

**Tools:** Task, Skill, Read, Grep, Glob, Bash

**Remove from tools:** Write, Edit, AskUserQuestion (director is pure orchestrator)

**Update:**
- Planning mode: invoke clarify skill → interviewer agent → planner agent → critique skill → interviewer agent → loop
- Execution mode: invoke architect agent → implementer agent → check-drift skill → review-step skill → audit-code skill → logger skill → committer skill

#### specks-interviewer.md

**Tools:** Read, Grep, Glob, Bash, AskUserQuestion

**Update:**
- Receives data from director (questions, results, options)
- Presents to user via AskUserQuestion
- Returns user decisions to director
- Formats clarifier/critic output for presentation

#### specks-planner.md

**Keep as-is** - good design.

#### specks-architect.md

**Keep as-is** - good design.

#### specks-implementer.md

**Keep as-is** - already uses Skill tool correctly.

### 4.2 New Skills to Create

#### `/specks-clarify`

```markdown
---
name: specks-clarify
description: Analyze idea or critic feedback to generate clarifying questions.
argument-hint: "mode=idea|revision, input=<text>"
---

Analyze the input and generate structured clarifying questions.

Returns JSON:
{
  "analysis": { "understood_intent": "...", "ambiguities": [...] },
  "questions": [
    { "question": "...", "options": [...], "default": "..." }
  ],
  "assumptions": [...]
}
```

#### `/specks-critique`

```markdown
---
name: specks-critique
description: Review a speck for skeleton compliance, quality, and implementability.
argument-hint: "<speck-path>"
---

Review the speck and return structured assessment.

Returns JSON:
{
  "skeleton_compliant": true|false,
  "areas": { "completeness": "PASS|WARN|FAIL", ... },
  "issues": [{ "priority": "HIGH|MEDIUM|LOW", "description": "..." }],
  "recommendation": "APPROVE|REVISE|REJECT"
}
```

#### `/specks-check-drift`

```markdown
---
name: specks-check-drift
description: Compare git status against expected touch set for drift detection.
argument-hint: "<expected-files...>"
---

Check for implementation drift.

Returns JSON:
{
  "expected_files": [...],
  "actual_changes": [...],
  "unexpected_changes": [...],
  "missing_changes": [...],
  "drift_severity": "none|minor|major",
  "recommendation": "CONTINUE|PAUSE|HALT"
}
```

#### `/specks-review-step`

```markdown
---
name: specks-review-step
description: Verify a completed step matches the plan specification.
argument-hint: "<speck-path> <step-anchor>"
---

Check tasks completed, tests written, artifacts produced, references followed.

Returns JSON:
{
  "tasks_complete": true|false,
  "tests_match_plan": true|false,
  "artifacts_produced": true|false,
  "issues": [...],
  "recommendation": "APPROVE|REVISE|ESCALATE"
}
```

#### `/specks-audit-code`

```markdown
---
name: specks-audit-code
description: Check code quality, performance, and security of recent changes.
argument-hint: "[files...]"
---

Review code for quality issues.

Returns JSON:
{
  "categories": { "structure": "PASS|WARN|FAIL", ... },
  "issues": [{ "severity": "critical|major|minor", "file": "...", "description": "..." }],
  "recommendation": "APPROVE|FIX_REQUIRED|MAJOR_REVISION"
}
```

### 4.3 Skills to Rename/Keep

#### `/specks-logger` (renamed from `/update-specks-implementation-log`)

Copy and rename the existing skill. Keep the same functionality.

#### `/specks-committer` (renamed from `/prepare-git-commit-message`)

Copy and rename the existing skill. Extend to also execute `git add` and `git commit` since director delegates git operations.

#### `/implement-plan` (keep as-is)

Already well-designed for implementer agent invocation.

### 4.4 Skills Entry Points

#### `/specks-plan` (update)

```markdown
---
name: specks-plan
description: Create or revise a speck through agent collaboration.
argument-hint: "\"idea\" or path/to/speck.md"
---

## What This Skill Does

Spawn the specks-director agent with mode=plan.

## Invocation

Use Task tool to spawn specks-director:
- subagent_type: specks-director
- prompt: "mode=plan, input=<idea or speck path>"
```

#### `/specks-execute` (update)

```markdown
---
name: specks-execute
description: Execute a speck through agent orchestration.
argument-hint: "path/to/speck.md [options]"
---

## What This Skill Does

Spawn the specks-director agent with mode=execute.

## Invocation

Use Task tool to spawn specks-director:
- subagent_type: specks-director
- prompt: "mode=execute, speck=<path>, commit-policy=<manual|auto>"
```

### 4.5 Agent Definitions to Remove

Delete these files entirely (no deprecation, clean break):

```
agents/specks-clarifier.md    # Becomes /specks-clarify skill
agents/specks-critic.md       # Becomes /specks-critique skill
agents/specks-monitor.md      # Becomes /specks-check-drift skill
agents/specks-reviewer.md     # Becomes /specks-review-step skill
agents/specks-auditor.md      # Becomes /specks-audit-code skill
agents/specks-logger.md       # Becomes /specks-logger skill
agents/specks-committer.md    # Becomes /specks-committer skill
```

---

## Part 5: Implementation Plan

### Phase 3.1: Create New Skills

1. Create `/specks-clarify` skill
2. Create `/specks-critique` skill
3. Create `/specks-check-drift` skill
4. Create `/specks-review-step` skill
5. Create `/specks-audit-code` skill
6. Copy and rename `/update-specks-implementation-log` → `/specks-logger`
7. Copy and rename `/prepare-git-commit-message` → `/specks-committer`
8. Update `/specks-committer` to also execute git operations

### Phase 3.2: Update Agent Definitions

1. Update `specks-director.md` - pure orchestrator, remove Write/Edit/AskUserQuestion
2. Update `specks-interviewer.md` - receives data, presents to user, returns decisions
3. Update `/specks-plan` skill - spawns director with mode=plan
4. Update `/specks-execute` skill - spawns director with mode=execute

### Phase 3.3: Remove Rust Orchestration Code

Remove these files entirely:
- `crates/specks/src/commands/plan.rs`
- `crates/specks/src/commands/execute.rs`
- `crates/specks/src/planning_loop/` (entire directory)
- `crates/specks/src/streaming.rs`
- `crates/specks/src/interaction/` (entire directory)

Update `cli.rs` to remove `plan` and `execute` commands.

### Phase 3.4: Remove Agent Definitions

Delete agent files that became skills (listed in 4.5).

### Phase 3.5: Update Documentation

- Update README to focus on Claude Code workflow
- Remove references to `specks plan` and `specks execute` CLI commands
- Document `/specks-plan` and `/specks-execute` as the primary interface

---

## Appendix A: Performance Comparison

### Current (Rust Orchestration via CLI)

```
specks plan "idea"
  -> spawn claude (clarifier)     ~30-60s
  -> spawn claude (planner)       ~60-120s
  -> spawn claude (critic)        ~30-60s
  -> spawn claude (interviewer)   ~30-60s
  -> (repeat for revisions)
Total: 6+ minutes for single iteration
```

### Proposed (Director Inside Claude Code)

```
/specks-plan "idea"
  -> Skill: clarify               ~5-10s (inline)
  -> Task: interviewer            ~10-20s (warm context)
  -> Task: planner                ~30-60s (warm context)
  -> Skill: critique              ~5-10s (inline)
  -> Task: interviewer            ~10-20s (warm context)
Total: 1-2 minutes for single iteration
```

**Expected speedup: 3-6x**

---

## Appendix B: Skill vs Agent Decision Matrix

| Factor | Agent | Skill |
|--------|-------|-------|
| Writes files | Yes | No |
| Needs isolated context | Yes | No |
| Complex reasoning | Yes | Sometimes |
| Structured JSON output | Possible | Preferred |
| User interaction | Via AskUserQuestion | No |
| Model requirements | Can specify | Uses session model |
| Startup overhead | ~10-30s | ~1-5s |

**Rule of thumb:**
- If it WRITES files and needs Opus reasoning: Agent (Task tool)
- If it READS and produces structured output: Skill (Skill tool)
- If it handles user interaction: Agent with AskUserQuestion (interviewer)
- If it's a shell command: Bash tool

---

## Conclusion

The recommended Phase 3 architecture embraces Claude Code as the primary environment with a clear separation of concerns:

1. **5 Agents remain:** director (pure orchestrator), planner, interviewer, architect, implementer
2. **7 Skills (new/renamed):** clarify, critique, check-drift, review-step, audit-code, logger, committer
3. **CLI becomes utility only:** init, validate, list, status, setup, beads
4. **Dramatic performance improvement:** 3-6x faster through eliminated process spawning
5. **Clean architecture:** director only orchestrates, specialists do work

This is not a rewrite - it's a simplification. The existing agent definitions for planner, architect, and implementer are solid. The new skills formalize what were implicit protocols. The main change is accepting that Claude Code is the right execution environment and optimizing for it.
