## Phase 5.0: Agent Model and Permission Enhancement {#phase-agent-enhancement}

**Purpose:** Enhance all agent specifications with appropriate model choices (haiku/sonnet/opus) and permission flags (acceptEdits/dontAsk) based on Claude Code sub-agent documentation, optimizing for reliability and reduced user friction.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | specks team |
| Status | draft |
| Target branch | main |
| Tracking issue/PR | N/A |
| Last updated | 2026-02-08 |
| Beads Root | `specks-qh7` |

---

### Phase Overview {#phase-overview}

#### Context {#context}

The specks project has 11 agents defined in `agents/*.md` files. These agents currently lack explicit model specifications and permission flags documented in Claude Code's sub-agent system. Without these settings, agents use defaults that may not be optimal for their specific roles - for example, a fast read-only analysis agent like the clarifier could benefit from haiku's speed, while the author agent writing complex specks needs opus for deep reasoning.

#### Strategy {#strategy}

- Audit all 11 agents to understand their roles, tools, and complexity requirements
- Apply model assignments based on task complexity: haiku for simple checks, sonnet for balanced work, opus for critical writing/architecture
- Apply permission flags based on tool access: `acceptEdits` for write-capable agents, `dontAsk` for read-only agents
- Update YAML frontmatter in each agent file with new `model` and `permissions` fields
- Validate changes against Claude Code documentation patterns

#### Stakeholders / Primary Customers {#stakeholders}

1. Specks users invoking `/specks:planner` and `/specks:implementer` workflows
2. Specks maintainers managing agent configurations

#### Success Criteria (Measurable) {#success-criteria}

- All 11 agents have explicit `model:` field in frontmatter (verify via grep)
- All write-capable agents have `acceptEdits` permission (author, coder, logger, committer)
- All read-only agents have `dontAsk` permission (clarifier, critic, architect, reviewer, auditor)
- Setup agents use haiku model (planner-setup, implementer-setup)
- YAML frontmatter remains valid after changes (no syntax errors)

#### Scope {#scope}

1. Add `model:` field to all 11 agents in `agents/` directory
2. Add `permissions:` field where appropriate
3. Document model/permission rationale in each agent's section comments

#### Non-goals (Explicitly out of scope) {#non-goals}

- Changing agent behavior or contracts beyond frontmatter
- Adding new agents
- Modifying skill files (only agents are in scope)

#### Dependencies / Prerequisites {#dependencies}

- Access to Claude Code sub-agent documentation at https://code.claude.com/docs/en/sub-agents
- Understanding of current agent roles from existing agent files

#### Constraints {#constraints}

- Must use valid YAML frontmatter syntax
- Model names must match Claude Code expectations (haiku, sonnet, opus)
- Permission names must match Claude Code expectations (acceptEdits, dontAsk)

#### Assumptions {#assumptions}

- The YAML frontmatter format supports `model:` and `permissions:` fields per Claude Code documentation
- Haiku, sonnet, and opus are the valid model choices
- acceptEdits and dontAsk are valid permission flags that can be combined

---

### 5.0.0 Design Decisions {#design-decisions}

#### [D01] Model Assignment Strategy (DECIDED) {#d01-model-strategy}

**Decision:** Use sonnet as the default model, with opus for critical writing/architecture tasks and haiku for simple prerequisite checks.

**Rationale:**
- Sonnet provides good balance of capability and speed for most analysis/implementation tasks
- Opus excels at complex authoring and architectural reasoning - worth the cost for critical outputs
- Haiku is fast and cheap, ideal for simple checks that don't require deep reasoning
- This matches the user's stated priority: "Optimize for reliability"

**Implications:**
- author-agent and architect-agent get opus (complex output generation)
- Setup agents get haiku (simple checks)
- All other agents get sonnet (balanced)

#### [D02] Aggressive Write Permissions (DECIDED) {#d02-write-permissions}

**Decision:** Apply `acceptEdits` to all agents that have Write, Edit, or Bash (with file operations) tools.

