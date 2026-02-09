## Phase 1.0: Remove Runs Directory and Simplify Planning {#phase-remove-runs}

**Purpose:** Eliminate the obsolete `.specks/runs/` directory references from the codebase, documentation, and configuration since the planner is stateless and the implementer uses worktrees.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | specks-dev |
| Status | draft |
| Target branch | main |
| Tracking issue/PR | N/A |
| Last updated | 2025-02-08 |

---

### Phase Overview {#phase-overview}

#### Context {#context}

The specks codebase evolved from an earlier design that used `.specks/runs/` directories for session state and agent artifacts. However, the current architecture is different:

- The **planner skill** is stateless—no session directories, no metadata.json, no conflict detection
- The **implementer** uses git worktrees (`.specks-worktrees/`) for isolated implementation environments, not runs directories
- References to `.specks/runs/` in documentation, .gitignore, and skill files are now obsolete and confusing

This cleanup removes dead code paths and documentation to accurately reflect the current architecture.

#### Strategy {#strategy}

- Remove `.specks/runs/` entry from .gitignore (it was never actually created in current code)
- Update README.md to remove "Run Artifacts" section that describes obsolete functionality
- Update documentation files (getting-started.md, execute-plan.md) to remove runs references
- Update implement-plan skill to remove halt signal file references
- Leave historical specks (specks-1 through specks-4) unchanged as they are artifacts

#### Stakeholders / Primary Customers {#stakeholders}

1. Developers using specks (cleaner, accurate documentation)
2. Contributors to specks (no confusion about non-existent runs directory)

#### Success Criteria (Measurable) {#success-criteria}

- `grep -r "\.specks/runs" .` returns only historical specks (specks-1.md through specks-4.md) and this speck file
- .gitignore contains only `.specks-worktrees/` for specks-related ignores, not `.specks/runs/`
- README.md has no "Run Artifacts" section
- All documentation accurately describes the worktree-based implementation workflow

#### Scope {#scope}

1. Remove `.specks/runs/` from .gitignore
2. Remove "Run Artifacts" section from README.md
3. Update docs/getting-started.md to remove runs references
4. Update docs/tutorials/execute-plan.md to remove runs references
5. Update implement-plan skill to remove halt signal references

#### Non-goals (Explicitly out of scope) {#non-goals}

- Changing historical specks (specks-1.md through specks-4.md, specks-8.md)
- Modifying the implementation log (it already uses correct terminology)
- Adding new functionality—this is purely cleanup

#### Dependencies / Prerequisites {#dependencies}

- None—this is a standalone cleanup

#### Constraints {#constraints}

- Must not break any existing functionality
- Must preserve historical speck files unchanged

#### Assumptions {#assumptions}

- The planner skill is already effectively stateless (confirmed by inspection)
- session.rs is used by the implementer for worktree sessions, not planning
- No Rust code currently references runs directories (confirmed by grep)

---

### 1.0.0 Design Decisions {#design-decisions}

#### [D01] Remove runs directory references completely (DECIDED) {#d01-remove-runs}

**Decision:** Remove all references to `.specks/runs/` from non-historical files.

**Rationale:**
- The runs directory was never implemented in the current codebase
- The planner is stateless and does not create session directories
- The implementer uses worktrees, not runs directories
- Outdated documentation causes confusion

**Implications:**
- .gitignore will be simpler
- Documentation will accurately reflect current architecture
- No code changes needed (no Rust code references runs)

#### [D02] Preserve historical specks unchanged (DECIDED) {#d02-preserve-historical}

**Decision:** Do not modify specks-1.md through specks-4.md or specks-8.md even though they reference runs directories.

**Rationale:**
- These are historical artifacts documenting past work
- Modifying them would rewrite history
- grep output showing these files is acceptable

**Implications:**
- Success criteria allows historical specks in grep results

---

### 1.0.5 Execution Steps {#execution-steps}

#### Step 0: Update .gitignore {#step-0}

**Commit:** `chore: remove obsolete .specks/runs/ from .gitignore`

