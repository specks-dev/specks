## Phase 1.0: Fix Implementer Skill Beads Workflow {#phase-beads-workflow-fix}

**Purpose:** Fix the beads workflow in the implementer skill to correctly sync beads once at session start, extract bead IDs from the speck file, and pass correct IDs to the committer agent.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | specks |
| Status | active |
| Target branch | main |
| Tracking issue/PR | - |
| Last updated | 2025-02-08 |
| Beads Root | `specks-v0r` |

---

### Phase Overview {#phase-overview}

#### Context {#context}

The implementer skill has several bugs in its beads integration. The workflow references a non-existent `--step` flag for `specks beads sync`, attempts to sync beads per-step instead of once at session start, and fails to extract bead IDs from the speck file after sync. This results in null bead IDs being passed to the committer agent, breaking bead closure.

#### Strategy {#strategy}

- Move beads sync to implementer-setup-agent (one-shot at session initialization)
- Remove incorrect `--step` flag references from implementer skill
- Add bead ID extraction from speck file using `**Bead:**` line pattern
- Store step-to-bead mapping in session metadata
- Update implementer skill to read bead IDs from metadata when invoking committer

#### Stakeholders / Primary Customers {#stakeholders}

1. Developers using specks to implement plans
2. CI/CD systems tracking implementation progress via beads

#### Success Criteria (Measurable) {#success-criteria}

- `specks beads sync` is called exactly once per session (at initialization)
- Every step has a bead ID in session metadata after initialization
- Committer agent receives valid bead IDs (never null)
- All existing tests continue to pass

#### Scope {#scope}

1. Update implementer-setup-agent to run beads sync
2. Update implementer-setup-agent to extract bead IDs from synced speck
3. Update implementer-setup-agent output contract to include bead mappings
4. Update implementer skill to remove per-step sync
5. Update implementer skill to pass correct bead ID from session metadata to committer

#### Non-goals (Explicitly out of scope) {#non-goals}

- Changing the beads sync CLI implementation
- Modifying the committer-agent contract
- Adding new beads commands

#### Dependencies / Prerequisites {#dependencies}

- `specks beads sync` command works correctly (verified)
- Speck file format with `**Bead:**` lines is stable

#### Constraints {#constraints}

- Must maintain backward compatibility with existing specks
- Setup agent uses only: Read, Grep, Glob, Bash

#### Assumptions {#assumptions}

- The beads sync command writes bead IDs back to the speck file atomically
- Bead IDs in specks use the project-specific prefix (e.g., `specks-xxx` not `bd-xxx`)
- The parser in specks-core already correctly extracts `**Bead:**` lines from steps

---

### 1.0.0 Design Decisions {#design-decisions}

#### [D01] Beads sync runs once at session initialization (DECIDED) {#d01-sync-once}

**Decision:** Move `specks beads sync` from per-step execution to a single call during setup-agent initialization.

**Rationale:**
- The sync command is designed to operate on the entire speck atomically
- Per-step sync is redundant and wasteful (sync is idempotent but unnecessary)
- Bead IDs must exist before step execution for proper tracking

**Implications:**
- Setup agent gains responsibility for beads sync
- Implementer skill loses per-step sync code
- Session metadata must store bead mappings

---

#### [D02] Bead IDs extracted by reading speck file after sync (DECIDED) {#d02-extract-from-speck}

**Decision:** After running `specks beads sync`, the setup agent reads the speck file to extract `**Bead:**` lines from each step.

**Rationale:**
- The sync command writes bead IDs directly to the speck file
- JSON output from sync may be harder to parse reliably
- Reading the speck ensures we get the canonical bead ID assignments

