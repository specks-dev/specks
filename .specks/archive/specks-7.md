## Phase B2: Parser Hardening {#phase-parser-hardening}

**Purpose:** Harden the markdown parser to fail-fast on ambiguous content instead of silently dropping it, add code block awareness, near-miss detection, structural completeness validation, surface diagnostics in `specks validate`, and enforce validation as a hard gate in the critic-agent and worktree create CLI.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|------|-------|
| Owner | kocienda |
| Status | draft |
| Target branch | main |
| Tracking issue/PR | -- |
| Last updated | 2026-02-12 |
| Beads Root | `specks-70x` |

---

### Phase Overview {#phase-overview}

#### Context {#context}

The parser (`crates/specks-core/src/parser.rs`) uses ~15 regex patterns that silently skip content when formatting deviates. The validator (`crates/specks-core/src/validator.rs`) operates on parsed output, so it can only check what the parser extracted -- if the parser silently drops a step, the validator never sees it. The parser also has zero awareness of fenced code blocks: every regex matches against every line including lines inside ``` blocks, creating phantom anchors, steps, and decisions from code examples.

This is a reliability gap between the planner (which creates specks) and the implementer (which executes them). A speck that passes validation today may contain silently-dropped steps or phantom content from code blocks. This phase closes that gap by making the parser report what it skipped, adding structural completeness checks to the validator, and enforcing clean validation as a hard gate before implementation begins.

#### Strategy {#strategy}

- Start with the `ParseDiagnostic` type and code block awareness in the parser, since all subsequent work depends on having a diagnostics channel and correct line classification.
- Add near-miss detection to the parser to catch the most dangerous silent failures: step headers, decision headers, phase headers, metadata fields, and anchor casing.
- Add structural completeness checks to the validator that cross-reference decisions, anchors, and step fields.
- Wire parse diagnostics through `ValidationResult` so `specks validate` surfaces everything in one unified output.
- Enforce clean validation in the critic-agent (deterministic check replaces LLM judgment for structural compliance) and in the `specks worktree create` CLI (code-enforced pre-flight check).
- Each step produces independently testable and shippable improvements; no step depends on a later step.

#### Stakeholders / Primary Customers {#stakeholders}

1. Planner skill (author-agent and critic-agent) -- produces and validates specks
2. Implementer skill (setup-agent) -- consumes specks via worktree create
3. Developers writing specks manually -- get immediate feedback on formatting mistakes

#### Success Criteria (Measurable) {#success-criteria}

> Make these falsifiable. Avoid "works well".

- `specks validate` reports a P006 diagnostic when a step header appears inside a fenced code block (unit test with code block fixture)
- `specks validate` reports P001-P005 and P007 diagnostics for near-miss formatting of step headers, decision headers, phase headers, metadata fields, anchors, and commit lines (one unit test per P-code)
- `specks validate` reports W009-W013 for missing commit lines, missing tasks, uncited decisions, undefined cited decisions, and broken anchor references (one unit test per W-code)
- `specks validate --json` output includes a `diagnostics` array alongside the existing `issues` array
- `specks validate --level lenient` suppresses P-code diagnostics; `--level strict` shows all (new `--level` CLI argument replaces the current `--strict` boolean flag)
- Critic-agent runs `specks validate --json --level strict` as its first action and immediately REJECTs on any errors or P-diagnostics
- `specks worktree create` refuses to create a worktree when the speck has validation errors or parse diagnostics

#### Scope {#scope}

1. `ParseDiagnostic` type on `Speck` struct and fenced code block awareness in parser
2. Near-miss detection for step headers, decision headers, phase headers, metadata fields, anchors, and commit lines (P001-P007)
3. Structural completeness validation for commit lines, tasks, decisions cross-references, and anchor references (W009-W013)
4. Surfacing parse diagnostics in `specks validate` text and JSON output
5. Critic-agent enforcement via `specks validate` as a hard gate
6. Pre-flight validation in `specks worktree create` CLI

#### Non-goals (Explicitly out of scope) {#non-goals}

- Auto-fix / `--fix` flag for diagnostics
- Changing the markdown speck format itself
- Modifying the skeleton template (`specks-skeleton.md`)
- Changes to `sync.rs` or beads integration
- Markdown AST parsing (we use toggle-flag approach for code blocks)

#### Dependencies / Prerequisites {#dependencies}

- Phase B (specks-6, Agent-Bead Communication) is complete
- Existing parser regex patterns and validator checks remain stable as a baseline

#### Constraints {#constraints}

- Warnings-as-errors policy: `cargo build` must pass with zero warnings throughout
- All new P-codes and W-codes must be deterministic (same input always produces same diagnostics)
- Parse diagnostics are informational warnings, never errors -- they indicate near-misses the parser recovered from

#### Assumptions {#assumptions}

- P-code diagnostics are warnings (never errors) -- they indicate near-misses that the parser recovered from
- Code block awareness only applies to structural regex patterns (STEP_HEADER, DECISION_HEADER, PHASE_HEADER, SECTION_HEADER, etc.)
- A new `--level lenient/normal/strict` CLI argument (replacing the current `--strict` boolean flag) will apply to both validation issues and parse diagnostics
- W011 (decision defined but never cited) ignores decisions with status OPEN or DEFERRED
- Enforcement in critic-agent means immediately REJECT on any parse diagnostics, not just errors

---

### 7.0 Design Decisions {#design-decisions}

#### [D01] ParseDiagnostics as field on Speck struct (DECIDED) {#d01-diagnostics-on-speck}

**Decision:** Add a `diagnostics: Vec<ParseDiagnostic>` field to the `Speck` struct rather than returning diagnostics as a separate return type from `parse_speck()`.

**Rationale:**
- Keeps the `parse_speck()` signature stable -- it still returns `Result<Speck, SpecksError>`
- Diagnostics travel with the parsed speck through the entire pipeline without threading extra parameters
- Callers that do not care about diagnostics (e.g., `bd show` rendering) can ignore the field

**Implications:**
- `Speck` struct gains a new `diagnostics` field (default empty vec)
- All downstream consumers of `Speck` see diagnostics without code changes
- JSON serialization of `Speck` includes diagnostics automatically

#### [D02] P-codes added to ValidationResult diagnostics field (DECIDED) {#d02-unified-output}

**Decision:** Add a `diagnostics: Vec<ParseDiagnostic>` field to `ValidationResult` and merge parse diagnostics from `Speck.diagnostics` into it during validation, producing a single unified output for `specks validate`.

**Rationale:**
- Consumers call one function (`validate_speck_with_config`) and get everything: validation issues AND parse diagnostics
- `specks validate` text and JSON output have one place to look for all problems
- Avoids forcing callers to separately inspect the `Speck` and the `ValidationResult`

**Implications:**
- `ValidationResult` struct gains a `diagnostics: Vec<ParseDiagnostic>` field
- `validate_speck_with_config()` copies `speck.diagnostics` into the result
- JSON output from `specks validate --json` includes a `diagnostics` array on `ValidateData` (alongside `files`), NOT on `JsonResponse` (which would be a breaking schema change for all commands)
- Text output shows diagnostics with line numbers after validation issues

#### [D03] Code block awareness via toggle flag (DECIDED) {#d03-code-block-toggle}

**Decision:** Track fenced code block state in the parser using a simple boolean toggle (`in_code_block`) that flips when a line starts with `` ``` ``. Skip all structural regex matching while inside a code block. Do not use a markdown AST parser.

