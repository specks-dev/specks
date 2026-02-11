## Phase 5.0: Rich Beads Sync {#phase-rich-sync}

**Purpose:** Enrich `specks beads sync` to populate bead `description`, `acceptance_criteria`, and `design` fields with full step content, transforming thin pointer beads into self-contained work items.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | kocienda |
| Status | draft |
| Target branch | main |
| Tracking issue/PR | — |
| Last updated | 2026-02-11 |
| Beads Root | `specks-v3v` |

---

### Phase Overview {#phase-overview}

#### Context {#context}

Today, `specks beads sync` creates thin beads where each step bead gets a title and a one-line description that is just a pointer back to the speck file (e.g., `"Specks: .specks/specks-5.md#step-0\nCommit: feat(api): add client"`). Every agent that needs step requirements must open the speck, find the right section, and extract tasks/tests/checkpoints. The bead is a tracking token, not a work item.

This is Phase A of the beads enrichment plan documented in `.specks/beads-enrichment-phases.md`. It is the foundation that all subsequent phases (B: Agent-Bead Communication, C: Eliminate Session File, D: Status from Beads) depend on.

#### Strategy {#strategy}

- Extend the parser to capture Artifacts sections from steps, adding an `artifacts` field to `Step` and `Substep` in `types.rs`.
- Add content-rendering methods to the `Step` and `Substep` types in `types.rs` so step content can be materialized as markdown strings.
- Add raw section extraction to the `Speck` type for overview sections (purpose, strategy, success criteria) that the parser does not currently capture as structured fields.
- Extend `IssueDetails` in `beads.rs` with optional fields (`design`, `acceptance_criteria`, `notes`) so the full bead response can be deserialized.
- Add `BeadsCli` methods for updating bead content fields (`update_description`, `update_design`, `update_acceptance`) and extend `create`/`create_with_deps` to accept optional rich content fields.
- Extend `bd-fake` test mock to support the `update` command and rich field storage/retrieval (`design`, `acceptance_criteria`, `notes`), and extend `create` to accept `--design`, `--acceptance`, `--notes` flags.
- Modify sync logic in `sync.rs` to generate and write rich content for both root and step beads, using the `--enrich` flag to control overwrite behavior.
- Follow a best-effort error model: sync as many beads as possible, collect errors, report them at the end.

#### Stakeholders / Primary Customers {#stakeholders}

1. Implementation agents (architect, coder, reviewer) that consume step bead content
2. Human developers using `bd show` to inspect step requirements

#### Success Criteria (Measurable) {#success-criteria}

- `bd show <step-bead-id> --json` returns `description`, `acceptance_criteria`, and `design` fields with meaningful markdown content after a `specks beads sync --enrich` run.
- `bd show <root-bead-id> --json` returns a `description` containing purpose, strategy, and success criteria from the speck.
- All existing sync tests continue to pass without modification (backward compatibility).
- `cargo nextest run` passes with zero warnings.

#### Scope {#scope}

1. Parser extension to capture Artifacts sections into `Step` and `Substep` types
2. Content-rendering methods on `Step`, `Substep`, and `Speck` types
3. `IssueDetails` struct extensions for additional bead fields (`design`, `acceptance_criteria`, `notes`)
4. New `BeadsCli` methods for updating description, design, and acceptance_criteria; extended `create` methods with optional rich content fields
5. `bd-fake` test mock extended with `update` command and rich field support
6. `--enrich` CLI flag on `specks beads sync`
7. Enrichment logic in `sync.rs` for root and step beads
8. Unit, integration, and golden tests for all new functions
9. Move beads sync from implementer-setup to planner finalization (beads creation is a planning output)

#### Non-goals (Explicitly out of scope) {#non-goals}

- Agent-bead communication (Phase B) -- agents do not read/write bead fields in this phase
- Session file elimination (Phase C) -- no session changes
- Status from beads (Phase D) -- no status command changes
- Substep-level bead enrichment beyond what the parent step already covers
- Writing to the `notes` field -- reserved for Phase B agent runtime use
- Changes to the implementer-setup-agent or `specks worktree create` -- sync inside worktree create is already idempotent and becomes a fast no-op when beads exist from planning

#### Dependencies / Prerequisites {#dependencies}

- `bd` CLI must support `update --description`, `update --design`, and `update --acceptance` subcommands. Verified: these exist in `bd update` and `bd create`.
- Existing `specks beads sync` must be working correctly.

#### Constraints {#constraints}

- All beads interaction goes through `BeadsCli` in `beads.rs` (invariant from beads-enrichment-phases.md).
- Warnings are errors: `-D warnings` is enforced via `.cargo/config.toml`.
- No `std::env::set_current_dir` in tests.