**Rationale:**
- User explicitly chose "Aggressive: acceptEdits for all agents that write files"
- Reduces friction during implementation workflows
- Agents with write tools: author, coder, logger, committer, planner-setup (creates session dirs)

**Implications:**
- author-agent: acceptEdits (has Write, Edit)
- coder-agent: acceptEdits (has Write, Edit, Bash)
- logger-agent: acceptEdits (has Edit)
- committer-agent: acceptEdits (has Bash for git operations)
- planner-setup-agent: acceptEdits (has Write, Bash for mkdir)

#### [D03] Read-Only Agent Permissions (DECIDED) {#d03-readonly-permissions}

**Decision:** Apply `dontAsk` to all read-only agents that only use Read, Grep, Glob, WebFetch, WebSearch tools.

**Rationale:**
- User confirmed: "Yes: Apply dontAsk to all read-only agents"
- Read-only operations are safe and don't need user confirmation
- Reduces interruptions during analysis phases

**Implications:**
- clarifier-agent: dontAsk (Read, Grep, Glob, WebFetch, WebSearch only)
- critic-agent: dontAsk (Read, Grep, Glob only)
- architect-agent: dontAsk (Read, Grep, Glob, WebFetch, WebSearch only)
- reviewer-agent: dontAsk (Read, Grep, Glob only)
- auditor-agent: dontAsk (Read, Grep, Glob only)

#### [D04] implementer-setup-agent Is Read-Only (DECIDED) {#d04-implementer-setup-readonly}

**Decision:** Treat implementer-setup-agent as read-only despite having Bash tool.

**Rationale:**
- Agent description explicitly states "This agent is READ-ONLY for analysis. It does NOT create sessions or write files."
- The Bash tool is only used for `test -f` checks and `specks beads status` queries
- No file creation or modification occurs

**Implications:**
- implementer-setup-agent gets `dontAsk` permission
- Uses haiku model since it's a simple prerequisite checker

---

### Agent Model and Permission Assignments {#agent-assignments}