**References:** [D01] Remove runs directory references completely, (#context, #strategy)

**Artifacts:**
- Modified `.gitignore`

**Tasks:**
- [ ] Remove line 26-27 (comment and `.specks/runs/` entry) from .gitignore

**Tests:**
- [ ] Verify .gitignore still contains `.specks-worktrees/`
- [ ] Verify .gitignore no longer contains `.specks/runs/`

**Checkpoint:**
- [ ] `grep "runs" .gitignore` returns no results
- [ ] `grep "worktrees" .gitignore` returns `.specks-worktrees/`

**Rollback:**
- Revert .gitignore changes

**Commit after all checkpoints pass.**

---

#### Step 1: Update README.md {#step-1}

**Depends on:** #step-0

**Commit:** `docs: remove obsolete Run Artifacts section from README`

**References:** [D01] Remove runs directory references completely, (#context, #strategy)

**Artifacts:**
- Modified `README.md`

**Tasks:**
- [ ] Remove the "Run Artifacts" subsection (lines 236-243) that describes `.specks/runs/<session-id>/`
- [ ] Remove "runs/` - Agent run artifacts (gitignored)" from Quick Start section (around line 81)

**Tests:**
- [ ] Visual inspection of README.md for coherent flow

**Checkpoint:**
- [ ] `grep -c "runs/" README.md` returns 0
- [ ] `grep -c "\.specks/runs" README.md` returns 0

**Rollback:**
- Revert README.md changes

**Commit after all checkpoints pass.**

---

#### Step 2: Update getting-started.md {#step-2}

**Depends on:** #step-1

**Commit:** `docs: remove runs references from getting-started guide`

**References:** [D01] Remove runs directory references completely, (#context, #strategy)

**Artifacts:**
- Modified `docs/getting-started.md`

**Tasks:**
- [ ] Remove lines 347-349 that reference `.specks/runs/*/\.halt` for monitor halt files

**Tests:**
- [ ] Visual inspection of getting-started.md for coherent flow

**Checkpoint:**
- [ ] `grep -c "runs" docs/getting-started.md` returns 0

**Rollback:**
- Revert docs/getting-started.md changes

**Commit after all checkpoints pass.**

---

#### Step 3: Update execute-plan.md {#step-3}

**Depends on:** #step-2

**Commit:** `docs: remove runs directory references from execute-plan tutorial`

**References:** [D01] Remove runs directory references completely, (#context, #strategy)

**Artifacts:**
- Modified `docs/tutorials/execute-plan.md`

**Tasks:**
- [ ] Remove lines 115-131 that describe monitoring the runs directory structure
- [ ] Remove lines 143-144 referencing the run directory in completion output
- [ ] Remove lines 225-229 that describe checking the halt file
- [ ] Update lines 273-323 "Understanding Run Artifacts" section - remove entirely since this describes obsolete functionality
- [ ] Ensure document still flows coherently after removals

**Tests:**
- [ ] Visual inspection of execute-plan.md for coherent flow

**Checkpoint:**
- [ ] `grep -c "runs/" docs/tutorials/execute-plan.md` returns 0
- [ ] `grep -c "\.specks/runs" docs/tutorials/execute-plan.md` returns 0

**Rollback:**
- Revert docs/tutorials/execute-plan.md changes

**Commit after all checkpoints pass.**

---

#### Step 4: Update implement-plan skill {#step-4}

**Depends on:** #step-3

**Commit:** `chore: remove obsolete halt signal reference from implement-plan skill`

**References:** [D01] Remove runs directory references completely, (#context, #strategy)

**Artifacts:**
- Modified `.claude/skills/implement-plan/SKILL.md`

**Tasks:**
- [ ] Remove lines 140-143 that describe halt signal awareness and `.specks/runs/{uuid}/.halt`
- [ ] Ensure the "Integration with Specks Agent Suite" section still reads coherently

**Tests:**
- [ ] Visual inspection of SKILL.md for coherent flow

**Checkpoint:**
- [ ] `grep -c "runs" .claude/skills/implement-plan/SKILL.md` returns 0

**Rollback:**
- Revert .claude/skills/implement-plan/SKILL.md changes

**Commit after all checkpoints pass.**

---

#### Step 5: Final Verification {#step-5}

**Depends on:** #step-4

**Commit:** N/A (verification only)

**References:** [D01] Remove runs directory references completely, [D02] Preserve historical specks unchanged, (#success-criteria)

**Artifacts:**
- None (verification step)

**Tasks:**
- [ ] Run comprehensive grep to verify only historical specks and this speck reference runs

**Tests:**
- [ ] Integration test: full codebase search for runs references

**Checkpoint:**
- [ ] `grep -r "\.specks/runs" . --include="*.md" --include="*.rs" --include="*.toml" | grep -v "specks-[1-8].md" | grep -v "specks-9.md" | grep -v "implementation-log"` returns empty
- [ ] `specks validate` passes for this speck

**Rollback:**
- N/A (verification only)

**Commit after all checkpoints pass.**

---

### 1.0.6 Deliverables and Checkpoints {#deliverables}

**Deliverable:** A cleaner codebase with accurate documentation that reflects the stateless planner and worktree-based implementer architecture.

#### Phase Exit Criteria ("Done means…") {#exit-criteria}

- [ ] .gitignore no longer references `.specks/runs/`
- [ ] README.md no longer has "Run Artifacts" section
- [ ] docs/getting-started.md no longer references runs directories
- [ ] docs/tutorials/execute-plan.md no longer references runs directories
- [ ] implement-plan skill no longer references halt signal in runs directory
- [ ] All documentation accurately describes current architecture

**Acceptance tests:**
- [ ] `grep -r "\.specks/runs" . --include="*.md" | wc -l` returns only historical specks count
- [ ] `specks validate specks-9.md` passes

#### Roadmap / Follow-ons (Explicitly Not Required for Phase Close) {#roadmap}

- [ ] Consider removing historical specks that are no longer relevant
- [ ] Add documentation about the worktree workflow

| Checkpoint | Verification |
|------------|--------------|
| Runs references removed | grep search returns only historical files |
| Documentation accurate | Manual review of updated docs |

**Commit after all checkpoints pass.**
