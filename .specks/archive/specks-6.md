## Phase 2.0: Agent-Bead Communication {#phase-agent-bead-comm}

**Purpose:** Replace inter-agent artifact files with bead field updates so that each implementation agent reads from and writes to the step's bead via specks CLI commands, making the bead the single coordination point during implementation.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | kocienda |
| Status | draft |
| Target branch | main |
| Tracking issue/PR | -- |
| Last updated | 2026-02-11 |
| Beads Root | `specks-bvq` |

---

### Phase Overview {#phase-overview}

#### Context {#context}

Currently, implementation agents communicate through artifact JSON files written to an external directory (`<repo>/.specks-worktrees/.artifacts/<session-id>/step-N/`). The architect writes `architect-output.json`, the coder reads it and writes `coder-output.json`, the reviewer reads both, and the committer extracts summaries from the reviewer's artifact. This creates a dependency chain on local filesystem paths, file naming conventions, and a directory structure that lives outside the worktree.

Phase A (Rich Sync, specks-5) populated bead fields with step content during sync. Beads now contain `description`, `acceptance_criteria`, `design`, and `notes` fields with meaningful content. This phase leverages those fields as the communication channel between agents, replacing the artifact file pipeline.

#### Strategy {#strategy}

- Add new `specks beads` CLI subcommands (`inspect`, `append-design`, `update-notes`, `append-notes`) so agents can both read and write bead fields through the specks CLI, never via direct `bd` calls
- Add `update_notes()`, `append_notes()`, and `append_design()` methods to `BeadsCli` in `beads.rs`, with optional `working_dir` parameter for worktree context
- All `BeadsCli` write methods use a temp file + `--body-file` strategy when content exceeds a size threshold, preventing shell `ARG_MAX` failures on large bead fields
- Architect, coder, and reviewer self-fetch bead content via `specks beads inspect <bead_id> --json` using their Bash tool -- the orchestrator passes only `bead_id`, never bead content. Committer is the exception: it receives `bead_id` + `log_entry` from the orchestrator (no self-fetch needed).
- Modify agent definitions to read bead fields (self-fetched) and write results via `specks beads` subcommands
- Update the implementer skill to pass `bead_id` instead of `artifact_dir` paths, removing both artifact path construction and any CLI calls from the orchestration loop
- Move artifact directory from external location to `{worktree}/.specks/artifacts/` for debug-only post-mortem use
- Agents still write JSON artifacts for debugging, but no agent reads another agent's artifacts in the normal workflow

#### Stakeholders / Primary Customers {#stakeholders}

1. Implementation agents (architect, coder, reviewer, committer) -- primary consumers of bead-mediated data
2. Implementer skill orchestrator -- simplified dispatch without artifact path management
3. Developers debugging failed implementations -- artifact files preserved in worktree for post-mortem

#### Success Criteria (Measurable) {#success-criteria}

- No agent reads another agent's artifact file as part of the normal implementation workflow (verified by grep of agent definitions for `artifact_dir` reads)
- All inter-agent data flows through bead fields (verified by `specks beads inspect <step-id>` after completion showing full history: specification, strategy, results, review)
- `specks beads inspect`, `specks beads append-design`, `specks beads update-notes`, and `specks beads append-notes` CLI commands exist and function correctly (verified by integration tests)
- Debugging artifacts still available inside worktree at `{worktree}/.specks/artifacts/` for post-mortem analysis (verified by checking directory after implementation run)

#### Scope {#scope}

1. New `BeadsCli` methods: `update_notes()`, `append_notes()`, `append_design()` with optional `working_dir` and temp-file fallback for large content
2. New specks CLI subcommands: `specks beads inspect`, `specks beads append-design`, `specks beads update-notes`, `specks beads append-notes`
3. Agent definition updates: architect, coder, reviewer self-fetch bead content and write via CLI; committer receives data inline from orchestrator
4. Implementer skill update: pass only `bead_id` instead of `artifact_dir`; orchestrator does zero CLI work
5. Artifact directory relocation to `{worktree}/.specks/artifacts/`
6. Session/worktree infrastructure changes for new artifact path

#### Non-goals (Explicitly out of scope) {#non-goals}

- Phase A (Rich Sync) -- already complete
- Phase C (Eliminate Session) -- separate speck, depends on this phase
- Phase D (Status from Beads) -- separate speck
- Removing artifact files entirely -- kept for debugging

#### Dependencies / Prerequisites {#dependencies}

- Phase A (specks-5) must be complete: bead fields populated during sync
- Beads CLI (`bd`) must support `--design`, `--notes`, `--acceptance` flags on `bd update`
- Existing `BeadsCli::update_design()`, `update_description()`, `update_acceptance()` methods in `beads.rs`

#### Constraints {#constraints}

- All beads interaction goes through `BeadsCli` in `beads.rs` -- no direct `bd` CLI calls from agents or any code outside `beads.rs`
- Agents use `specks beads <subcommand>` to write bead fields, never `bd` directly
- The speck file is read-only during implementation
- Warnings are errors: `-D warnings` enforced via `.cargo/config.toml`

#### Assumptions {#assumptions}

- Phase A is complete: bead `description`, `acceptance_criteria`, `design` fields are populated during sync
- Architect appends to `design` field below references (does not overwrite sync content)
- Coder writes `notes` first; reviewer appends below a `---` separator
- Committer receives `bead_id` and `log_entry` from orchestrator; it does not self-fetch bead content. The orchestrator constructs `log_entry` from coder/reviewer Task responses.
- `BeadsCli` write methods accept optional `working_dir` parameter for worktree context
- The orchestrator passes `bead_id` to agents; architect, coder, and reviewer self-fetch bead content via `specks beads inspect`. Committer receives `bead_id` + `log_entry` inline (no self-fetch).
- Artifact directory cleanup happens automatically when worktree is removed after merge
- `bd` requires `current_dir` set to a path containing or linked to `.beads/`; git worktrees do not automatically resolve this (to be verified in Step 0)
- Agent definitions document which bead fields they read and write in their contracts

---

### 2.0.0 Design Decisions {#design-decisions}

#### [D01] Bead is the single coordination point (DECIDED) {#d01-bead-coordination}