**Table T01: Agent Configuration Matrix** {#t01-agent-matrix}

| Agent | Model | Permissions | Rationale |
|-------|-------|-------------|-----------|
| clarifier-agent | sonnet | dontAsk | Balanced analysis, read-only |
| author-agent | opus | acceptEdits | Complex writing, has Write/Edit |
| critic-agent | sonnet | dontAsk | Balanced analysis, read-only |
| architect-agent | opus | dontAsk | Complex architecture, read-only |
| coder-agent | sonnet | acceptEdits | Implementation, has Write/Edit/Bash |
| reviewer-agent | sonnet | dontAsk | Balanced analysis, read-only |
| auditor-agent | sonnet | dontAsk | Balanced analysis, read-only |
| logger-agent | sonnet | acceptEdits | Has Edit tool |
| committer-agent | sonnet | acceptEdits | Has Bash for git ops |
| planner-setup-agent | haiku | acceptEdits | Simple checks, has Write/Bash |
| implementer-setup-agent | haiku | dontAsk | Simple checks, read-only behavior |

---

### 5.0.1 YAML Frontmatter Format {#frontmatter-format}

**Spec S01: Enhanced Frontmatter Schema** {#s01-frontmatter-schema}

The enhanced YAML frontmatter format for agents:

```yaml
---
name: agent-name
description: Agent description
model: haiku | sonnet | opus
tools: Tool1, Tool2, Tool3
permissions:
  - acceptEdits
  - dontAsk
---
```

**Field Definitions:**

| Field | Required | Values | Description |
|-------|----------|--------|-------------|
| `name` | yes | string | Agent identifier (kebab-case) |
| `description` | yes | string | Brief role description |
| `model` | yes | haiku, sonnet, opus | Model to use for this agent |
| `tools` | yes | comma-separated | Tools available to agent |
| `permissions` | no | list | Permission flags (acceptEdits, dontAsk) |

---

### 5.0.5 Execution Steps {#execution-steps}

**Bead:** `specks-qh7.1`

#### Step 0: Verify Claude Code Documentation {#step-0}

**Bead:** `specks-qh7.1`

**Commit:** `docs: verify agent frontmatter format against Claude Code docs`

**References:** [D01] Model Assignment Strategy, [D02] Aggressive Write Permissions, (#context, #frontmatter-format)

**Artifacts:**
- Verified understanding of Claude Code sub-agent YAML format

**Tasks:**
- [ ] Review Claude Code sub-agent documentation at https://code.claude.com/docs/en/sub-agents
- [ ] Confirm `model:` field is supported in frontmatter
- [ ] Confirm `permissions:` field format (list of strings)
- [ ] Note any additional fields that may be useful

**Tests:**
- [ ] Manual verification: documentation confirms frontmatter fields

**Checkpoint:**
- [ ] Document format verified and understood

**Rollback:**
- No changes made; documentation review only

**Commit after all checkpoints pass.**

**Bead:** `specks-qh7.2`

---

#### Step 1: Update Planning Agents {#step-1}

**Depends on:** #step-0

**Bead:** `specks-qh7.2`

**Commit:** `feat(agents): add model and permissions to planning agents`

**References:** [D01] Model Assignment Strategy, [D02] Aggressive Write Permissions, [D03] Read-Only Agent Permissions, Table T01, Spec S01, (#agent-assignments)

**Artifacts:**
- Updated `agents/clarifier-agent.md`
- Updated `agents/author-agent.md`
- Updated `agents/critic-agent.md`
- Updated `agents/planner-setup-agent.md`

**Tasks:**
- [ ] Update clarifier-agent.md: add `model: sonnet`, add `permissions: [dontAsk]`
- [ ] Update author-agent.md: add `model: opus`, add `permissions: [acceptEdits]`
- [ ] Update critic-agent.md: add `model: sonnet`, add `permissions: [dontAsk]`
- [ ] Update planner-setup-agent.md: add `model: haiku`, add `permissions: [acceptEdits]`

**Tests:**
- [ ] Unit test: YAML frontmatter parses correctly for each file (manual yamllint or grep)

**Checkpoint:**
- [ ] `grep -l "^model:" agents/clarifier-agent.md agents/author-agent.md agents/critic-agent.md agents/planner-setup-agent.md` returns all 4 files
- [ ] YAML frontmatter is valid in all 4 files

**Rollback:**
- `git checkout -- agents/clarifier-agent.md agents/author-agent.md agents/critic-agent.md agents/planner-setup-agent.md`

**Commit after all checkpoints pass.**

**Bead:** `specks-qh7.3`

---

#### Step 2: Update Implementation Agents {#step-2}

**Depends on:** #step-0

**Bead:** `specks-qh7.3`

**Commit:** `feat(agents): add model and permissions to implementation agents`

**References:** [D01] Model Assignment Strategy, [D02] Aggressive Write Permissions, [D03] Read-Only Agent Permissions, [D04] implementer-setup-agent Is Read-Only, Table T01, Spec S01, (#agent-assignments)

**Artifacts:**
- Updated `agents/architect-agent.md`
- Updated `agents/coder-agent.md`
- Updated `agents/reviewer-agent.md`
- Updated `agents/auditor-agent.md`
- Updated `agents/logger-agent.md`
- Updated `agents/committer-agent.md`
- Updated `agents/implementer-setup-agent.md`

**Tasks:**
- [ ] Update architect-agent.md: add `model: opus`, add `permissions: [dontAsk]`
- [ ] Update coder-agent.md: add `model: sonnet`, add `permissions: [acceptEdits]`
- [ ] Update reviewer-agent.md: add `model: sonnet`, add `permissions: [dontAsk]`
- [ ] Update auditor-agent.md: add `model: sonnet`, add `permissions: [dontAsk]`
- [ ] Update logger-agent.md: add `model: sonnet`, add `permissions: [acceptEdits]`
- [ ] Update committer-agent.md: add `model: sonnet`, add `permissions: [acceptEdits]`
- [ ] Update implementer-setup-agent.md: add `model: haiku`, add `permissions: [dontAsk]`

**Tests:**
- [ ] Unit test: YAML frontmatter parses correctly for each file

**Checkpoint:**
- [ ] `grep -l "^model:" agents/architect-agent.md agents/coder-agent.md agents/reviewer-agent.md agents/auditor-agent.md agents/logger-agent.md agents/committer-agent.md agents/implementer-setup-agent.md` returns all 7 files
- [ ] YAML frontmatter is valid in all 7 files

**Rollback:**
- `git checkout -- agents/architect-agent.md agents/coder-agent.md agents/reviewer-agent.md agents/auditor-agent.md agents/logger-agent.md agents/committer-agent.md agents/implementer-setup-agent.md`

**Commit after all checkpoints pass.**

**Bead:** `specks-qh7.4`

---

#### Step 3: Validate All Agents {#step-3}

**Depends on:** #step-1, #step-2

**Bead:** `specks-qh7.4`

**Commit:** `test(agents): validate model and permission consistency`

**References:** [D01] Model Assignment Strategy, Table T01, (#agent-assignments, #success-criteria)

**Artifacts:**
- Validation results confirming all 11 agents are properly configured

**Tasks:**
- [ ] Verify all 11 agents have `model:` field
- [ ] Verify write agents have `acceptEdits`: author, coder, logger, committer, planner-setup
- [ ] Verify read-only agents have `dontAsk`: clarifier, critic, architect, reviewer, auditor, implementer-setup
- [ ] Verify setup agents use haiku: planner-setup, implementer-setup
- [ ] Verify opus agents are correct: author, architect

**Tests:**
- [ ] Integration test: `grep "^model:" agents/*.md | wc -l` equals 11
- [ ] Integration test: `grep -l "acceptEdits" agents/*.md | wc -l` equals 5
- [ ] Integration test: `grep -l "dontAsk" agents/*.md | wc -l` equals 6

**Checkpoint:**
- [ ] All 11 agents have model field
- [ ] Permission distribution matches Table T01 (5 acceptEdits, 6 dontAsk)
- [ ] Model distribution: 2 haiku, 7 sonnet, 2 opus

**Rollback:**
- If validation fails, review individual agent files against Table T01

**Commit after all checkpoints pass.**

---

### 5.0.6 Deliverables and Checkpoints {#deliverables}

**Deliverable:** All 11 agents in `agents/*.md` have explicit `model:` and `permissions:` fields in their YAML frontmatter, configured per the agent matrix (Table T01).

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

- [ ] `grep "^model:" agents/*.md | wc -l` returns 11
- [ ] `grep -l "acceptEdits" agents/*.md` returns exactly: author-agent.md, coder-agent.md, logger-agent.md, committer-agent.md, planner-setup-agent.md
- [ ] `grep -l "dontAsk" agents/*.md` returns exactly: clarifier-agent.md, critic-agent.md, architect-agent.md, reviewer-agent.md, auditor-agent.md, implementer-setup-agent.md
- [ ] `grep "model: haiku" agents/*.md` returns: planner-setup-agent.md, implementer-setup-agent.md
- [ ] `grep "model: opus" agents/*.md` returns: author-agent.md, architect-agent.md
- [ ] All agent files have valid YAML frontmatter (no parse errors)

**Acceptance tests:**
- [ ] Integration test: All agents load without YAML parse errors
- [ ] Integration test: Agent count verification (11 total, 2 haiku, 7 sonnet, 2 opus)

#### Roadmap / Follow-ons (Explicitly Not Required for Phase Close) {#roadmap}

- [ ] Consider adding `temperature` or other model parameters if Claude Code supports them
- [ ] Monitor agent performance with new settings and adjust model assignments if needed
- [ ] Document model assignment rationale in a central location (CLAUDE.md or dedicated doc)

| Checkpoint | Verification |
|------------|--------------|
| All agents have model field | `grep "^model:" agents/*.md \| wc -l` equals 11 |
| Write agents have acceptEdits | Count equals 5 |
| Read-only agents have dontAsk | Count equals 6 |
| YAML valid | No parse errors when reading frontmatter |

**Commit after all checkpoints pass.**