**Rationale:**
- A toggle flag adds ~5 lines to the parser loop; an AST parser is a new dependency and a rewrite
- The toggle handles the only case that matters: fenced code blocks containing example speck syntax
- Edge cases (indented code blocks, nested fences) are rare in speck files and not worth the complexity

**Implications:**
- Lines inside ``` blocks are not matched by STEP_HEADER, DECISION_HEADER, PHASE_HEADER, SECTION_HEADER, ANCHOR, METADATA_ROW, or any other structural pattern
- CHECKBOX and plain-text patterns can still match inside code blocks (these do not cause structural confusion)
- A P006 diagnostic is emitted when structural content (step header, decision header, etc.) is found inside a code block, confirming the filter is working

#### [D04] Near-miss detection uses relaxed regex (DECIDED) {#d04-near-miss-regex}

**Decision:** Near-miss detection uses relaxed (case-insensitive or loosened) regex patterns that fire only when the strict regex fails. The relaxed pattern matches content that "looks like" a step header, decision header, etc. but does not conform to the strict format.

**Rationale:**
- The strict regex already extracts valid content; near-miss detection catches what slips through
- Running the relaxed pattern only on lines that failed strict matching avoids false positives on valid content
- Each near-miss produces a diagnostic with a suggestion showing the correct format

**Implications:**
- Near-miss patterns are defined alongside their strict counterparts in `parser.rs::patterns`
- Each near-miss check runs per-line only when the strict pattern did not match
- Near-miss diagnostics include the line number and a suggestion string

#### [D05] Critic runs specks validate as hard gate (DECIDED) {#d05-critic-validate-gate}

**Decision:** Add `Bash` to the critic-agent's tools. The critic's first action is to run `specks validate <file> --json --level strict`. If any errors or P-diagnostics exist, the critic MUST REJECT immediately with the validation output as the reason. This separates structural validation (deterministic, code-checked) from quality review (LLM judgment). The restriction to only use Bash for `specks validate` commands is a **prompt-enforced soft constraint**, not a code-level hard restriction -- the agent prompt explicitly instructs the LLM not to use Bash for any other purpose.

**Rationale:**
- Deterministic validation catches structural problems faster and more reliably than LLM pattern matching
- Every new P-diagnostic and W-check is automatically enforced through the existing planner revision loop
- The 5 LLM-checked skeleton compliance items in the critic become redundant once `specks validate` covers them
- A prompt-enforced restriction is sufficient here because: (a) the critic operates within the planner loop where its output is structured JSON, (b) misuse of Bash would not bypass the validation gate, only add noise, and (c) Claude Code's `permissionMode: dontAsk` already limits the agent's tool access to its declared set

**Implications:**
- `critic-agent.md` gains Bash in its tools list with a clear prompt instruction restricting usage to `specks validate` commands
- The critic's skeleton compliance checks become "run specks validate and require clean output"
- Structural issues get a deterministic, reproducible verdict; quality issues remain LLM judgment
- The Bash restriction is soft (prompt-enforced); a future phase could add code-level sandboxing if drift is observed

#### [D06] Worktree create validates internally (DECIDED) {#d06-worktree-preflight}

**Decision:** The `specks worktree create` CLI command runs parse + validate on the speck file internally before creating the worktree. If the speck has errors or parse diagnostics, the command returns an error and refuses to create the worktree. This is code-enforced, not agent-enforced.

**Rationale:**
- Catches specks written before parser hardening that have not been re-validated
- The setup agent does not need to validate separately -- the CLI handles it
- A code-enforced gate cannot be bypassed by LLM hallucination or prompt drift

**Implications:**
- `specks worktree create` calls `parse_speck()` and `validate_speck_with_config()` before proceeding
- If validation fails, the error output includes the specific issues/diagnostics
- The setup agent's workflow simplifies: if worktree creation succeeds, the speck is known-valid

#### [D07] New --level flag replaces --strict and applies to diagnostics (DECIDED) {#d07-level-applies-to-diagnostics}

**Decision:** Introduce a new `--level <lenient|normal|strict>` CLI argument on `specks validate`, replacing the current `--strict` boolean flag. The `--level` flag applies to both validation issues and parse diagnostics. Lenient mode suppresses parse diagnostics and warnings entirely; normal mode (default) shows warnings and diagnostics; strict mode shows all diagnostics and treats them as blocking.

**Rationale:**
- The current CLI only has `--strict` (boolean) which maps to `ValidationLevel::Strict`; there is no way to request `lenient` from the CLI despite it being supported in `ValidationConfig`
- A three-valued `--level` flag exposes the full range of `ValidationLevel` and provides a consistent interface
- The existing `--strict` flag is kept as a deprecated alias for `--level strict` to avoid breaking existing scripts
- The config file already has `validation_level` which maps to `ValidationLevel`; the CLI `--level` overrides the config value

**Implications:**
- `cli.rs`: Replace `strict: bool` on `Validate` with `level: Option<String>` and a deprecated `strict: bool` alias
- `validate.rs`: Resolve the effective level from `--level` > `--strict` > `config.toml` > default (normal)
- `validate_speck_with_config()` checks `config.level` before including diagnostics in the result
- Critic-agent always passes `--level strict`
- Worktree create uses normal level by default

---

### 7.0.1 ParseDiagnostic Specification {#parse-diagnostic-spec}

**Spec S01: ParseDiagnostic Type** {#s01-parse-diagnostic}

```rust
/// A diagnostic emitted during parsing (near-miss, code block content, etc.)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParseDiagnostic {
    /// Diagnostic code (e.g., "P001", "P006")
    pub code: String,
    /// Human-readable message
    pub message: String,
    /// Line number where the diagnostic was triggered
    pub line: usize,
    /// Optional suggestion for fixing the issue
    pub suggestion: Option<String>,
}
```

**Table T01: Parse Diagnostic Codes** {#t01-diagnostic-codes}

| Code | Name | Trigger | Severity |
|------|------|---------|----------|
| P001 | Step header near-miss | Line matches `(?i)^#{3,5}\s+step\s+` but not strict `STEP_HEADER` | Warning |
| P002 | Decision header near-miss | Line matches `(?i)\[D\d+\]` or `(?i)\[Q\d+\]` but not strict `DECISION_HEADER` | Warning |
| P003 | Phase header near-miss | Line matches `(?i)^##\s+phase\s+` but not strict `PHASE_HEADER` | Warning |
| P004 | Unrecognized metadata field | METADATA_ROW matched but field name not in known set (Owner, Status, Target branch, Tracking issue/PR, Last updated, Beads Root) | Warning |
| P005 | Invalid anchor format (near-miss) | Anchor syntax `{#...}` found but content does not match strict `[a-z0-9][a-z0-9-]*` (e.g., `{#MyAnchor}`, `{#my_anchor}`, `{#my anchor}`) -- the anchor is silently dropped by the strict ANCHOR regex | Warning |
| P006 | Structural content in code block | Line inside ``` block would have matched a structural pattern | Info |
| P007 | Commit line near-miss | Line matches `(?i)^\*?\*?commit\*?\*?:` but not the strict `COMMIT_LINE` pattern (e.g., `**Commit:** message` without backticks, or `Commit:` without bold) | Warning |

**Table T02: Structural Completeness Warning Codes** {#t02-completeness-codes}

| Code | Name | Trigger | Severity |
|------|------|---------|----------|
| W009 | Step missing Commit line | Step has no parsed `commit_message` (the strict `COMMIT_LINE` regex did not match). W009 checks the parsed field only; near-miss commit lines (present but malformed) are caught by P007 during parsing. | Warning |
| W010 | Step missing Tasks | Step has no task items (empty tasks vec) | Warning |
| W011 | Decision defined but never cited | Decision with status DECIDED exists but no step's References line cites its [DNN] ID. Ignores OPEN/DEFERRED decisions. | Warning |
| W012 | Decision cited but not defined | A step's References line cites [DNN] but no decision with that ID exists in the Design Decisions section | Warning |
| W013 | Anchor referenced but not defined | An anchor referenced in a step's References line (#anchor-name) is not defined anywhere in the document. Checks References lines ONLY, not Depends on lines (Depends on anchors are already checked by E010 `check_dependency_references`). Subsumes existing W005 for reference anchors. | Warning |

---

### 7.0.2 Near-Miss Regex Patterns {#near-miss-patterns}

**Spec S02: Near-Miss Patterns** {#s02-near-miss-patterns}

Each near-miss pattern is a relaxed version of its strict counterpart. The near-miss check fires only when the strict pattern does NOT match on a given line.

| Strict Pattern | Near-Miss Pattern | P-Code |
|---------------|-------------------|--------|
| `STEP_HEADER`: `^#{3,5}\s+Step\s+(\d+(?:\.\d+)?):?\s*(.+?)...` | `(?i)^#{1,6}\s+step\s+\d+` | P001 |
| `DECISION_HEADER`: `^####\s+\[([DQ]\d+)\]\s*(.+?)...` | `(?i)^\s*#{1,6}\s*\[D\d+\]` or `(?i)^\s*#{1,6}\s*\[Q\d+\]` | P002 |
| `PHASE_HEADER`: `^##\s+Phase\s+[\d.]+:\s*(.+?)...` | `(?i)^#{1,6}\s+phase\s+` | P003 |
| `METADATA_ROW`: `^\|\s*([^|]+?)\s*\|\s*([^|]*?)\s*\|` | (same regex matches, but field name not in known set) | P004 |
| `ANCHOR`: `\{#([a-z0-9-]+)\}` | `\{#([^}]+)\}` where captured group does NOT match strict `^[a-z0-9][a-z0-9-]*$` (catches uppercase, underscores, spaces, leading hyphens, etc.) | P005 |
| `COMMIT_LINE`: `^\*\*Commit:\*\*\s*` `` `([^`]+)` `` | `(?i)^\*?\*?commit\*?\*?:\s*` (matches commit-like lines without backtick-wrapped message or without bold) | P007 |

---

### 7.0.3 Symbols to Add / Modify {#symbols}

**Table T03: New and Modified Symbols** {#t03-symbols}

| Symbol | Kind | Location | Notes |
|--------|------|----------|-------|
| `ParseDiagnostic` | struct | `types.rs` | New type per Spec S01 |
| `Speck.diagnostics` | field | `types.rs` | `Vec<ParseDiagnostic>`, default empty |
| `ValidationResult.diagnostics` | field | `validator.rs` | `Vec<ParseDiagnostic>`, default empty |
| `patterns::NEAR_MISS_STEP` | static | `parser.rs` | Relaxed step header pattern |
| `patterns::NEAR_MISS_DECISION` | static | `parser.rs` | Relaxed decision header pattern |
| `patterns::NEAR_MISS_PHASE` | static | `parser.rs` | Relaxed phase header pattern |
| `patterns::INVALID_ANCHOR` | static | `parser.rs` | Broad anchor pattern `\{#([^}]+)\}` for detecting anchors that fail strict matching (renamed from NON_KEBAB_ANCHOR) |
| `patterns::NEAR_MISS_COMMIT` | static | `parser.rs` | Relaxed commit line pattern |
| `KNOWN_METADATA_FIELDS` | const array | `parser.rs` | `["Owner", "Status", "Target branch", "Tracking issue/PR", "Last updated", "Beads Root"]` |
| `DECISION_ID_CAPTURE` | static | `validator.rs` | `\[(D\d{2,})\]` -- captures decision ID from References lines for W011/W012. Distinct from existing `DECISION_CITATION` (no capture group, presence-only). |
| `check_commit_lines` | fn | `validator.rs` | W009 check |
| `check_step_tasks` | fn | `validator.rs` | W010 check |
| `check_uncited_decisions` | fn | `validator.rs` | W011 check |
| `check_undefined_cited_decisions` | fn | `validator.rs` | W012 check |
| `check_undefined_referenced_anchors` | fn | `validator.rs` | W013 check |
| `Validate.level` | field | `cli.rs` | New `--level <lenient\|normal\|strict>` CLI argument |
| `WorktreeCreate.skip_validation` | field | `worktree.rs` | New `--skip-validation` flag on `specks worktree create` |
| `JsonDiagnostic` | struct | `output.rs` | JSON serialization type for `ParseDiagnostic` |
| `ValidateData.diagnostics` | field | `output.rs` | `Vec<JsonDiagnostic>` on validate-specific data (not on `JsonResponse`) |
| `ValidatedFile.diagnostic_count` | field | `output.rs` | Per-file diagnostic count in JSON output |

---

### 7.0.5 Execution Steps {#execution-steps}

#### Step 0: Add ParseDiagnostics and code block awareness {#step-0}

**Bead:** `specks-70x.1`

**Commit:** `feat(parser): add ParseDiagnostic type and code block awareness`

**References:** [D01] ParseDiagnostics as field on Speck struct, [D03] Code block awareness via toggle flag, Spec S01, Table T01, (#parse-diagnostic-spec, #context, #strategy)

**Artifacts:**
- New `ParseDiagnostic` struct in `crates/specks-core/src/types.rs`
- New `diagnostics: Vec<ParseDiagnostic>` field on `Speck` struct
- Code block toggle logic in `crates/specks-core/src/parser.rs`
- P006 diagnostic emission for structural content found inside code blocks

**Tasks:**
- [ ] Add `ParseDiagnostic` struct to `types.rs` per Spec S01 (code, message, line, suggestion fields). Include `PartialEq` in the derive list (`#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]`) so unit tests can use `assert_eq!` on diagnostics directly.
- [ ] Add `diagnostics: Vec<ParseDiagnostic>` field to `Speck` struct with `#[serde(default)]`
- [ ] Add `in_code_block: bool` state variable to the parser loop in `parse_speck()`
- [ ] Toggle `in_code_block` when a line starts with `` ``` `` (trimmed)
- [ ] **CRITICAL PLACEMENT**: The `in_code_block` check and toggle MUST go at the VERY TOP of the loop body, BEFORE anchor extraction (currently at ~line 96 in `parse_speck()`). The parser's first action per-line is extracting anchors via `patterns::ANCHOR`; if the code block check comes after that, anchors inside code blocks will still be collected. Ordering: (1) toggle `in_code_block` on fence lines, (2) if `in_code_block`, optionally emit P006 then `continue` to next line, (3) anchor extraction, (4) all other structural matching.
- [ ] When `in_code_block` is true, skip ALL structural regex matching including ANCHOR extraction (STEP_HEADER, DECISION_HEADER, PHASE_HEADER, SECTION_HEADER, ANCHOR, METADATA_ROW, DEPENDS_ON, BEAD_LINE, BEADS_HINTS, COMMIT_LINE, REFERENCES_LINE, PURPOSE_LINE, BEADS_ROOT_ROW)
- [ ] When `in_code_block` is true and a line would have matched a structural pattern, emit P006 diagnostic. **P006 emission strategy**: Test a focused subset of 3 high-value structural patterns only: `STEP_HEADER`, `DECISION_HEADER`, and `PHASE_HEADER`. Do NOT test all 13 structural patterns -- the remaining patterns (METADATA_ROW, SECTION_HEADER, DEPENDS_ON, BEAD_LINE, BEADS_HINTS, COMMIT_LINE, REFERENCES_LINE, PURPOSE_LINE, BEADS_ROOT_ROW, ANCHOR) are low-risk for code block confusion and testing all 13 would add O(13) regex evaluations per code-block line for negligible benefit. A single combined check is not feasible because the patterns are heterogeneous. The 3 chosen patterns are the ones most likely to appear in speck examples inside code blocks (e.g., skeleton documentation, agent prompts).
- [ ] Export `ParseDiagnostic` from `crates/specks-core/src/lib.rs`

**Tests:**
- [ ] Unit test: parsing a speck with a step header inside a ``` block does NOT create a step from it
- [ ] Unit test: parsing a speck with a step header inside a ``` block emits P006 diagnostic with correct line number
- [ ] Unit test: parsing a speck with a decision header inside a ``` block does NOT create a decision from it
- [ ] Unit test: anchor `{#some-anchor}` inside a ``` block is NOT collected in `speck.anchors` (verifies code block check precedes anchor extraction)
- [ ] Unit test: code blocks that do not contain structural content produce no P006 diagnostics
- [ ] Unit test: nested content after code block closes is parsed normally
- [ ] Unit test: `Speck.diagnostics` field is empty vec when no diagnostics are emitted

**Checkpoint:**
- [ ] `cargo nextest run` passes
- [ ] `cargo build` passes with zero warnings
- [ ] Existing parser tests continue to pass unchanged

**Rollback:**
- Revert commit; `ParseDiagnostic` struct and code block logic are self-contained

**Commit after all checkpoints pass.**

---

#### Step 1: Near-miss detection {#step-1}

**Depends on:** #step-0

**Bead:** `specks-70x.2`

**Commit:** `feat(parser): add near-miss detection for steps, decisions, phases, metadata, and anchors`

**References:** [D04] Near-miss detection uses relaxed regex, Spec S02, Table T01, (#near-miss-patterns, #parse-diagnostic-spec)

**Artifacts:**
- New relaxed regex patterns in `parser.rs::patterns` module (NEAR_MISS_STEP, NEAR_MISS_DECISION, NEAR_MISS_PHASE, NEAR_MISS_COMMIT, INVALID_ANCHOR)
- `KNOWN_METADATA_FIELDS` constant in `parser.rs`
- P001-P005, P007 diagnostic emission in the parser loop

**Tasks:**
- [ ] Add `NEAR_MISS_STEP` pattern: `(?i)^#{1,6}\s+step\s+\d+`
- [ ] Add `NEAR_MISS_DECISION` pattern: `(?i)^\s*#{1,6}\s*\[[DQ]\d+\]`
- [ ] Add `NEAR_MISS_PHASE` pattern: `(?i)^#{1,6}\s+phase\s+`
- [ ] Add `NEAR_MISS_COMMIT` pattern: `(?i)^\*?\*?commit\*?\*?:\s*` (matches commit-like lines)
- [ ] Add `INVALID_ANCHOR` pattern: `\{#([^}]+)\}` (broad anchor syntax match; captures any content between `{#` and `}`)
- [ ] Add `KNOWN_METADATA_FIELDS` const: `["Owner", "Status", "Target branch", "Tracking issue/PR", "Last updated", "Beads Root"]`
- [ ] **CRITICAL PLACEMENT -- `matched` flag pattern**: Not all strict patterns use `continue` after matching. Specifically, `SECTION_HEADER` (parser.rs ~line 168-187) falls through to subsequent checks, and plain bullet items in the Artifacts section (~line 424-436) also fall through. If near-miss checks run unconditionally at the end of the loop, a line like `### Phase Overview` with an anchor would match `SECTION_HEADER` (no continue), then fall through to the near-miss section where `NEAR_MISS_PHASE` would fire a **false P003 diagnostic**. To prevent this: (a) add a `let mut matched = false;` variable at the top of each loop iteration, (b) set `matched = true` inside every strict pattern match branch -- both those that `continue` AND those that fall through (SECTION_HEADER, Artifacts bullets, etc.), (c) gate ALL near-miss checks on `if !matched { ... }` at the end of the loop body. This ensures a line already claimed by any strict pattern is never tested against near-miss patterns. Ordering: (1) code block toggle, (2) `let mut matched = false;`, (3) all strict pattern chains (setting `matched = true` on match), (4) near-miss detection section guarded by `if !matched`.
- [ ] Add `let mut matched = false;` at the top of the main parser loop body (after code block toggle). Set `matched = true` inside every existing strict pattern match branch -- including `SECTION_HEADER` (which does NOT `continue`) and Artifacts plain bullet items (which also do NOT `continue`). For patterns that `continue`, setting `matched` is technically redundant but keeps the pattern uniform.
- [ ] In near-miss section (guarded by `if !matched`): when `NEAR_MISS_STEP` matches the line, emit P001 with line number and suggestion showing correct format (the line was NOT matched by any strict pattern above, so it is safe to flag)
- [ ] In near-miss section (guarded by `if !matched`): when `NEAR_MISS_DECISION` matches, emit P002
- [ ] In near-miss section (guarded by `if !matched`): when `NEAR_MISS_PHASE` matches, emit P003
- [ ] In near-miss section (guarded by `if !matched`): when `NEAR_MISS_COMMIT` matches, emit P007 with suggestion showing correct format (bold + backtick-wrapped message)
- [ ] In near-miss section (guarded by `if !matched`): when `INVALID_ANCHOR` matches and the captured group does NOT match the strict `VALID_ANCHOR` regex (`^[a-z0-9][a-z0-9-]*$`), emit P005 with suggestion showing the kebab-case equivalent
- [ ] P004 is the exception: it fires INSIDE the METADATA_ROW strict match branch (when the row matched but the field name is unrecognized), not in the near-miss section at the end
- [ ] All near-miss checks skip lines inside code blocks (rely on `in_code_block` from Step 0)

**Tests:**
- [ ] Unit test: `### step 0: lowercase` triggers P001 with suggestion
- [ ] Unit test: `## Step 1: Wrong heading level` triggers P001 (heading level `##` is outside the strict `#{3,5}` range but matches near-miss `#{1,6}`)
- [ ] Unit test: `#### Step 1 Missing Colon` does NOT trigger P001 (the strict regex has an optional colon `:?` so this is a valid match -- the parser captures it as a step header and `continue`s before reaching near-miss checks)
- [ ] Unit test: `#### [d01] lowercase decision (DECIDED)` triggers P002
- [ ] Unit test: `## phase 1.0: lowercase` triggers P003
- [ ] Unit test: metadata row with `| Author |` triggers P004 listing known fields
- [ ] Unit test: `{#MyAnchor}` triggers P005 (uppercase)
- [ ] Unit test: `{#my_anchor}` triggers P005 (underscore -- silently dropped by strict ANCHOR regex)
- [ ] Unit test: `{#my anchor}` triggers P005 (space)
- [ ] Unit test: anchor with leading hyphen (e.g., `-leading-hyphen` inside `{#...}`) triggers P005 (leading hyphen fails `^[a-z0-9]` start)
- [ ] Unit test: `{#valid-anchor}` does NOT trigger P005
- [ ] Unit test: `**Commit:** message without backticks` triggers P007 with suggestion
- [ ] Unit test: `Commit: message` (no bold, no backticks) triggers P007
- [ ] Unit test: correctly-formatted step headers produce NO P001 diagnostic
- [ ] Unit test: correctly-formatted `` **Commit:** `message` `` produces NO P007 diagnostic
- [ ] Unit test: near-miss patterns inside code blocks do NOT produce P001-P005/P007 (only P006)
- [ ] Unit test: a section header like `### Phase Overview` with a valid anchor does NOT trigger P003 (SECTION_HEADER matches it -- the `matched` flag prevents false near-miss detection even though SECTION_HEADER does not `continue`)

**Checkpoint:**
- [ ] `cargo nextest run` passes
- [ ] `cargo build` passes with zero warnings
- [ ] Existing parser tests continue to pass unchanged

**Rollback:**
- Revert commit; near-miss patterns are additive and do not affect parsing logic

**Commit after all checkpoints pass.**

---

#### Step 2: Structural completeness validation {#step-2}

**Depends on:** #step-0

**Bead:** `specks-70x.3`

**Commit:** `feat(validator): add structural completeness checks W009-W013`

**References:** [D02] P-codes added to ValidationResult diagnostics field, Table T02, (#t02-completeness-codes, #design-decisions)

**Artifacts:**
- New check functions in `crates/specks-core/src/validator.rs`: `check_commit_lines`, `check_step_tasks`, `check_uncited_decisions`, `check_undefined_cited_decisions`, `check_undefined_referenced_anchors`
- W009-W013 validation issues added to warning checks section

**Tasks:**
- [ ] Add `check_commit_lines()`: for each step and substep, if `commit_message` is `None`, emit W009
- [ ] Add `check_step_tasks()`: for each step and substep, if `tasks` is empty, emit W010
- [ ] Add `DECISION_ID_CAPTURE` regex: `\[(D\d{2,})\]` (with capture group to extract the decision ID string, e.g., "D01"). Note: the existing `DECISION_CITATION` regex (`\[D\d{2}\]`) has no capture group and requires exactly 2 digits -- it is used for presence-checking in E018 and must not be reused for ID extraction. The new regex allows 2+ digits for forward compatibility.
- [ ] Add `check_uncited_decisions()`: collect all decision IDs with status != OPEN and != DEFERRED; collect all [DNN] citations from all step/substep References lines using the new `DECISION_ID_CAPTURE` regex; emit W011 for decisions never cited
- [ ] Add `check_undefined_cited_decisions()`: collect all [DNN] citations from step/substep References using `DECISION_ID_CAPTURE`; collect all defined decision IDs; emit W012 for cited IDs not in defined set
- [ ] Add `check_undefined_referenced_anchors()`: collect all #anchor-name references from References lines ONLY (not Depends on lines -- those are already checked by E010 `check_dependency_references` which emits an Error for broken dependency anchors). Check each against the anchor map; emit W013 for undefined anchors. Must iterate both `step.references` and `substep.references` (the existing W005 only iterates top-level steps, missing substeps). This check subsumes the existing W005.
- [ ] **EXPLICIT W005 REPLACEMENT**: Remove the `check_reference_anchors()` call (W005) from `validate_speck_with_config()` (currently at validator.rs ~line 268) and replace it with a call to `check_undefined_referenced_anchors()` (W013). The W005 function body (`check_reference_anchors`) can be deleted or kept as dead code with an `#[allow(dead_code)]` annotation and a comment noting it is superseded by W013. The key requirement is that both W005 and W013 MUST NOT both run -- they check the same anchors and would produce duplicate warnings. Only `check_undefined_referenced_anchors()` (W013) should be called.
- [ ] Wire all five checks into `validate_speck_with_config()` in the warning checks section, gated by `config.level.include_warnings()`
- [ ] **EXISTING TEST IMPACT NOTE**: Many existing test fixtures and inline test strings in `validator.rs` tests (e.g., `test_e001_missing_section`, `test_e004_missing_references`, `test_w003_step_missing_checkpoints`, etc.) do not include `**Commit:**` lines or `**Tasks:**` sections. After adding W009/W010 checks, these fixtures will emit new warnings. While `valid` remains true (warnings do not fail validation), any tests that assert specific warning counts (e.g., `assert_eq!(w003_issues.len(), 1)`) may need adjustment. The implementer MUST audit all existing validator tests after wiring W009/W010 and update any assertions on warning counts that are affected by the new warnings from existing fixtures.

**Tests:**
- [ ] Unit test: step without `**Commit:**` line triggers W009
- [ ] Unit test: step with `**Commit:**` line does NOT trigger W009
- [ ] Unit test: step without tasks triggers W010
- [ ] Unit test: DECIDED decision never cited in any References triggers W011
- [ ] Unit test: OPEN decision never cited does NOT trigger W011
- [ ] Unit test: DEFERRED decision never cited does NOT trigger W011
- [ ] Unit test: [D03] cited in References but no D03 decision exists triggers W012
- [ ] Unit test: #nonexistent-anchor in step References triggers W013
- [ ] Unit test: #nonexistent-anchor in substep References triggers W013 (verifies substep iteration, fixing W005 coverage gap)
- [ ] Unit test: #nonexistent-anchor in Depends on triggers E010 (NOT W013 -- validates no overlap)
- [ ] Unit test: all decisions cited and all anchors defined produces no W011/W012/W013

**Checkpoint:**
- [ ] `cargo nextest run` passes
- [ ] `cargo build` passes with zero warnings
- [ ] Existing validation tests continue to pass (W005 may be subsumed by W013)

**Rollback:**
- Revert commit; new checks are additive functions gated by validation level

**Commit after all checkpoints pass.**

---

#### Step 3: Surface diagnostics in specks validate output {#step-3}

**Depends on:** #step-0, #step-2

**Bead:** `specks-70x.4`

**Commit:** `feat(validate): surface parse diagnostics in text and JSON output`

**References:** [D02] P-codes added to ValidationResult diagnostics field, [D07] New --level flag replaces --strict, (#parse-diagnostic-spec, #t01-diagnostic-codes, #d07-level-applies-to-diagnostics)

**Artifacts:**
- New `diagnostics: Vec<ParseDiagnostic>` field on `ValidationResult`
- Updated `validate_speck_with_config()` to copy `speck.diagnostics` into result
- New `--level <lenient|normal|strict>` CLI argument on `specks validate` (replaces `--strict`)
- Updated `crates/specks/src/cli.rs` with new `level` argument and deprecated `strict` alias
- Updated `crates/specks/src/commands/validate.rs` text and JSON output
- Updated `crates/specks/src/output.rs` JSON types for diagnostics

**Tasks:**
- [ ] Add `diagnostics: Vec<ParseDiagnostic>` field to `ValidationResult` with `#[serde(default)]`
- [ ] In `validate_speck_with_config()`, after all checks, copy `speck.diagnostics` into `result.diagnostics` (filtered by `config.level`: lenient = skip all, normal = include, strict = include)
- [ ] Add `diagnostic_count()` method to `ValidationResult`
- [ ] Add `diagnostic_count: usize` field to `ValidatedFile` in `output.rs` (alongside existing `error_count` and `warning_count`) so per-file JSON output reports diagnostic counts
- [ ] In `cli.rs`, add `--level <lenient|normal|strict>` argument to the `Validate` command: `#[arg(long, value_name = "LEVEL")]` with type `Option<String>`
- [ ] In `cli.rs`, keep `--strict` as a deprecated boolean alias (maps to `--level strict`); add `#[arg(long, hide = true)]` or a deprecation note in help text
- [ ] **COMPILATION FIX -- main.rs call site**: Adding `level: Option<String>` to `Commands::Validate` in `cli.rs` requires updating the match arm in `main.rs` (line ~20) that destructures `Commands::Validate { file, strict }` to also destructure `level`. The `run_validate()` call must pass `level` as a new parameter. Currently: `commands::run_validate(file, strict, cli.json, cli.quiet)`. After: `commands::run_validate(file, strict, level, cli.json, cli.quiet)`.
- [ ] **COMPILATION FIX -- run_validate() signature**: Update `run_validate()` in `commands/validate.rs` to accept the new `level: Option<String>` parameter. Current signature: `pub fn run_validate(file: Option<String>, strict: bool, json_output: bool, quiet: bool)`. New signature: `pub fn run_validate(file: Option<String>, strict: bool, level: Option<String>, json_output: bool, quiet: bool)`.
- [ ] In `run_validate()`, resolve the effective validation level with precedence: `--level` > `--strict` > `config.toml validation_level` > default (normal). Replace the current `if strict { Strict } else { config }` logic.
- [ ] Update `output_text()` in `validate.rs` to print diagnostics section after issues: format `warning[P001]: line 42: <message>` with optional suggestion
- [ ] Add `diagnostics: Vec<JsonDiagnostic>` field to `ValidateData` in `output.rs` with `#[serde(default)]` (NOT to `JsonResponse` -- putting it on the generic envelope would be a breaking schema change for all commands). The diagnostics array sits alongside the existing `files` field in `ValidateData`, scoped to the validate command only.
- [ ] **COMPILATION FIX**: Adding `diagnostics` to `ValidateData` breaks all existing constructor sites in `validate.rs` that construct `ValidateData { files: vec![] }` or `ValidateData { files }` (approximately lines 35, 76, 101, 114, 222, 224). Update EVERY constructor site to include `diagnostics: vec![]`. There are at least 6 sites. Alternatively, add `#[derive(Default)]` to `ValidateData` and use `ValidateData { files: vec![], ..Default::default() }` at all sites, or use `#[serde(default)]` on the `diagnostics` field so deserialization works, but constructor sites still need updating for compilation.
- [ ] Add `JsonDiagnostic` type in `output.rs` with fields: `code: String`, `message: String`, `line: Option<usize>`, `suggestion: Option<String>`, `file: Option<String>`. Implement `From<&ParseDiagnostic>` for `JsonDiagnostic`.
- [ ] Update `output_json()` in `validate.rs` to populate `ValidateData.diagnostics` from `ValidationResult.diagnostics`
- [ ] Update existing CLI tests in `cli.rs` that reference `--strict` to also test `--level strict` and `--level lenient`

**Tests:**
- [ ] Unit test: `ValidationResult.diagnostics` contains parse diagnostics from `Speck.diagnostics`
- [ ] Unit test: lenient level produces empty diagnostics in result
- [ ] Unit test: strict level includes all diagnostics in result
- [ ] CLI test: `specks validate --level strict` parses correctly and sets level to Strict
- [ ] CLI test: `specks validate --level lenient` parses correctly and sets level to Lenient
- [ ] CLI test: `specks validate --strict` still works (backward compatible, maps to Strict)
- [ ] CLI test: `specks validate` with no level flag uses config or default (Normal)
- [ ] Integration test: `specks validate` text output includes `warning[P001]:` line for a near-miss fixture
- [ ] Integration test: `specks validate --json` output includes `diagnostics` array
- [ ] Golden test: validate output for a fixture with known P-codes matches expected text

**Checkpoint:**
- [ ] `cargo nextest run` passes
- [ ] `cargo build` passes with zero warnings
- [ ] `specks validate .specks/specks-7.md` runs without error (this speck itself)

**Rollback:**
- Revert commit; diagnostics field additions are backward-compatible (default empty)

**Commit after all checkpoints pass.**

---

#### Step 4: Enforce validation as hard gate in critic-agent {#step-4}

**Depends on:** #step-3

**Bead:** `specks-70x.5`

**Commit:** `feat(agents): enforce specks validate as hard gate in critic-agent`

**References:** [D05] Critic runs specks validate as hard gate, (#design-decisions, #context)

**Artifacts:**
- Updated `agents/critic-agent.md` with Bash tool and validation-first workflow
- Revised skeleton compliance checks: deterministic `specks validate` replaces 5 LLM-checked items

**Tasks:**
- [ ] Add `Bash` to critic-agent.md tools line (becomes: `tools: Read, Grep, Glob, Bash`)
- [ ] Add clear instruction in critic-agent prompt: "Your FIRST action for every review is to run `specks validate <file> --json --level strict`"
- [ ] Add instruction: "If the validation output contains ANY errors or ANY diagnostics (P-codes), you MUST immediately REJECT with the validation output as the reason. Do not proceed to quality review."
- [ ] Update the "Skeleton Compliance Checks" section: replace the 5 individual LLM-checked items with "Run `specks validate --json --level strict` and require clean output (zero errors, zero diagnostics)"
- [ ] Add prompt instruction restricting Bash usage to `specks validate` commands only (this is a soft, prompt-enforced constraint per [D05]; not code-enforced)
- [ ] Update the recommendation logic: validation failure = immediate REJECT before any quality assessment

**Tests:**
- [ ] Automated test: read `agents/critic-agent.md`, parse YAML frontmatter, assert `tools` field contains `Bash`
- [ ] Automated test: read `agents/critic-agent.md`, assert body contains `specks validate` instruction text (e.g., grep for "specks validate" and "REJECT")
- [ ] Automated test: read `agents/critic-agent.md`, assert body contains Bash restriction instruction (e.g., grep for "only" and "specks validate" in proximity)
- [ ] Manual verification: read the updated critic-agent.md and confirm the validation-first workflow is clear and unambiguous

**Checkpoint:**
- [ ] Automated agent-file tests pass (YAML frontmatter parses, tools include Bash, body contains validation workflow instructions)
- [ ] Read through the updated agent definition and confirm flow: validate first -> reject on issues -> quality review only if clean

**Rollback:**
- Revert commit; restores previous critic-agent.md without Bash tool

**Commit after all checkpoints pass.**

---

#### Step 5: Pre-flight validation in worktree create CLI {#step-5}

**Depends on:** #step-3

**Bead:** `specks-70x.6`

**Commit:** `feat(worktree): add pre-flight validation to worktree create`

**References:** [D06] Worktree create validates internally, [D07] Existing --level flag applies to diagnostics, (#d06-worktree-preflight, #scope)

**Artifacts:**
- Updated `crates/specks/src/commands/worktree.rs` create command with validation gate
- New error type or error handling for validation-blocked worktree creation

**Tasks:**
- [ ] In the `create` handler of `worktree.rs`, after reading the speck file content and before calling `create_worktree()`, call `parse_speck()` and `validate_speck_with_config()` with normal level
- [ ] If `parse_speck()` returns an error, return an error message with the parse failure
- [ ] If `validate_speck_with_config()` returns `valid == false` or `diagnostics` is non-empty, return an error listing the issues and diagnostics, and refuse to create the worktree
- [ ] Format the error output to show each issue/diagnostic with line numbers, matching the `specks validate` text format
- [ ] For JSON output mode, include the validation result in the error response
- [ ] Add a `--skip-validation` flag as an escape hatch for exceptional cases (e.g., migrating legacy specks)

**Tests:**
- [ ] Integration test: `specks worktree create` with a valid speck succeeds (existing behavior preserved)
- [ ] Integration test: `specks worktree create` with a speck that has validation errors returns error and does NOT create worktree
- [ ] Integration test: `specks worktree create` with a speck that has P-code diagnostics returns error and does NOT create worktree
- [ ] Integration test: `specks worktree create --skip-validation` bypasses the check
- [ ] Unit test: the validation is called with normal level by default

**Checkpoint:**
- [ ] `cargo nextest run` passes
- [ ] `cargo build` passes with zero warnings
- [ ] Existing worktree create tests continue to pass

**Rollback:**
- Revert commit; validation gate is a pre-check before the existing `create_worktree()` call

**Commit after all checkpoints pass.**

---

### 7.0.6 Deliverables and Checkpoints {#deliverables}

**Deliverable:** Parser hardening with fail-fast diagnostics, structural completeness validation, and enforcement gates in the critic-agent and worktree create CLI.

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

- [ ] `ParseDiagnostic` type exists on `Speck` struct and parser emits P006 for structural content in code blocks
- [ ] Parser emits P001-P005 and P007 for near-miss formatting of step headers, decision headers, phase headers, metadata fields, anchors, and commit lines
- [ ] Validator checks W009-W013 for missing commit lines, missing tasks, uncited decisions, undefined cited decisions, and broken anchor references
- [ ] `specks validate` text and JSON output surfaces parse diagnostics alongside validation issues
- [ ] New `--level <lenient|normal|strict>` CLI argument works; `--strict` remains as deprecated alias; config.toml `validation_level` is overridden by CLI
- [ ] `--level lenient` suppresses diagnostics; `--level strict` shows all
- [ ] Critic-agent runs `specks validate --json --level strict` as first action and REJECTs on any diagnostics
- [ ] `specks worktree create` refuses to create worktree when speck has validation errors or parse diagnostics
- [ ] All existing tests pass; no regressions
- [ ] `cargo build` passes with zero warnings throughout

**Acceptance tests:**
- [ ] Unit test: P006 fires for step header in code block
- [ ] Unit test: P001 fires for `### step 0: lowercase`
- [ ] Unit test: W011 fires for DECIDED decision never cited
- [ ] Integration test: `specks validate --json` includes `diagnostics` array
- [ ] Integration test: `specks worktree create` blocks on invalid speck

#### Roadmap / Follow-ons (Explicitly Not Required for Phase Close) {#roadmap}

- [ ] Auto-fix / `--fix` flag for common P-code diagnostics (suggest and apply corrections)
- [ ] Extend near-miss detection to References line format (`**References:**` vs `References:` without bold)
- [ ] Add `specks doctor` check for specks that pre-date parser hardening and need re-validation

| Checkpoint | Verification |
|------------|--------------|
| Parser emits P-codes | `cargo nextest run` with P-code test fixtures |
| Validator emits W-codes | `cargo nextest run` with W-code test fixtures |
| Validate output includes diagnostics | `specks validate --json` on test fixture |
| Critic-agent uses validation gate | Read agent definition, verify workflow |
| Worktree create blocks on invalid | Integration test with invalid fixture |

**Commit after all checkpoints pass.**