**Decision:** All inter-agent data during implementation flows through bead fields. No agent reads another agent's artifact files in the normal workflow.

**Rationale:**
- Eliminates dependency on filesystem paths and file naming conventions
- Makes `bd show <step-id>` a complete audit trail of the implementation
- Reduces orchestrator complexity: pass a bead ID instead of constructing artifact directory paths

**Implications:**
- Agents need CLI commands to both read and write bead fields
- Orchestrator passes only `bead_id`; each agent self-fetches bead content via `specks beads inspect`
- The orchestrator never calls any CLI tool -- it is a pure dispatcher with only Task and AskUserQuestion
- Artifact files become debug-only, never read in normal flow

#### [D02] All bead writes go through specks CLI subcommands (DECIDED) {#d02-specks-cli-writes}

**Decision:** Agents write bead fields via `specks beads update-design`, `specks beads update-notes`, `specks beads append-notes` subcommands. No agent runs `bd` commands directly.

**Rationale:**
- Preserves the invariant that all beads interaction goes through `BeadsCli` in `beads.rs`
- Provides a clean abstraction boundary for future pivot away from beads
- Agents already have `Bash` tool access to run `specks` commands

**Implications:**
- Need new specks CLI subcommands backed by `BeadsCli` methods
- Agents use `specks beads update-notes <bead-id> --content "<markdown>"` syntax
- `BeadsCli` methods need optional `working_dir` for worktree context

#### [D03] Notes field uses append convention with separator (DECIDED) {#d03-notes-append}

**Decision:** Coder writes to `notes` first (via `specks beads update-notes`). Reviewer appends below a `---` separator (via `specks beads append-notes`). Neither overwrites the other's content.

**Rationale:**
- Preserves chronological record of implementation results and review
- Separator makes it easy for committer to parse both sections
- Append is safer than overwrite for concurrent-like writes

**Implications:**
- Need both `update_notes()` (overwrite) and `append_notes()` (append with separator) methods
- Committer does not read `notes` from the bead; it receives `log_entry` from the orchestrator (constructed from coder/reviewer Task responses)
- `bd show --json` returns the combined notes field for audit/debugging purposes

#### [D04] Reviewer gets Bash tool for specks CLI commands (DECIDED) {#d04-reviewer-bash}

**Decision:** Add `Bash` to reviewer-agent's tools so it can run `specks beads append-notes` to write review results to the bead.

**Rationale:**
- Reviewer currently has Read, Grep, Glob, Write, Edit -- no way to run CLI commands
- Needed to write review results to bead via specks CLI
- Reviewer MUST NOT run `bd` commands directly -- only `specks beads` subcommands

**Implications:**
- Update reviewer-agent.md tool list to include `Bash`
- Document in agent contract that Bash is only for `specks beads` CLI commands
- Reviewer continues to be primarily a read-and-verify agent with this narrow write capability

#### [D05] Artifacts kept in worktree for debugging (DECIDED) {#d05-debug-artifacts}

**Decision:** Agents continue writing JSON artifacts to `{worktree}/.specks/artifacts/` for debugging, but no agent reads another agent's artifacts in the normal workflow.

**Rationale:**
- Post-mortem debugging requires seeing what each agent produced
- Moving inside worktree means automatic cleanup when worktree is removed
- No behavior change needed for merge/cleanup commands

**Implications:**
- Artifact directory moves from `<repo>/.specks-worktrees/.artifacts/<session-id>/` to `{worktree}/.specks/artifacts/`
- Agents write artifacts as a side effect, not as a communication channel
- Setup agent creates `{worktree}/.specks/artifacts/` directory
- `session.rs::artifacts_dir()` function updated or deprecated
- External artifacts cleanup in worktree removal no longer needed

#### [D06] BeadsCli methods accept optional working_dir (DECIDED) {#d06-working-dir}

**Decision:** `BeadsCli` write methods (`update_notes`, `append_notes`, `update_design`) accept an optional `working_dir: Option<&Path>` parameter and set `current_dir` on the spawned `Command` so `bd` finds the `.beads/` directory when running in a worktree.

**Rationale:**
- Worktrees have their own `.beads/` context
- Without setting `current_dir`, `bd` may use the wrong beads database
- Consistent pattern across all write methods

**Implications:**
- All new `BeadsCli` methods include `working_dir` parameter
- Existing methods (`update_design`, `update_description`, `update_acceptance`) should also gain this parameter
- Specks CLI subcommands accept `--working-dir` flag and pass it through

#### [D07] Agents self-fetch bead content (DECIDED) {#d07-agent-self-fetch}

**Decision:** Each agent fetches its own bead content by running `specks beads inspect <bead_id> --json --working-dir <worktree>` using its Bash tool. The orchestrator passes only `bead_id` and `worktree_path` -- it never runs CLI commands or fetches bead content itself.

**Rationale:**
- The implementer skill is a pure orchestrator with only `Task` and `AskUserQuestion` tools -- it cannot run Bash, Read, or any other tool
- Having agents self-fetch eliminates the need for the orchestrator to do any work between agent dispatches
- All implementation agents already have Bash tool access (reviewer gains it via [D04])