#### Assumptions {#assumptions}

- The `bd` CLI commands for field updates (`bd update --description/--design/--acceptance`) exist and work. Verified against `bd update --help`.
- Markdown content generation uses H2 (`##`) headings for top-level sections within bead fields.
- By default (without `--enrich`), sync only populates content fields for newly created beads. With `--enrich`, overwrite all fields unconditionally.
- Content extraction helpers are added to `types.rs` as `Step` and `Speck` methods.
- The `IssueDetails` struct extensions use `#[serde(default)]` for backward compatibility.
- Decision text in the design field uses title only (e.g., `[D01] REST API client`), not the full decision body.
- The `bd-fake` test mock must be extended before integration tests can run. This is explicit work in Step 1.

---

### 5.0.0 Design Decisions {#design-decisions}

#### [D01] Overwrite all fields unconditionally when enriching (DECIDED) {#d01-enrich-overwrite}

**Decision:** When `--enrich` is passed, overwrite `description`, `acceptance_criteria`, and `design` fields on all beads unconditionally, ensuring sync consistency with the current speck content.

**Rationale:**
- Eliminates stale content risk when speck is edited after initial sync.
- Simpler implementation: no diffing or merge logic needed.
- The speck is the authoritative source; bead content is a derived projection.

**Implications:**
- Any manual edits to bead content fields will be lost on re-sync with `--enrich`.
- Without `--enrich`, existing beads keep their current content (only new beads get rich content).

---

#### [D02] Root bead description contains purpose, strategy, and success criteria (DECIDED) {#d02-root-content}

**Decision:** The root bead `description` field contains the speck purpose statement, strategy bullets, and success criteria, extracted from the raw speck content by section heading location.

**Rationale:**
- These three sections give the most useful overview of the phase at a glance.
- The parser already captures `purpose`; strategy and success criteria can be extracted from `raw_content` by locating anchors.

**Implications:**
- Must add a `Speck::extract_section_content(anchor)` helper to pull raw markdown between headings.
- Root bead `design` field gets a summary of design decisions (ID + title only per [D03]).
- Root bead `acceptance_criteria` field gets the phase exit criteria from the Deliverables section.

---

#### [D03] Decision text uses title only in design field (DECIDED) {#d03-decision-text}

**Decision:** When rendering the design field for a step bead, decision references like `[D01]` are expanded to `[D01] <title>` using `Decision.title` from the parsed speck, not the full decision body.

**Rationale:**
- Keeps the design field concise and scannable.
- Full decision text would bloat the field and duplicate content already in the speck.
- Agents that need full decision context can look up the decision by ID.

**Implications:**
- The `resolve_step_references` helper needs access to the parsed `Speck.decisions` list to map IDs to titles.

---

#### [D04] Best-effort sync with error collection (DECIDED) {#d04-best-effort}

**Decision:** Sync as many beads as possible, collecting enrichment errors per bead, and report all errors at the end rather than failing on the first error.

**Rationale:**
- A single bead's `bd update` failure should not block enrichment of the remaining beads.
- Matches the existing sync pattern where individual bead operations are independent.

**Implications:**
- Need an `EnrichmentError` collection in the sync loop.
- Exit code and JSON output must reflect partial success (e.g., `"warnings"` array in JSON).
- Text output lists which beads failed and why.

---

#### [D05] Content rendered as markdown with H2 headings (DECIDED) {#d05-markdown-format}

**Decision:** Bead content fields use markdown formatting with H2 (`##`) headings for top-level sections (e.g., `## Tasks`, `## Tests`, `## Checkpoints`).

**Rationale:**
- Consistent with the beads enrichment phases convention document.
- H2 is appropriate since the bead title serves as the implicit H1.
- Markdown renders well in `bd show` output and web interfaces.

**Implications:**
- All content-rendering methods produce markdown strings.
- Checkbox items are rendered with their original `- [ ]` / `- [x]` syntax.

---

#### [D06] New BeadsCli methods use bd update subcommand (DECIDED) {#d06-beads-cli-update}

**Decision:** New `BeadsCli` methods for updating bead fields use `bd update <id> --description <content>` (and similar flags for `--design`, `--acceptance`). The `create` and `create_with_deps` methods are extended with optional `design`, `acceptance`, and `notes` parameters so new beads get rich content in a single CLI call.

**Rationale:**
- `bd update` is the documented command for modifying existing beads (verified via `bd update --help`).
- `bd create` also accepts `--design`, `--acceptance`, `--notes` (verified via `bd create --help`), enabling single-call creation with rich content.
- Separating update from create keeps existing callers unchanged.