**Implications:**
- Setup agent must parse `**Bead:** \`xxx\`` pattern from step blocks
- Parser uses regex: `` \*\*Bead:\*\*\s*`([^`]+)` ``

---

#### [D03] Missing bead ID after sync is a HALT condition (DECIDED) {#d03-missing-bead-halt}

**Decision:** If any step lacks a bead ID after sync completes, the setup agent returns `status: "error"` and halts.

**Rationale:**
- Beads are required for proper workflow tracking
- A missing bead ID indicates sync failure or speck corruption
- Proceeding without beads would break committer-agent

**Implications:**
- Setup agent validates all steps have beads before returning "ready"
- Error message identifies which steps lack beads

---

#### [D04] Bead mapping stored in session metadata (DECIDED) {#d04-metadata-storage}

**Decision:** The setup agent output includes a `bead_mapping` object that maps step anchors to bead IDs. The implementer stores this in session metadata.

**Rationale:**
- Bead IDs are needed per-step during implementation loop
- Storing in metadata avoids re-parsing speck file each step
- Clear data structure for orchestrator to consume

**Implications:**
- Setup agent output contract adds `bead_mapping` field
- Implementer metadata.json adds `bead_mapping` field
- Implementer reads bead ID from metadata when invoking committer

---

### 1.0.1 Specification {#specification}

#### Setup Agent Output Contract Changes {#spec-setup-output}

**Spec S01: Updated Setup Agent Output** {#s01-setup-output}

Add to setup agent output (when `status: "ready"`):

```json
{
  "status": "ready",
  "beads": {
    "sync_performed": true,
    "root_bead": "specks-abc123",
    "bead_mapping": {
      "#step-0": "specks-abc123.0",
      "#step-1": "specks-abc123.1",
      "#step-2": "specks-abc123.2"
    }
  },
  ...existing fields...
}
```

| Field | Type | Description |
|-------|------|-------------|
| `beads.sync_performed` | boolean | True if sync was run this session |
| `beads.root_bead` | string | Root bead ID from Plan Metadata |
| `beads.bead_mapping` | object | Map of step anchor to bead ID |

---

#### Session Metadata Changes {#spec-session-metadata}

**Spec S02: Updated Session Metadata** {#s02-session-metadata}

Add to `metadata.json`:

```json
{
  ...existing fields...,
  "root_bead": "specks-abc123",
  "bead_mapping": {
    "#step-0": "specks-abc123.0",
    "#step-1": "specks-abc123.1"
  }
}
```

---

#### Implementer Skill Changes {#spec-implementer-changes}

**Spec S03: Step Preparation Changes** {#s03-step-prep}

**Remove** (lines 173-175 of current SKILL.md):
```markdown
2. Sync bead: `specks beads sync <speck_path> --step #step-N`
3. Store bead ID from sync output
```

**Replace with:**
```markdown
2. Read bead ID from session metadata: `bead_mapping[step_anchor]`
3. If bead ID is missing: HALT with error (should not happen if setup succeeded)
```

---

**Spec S04: Committer Invocation Changes** {#s04-committer-invocation}

Update committer invocation to use bead ID from metadata:

```markdown
"bead_id": metadata.bead_mapping[step_anchor]
```

Instead of relying on sync output (which was null).

---

#### Beads Reference Section Changes {#spec-beads-reference}

**Spec S05: Updated Beads Reference** {#s05-beads-reference}

**Remove** (lines 351-355):
```markdown
**Sync before step:**
```bash
specks beads sync <speck_path> --step #step-N
```
```

**Replace with:**
```markdown
**Sync at session start** (handled by setup-agent):
```bash
specks beads sync <speck_path>
```
Bead IDs are stored in session metadata and used per-step.

---

### 1.0.2 Symbol Inventory {#symbol-inventory}

#### Files to Modify {#files-to-modify}

| File | Purpose |
|------|---------|
| `skills/implementer/SKILL.md` | Remove per-step sync, use metadata for bead IDs |
| `agents/implementer-setup-agent.md` | Add beads sync, extract bead IDs, update output |

---

### 1.0.3 Execution Steps {#execution-steps}

**Bead:** `specks-v0r.1`

#### Step 0: Update implementer-setup-agent to handle beads sync {#step-0}

**Bead:** `specks-v0r.1`

**Commit:** `fix(agents): add beads sync and bead extraction to setup-agent`