**Implications:**
- `specks beads inspect` CLI subcommand needed (wraps `BeadsCli::show()`)
- Agent input contracts pass `bead_id` + `worktree_path`, not `bead_content` objects
- Architect, coder, and reviewer call `specks beads inspect <bead_id> --json --working-dir <worktree>` as their first action to get current bead state
- Exception: committer does NOT self-fetch; it receives `bead_id` (for `--bead` flag) + `log_entry` (from orchestrator's accumulated agent responses)
- Orchestrator prompt construction is trivial: just embed `bead_id` and `worktree_path`

#### [D08] Design field uses append command (DECIDED) {#d08-append-design}

**Decision:** Architect writes to the `design` field using `specks beads append-design` (parallel to `append-notes`), which reads the current design content, appends a `---` separator and the new content, and writes back. This preserves the sync-generated references.

**Rationale:**
- The `design` field is initially populated by sync with plan references ([D01] titles, anchor links)
- Architect must preserve that content and append its strategy below
- Using an overwrite command (`update-design`) would require the architect to manually read-then-append, which is fragile
- `append-design` is consistent with `append-notes` and handles the read-modify-write atomically

**Implications:**
- Add `append_design()` method to `BeadsCli` (parallel to `append_notes()`)
- Add `specks beads append-design` CLI subcommand
- Architect uses `append-design` instead of `update-design` for strategy writes
- `update-design` remains available for cases where overwrite is intended (e.g., sync re-enrichment)

#### [D09] BeadsCli write methods use temp file for large content (DECIDED) {#d09-body-file}

**Decision:** All `BeadsCli` write methods (`update_design`, `update_description`, `update_acceptance`, `update_notes`, and the new `append_design`, `append_notes`) detect when content exceeds a size threshold (64KB) and automatically write it to a temp file, passing `--body-file <path>` to `bd update` instead of `--<field> <content>` as a command-line argument.

**Rationale:**
- After Phase A enrichment, `design` fields contain rendered references + decision text. Architect appends strategy. The combined content can be large.
- macOS `ARG_MAX` is ~256KB; hitting this limit silently truncates or fails the `bd update` call
- The `--content-file` flag on new CLI subcommands is not sufficient -- the `BeadsCli` methods themselves must handle large content since `append_design()` and `append_notes()` do read-modify-write internally (read via `show()`, concatenate, then call `update_design()` / `update_notes()` which pass the combined blob as a CLI arg)
- Temp files are written to `std::env::temp_dir()` and deleted after the `bd` command completes

**Implications:**
- Refactor all `BeadsCli` update methods to check content length before building the command
- Below threshold: use `cmd.arg("--<field>").arg(content)` (current behavior)
- Above threshold: write content to a `NamedTempFile`, use `cmd.arg("--body-file").arg(temp_path)`
- The threshold (64KB) provides a safety margin well below the 256KB `ARG_MAX` limit
- `bd update` must support `--body-file` flag (verify in Step 0)
- Existing `create()` and `create_with_deps()` methods also pass content as args; add the same fallback there

---

### 2.0.1 Field Ownership Convention {#field-ownership}

**Table T01: Bead Field Ownership** {#t01-field-ownership}

| Bead Field | Written By | When | Content |
|-----------|-----------|------|---------|
| `description` | Sync (Phase A) | During `specks beads sync` | Tasks, artifacts, commit template, rollback |
| `acceptance_criteria` | Sync (Phase A) | During `specks beads sync` | Tests + checkpoints |
| `design` | Sync + Architect | Sync: references; Architect: appends via `append-design` | References from sync, then `---` separator, then architect strategy |
| `notes` | Coder + Reviewer | Coder writes first, reviewer appends | Coder results, then `---` separator, then review |
| `close_reason` | Committer | On `bd close` (via `specks step-commit`) | `"Committed: <hash> -- <summary>"` (from orchestrator-provided `log_entry`) |
| `metadata` | Various | As needed | Arbitrary JSON metadata (not used in Phase B; reserved for Phase D) |

**Revision Loop Behavior** {#revision-loop-behavior}

During the coder-reviewer revision loop:

1. **Coder** always uses `update-notes` (overwrite), even on revision. This resets the notes field, wiping any stale reviewer content from the previous cycle.
2. **Reviewer** always uses `append-notes` (append with `---` separator) to add review results below the coder's fresh notes.
3. If the reviewer recommends **REVISE**: the coder overwrites notes again with fresh results, the reviewer appends again. Each cycle starts clean.
4. **Final state** after APPROVE: notes field contains the last coder results + the final approval review.

This convention means the notes field always reflects the most recent cycle, not an accumulation of all revision attempts.

**Spec S01: Design Field Format** {#s01-design-format}

```markdown
## References
- [D01] Decision title (DECIDED): One-line decision
- [D03] Another decision (DECIDED): One-line decision
- Anchors: #anchor-1, #anchor-2

---

## Architect Strategy
**Approach:** High-level description of implementation approach
**Expected files:** file1.rs, file2.rs
**Test plan:** How to verify the implementation
**Risks:** Any identified risks
```

**Spec S02: Notes Field Format** {#s02-notes-format}

```markdown
## Coder Results
**Success:** true
**Files created:** src/api/client.rs, src/api/retry.rs
**Files modified:** Cargo.toml
**Tests:** passed (12/12)
**Build:** clean

---

## Review
**Recommendation:** APPROVE
**Plan conformance:** All tasks completed, all checkpoints verified
**Issues:** None
```

**Revision cycle:** On each revision, the coder uses `update-notes` (overwrite) to reset the notes field with fresh results. The reviewer then uses `append-notes` to add a new review below the separator. If the reviewer recommends REVISE, the cycle repeats: coder overwrites, reviewer appends. The notes field always reflects only the most recent cycle -- previous coder results and reviewer feedback are not preserved across revisions.

---

### 2.0.2 New CLI Subcommands {#new-cli-subcommands}

**Spec S03: specks beads inspect** {#s03-beads-inspect}

```
specks beads inspect <bead-id> [--working-dir <path>] [--json]
```

Returns full bead details including all content fields (`description`, `acceptance_criteria`, `design`, `notes`, `close_reason`, `metadata`, `status`, etc.). Used by every agent to self-fetch bead content at the start of each step. Wraps `BeadsCli::show()`.

Distinct from `specks beads status`, which shows execution state (complete/ready/blocked). `inspect` shows full bead content (all fields).

JSON output matches the `IssueDetails` struct: `id`, `title`, `description`, `status`, `priority`, `issue_type`, `dependencies`, `dependents`, `design`, `acceptance_criteria`, `notes`, `close_reason`, `metadata`.

**Spec S04: specks beads append-design** {#s04-append-design}

```
specks beads append-design <bead-id> --content "<markdown>" [--content-file <path>] [--working-dir <path>] [--json]
```

Reads the current `design` field, appends a `---` separator and the new content, then writes the combined result. Used by the architect to append its strategy below the sync-generated references.

Implementation: calls `BeadsCli::show()` to read current design, concatenates with separator, calls `BeadsCli::update_design()` to write back.

**Spec S05: specks beads update-notes** {#s05-update-notes}

```
specks beads update-notes <bead-id> --content "<markdown>" [--content-file <path>] [--working-dir <path>] [--json]
```

Overwrites the `notes` field. Used by the coder to write initial implementation results.

**Spec S06: specks beads append-notes** {#s06-append-notes}

```
specks beads append-notes <bead-id> --content "<markdown>" [--content-file <path>] [--working-dir <path>] [--json]
```

Reads the current `notes` field, appends a `---` separator and the new content, then writes the combined result. Used by the reviewer to add review results below coder results.

Implementation: calls `BeadsCli::show()` to read current notes, concatenates with separator, calls `BeadsCli::update_notes()` to write back.

Note: `specks beads update-design` already exists via the existing `BeadsCli::update_design()` method. It is retained for overwrite use cases (e.g., sync re-enrichment) but agents use `append-design` for normal workflow.

All write subcommands support `--content-file <path>` as an alternative to `--content`. When `--content-file` is used, the subcommand reads content from the file and passes it to the `BeadsCli` method. Independently, the `BeadsCli` methods themselves auto-detect large content (>64KB) and use a temp file + `--body-file` flag when calling `bd update`, per [D09].

---

### 2.0.3 Agent Data Flow {#agent-data-flow}

**Table T02: Agent Read/Write via Beads** {#t02-agent-readwrite}

| Agent | Self-Fetches (via `specks beads inspect`) | Writes To Bead (via `specks beads` CLI) |
|-------|-------------------------------------------|----------------------------------------|
| **Setup** | `bd ready --parent <root>` for ready steps (via existing CLI) | Nothing (creates worktree + infrastructure) |
| **Architect** | `description`, `acceptance_criteria`, `design` | `design` field via `specks beads append-design` |
| **Coder** | `description`, `design` (references + architect strategy) | `notes` field via `specks beads update-notes` (overwrite, even on revision) |
| **Reviewer** | `description`, `acceptance_criteria`, `design`, `notes` (coder results) | `notes` field via `specks beads append-notes` |
| **Committer** | Does not self-fetch; receives `bead_id` + `log_entry` from orchestrator | `close_reason` on `bd close` (via existing `specks step-commit`) |

**Diagram Diag01: Agent Communication Flow** {#diag01-agent-flow}

```
  Orchestrator passes bead_id + worktree_path to architect, coder, reviewer.
  Those agents self-fetch and self-write via specks beads CLI.
  Committer receives bead_id + log_entry from orchestrator (no self-fetch).

  ┌─────────────────────────────────────────────────────────┐
  │                    BEAD (step-N)                         │
  │                                                         │
  │  description ──────────────────► Architect, Coder       │
  │  acceptance_criteria ──────────► Architect, Reviewer     │
  │  design ◄──────────────────────► Architect (read+append)│
  │         ──────────────────────► Coder (read)            │
  │         ──────────────────────► Reviewer (read)         │
  │  notes ◄───────────────────────► Coder (write first)    │
  │        ◄───────────────────────► Reviewer (append)      │
  │  close_reason ◄────────────────► Committer (set)        │
  │                                                         │
  └─────────────────────────────────────────────────────────┘

  Architect, Coder, Reviewer each run:
    specks beads inspect <bead_id> --json --working-dir <worktree>
  to read fields, then write via:
    specks beads append-design / update-notes / append-notes

  Committer receives log_entry (constructed by orchestrator from
  coder/reviewer Task responses) and bead_id (for --bead flag).
  Committer does NOT run specks beads inspect.

  Revision loop (coder-reviewer):
    Coder: update-notes (overwrite) -> Reviewer: append-notes
    If REVISE: Coder overwrites notes again, Reviewer appends fresh review
```

---

### Risks and Mitigations {#risks}

| Risk | Impact | Likelihood | Mitigation | Trigger to revisit |
|------|--------|------------|------------|--------------------|
| Large bead field content exceeds `bd update` argument limits | high | low | BeadsCli methods auto-switch to temp file + `--body-file` for content >64KB [D09] | Content > 64KB |
| Reviewer Bash access used for non-beads commands | med | low | Document restriction in agent contract, audit during review | Agent runs unexpected commands |
| Append race condition if agents overlap | low | low | Sequential orchestration ensures no overlap | Architecture changes to parallel |

**Risk R01: Content Size Limits** {#r01-content-size}

- **Risk:** After Phase A enrichment and architect append, the combined `design` field content can be large enough to exceed shell `ARG_MAX` (~256KB on macOS) when passed as a command-line argument via `cmd.arg("--design").arg(content)`. The `append_design()` and `append_notes()` methods do read-modify-write internally, so the content passed to `update_design()` / `update_notes()` grows with each append.
- **Mitigation:** Two layers of protection per [D09]:
  1. **BeadsCli methods** (internal): Auto-detect content >64KB and write to a `NamedTempFile`, passing `--body-file <path>` to `bd update` instead of `--<field> <content>`. Temp file deleted after command completes.
  2. **CLI subcommands** (external): Support `--content-file <path>` flag so agents can write content to a file first and avoid shell quoting issues.
- **Residual risk:** Assumes `bd update` supports `--body-file` flag; to be verified in Step 0. If not supported, fall back to stdin piping.

---

### 2.0.4 Symbol Inventory {#symbol-inventory}

#### 2.0.4.1 New files {#new-files}

| File | Purpose |
|------|---------|
| `crates/specks/src/commands/beads/inspect.rs` | `specks beads inspect` subcommand handler |
| `crates/specks/src/commands/beads/update.rs` | `append-design`, `update-notes`, `append-notes` subcommand handlers |

#### 2.0.4.2 Symbols to add / modify {#symbols}

| Symbol | Kind | Location | Notes |
|--------|------|----------|-------|
| `IssueDetails::close_reason` | field | `beads.rs` | New: `Option<String>`, `#[serde(default)]` |
| `IssueDetails::metadata` | field | `beads.rs` | New: `Option<serde_json::Value>`, `#[serde(default)]` |
| `BeadsCli::cmd()` | fn (retained) | `beads.rs` | Kept as convenience wrapper calling `cmd_with_dir(None)` |
| `BeadsCli::cmd_with_dir()` | fn (new) | `beads.rs` | Accepts `Option<&Path>` for `current_dir`; called directly by methods that accept `working_dir` |
| `BeadsCli::write_arg_or_body_file()` | fn | `beads.rs` | Private helper: if content >64KB, write to temp file and use `--body-file`; else use `cmd.arg()` |
| `BeadsCli::update_notes()` | fn | `beads.rs` | New: overwrites notes field; uses body-file fallback |
| `BeadsCli::append_notes()` | fn | `beads.rs` | New: read-modify-write with `---` separator |
| `BeadsCli::append_design()` | fn | `beads.rs` | New: read-modify-write with `---` separator |
| `BeadsCli::show()` | fn (modified) | `beads.rs` | Add `working_dir: Option<&Path>` parameter |
| `BeadsCli::bead_exists()` | fn (modified) | `beads.rs` | Add `working_dir: Option<&Path>` parameter; pass through to `show()` |
| `BeadsCli::update_design()` | fn (modified) | `beads.rs` | Add `working_dir: Option<&Path>` parameter; use body-file fallback |
| `BeadsCli::update_description()` | fn (modified) | `beads.rs` | Add `working_dir: Option<&Path>` parameter; use body-file fallback |
| `BeadsCli::update_acceptance()` | fn (modified) | `beads.rs` | Add `working_dir: Option<&Path>` parameter; use body-file fallback |
| `BeadsCli::create()` | fn (modified) | `beads.rs` | Use body-file fallback for content args (description, design, acceptance, notes) |
| `BeadsCli::create_with_deps()` | fn (modified) | `beads.rs` | Use body-file fallback for content args (description, design, acceptance, notes) |
| `BODY_FILE_THRESHOLD` | const | `beads.rs` | `64 * 1024` (64KB) -- switch to temp file above this |
| `BeadsCommands::Inspect` | enum variant | `beads/mod.rs` | New CLI subcommand |
| `BeadsCommands::AppendDesign` | enum variant | `beads/mod.rs` | New CLI subcommand |
| `BeadsCommands::UpdateNotes` | enum variant | `beads/mod.rs` | New CLI subcommand |
| `BeadsCommands::AppendNotes` | enum variant | `beads/mod.rs` | New CLI subcommand |
| `run_inspect()` | fn | `beads/inspect.rs` | Handler for `specks beads inspect` |
| `run_append_design()` | fn | `beads/update.rs` | Handler for `specks beads append-design` |
| `run_update_notes()` | fn | `beads/update.rs` | Handler for `specks beads update-notes` |
| `run_append_notes()` | fn | `beads/update.rs` | Handler for `specks beads append-notes` |
| `artifacts_dir()` | fn (modified) | `session.rs` | Signature change: `(worktree_path: &Path) -> PathBuf` |

---

### 2.0.5 Execution Steps {#execution-steps}

#### Step 0: Add BeadsCli methods and CLI subcommands {#step-0}

**Bead:** `specks-bvq.1`

**Commit:** `feat(beads): add inspect, append-design, update-notes, append-notes CLI subcommands with body-file fallback`

**References:** [D02] All bead writes go through specks CLI, [D03] Notes field append convention, [D06] BeadsCli methods accept optional working_dir, [D07] Agents self-fetch bead content, [D08] Design field uses append command, [D09] BeadsCli write methods use temp file for large content, Spec S03, Spec S04, Spec S05, Spec S06, Table T01, (#new-cli-subcommands, #field-ownership, #symbol-inventory)

**Artifacts:**
- New file: `crates/specks/src/commands/beads/inspect.rs` -- `specks beads inspect` subcommand
- New file: `crates/specks/src/commands/beads/update.rs` -- `append-design`, `update-notes`, `append-notes` subcommand implementations
- Modified: `crates/specks-core/src/beads.rs` -- new methods, `IssueDetails` fields, body-file fallback, refactored `cmd()` helper
- Modified: `crates/specks/src/commands/beads/mod.rs` -- register new subcommands

**Tasks:**
- [ ] Verify prerequisite: `bd update --body-file <path>` is supported by the beads CLI. If not, fall back to stdin piping (`cmd.stdin(Stdio::piped())` + write to stdin). Document the result.
- [ ] Verify prerequisite: confirm how `bd` discovers `.beads/` from a git worktree context. Run `bd show <id>` from inside a worktree to test. Document whether `working_dir` is required or if git worktree linking handles it. Add result as an updated assumption.
- [ ] Add `close_reason: Option<String>` field to `IssueDetails` struct with `#[serde(default)]`
- [ ] Add `metadata: Option<serde_json::Value>` field to `IssueDetails` struct with `#[serde(default)]`
- [ ] Add `BODY_FILE_THRESHOLD` constant (`64 * 1024` bytes) to `beads.rs`
- [ ] Add private helper `write_arg_or_body_file(cmd, flag, content, _temp_holder)`: if `content.len() > BODY_FILE_THRESHOLD`, write content to a `NamedTempFile`, add `--body-file <path>` to cmd; else add `cmd.arg(flag).arg(content)`. The `_temp_holder` is an `Option<NamedTempFile>` passed by the caller to keep the temp file alive until the command completes.
- [ ] Add `cmd_with_dir(working_dir: Option<&Path>)` private method that calls `.current_dir()` when provided; keep existing `cmd()` as a convenience wrapper calling `cmd_with_dir(None)`. Methods that accept `working_dir` parameter use `cmd_with_dir()` directly; all other methods (is_installed, create, close, sync, list_by_ids, children, ready, dep_add, dep_remove, dep_list) continue using `cmd()` unchanged.
- [ ] Add optional `working_dir: Option<&Path>` parameter to existing `show()`, `bead_exists()`, `update_design()`, `update_description()`, `update_acceptance()` methods
- [ ] Update `bead_exists()` to accept and pass through `working_dir` to `show()` (currently calls `self.show(id).is_ok()` with no working_dir, which gives wrong results from worktree context)
- [ ] Refactor existing `update_design()`, `update_description()`, `update_acceptance()` to use `write_arg_or_body_file()` for content argument
- [ ] Refactor existing `create()` and `create_with_deps()` to use `write_arg_or_body_file()` for their content arguments (`description`, `design`, `acceptance`, `notes`). After Phase A enrichment, `sync.rs` calls `create_with_deps()` with rendered markdown that can be substantial.
- [ ] Add `update_notes(id, content, working_dir)` method to `BeadsCli` (uses `write_arg_or_body_file()`)
- [ ] Add `append_notes(id, content, working_dir)` method to `BeadsCli` (reads current via `show()`, appends with `---` separator, writes back via `update_notes()`)
- [ ] Add `append_design(id, content, working_dir)` method to `BeadsCli` (reads current via `show()`, appends with `---` separator, writes back via `update_design()`)
- [ ] Add `Inspect`, `AppendDesign`, `UpdateNotes`, `AppendNotes` variants to `BeadsCommands` enum
- [ ] Implement `run_inspect` handler: calls `BeadsCli::show()`, outputs JSON or formatted text
- [ ] Implement `run_append_design`, `run_update_notes`, `run_append_notes` handler functions
- [ ] Each write subcommand accepts `<bead-id>`, `--content <text>`, `--content-file <path>`, `--working-dir <path>`, and `--json`
- [ ] `specks beads inspect` accepts `<bead-id>`, `--working-dir <path>`, and `--json`

**Tests:**
- [ ] Unit test: `IssueDetails` deserializes `close_reason` and `metadata` fields when present
- [ ] Unit test: `IssueDetails` deserializes without `close_reason` and `metadata` (backward compatibility)
- [ ] Unit test: `write_arg_or_body_file()` uses `--body-file` when content exceeds threshold
- [ ] Unit test: `write_arg_or_body_file()` uses inline arg when content is below threshold
- [ ] Unit test: `BeadsCli::update_notes()` builds correct `bd update --notes` command
- [ ] Unit test: `BeadsCli::append_notes()` reads existing notes, appends separator, calls update
- [ ] Unit test: `BeadsCli::append_design()` reads existing design, appends separator, calls update
- [ ] Unit test: `cmd_with_dir()` with `working_dir` sets `current_dir` on Command
- [ ] Integration test: `specks beads inspect <id> --json` returns bead fields including `close_reason` and `metadata`
- [ ] Integration test: `specks beads inspect <id> --json --working-dir <worktree>` works from outside the worktree
- [ ] Integration test: `specks beads update-notes <id> --content "test"` updates bead
- [ ] Integration test: `specks beads append-notes <id> --content "review"` appends with separator
- [ ] Integration test: `specks beads append-design <id> --content "strategy"` appends with separator

**Checkpoint:**
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo nextest run` passes all new and existing tests
- [ ] `specks beads inspect --help` prints usage
- [ ] `specks beads append-notes --help` prints usage
- [ ] Prerequisite verification results documented (bd --body-file support, worktree .beads/ discovery)

**Rollback:**
- Revert commit; no state changes outside code

**Commit after all checkpoints pass.**

---

#### Step 1: Move artifact directory into worktree {#step-1}

**Depends on:** #step-0

**Bead:** `specks-bvq.2`

**Commit:** `refactor(session): move artifacts directory into worktree`

**References:** [D05] Artifacts kept in worktree for debugging, (#strategy, #context)

**Artifacts:**
- Modified: `crates/specks-core/src/session.rs` -- update `artifacts_dir()` to return `{worktree}/.specks/artifacts/`
- Modified: `crates/specks/src/commands/worktree.rs` -- create artifacts dir inside worktree instead of external location
- Modified: `crates/specks-core/src/worktree.rs` -- remove external artifacts cleanup during worktree removal (no longer needed)

**Tasks:**
- [ ] Change `artifacts_dir()` signature from `fn artifacts_dir(repo_root: &Path, session_id: &str) -> PathBuf` to `fn artifacts_dir(worktree_path: &Path) -> PathBuf` returning `{worktree_path}/.specks/artifacts/`
- [ ] Update all callers of `artifacts_dir()`: `delete_session()`, `worktree.rs` cleanup, `worktree create` command
- [ ] Update worktree create command to create `{worktree}/.specks/artifacts/` directory instead of external path
- [ ] Remove external artifacts directory creation from worktree create flow in `commands/worktree.rs`
- [ ] Remove external artifacts cleanup from worktree removal in `worktree.rs` (artifacts are now inside the worktree and removed with it)
- [ ] Update `cleanup_orphaned_sessions()` in `worktree.rs` to remove or skip the artifact directory scanning block (currently scans `.specks-worktrees/.artifacts/` for orphaned directories), since external `.artifacts/` directories are no longer created
- [ ] Update `delete_session()` to skip artifacts cleanup (artifacts live inside worktree, not in external location)

**Tests:**
- [ ] Unit test: `artifacts_dir()` returns path inside worktree
- [ ] Integration test: `specks worktree create` creates `.specks/artifacts/` inside worktree
- [ ] Integration test: worktree removal does not leave orphaned external artifacts

**Checkpoint:**
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo nextest run` passes all tests
- [ ] `specks worktree create` places artifacts inside worktree (verify with `ls`)

**Rollback:**
- Revert commit; existing external artifacts are unaffected

**Commit after all checkpoints pass.**

---

#### Step 2: Update agent definitions for bead-mediated communication {#step-2}

**Depends on:** #step-0, #step-1

**Bead:** `specks-bvq.3`

**Commit:** `feat(agents): update all agent definitions for bead-mediated communication`

**References:** [D01] Bead is the single coordination point, [D02] All bead writes go through specks CLI, [D03] Notes field append convention, [D04] Reviewer gets Bash tool, [D05] Artifacts kept in worktree, [D07] Agents self-fetch bead content, [D08] Design field uses append command, Table T01, Table T02, Spec S01, Spec S02, Spec S03, Spec S04, Spec S05, Spec S06, (#field-ownership, #agent-data-flow, #revision-loop-behavior)

**Artifacts:**
- Modified: `agents/architect-agent.md`
- Modified: `agents/coder-agent.md`
- Modified: `agents/reviewer-agent.md`
- Modified: `agents/committer-agent.md`

**Tasks:**

*Architect agent (`agents/architect-agent.md`):*
- [ ] Update input contract: replace `artifact_dir` with `bead_id`; remove `bead_content` -- architect self-fetches via `specks beads inspect <bead_id> --json --working-dir <worktree>`
- [ ] Add behavior rule: first action is `specks beads inspect <bead_id> --json --working-dir <worktree>` to fetch `description`, `acceptance_criteria`, `design` fields
- [ ] Update output contract: architect still returns strategy JSON to orchestrator, but also appends strategy to bead `design` field via `specks beads append-design`
- [ ] Add behavior rule: write design update via `specks beads append-design <bead_id> --content "<strategy>" --working-dir <worktree>` (or `--content-file` for large strategies)
- [ ] Update artifact writing rule: write to `{worktree}/.specks/artifacts/step-N/architect-output.json` for debugging only
- [ ] Document which bead fields architect reads (description, acceptance_criteria, design) and writes (design via append)
- [ ] Update resume prompts to pass `bead_id` instead of `artifact_dir`; remove any `bead_content` passing

*Coder agent (`agents/coder-agent.md`):*
- [ ] Update input contract: replace `artifact_dir` with `bead_id`; remove `bead_content` and `strategy` -- coder self-fetches via `specks beads inspect`
- [ ] Add behavior rule: first action is `specks beads inspect <bead_id> --json --working-dir <worktree>` to fetch `description` (tasks) and `design` (references + architect strategy after separator)
- [ ] Strategy is now in the `design` field (after the `---` separator), not a separate JSON field from the architect
- [ ] Add behavior rule: after implementation, write results to bead notes via `specks beads update-notes <bead_id> --content "<notes>" --working-dir <worktree>`. On revision, coder uses `update-notes` (overwrite), not `append-notes`, to reset stale reviewer content from the previous cycle.
- [ ] Update artifact writing rule: write to `{worktree}/.specks/artifacts/step-N/coder-output.json` for debugging only
- [ ] Document which bead fields coder reads (description, design) and writes (notes via overwrite)
- [ ] Update resume prompts to pass `bead_id` instead of `artifact_dir`; remove any `bead_content` or `strategy` passing

*Reviewer agent (`agents/reviewer-agent.md`):*
- [ ] Add `Bash` to tools list in YAML frontmatter (currently Read, Grep, Glob, Write, Edit)
- [ ] Update input contract: replace `artifact_dir`, `architect_output`, and `coder_output` with just `bead_id` -- reviewer self-fetches all fields
- [ ] Add behavior rule: first action is `specks beads inspect <bead_id> --json --working-dir <worktree>` to fetch `description`, `acceptance_criteria`, `design`, `notes` (coder results)
- [ ] Add behavior rule: after review, append results to bead notes via `specks beads append-notes <bead_id> --content "<review>" --working-dir <worktree>`. On re-review after coder revision, reviewer appends fresh review below coder's new results (coder has already overwritten notes with fresh content).
- [ ] Add behavior rule: Bash tool is ONLY for `specks beads` CLI commands, not for running builds or tests
- [ ] Update artifact writing rule: write to `{worktree}/.specks/artifacts/step-N/reviewer-output.json` for debugging only
- [ ] Document which bead fields reviewer reads (description, acceptance_criteria, design, notes) and writes (notes via append)
- [ ] Update resume prompts to pass `bead_id` instead of `artifact_dir` and separate outputs; remove any `bead_content` passing

*Committer agent (`agents/committer-agent.md`):*
- [ ] Update commit mode input contract: committer receives `bead_id` (for `--bead` flag on `specks step-commit`) and `log_entry` (constructed by orchestrator from coder/reviewer Task responses). Committer does NOT self-fetch from the bead -- the orchestrator already has all needed data from agent return values.
- [ ] Remove references to artifact directory in committer documentation
- [ ] Document that committer writes `close_reason` via `bd close` (through `specks step-commit --close-reason`) but does not read bead fields

**Tests:**
- [ ] Verify all four agent definitions have valid markdown with correct YAML frontmatter
- [ ] Verify all input/output contracts are valid JSON schemas
- [ ] Verify reviewer-agent.md includes `Bash` in tools list

**Checkpoint:**
- [ ] `grep -r 'artifact_dir' agents/` returns no matches in input contracts (only in debug artifact write rules)
- [ ] All four agent definitions reference `bead_id` not `artifact_dir` in input contracts
- [ ] All agent definitions contain `specks beads inspect` in their behavior rules
- [ ] No `bead_content`, `architect_output`, `coder_output`, or `strategy` objects in any agent input contract

**Rollback:**
- Revert to previous agent definitions

**Commit after all checkpoints pass.**

---

#### Step 3: Update implementer skill for bead-mediated orchestration {#step-3}

**Depends on:** #step-2

**Bead:** `specks-bvq.5`

**Commit:** `feat(implementer): orchestrate via bead_id instead of artifact paths`

**References:** [D01] Bead is the single coordination point, [D07] Agents self-fetch bead content, Table T02, Diagram Diag01, (#agent-data-flow, #strategy)

**Artifacts:**
- Modified: `skills/implementer/SKILL.md`

**Tasks:**
- [ ] Remove `artifact_dir` construction from per-step loop (currently `<artifacts_base>/step-N`)
- [ ] Remove all `bd show` or `specks beads inspect` calls from the orchestrator -- the orchestrator MUST NOT run any CLI commands; it is a pure dispatcher with only `Task` and `AskUserQuestion`
- [ ] Update architect spawn/resume prompts: replace `artifact_dir` with `bead_id` and `worktree_path`; do NOT pass `bead_content` (agent self-fetches)
- [ ] Update coder spawn/resume prompts: replace `artifact_dir` and `strategy` with `bead_id` and `worktree_path`; do NOT pass `bead_content` or `strategy` (agent self-fetches from bead design field)
- [ ] Update reviewer spawn/resume prompts: replace `artifact_dir`, `architect_output`, `coder_output` with `bead_id` and `worktree_path`; do NOT pass `bead_content` (agent self-fetches)
- [ ] Update committer spawn/resume prompts: pass `bead_id` (for `--bead` flag) and `log_entry` (constructed from coder/reviewer Task responses, same as today). Committer does NOT self-fetch -- orchestrator continues constructing `log_entry` from its accumulated agent responses.
- [ ] Orchestrator still receives agent JSON output (strategy, coder results, review) for decision-making (drift check, REVISE/APPROVE/ESCALATE) -- agents return this in their response, orchestrator does not need to read beads
- [ ] Remove `artifacts_base` from session stored data
- [ ] Update progress reporting messages to remove artifact references
- [ ] Simplify per-step loop: orchestrator only needs `bead_id` (from `bead_mapping`) and `worktree_path` to dispatch any agent
- [ ] Update the context-exhaustion recovery path (continuation mode): when spawning a fresh coder mid-step with `continuation: true` and `files_already_modified`, pass `bead_id` and `worktree_path` instead of `artifact_dir` and `strategy`. The continuation coder self-fetches from the bead to get current design (which includes the architect strategy). Notes field may be empty if the previous coder did not complete -- this is expected.

**Tests:**
- [ ] Verify skill definition has correct YAML frontmatter
- [ ] Verify orchestration loop passes only `bead_id` and `worktree_path` to agents (no `bead_content`, `artifact_dir`, or CLI calls)

**Checkpoint:**
- [ ] No references to `artifact_dir`, `artifacts_base`, `bead_content`, `bd show`, or `specks beads inspect` in implementer SKILL.md
- [ ] Orchestrator uses only `Task` and `AskUserQuestion` tools -- no Bash, Read, or other direct tools
- [ ] `bead_id` appears in all agent spawn/resume prompts

**Rollback:**
- Revert to previous skill definition

**Commit after all checkpoints pass.**

---

#### Step 4: Update setup agent for new artifact path {#step-4}

**Depends on:** #step-1, #step-3

**Bead:** `specks-bvq.6`

**Commit:** `feat(setup): update setup agent for worktree-local artifacts and bead-mediated flow`

**References:** [D05] Artifacts kept in worktree for debugging, (#strategy)

**Artifacts:**
- Modified: `agents/implementer-setup-agent.md`

**Tasks:**
- [ ] Update output contract: remove `artifacts_base` from session object (or change to worktree-local path)
- [ ] If artifacts are still needed in output, change path to `{worktree_path}/.specks/artifacts/`
- [ ] Ensure setup agent does not create external artifact directories
- [ ] Update all artifact path references in setup agent documentation to use `{worktree}/.specks/artifacts/`
- [ ] Update documentation to reflect that artifacts are inside worktree

**Tests:**
- [ ] Verify agent definition is valid markdown with correct YAML frontmatter
- [ ] Verify output contract is consistent with implementer skill expectations

**Checkpoint:**
- [ ] Setup agent output references worktree-local artifacts path or omits artifacts_base entirely
- [ ] No references to external artifacts path (`<repo>/.specks-worktrees/.artifacts/`) in setup agent definition

**Rollback:**
- Revert to previous agent definition

**Commit after all checkpoints pass.**

---

#### Step 5: Integration verification and cleanup {#step-5}

**Depends on:** #step-3, #step-4

**Bead:** `specks-bvq.7`

**Commit:** `test(beads): add integration tests for bead-mediated agent communication`

**References:** [D01] Bead is the single coordination point, [D03] Notes field append convention, Spec S01, Spec S02, Table T01, (#success-criteria)

**Artifacts:**
- New or modified test files verifying end-to-end bead communication
- Modified: any remaining references to external artifact paths

**Tasks:**
- [ ] Add integration test: architect appends to design field via `specks beads append-design`, content is retrievable via `specks beads inspect`
- [ ] Add integration test: coder writes to notes, reviewer appends below separator, combined content preserved
- [ ] Add integration test: notes field follows the separator convention (Spec S02)
- [ ] Audit codebase for remaining references to external artifacts path (`<repo>/.specks-worktrees/.artifacts/`)
- [ ] Remove any dead code related to external artifacts directory management
- [ ] Verify `specks doctor` handles new artifact location correctly

**Tests:**
- [ ] Integration test: full append-notes round-trip (write, append, read back)
- [ ] Integration test: `bd close` with reason records commit info
- [ ] Golden test: bead content after full architect + coder + reviewer cycle matches expected format

**Checkpoint:**
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo nextest run` passes all tests
- [ ] `grep -r '.specks-worktrees/.artifacts' crates/` returns no production code matches (only test fixtures if any)
- [ ] `specks beads inspect <step-id> --json` after mock implementation shows all fields populated

**Rollback:**
- Revert commit; tests are additive

**Commit after all checkpoints pass.**

---

### 2.0.6 Deliverables and Checkpoints {#deliverables}

**Deliverable:** All implementation agent communication flows through bead fields instead of artifact files, with new specks CLI subcommands for bead field updates and debugging artifacts preserved in the worktree.

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

- [ ] `specks beads inspect`, `specks beads append-design`, `specks beads update-notes`, `specks beads append-notes` CLI commands exist and pass tests
- [ ] No agent reads another agent's artifact file during normal workflow (grep of agent definitions confirms)
- [ ] `specks beads inspect <step-id>` after implementation shows: description (from sync), design (references + architect strategy), notes (coder results + reviewer review), close_reason (commit info)
- [ ] Implementer skill passes only `bead_id` and `worktree_path` to agents -- no `bead_content`, `artifact_dir`, or CLI calls in orchestrator
- [ ] Artifacts directory lives inside worktree at `{worktree}/.specks/artifacts/`
- [ ] All tests pass: `cargo nextest run`

**Acceptance tests:**
- [ ] Integration test: `specks beads append-notes` preserves existing content and adds separator
- [ ] Integration test: full agent cycle produces correct bead field contents
- [ ] Unit test: `BeadsCli::append_notes()` correctly concatenates with separator

#### Roadmap / Follow-ons (Explicitly Not Required for Phase Close) {#roadmap}

- [ ] Phase C: Eliminate Session -- replace session JSON with bead-derived state
- [ ] Phase D: Status from Beads -- derive implementation status entirely from bead queries
- [ ] Remove `artifacts_base` from session JSON schema entirely (can be done in Phase C)

| Checkpoint | Verification |
|------------|--------------|
| CLI subcommands work | `specks beads inspect --help` and `specks beads append-notes --help` print usage |
| No artifact-based communication | `grep -r 'artifact_dir' agents/` shows only debug writes |
| Bead fields populated | `specks beads inspect <step-id> --json` returns all expected fields |
| Orchestrator is pure dispatcher | No `Bash`, `bd show`, or `specks beads` calls in SKILL.md |
| Tests pass | `cargo nextest run` exits 0 |

**Commit after all checkpoints pass.**