**Implications:**
- Each update method follows the same error-handling pattern as existing `BeadsCli` methods.
- The `create`/`create_with_deps` signature extensions use `Option` parameters with default `None` to maintain backward compatibility.

---

### 5.0.1 Bead Field Mapping {#bead-field-mapping}

**Table T01: Step Bead Field Mapping** {#t01-step-bead-fields}

| Bead Field | Content | Source in Speck |
|-----------|---------|-----------------|
| `title` | `"Step N: <title>"` | Step heading (already done) |
| `description` | Full work specification: tasks, artifacts, commit template | Step's Tasks, Artifacts, and Commit sections |
| `acceptance_criteria` | Verification requirements: tests + checkpoints | Step's Tests and Checkpoint sections |
| `design` | Plan references: decision titles and anchor references | Step's References line, with `[D01]` expanded to title |

**Table T02: Root Bead Field Mapping** {#t02-root-bead-fields}

| Bead Field | Content | Source in Speck |
|-----------|---------|-----------------|
| `title` | Phase title from speck | Phase heading (already done) |
| `description` | Purpose + Strategy + Success Criteria | Phase Overview sections |
| `design` | Summary of design decisions (ID + title) | Design Decisions section |
| `acceptance_criteria` | Phase exit criteria | Deliverables section |

---

### 5.0.2 Content Generation Templates {#content-templates}

**Spec S01: Step Description Template** {#s01-step-description}

```markdown
## Tasks
- [ ] Create `src/api/client.rs` with retry logic
- [ ] Add `reqwest` dependency to Cargo.toml

## Artifacts
- New file: `src/api/client.rs`
- Modified: `Cargo.toml`

## Commit Template
feat(api): add client with retry support
```

**Spec S02: Step Acceptance Criteria Template** {#s02-step-acceptance}

```markdown
## Tests
- [ ] Unit test: retry with exponential backoff
- [ ] Integration test: client connects to mock server

## Checkpoints
- [ ] `cargo test` passes
- [ ] `cargo clippy` clean
```

**Spec S03: Step Design Template** {#s03-step-design}

```markdown
## References
- [D01] REST API client
- [D03] Retry strategy
- Anchors: #inputs-outputs, #error-scenarios
```

**Spec S04: Root Description Template** {#s04-root-description}

```markdown
## Purpose
<purpose statement from speck>

## Strategy
<strategy bullets from speck>

## Success Criteria
<success criteria from speck>
```

---

### 5.0.3 Symbol Inventory {#symbol-inventory}

#### 5.0.3.1 New files (if any) {#new-files}

No new crate files. Test mock is extended. Golden test fixtures are new.

#### 5.0.3.2 Symbols to add / modify {#symbols}

**Table T03: New and Modified Symbols** {#t03-symbols}

| Symbol | Kind | Location | Notes |
|--------|------|----------|-------|
| `Step.artifacts` | field | `types.rs` | `Vec<String>` — artifact items captured by parser |
| `Substep.artifacts` | field | `types.rs` | `Vec<String>` — artifact items captured by parser |
| `CurrentSection::Artifacts` | enum variant | `parser.rs` | New parser section state for Artifacts content |
| `Step::render_description()` | method | `types.rs` | Renders tasks, artifacts, and commit template as markdown |
| `Step::render_acceptance_criteria()` | method | `types.rs` | Renders tests + checkpoints as markdown |
| `Substep::render_description()` | method | `types.rs` | Same as Step but for substeps |
| `Substep::render_acceptance_criteria()` | method | `types.rs` | Same as Step but for substeps |
| `Speck::extract_section_by_anchor()` | method | `types.rs` | Extracts raw markdown between a heading with an anchor and the next same-level heading |
| `Speck::render_root_description()` | method | `types.rs` | Renders purpose + strategy + success criteria |
| `Speck::render_root_design()` | method | `types.rs` | Renders decision summary (ID + title) |
| `Speck::render_root_acceptance()` | method | `types.rs` | Renders phase exit criteria from deliverables |
| `resolve_step_design()` | fn | `sync.rs` | Expands `[D01]` references in step references to decision titles |
| `enrich_root_bead()` | fn | `sync.rs` | Enriches root bead with content from speck overview |
| `enrich_step_bead()` | fn | `sync.rs` | Enriches step bead with description, acceptance, design |
| `IssueDetails.design` | field | `beads.rs` | Optional String, `#[serde(default)]` |
| `IssueDetails.acceptance_criteria` | field | `beads.rs` | Optional String, `#[serde(default)]` |
| `IssueDetails.notes` | field | `beads.rs` | Optional String, `#[serde(default)]` |
| `BeadsCli::update_description()` | method | `beads.rs` | Calls `bd update <id> --description <content>` |
| `BeadsCli::update_design()` | method | `beads.rs` | Calls `bd update <id> --design <content>` |
| `BeadsCli::update_acceptance()` | method | `beads.rs` | Calls `bd update <id> --acceptance <content>` |
| `BeadsCli::create()` | method (modified) | `beads.rs` | Extended with optional `design`, `acceptance`, `notes` params |
| `BeadsCli::create_with_deps()` | method (modified) | `beads.rs` | Extended with optional `design`, `acceptance`, `notes` params |
| `SyncOptions.update_title` | field (removed) | `sync.rs` | Dead code, replaced by `--enrich` |
| `SyncOptions.update_body` | field (removed) | `sync.rs` | Dead code, replaced by `--enrich` |
| `SyncOptions.enrich` | field | `sync.rs` | Boolean flag for enrichment mode |

---

### 5.0.4 Test Plan Concepts {#test-plan-concepts}

#### Test Categories {#test-categories}

| Category | Purpose | When to use |
|----------|---------|-------------|
| **Unit** | Test content rendering methods in isolation | `render_description`, `render_acceptance_criteria`, `resolve_step_design`, section extraction |
| **Integration** | Test full sync with enrichment against beads CLI | `specks beads sync --enrich` end-to-end |
| **Golden** | Compare rendered content against known-good snapshots | Step description/acceptance/design output |

---

### 5.0.5 Execution Steps {#execution-steps}

#### Step 0: Extend parser for Artifacts and add content-rendering methods {#step-0}

**Bead:** `specks-v3v.1`

**Commit:** `feat(types): capture artifacts in parser and add content-rendering methods`

**References:** [D05] Content rendered as markdown with H2 headings, [D02] Root bead description contains purpose strategy and success criteria, [D03] Decision text uses title only in design field, Table T01, Table T02, Spec S01, Spec S02, Spec S04, (#bead-field-mapping, #content-templates, #symbols)

**Artifacts:**
- Modified: `crates/specks-core/src/types.rs`
- Modified: `crates/specks-core/src/parser.rs`

**Tasks:**
- [ ] Add `artifacts: Vec<String>` field to `Step` struct in `types.rs` (with `#[serde(default)]`). Each entry is a text line from the Artifacts section (e.g., `"New file: src/api/client.rs"`).
- [ ] Add `artifacts: Vec<String>` field to `Substep` struct in `types.rs` (same pattern).
- [ ] Add `CurrentSection::Artifacts` variant to the `CurrentSection` enum in `parser.rs`.
- [ ] In `parser.rs`, change the heading check that routes `"artifacts"` to `CurrentSection::Other` to route to `CurrentSection::Artifacts` instead.
- [ ] Add a `**Artifacts:**` bold-marker check in `parser.rs` (parallel to existing `**Tasks:**`, `**Tests:**`, `**Checkpoint:**` checks) that sets `current_section = CurrentSection::Artifacts`.
- [ ] In the checkbox/content parsing section of `parser.rs`, handle `CurrentSection::Artifacts` in two places: (a) In the checkbox match arm, when `current_section` is `Artifacts`, strip the bracket notation and push the text as a plain artifact item (e.g., `- [ ] some artifact` becomes `"some artifact"` in `step.artifacts`). (b) After the checkbox match, capture plain `- <text>` bullet lines under `CurrentSection::Artifacts` and push onto `step.artifacts` or `substep.artifacts`.
- [ ] Add `Step::render_description(&self) -> String` -- renders Tasks, Artifacts, and Commit Template as markdown with H2 headings. Tasks rendered as `- [ ] <text>` / `- [x] <text>` from `self.tasks`; Artifacts rendered as `- <text>` from `self.artifacts`; Commit Template from `self.commit_message`. Omit sections that are empty.
- [ ] Add `Step::render_acceptance_criteria(&self) -> String` -- renders Tests and Checkpoints sections as markdown with H2 headings, preserving checkbox syntax. Omit sections that are empty.
- [ ] Add `Substep::render_description(&self) -> String` and `Substep::render_acceptance_criteria(&self) -> String` -- mirror the Step methods using Substep fields.
- [ ] Add `Speck::extract_section_by_anchor(&self, anchor: &str) -> Option<String>` -- locates a heading with `{#<anchor>}` in `raw_content` and returns all content up to the next heading of the same or higher level. Returns `None` if anchor not found. Must handle: nested headings at different levels, last section in document (returns to end), and sections with no content.
- [ ] Add `Speck::render_root_description(&self) -> String` -- composes Purpose (from `self.purpose`), Strategy (from `extract_section_by_anchor("strategy")`), and Success Criteria (from `extract_section_by_anchor("success-criteria")`) under H2 headings.
- [ ] Add `Speck::render_root_design(&self) -> String` -- iterates `self.decisions` and renders each as `- [<id>] <title>` under an H2 References heading.
- [ ] Add `Speck::render_root_acceptance(&self) -> String` -- extracts exit criteria from `extract_section_by_anchor("exit-criteria")` or `extract_section_by_anchor("deliverables")`.

**Tests:**
- [ ] Unit test: Parser captures artifact items from `**Artifacts:**` section into `Step.artifacts`
- [ ] Unit test: Parser captures artifact items from heading-style `Artifacts:` section
- [ ] Unit test: Existing tests still pass (parser backward compatibility)
- [ ] Unit test: `Step::render_description` produces expected markdown from a Step with tasks, artifacts, and commit message
- [ ] Unit test: `Step::render_description` omits empty sections (e.g., no Artifacts heading if `artifacts` is empty)
- [ ] Unit test: `Step::render_acceptance_criteria` produces expected markdown from a Step with tests and checkpoints
- [ ] Unit test: `Speck::extract_section_by_anchor` extracts correct content for known anchors
- [ ] Unit test: `Speck::extract_section_by_anchor` returns None for missing anchors
- [ ] Unit test: `Speck::extract_section_by_anchor` handles last section in document (no following heading)
- [ ] Unit test: `Speck::extract_section_by_anchor` handles nested sub-headings correctly (includes them, stops at same-or-higher level)
- [ ] Unit test: `Speck::render_root_description` includes purpose, strategy, and success criteria
- [ ] Unit test: `Speck::render_root_design` lists all decisions by ID and title

**Checkpoint:**
- [ ] `cargo nextest run -p specks-core` passes with zero warnings
- [ ] `cargo clippy -p specks-core` clean

**Rollback:**
- Revert the commit; no external state changes.

**Commit after all checkpoints pass.**

---

#### Step 1: Extend BeadsCli, IssueDetails, and bd-fake for rich fields {#step-1}

**Depends on:** #step-0

**Bead:** `specks-v3v.2`

**Commit:** `feat(beads): add rich field support to BeadsCli, IssueDetails, and bd-fake`

**References:** [D06] New BeadsCli methods use bd update subcommand, (#symbols, #bead-field-mapping)

**Artifacts:**
- Modified: `crates/specks-core/src/beads.rs`
- Modified: `tests/bin/bd-fake`

**Tasks:**

*IssueDetails extensions:*
- [ ] Add optional fields to `IssueDetails`: `design: Option<String>`, `acceptance_criteria: Option<String>`, `notes: Option<String>`, all with `#[serde(default)]`.

*BeadsCli update methods:*
- [ ] Add `BeadsCli::update_description(&self, id: &str, content: &str) -> Result<(), SpecksError>` -- runs `bd update <id> --description <content>`.
- [ ] Add `BeadsCli::update_design(&self, id: &str, content: &str) -> Result<(), SpecksError>` -- runs `bd update <id> --design <content>`.
- [ ] Add `BeadsCli::update_acceptance(&self, id: &str, content: &str) -> Result<(), SpecksError>` -- runs `bd update <id> --acceptance <content>`.

*BeadsCli create extensions:*
- [ ] Extend `BeadsCli::create()` signature with optional parameters: `design: Option<&str>`, `acceptance: Option<&str>`, `notes: Option<&str>`. When provided, pass `--design`, `--acceptance`, `--notes` flags to `bd create`. Existing callers pass `None` for these (backward compatible).
- [ ] Extend `BeadsCli::create_with_deps()` with the same optional parameters.

*bd-fake test mock extensions:*
- [ ] Extend `bd-fake` state schema: issue objects in `issues.json` now store `design`, `acceptance_criteria`, and `notes` fields (default to empty string).
- [ ] Add `cmd_update()` function to `bd-fake`: supports `bd update <id> --description <content> --design <content> --acceptance <content> --notes <content>`. Updates the specified fields in `issues.json`. Returns success/failure.
- [ ] Extend `cmd_create()` in `bd-fake`: accept `--design`, `--acceptance`, `--notes` flags and store them in the created issue.
- [ ] Extend `cmd_show()` in `bd-fake`: include `design`, `acceptance_criteria`, and `notes` fields in output JSON.
- [ ] Add `update` to the case statement in `bd-fake`'s command dispatch.

**Tests:**
- [ ] Unit test: `IssueDetails` deserializes correctly with and without the new optional fields (backward compatibility).
- [ ] Unit test: `IssueDetails` with all fields populated round-trips through serde correctly.
- [ ] Integration test: `bd-fake update` sets fields and `bd-fake show` returns them.
- [ ] Integration test: `bd-fake create` with `--design` and `--acceptance` stores and returns fields.

**Checkpoint:**
- [ ] `cargo nextest run -p specks-core` passes with zero warnings
- [ ] `cargo clippy -p specks-core` clean
- [ ] Existing beads integration tests still pass (backward compatibility of bd-fake changes)

**Rollback:**
- Revert the commit; no external state changes.

**Commit after all checkpoints pass.**

---

#### Step 2: Add --enrich flag and enrichment logic to sync command {#step-2}

**Depends on:** #step-0, #step-1

**Bead:** `specks-v3v.3`

**Commit:** `feat(sync): add --enrich flag with rich bead content population`

**References:** [D01] Overwrite all fields unconditionally when enriching, [D02] Root bead description contains purpose strategy and success criteria, [D03] Decision text uses title only in design field, [D04] Best-effort sync with error collection, [D05] Content rendered as markdown with H2 headings, [D06] New BeadsCli methods use bd update subcommand, Table T01, Table T02, Spec S01, Spec S02, Spec S03, Spec S04, (#bead-field-mapping, #content-templates, #strategy, #symbols)

**Artifacts:**
- Modified: `crates/specks/src/commands/beads/sync.rs`
- Modified: `crates/specks/src/cli.rs`
- Modified: `crates/specks/src/main.rs`
- Modified: `crates/specks/src/commands/init.rs` (default config template)

**Tasks:**

*Remove dead `--update-title` and `--update-body` flags:*
- [ ] Remove `update_title` and `update_body` fields from `SyncOptions` in `sync.rs` (currently `#[allow(dead_code)]`).
- [ ] Remove `--update-title` and `--update-body` arg definitions from `BeadsCommands::Sync` in `cli.rs`.
- [ ] Remove `update_title` and `update_body` from the destructuring in `main.rs` where CLI args map to `SyncOptions`.
- [ ] Remove `update_title = false` and `update_body = false` lines from the default config template in `init.rs`.

*Add `--enrich` flag:*
- [ ] Add `--enrich` flag to the `beads sync` CLI subcommand in `cli.rs`.
- [ ] Add `enrich: bool` field to `SyncOptions`.
- [ ] Wire `enrich` from CLI args to `SyncOptions` in `main.rs`.
- [ ] Add `enrich` field to `SyncContext`.
- [ ] Implement `resolve_step_design(step: &Step, speck: &Speck) -> String` -- parses the step's `references` line, expands `[DNN]` patterns by looking up `speck.decisions` to find matching ID, renders as markdown with decision titles per [D03]. Unresolved references are passed through as-is. Anchor references (e.g., `#anchor-name`) are listed separately.
- [ ] Implement `enrich_root_bead(speck: &Speck, root_id: &str, speck_path: &str, ctx: &SyncContext) -> Result<Vec<String>, SpecksError>` -- calls `speck.render_root_description()`, `speck.render_root_design()`, `speck.render_root_acceptance()`, then uses `BeadsCli` update methods to write each field. Returns a vec of error messages (best-effort per [D04]).
- [ ] Implement `enrich_step_bead(step: &Step, bead_id: &str, speck: &Speck, speck_path: &str, ctx: &SyncContext) -> Result<Vec<String>, SpecksError>` -- calls `step.render_description()`, `step.render_acceptance_criteria()`, `resolve_step_design()`, then uses `BeadsCli` update methods to write description, acceptance_criteria, and design. Returns a vec of error messages.
- [ ] Integrate enrichment into `sync_speck_to_beads()`: after creating/verifying beads, if `ctx.enrich` is true, call `enrich_root_bead` and `enrich_step_bead` for each step. Collect errors and report at end.
- [ ] Update JSON output to include enrichment results: add `enriched: bool` and optional `enrich_errors: Vec<String>` to `SyncData`.
- [ ] Update text output to report enrichment status and any errors.

**Tests:**
- [ ] Unit test: `resolve_step_design` expands `[D01]` reference to decision title
- [ ] Unit test: `resolve_step_design` handles references with no matching decisions gracefully (passes through as-is)
- [ ] Unit test: `resolve_step_design` handles anchor references
- [ ] Integration test: full sync with `--enrich` flag on a test speck using bd-fake

**Checkpoint:**
- [ ] `cargo nextest run` passes with zero warnings (all crates)
- [ ] `cargo clippy` clean (all crates)
- [ ] `specks beads sync --help` shows the `--enrich` flag and does NOT show `--update-title` or `--update-body`

**Rollback:**
- Revert the commit; no external state changes. Existing sync behavior is unaffected when `--enrich` is not passed.

**Commit after all checkpoints pass.**

---

#### Step 3: Enrich new beads by default during creation {#step-3}

**Depends on:** #step-2

**Bead:** `specks-v3v.4`

**Commit:** `feat(sync): populate rich content on newly created beads`

**References:** [D01] Overwrite all fields unconditionally when enriching, [D04] Best-effort sync with error collection, [D05] Content rendered as markdown with H2 headings, (#bead-field-mapping, #strategy)

**Artifacts:**
- Modified: `crates/specks/src/commands/beads/sync.rs`

**Tasks:**

*Refactor ensure functions to use `SyncContext`:*
- [ ] Refactor `ensure_step_bead()` to take `&SyncContext` instead of individual `beads`, `dry_run`, `quiet` parameters. Add `speck: &Speck` and `existing_ids: &HashSet<String>` as remaining params. This makes it consistent with `ensure_root_bead()` which already takes `&SyncContext`.
- [ ] Refactor `ensure_substep_bead()` the same way: take `&SyncContext` + `speck` + `existing_ids` instead of individual fields.
- [ ] Update all call sites in `sync_speck_to_beads()` to pass `ctx` instead of destructured fields.

*Enrich at creation time:*
- [ ] Modify `ensure_root_bead()`: when a new root bead is created (not found in `existing_ids`), pass rich content via the extended `BeadsCli::create()` method — `description` from `speck.render_root_description()`, `design` from `speck.render_root_design()`, `acceptance` from `speck.render_root_acceptance()`. This happens regardless of `--enrich` flag. Add `speck: &Speck` parameter to the function signature.
- [ ] Modify `ensure_step_bead()`: when a new step bead is created, pass rich content via `BeadsCli::create()` — `description` from `step.render_description()`, `acceptance` from `step.render_acceptance_criteria()`, `design` from `resolve_step_design()`. Enrichment at creation uses a single `bd create` call (no separate update calls).
- [ ] Modify `ensure_substep_bead()` similarly for substep beads when using `--substeps children` mode.
- [ ] Track which beads were just created (return a `created: bool` alongside the bead ID). In the enrichment loop (Step 2), skip `enrich_step_bead` / `enrich_root_bead` for beads that were just created and already have rich content. This prevents double-updating when `--enrich` is also passed.

**Tests:**
- [ ] Integration test: newly created bead gets rich content even without `--enrich` (verify via `bd-fake show`)
- [ ] Integration test: existing bead is NOT enriched without `--enrich` flag
- [ ] Integration test: existing bead IS enriched with `--enrich` flag
- [ ] Integration test: dry-run mode does not create or update beads
- [ ] Integration test: with `--enrich`, newly created beads are not double-updated (verify single create call, no update call)

**Checkpoint:**
- [ ] `cargo nextest run` passes with zero warnings
- [ ] `cargo clippy` clean

**Rollback:**
- Revert the commit. Step 2's `--enrich` flag still works for explicit enrichment.

**Commit after all checkpoints pass.**

---

#### Step 4: End-to-end validation and golden tests {#step-4}

**Depends on:** #step-3

**Bead:** `specks-v3v.5`

**Commit:** `test(sync): add golden tests for enriched bead content`

**References:** [D02] Root bead description contains purpose strategy and success criteria, [D03] Decision text uses title only in design field, [D05] Content rendered as markdown with H2 headings, Spec S01, Spec S02, Spec S03, Spec S04, (#content-templates, #test-categories)

**Artifacts:**
- New: `tests/fixtures/valid/enrichment-test.md` (test speck fixture)
- New: `tests/fixtures/golden/step-description.md` (expected step description output)
- New: `tests/fixtures/golden/step-acceptance.md` (expected step acceptance output)
- New: `tests/fixtures/golden/step-design.md` (expected step design output)
- New: `tests/fixtures/golden/root-description.md` (expected root description output)

**Tasks:**
- [ ] Create a test speck fixture (`enrichment-test.md`) with representative steps, decisions, tasks, tests, checkpoints, and references.
- [ ] Create golden output files for each content-rendering function.
- [ ] Write golden tests that parse the fixture speck, render content, and compare against golden files.
- [ ] Write a golden test for `resolve_step_design` output.
- [ ] Verify that running `specks validate` on the test fixture passes.

**Tests:**
- [ ] Golden test: `Step::render_description` output matches `step-description.md`
- [ ] Golden test: `Step::render_acceptance_criteria` output matches `step-acceptance.md`
- [ ] Golden test: `resolve_step_design` output matches `step-design.md`
- [ ] Golden test: `Speck::render_root_description` output matches `root-description.md`

**Checkpoint:**
- [ ] `cargo nextest run` passes with zero warnings
- [ ] `cargo clippy` clean
- [ ] All golden tests pass

**Rollback:**
- Revert the commit; only test files are affected.

**Commit after all checkpoints pass.**

---

#### Step 5: Move beads sync to planner finalization {#step-5}

**Depends on:** #step-2

**Bead:** `specks-v3v.6`

**Commit:** `feat(planner): sync beads with enriched content after critic approves`

**References:** [D01] Overwrite all fields unconditionally when enriching, (#strategy, #scope)

**Artifacts:**
- Modified: `skills/planner/SKILL.md`

**Tasks:**
- [ ] In the planner skill's "Handle Critic Recommendation" section (step 5 of the orchestration), add a beads sync step that runs after APPROVE and before outputting the session end message. The planner spawns a Bash agent via Task to run `specks beads sync --enrich <speck_path> --json`. The speck path is already known from the author-agent's output.
- [ ] Update the orchestration diagram to show the sync step between critic-approve and done.
- [ ] Update the planner's session end message to include beads sync status (e.g., `Beads: synced ({steps_synced} steps)`).
- [ ] Handle sync failure gracefully: if the Bash agent returns an error (e.g., `bd` not installed), output a warning but still complete the planning session successfully. Bead sync is best-effort — the plan is valid without beads.

**Tests:**
- [ ] Manual test: run `/specks:planner` with a test idea, verify beads are created after critic approves
- [ ] Manual test: verify that `/specks:implementer` on the same speck finds beads already exist (no new beads created by worktree setup)

**Checkpoint:**
- [ ] Planner skill file is valid markdown with correct orchestration flow
- [ ] Sync step only runs on APPROVE path, not on REVISE or REJECT

**Rollback:**
- Revert the commit. Beads sync continues to work at implementation time via `specks worktree create`.

**Commit after all checkpoints pass.**

---

### 5.0.6 Deliverables and Checkpoints {#deliverables}

**Deliverable:** `specks beads sync` populates bead `description`, `acceptance_criteria`, and `design` fields with rich markdown content from the speck, making each step bead a self-contained work item. Beads are created and enriched at planning time, so implementation starts with fully populated work items.

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

- [ ] `bd show <step-bead-id> --json` returns non-empty `description`, `acceptance_criteria`, and `design` fields after `specks beads sync --enrich`
- [ ] `bd show <root-bead-id> --json` returns `description` with purpose, strategy, and success criteria
- [ ] `specks beads sync` (without `--enrich`) enriches only newly created beads, leaving existing beads unchanged
- [ ] `specks beads sync --enrich` overwrites all content fields on all beads unconditionally
- [ ] All existing sync tests pass without modification
- [ ] `cargo nextest run` passes with zero warnings across all crates
- [ ] Golden tests verify rendered content matches expected output
- [ ] Planner skill syncs beads with `--enrich` after critic approves

**Acceptance tests:**
- [ ] Integration test: full sync cycle creates beads with populated content fields
- [ ] Golden test: rendered content matches expected markdown snapshots
- [ ] Unit test: backward compatibility -- `IssueDetails` without new fields deserializes correctly

#### Roadmap / Follow-ons (Explicitly Not Required for Phase Close) {#roadmap}

- [ ] Phase B: Agent-Bead Communication -- agents read/write bead fields during implementation
- [ ] Phase C: Eliminate Session File -- derive state from git + beads
- [ ] Phase D: Status from Beads -- show real implementation state from bead data
- [ ] Add `--body-file` support to `BeadsCli` for content that exceeds command-line argument limits (~256KB on macOS)
- [ ] Extend parser to capture Rollback sections (dropped from this phase to limit scope)

| Checkpoint | Verification |
|------------|--------------|
| Types compile with new methods | `cargo build -p specks-core` |
| BeadsCli update methods compile | `cargo build -p specks-core` |
| Sync with --enrich flag works | `cargo nextest run -p specks` |
| All tests pass | `cargo nextest run` |
| No warnings | `cargo clippy --all-targets` |

**Commit after all checkpoints pass.**