**References:** [D01] Beads sync runs once at session initialization, [D02] Bead IDs extracted by reading speck file after sync, [D03] Missing bead ID after sync is a HALT condition, [D04] Bead mapping stored in session metadata, Spec S01, (#strategy, #context)

**Artifacts:**
- Updated `agents/implementer-setup-agent.md` with beads sync in Phase 1
- Updated output contract with `beads` field
- Bead ID extraction logic using regex pattern

**Tasks:**
- [ ] Add beads sync call in Phase 1 (Prerequisites Check), after beads availability check
- [ ] Add Phase 4b: Extract Bead IDs from Speck after sync
- [ ] Update output contract to include `beads` object with `sync_performed`, `root_bead`, `bead_mapping`
- [ ] Add validation that all steps have bead IDs before returning "ready"
- [ ] Update error handling for missing bead IDs

**Tests:**
- [ ] integration: Verify setup agent output includes bead_mapping
- [ ] integration: Verify missing bead ID causes error status

**Checkpoint:**
- [ ] `grep -q "beads sync" agents/implementer-setup-agent.md`
- [ ] `grep -q "bead_mapping" agents/implementer-setup-agent.md`

**Rollback:**
- Revert changes to `agents/implementer-setup-agent.md`

**Commit after all checkpoints pass.**

**Bead:** `specks-v0r.2`

---

#### Step 1: Update implementer skill to fix beads workflow {#step-1}

**Depends on:** #step-0

**Bead:** `specks-v0r.2`

**Commit:** `fix(skills): remove per-step beads sync, use metadata bead IDs`

**References:** [D01] Beads sync runs once at session initialization, [D04] Bead mapping stored in session metadata, Spec S02, Spec S03, Spec S04, Spec S05, (#spec-implementer-changes, #spec-beads-reference)

**Artifacts:**
- Updated `skills/implementer/SKILL.md` with corrected beads workflow
- Session metadata includes bead_mapping
- Committer receives correct bead_id

**Tasks:**
- [ ] Update session metadata schema to include `root_bead` and `bead_mapping`
- [ ] Remove `--step` flag from beads sync references (lines ~174, ~354)
- [ ] Remove per-step beads sync in section 4a
- [ ] Update section 4a to read bead ID from `metadata.bead_mapping[step_anchor]`
- [ ] Update section 4h committer invocation to use metadata bead ID
- [ ] Update Reference: Beads Integration section to reflect sync-at-start pattern
- [ ] Add error handling for missing bead ID (should not happen but defensive)

**Tests:**
- [ ] integration: Verify implementer skill references correct beads sync usage
- [ ] integration: Verify committer receives non-null bead_id

**Checkpoint:**
- [ ] `! grep -q "\-\-step" skills/implementer/SKILL.md` (no --step flag)
- [ ] `grep -q "bead_mapping\[step_anchor\]" skills/implementer/SKILL.md`

**Rollback:**
- Revert changes to `skills/implementer/SKILL.md`

**Commit after all checkpoints pass.**

---

### 1.0.4 Deliverables and Checkpoints {#deliverables}

**Deliverable:** Implementer skill correctly syncs beads once at session start and passes valid bead IDs to committer agent.

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

- [ ] No `--step` flag references in implementer skill (grep verification)
- [ ] Setup agent performs beads sync during initialization (documented in agent)
- [ ] Setup agent output includes bead_mapping (documented in agent)
- [ ] Implementer skill reads bead IDs from session metadata (documented in skill)
- [ ] Committer invocation uses metadata bead ID (documented in skill)

**Acceptance tests:**
- [ ] integration: End-to-end implementer workflow succeeds with bead closure

#### Roadmap / Follow-ons (Explicitly Not Required for Phase Close) {#roadmap}

- [ ] Add test fixtures for beads workflow in implementer
- [ ] Consider caching beads sync output in session directory

| Checkpoint | Verification |
|------------|--------------|
| No --step flag | `! grep -q "\-\-step" skills/implementer/SKILL.md` |
| Beads sync in setup | `grep -q "specks beads sync" agents/implementer-setup-agent.md` |
| Bead mapping in output | `grep -q "bead_mapping" agents/implementer-setup-agent.md` |

**Commit after all checkpoints pass.**