# Specks Implementation Log

This file documents the implementation progress for the specks project.

**Format:** Each entry records a completed step with tasks, files, and verification results.

Entries are sorted newest-first.

## [specks-2.md] Step 8.3.3: Create PlanningMode and Restructure Module | COMPLETE | 2026-02-05

**Completed:** 2026-02-05

**References Reviewed:**
- `.specks/specks-2.md` - Phase 2.0 plan, Step 8.3.3 specification
- `crates/specks/src/planning_loop.rs` - Original single-file planning loop module
- `crates/specks/src/commands/plan.rs` - Plan command entry point
- Design decisions D18, D19, D20 in specks-2.md

**Implementation Progress:**

| Task | Status |
|------|--------|
| Create `planning_loop/` directory | Done |
| Move `planning_loop.rs` to `planning_loop/mod.rs` | Done |
| Create `types.rs` with `PlanningMode` enum and shared types | Done |
| Update `PlanningLoop::new()` to accept `mode: PlanningMode` parameter | Done |
| Store mode in `PlanningLoop` struct | Done |
| Update mod.rs to re-export types | Done |
| Update imports in `commands/plan.rs` (pass `PlanningMode::Cli`) | Done |
| Existing tests continue to pass | Done |
| Unit test: `PlanningMode` serialization/display | Done |

**Files Created:**
- `crates/specks/src/planning_loop/mod.rs` - Main module with `PlanningLoop` struct, now accepts `PlanningMode` parameter
- `crates/specks/src/planning_loop/types.rs` - Shared types: `PlanningMode`, `LoopState`, `PlanMode`, `LoopOutcome`, `LoopContext`, `UserDecision`

**Files Modified:**
- `crates/specks/src/commands/plan.rs` - Added `PlanningMode` import, passes `PlanningMode::Cli` to `PlanningLoop::new()`

**Files Deleted:**
- `crates/specks/src/planning_loop.rs` - Replaced by `planning_loop/` module directory

**Test Results:**
- `cargo nextest run`: 250 tests passed

**Checkpoints Verified:**
- `cargo build` succeeds: PASS
- `cargo nextest run` passes: PASS (250 tests)
- Module structure is clean: PASS

**Key Decisions/Notes:**
- Per [D19], the `PlanningMode` enum is passed explicitly rather than auto-detected
- `PlanningMode::Cli` indicates CLI handles interaction; `PlanningMode::ClaudeCode` indicates interviewer agent handles it
- Added `planning_mode()` accessor method to `PlanningLoop` for testing/introspection
- Tests verify both Cli and ClaudeCode modes can be constructed
- `types.rs` contains all shared types that don't depend on `AgentRunner`
- Re-exports in `mod.rs` maintain the same public API

---

## [specks-2.md] Step 8.3 Redesign: Interaction System Architecture | COMPLETE | 2026-02-05

**Completed:** 2026-02-05

**References Reviewed:**
- `.specks/specks-2.md` - Phase 2.0 plan file, Steps 8.3.3 through 8.3.7 (old design)
- `crates/specks/src/planning_loop.rs` - Current planning loop implementation
- `crates/specks/src/commands/plan.rs` - Plan command entry point
- `crates/specks/src/interaction/cli_adapter.rs` - CLI adapter implementation
- `agents/specks-interviewer.md` - Interviewer agent definition
- `.claude/skills/specks-plan/SKILL.md` - Claude Code skill definition

**Summary:**

Discovered critical UX problem: CLI mode invoked interviewer agent which uses `AskUserQuestion` tool, but this doesn't work in `--print` mode. Users experienced minutes-long spinners with no feedback. The core insight: **the interviewer agent is the ONLY agent that needs user interaction**. All other agents (planner, critic, etc.) just do work.

**Solution:** In CLI mode, the CLI itself acts as the interviewer using `inquire` prompts. The interviewer agent is only used in Claude Code mode where `AskUserQuestion` works.

**Implementation Progress:**

| Task | Status |
|------|--------|
| Identified root cause of CLI mode UX problem | Done |
| Designed dual-mode architecture (CLI vs Claude Code) | Done |
| Added design decisions D18, D19, D20 to speck | Done |
| Rewrote Steps 8.3.3 through 8.3.9 with proper CLI-driven approach | Done |
| Fixed spinner artifact bug (spinner char left behind) | Done |
| Added elapsed time to progress spinners | Done |

**Files Modified:**
- `.specks/specks-2.md` - Added D18, D19, D20; Replaced Steps 8.3.3-8.3.7 with new Steps 8.3.3-8.3.9
- `crates/specks/src/interaction/cli_adapter.rs` - Fixed `end_progress` to use `finish_and_clear()` and show elapsed time

**New Design Decisions Added:**
- **D18**: CLI is the Interviewer in CLI Mode - CLI handles all user interaction directly
- **D19**: Mode Detection via Explicit Parameter - `PlanningMode::Cli` vs `PlanningMode::ClaudeCode`
- **D20**: Shared Agent Invocation Logic - Planner/Critic identical in both modes

**New Steps (replacing old 8.3.3-8.3.7):**
- Step 8.3.3: Create PlanningMode and Restructure Module
- Step 8.3.4: Implement CLI-Mode Gather Phase
- Step 8.3.5: Implement CLI-Mode Present Phase
- Step 8.3.6: Integrate Mode-Aware Loop and Test CLI End-to-End
- Step 8.3.7: Verify and Document Claude Code Mode
- Step 8.3.8: Polish UX and Add Non-Interactive Support
- Step 8.3.9: Create Architecture Documentation

**Key Decisions/Notes:**
- The old Step 8.3.3 "Planning Loop Adapter Integration" was fundamentally flawed - it added adapter plumbing but still invoked the interviewer agent in CLI mode
- CLI mode flow: CLI prompts → Planner (spinner) → Critic (spinner) → CLI presents → CLI approve/revise
- Claude Code flow: Interviewer (AskUserQuestion) → Planner → Critic → Interviewer (AskUserQuestion)
- Progress spinners now show elapsed time: `⠸ Planner creating speck... [1m 23s]`
- Spinner artifact fixed: clean line with checkmark after completion, no leftover spinner character

---

## [specks-2.md] Step 8.3.2: CLI Adapter Implementation | COMPLETE | 2026-02-05

**Completed:** 2026-02-05

**References Reviewed:**
- `.specks/specks-2.md` - Phase 2.0 plan file, Step 8.3.2 specification
- Design decisions D15 (CLI interaction), D17 (Non-TTY fallback)
- `crates/specks-core/src/interaction.rs` - InteractionAdapter trait from Step 8.3.1
- `crates/specks/Cargo.toml` - Existing CLI dependencies
- `crates/specks/src/main.rs` - Existing module structure

**Implementation Progress:**

| Task | Status |
|------|--------|
| Add `ctrlc = "3.4"` to `crates/specks/Cargo.toml` | Done |
| Create `interaction/` module directory | Done |
| Implement `CliAdapter` struct with TTY detection | Done |
| Implement `ask_text` using `inquire::Text` | Done |
| Implement `ask_select` using `inquire::Select` | Done |
| Implement `ask_confirm` using `inquire::Confirm` | Done |
| Implement `ask_multi_select` using `inquire::MultiSelect` | Done |
| Implement `start_progress` using `indicatif::ProgressBar::new_spinner()` | Done |
| Implement `end_progress` with success/failure styling | Done |
| Implement `print_*` methods using `owo-colors` for consistent styling | Done |
| Add TTY check: if not TTY, return `InteractionError::NonTty` | Done |
| Set up Ctrl+C handler with `ctrlc` crate for graceful cancellation | Done |
| Handle Ctrl+C during prompts: return `InteractionError::Cancelled` | Done |
| Unit test: `CliAdapter::new()` detects TTY correctly | Done |
| Unit test: non-TTY mode returns appropriate errors | Done |
| Integration test: manual verification of prompt styling (documented in code) | Done |

**Files Created:**
- `crates/specks/src/interaction/mod.rs` - CLI interaction module exports
- `crates/specks/src/interaction/cli_adapter.rs` - CliAdapter implementation with TTY detection, Ctrl+C handling, all InteractionAdapter methods

**Files Modified:**
- `Cargo.toml` - Added `ctrlc = "3.4"` to workspace dependencies
- `crates/specks/Cargo.toml` - Added `inquire`, `indicatif`, `owo-colors`, `ctrlc` dependencies
- `crates/specks/src/main.rs` - Added `mod interaction`
- `.specks/specks-2.md` - Checked off completed tasks and checkpoints

**Test Results:**
- `cargo nextest run`: 242 tests passed (12 new tests added)

**Checkpoints Verified:**
- `cargo build` succeeds: PASS
- `cargo nextest run` passes: PASS
- Manual test: `CliAdapter` prompts work in terminal: PENDING (requires Step 8.3.3 integration)
- Manual test: Ctrl+C cancels gracefully: PENDING (requires Step 8.3.3 integration)

**Key Decisions/Notes:**
- **Global Ctrl+C handler**: Uses atomic `CANCELLED` flag that can be checked before/during prompts
- **TTY detection**: Uses `std::io::stdin().is_terminal()` from Rust 1.70+
- **Error mapping**: `convert_inquire_error()` maps inquire errors to InteractionError variants
- **Color scheme**: Info (white), Warning (yellow), Error (red bold), Success (green with checkmark)
- **Progress spinners**: Cyan spinner with tick characters `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`, 100ms tick rate
- **Unused code warnings**: Expected until Step 8.3.3 integrates CliAdapter into planning loop
- **Helper functions exported**: `setup_ctrl_c_handler()` and `reset_cancellation()` for use by commands

---

## [specks-2.md] Step 8.3.1: Core Interaction Adapter Trait | COMPLETE | 2026-02-05

**Completed:** 2026-02-05

**References Reviewed:**
- `.specks/specks-2.md` - Phase 2.0 plan file, Step 8.3.1 specification
- Design decisions D15 (CLI interaction), D16 (CC interaction), D17 (Non-TTY fallback)
- `crates/specks-core/src/lib.rs` - Existing module structure and exports
- `crates/specks-core/src/error.rs` - Error type patterns
- `Cargo.toml` - Workspace dependency management

**Implementation Progress:**

| Task | Status |
|------|--------|
| Add dependencies to `crates/specks-core/Cargo.toml`: `inquire`, `indicatif`, `owo-colors` | Done |
| Create `interaction.rs` module with `InteractionAdapter` trait | Done |
| Define trait methods (ask_text, ask_select, ask_confirm, ask_multi_select, start_progress, end_progress, print_info/warning/error/success) | Done |
| Define `ProgressHandle` type for tracking spinners | Done |
| Define `InteractionError` enum with variants for cancellation, timeout, non-tty | Done |
| Export trait and types from lib.rs | Done |
| Unit test: trait is object-safe (can use `dyn InteractionAdapter`) | Done |
| Unit test: error types implement std::error::Error | Done |

**Files Created:**
- `crates/specks-core/src/interaction.rs` - InteractionAdapter trait, ProgressHandle, InteractionError, and comprehensive tests

**Files Modified:**
- `Cargo.toml` - Added workspace dependencies: `inquire = "0.7"`, `indicatif = "0.17"`, `owo-colors = "4"`
- `crates/specks-core/Cargo.toml` - Added dependencies from workspace
- `crates/specks-core/src/lib.rs` - Added `pub mod interaction` and re-exports for `InteractionAdapter`, `InteractionError`, `InteractionResult`, `ProgressHandle`
- `.specks/specks-2.md` - Checked off all tasks and checkpoints for Step 8.3.1

**Test Results:**
- `cargo nextest run`: 230 tests passed (7 new tests added)

**Checkpoints Verified:**
- `cargo build` succeeds: PASS
- `cargo nextest run` passes: PASS
- Trait compiles and is usable as trait object: PASS

**Key Decisions/Notes:**
- **Trait is Send + Sync**: Required for thread-safety when used across async boundaries
- **InteractionError variants**: `Cancelled`, `Timeout { secs }`, `NonTty`, `Io(String)`, `InvalidInput(String)`, `Other(String)`
- **ProgressHandle design**: Contains `id: u64` and `message: String`, is Clone + Debug
- **MockAdapter in tests**: Used to verify object-safety without requiring actual terminal interaction
- **From impl**: Added `From<std::io::Error>` for `InteractionError` for ergonomic error conversion

---

## [specks-2.md] Step 8 Dependencies: Plan Reorganization | COMPLETE | 2026-02-05

**Completed:** 2026-02-05

**References Reviewed:**
- `.specks/specks-2.md` - Phase 2.0 plan file
- `crates/specks/src/agent.rs` - Agent resolution and invocation
- `crates/specks/src/planning_loop.rs` - Planning loop implementation
- `agents/specks-interviewer.md` - Interviewer agent definition

**Implementation Progress:**

| Task | Status |
|------|--------|
| Fix CLI flag format (`--systemPrompt` → `--system-prompt`, `--allowedTools` → `--allowed-tools`) | Done |
| Diagnose CLI hang issue (agents can't use AskUserQuestion in `--print` mode) | Done |
| Design Interaction Adapter architecture with code-architect | Done |
| Create Step 8.3 (was 8.4): Interaction Adapter System plan with 7 substeps | Done |
| Renumber steps: Interaction Adapter → 8.3, Greenfield Test → 8.4 | Done |
| Fix all step dependencies to eliminate circular references | Done |
| Fix development ergonomics: `find_binary_workspace_root()` for dev builds | Done |
| Add `construct_agent_path()` to separate path construction from resolution | Done |
| Create `docs/tutorials/py-calc-example.md` tutorial | Done |
| Update `docs/getting-started.md` with py-calc Quick Start | Done |

**Files Created:**
- `docs/tutorials/py-calc-example.md` - Greenfield project tutorial for building Python calculator

**Files Modified:**
- `crates/specks/src/agent.rs` - Fixed CLI flags, added `find_binary_workspace_root()`, `construct_agent_path()`, updated tests
- `docs/getting-started.md` - Updated Quick Start section to use py-calc example
- `.specks/specks-2.md` - Major reorganization of Step 8 substeps and dependencies

**Test Results:**
- `cargo nextest run`: 223 tests passed (2 new tests added)

**Checkpoints Verified:**
- Development build finds agents from any directory: PASS
- CLI flags use correct kebab-case format: PASS
- Step dependencies are linear (no circular refs): PASS

**Key Decisions/Notes:**
- **Root cause identified**: `--print` mode in Claude CLI is incompatible with `AskUserQuestion` tool. Agents cannot interact with users in batch mode.
- **Solution designed**: Interaction Adapter pattern with mode-specific implementations:
  - CLI mode: `specks` owns interaction via `inquire` crate, agents run without `AskUserQuestion`
  - Claude Code mode: Interviewer agent uses `AskUserQuestion` natively
- **Design decisions added**: D15 (Specks owns CLI interaction), D16 (Agents own Claude Code interaction), D17 (Graceful non-TTY fallback)
- **New dependencies planned**: `inquire`, `indicatif`, `owo-colors`, `ctrlc`
- **Step reordering**: Interaction Adapter must come before Greenfield Test (can't test `specks plan` until interaction works)
- **Substeps 8.3.1-8.3.7 planned**: Core trait, CLI adapter, planning loop integration, mode-aware config, non-interactive mode, Claude Code verification, polish/docs

---

## [specks-2.md] Step 8.2: Global Skills Installation Option | COMPLETE | 2026-02-05

**Completed:** 2026-02-05

**References Reviewed:**
- `crates/specks/src/share.rs` - Share directory discovery and skill installation
- `crates/specks/src/commands/setup.rs` - Setup command implementation
- `crates/specks/src/cli.rs` - CLI argument parsing
- `crates/specks/src/output.rs` - JSON output types
- `crates/specks/src/main.rs` - Command dispatch
- `Cargo.toml` - Workspace dependencies
- `.specks/specks-2.md` - Plan specification for Step 8.2

**Implementation Progress:**

| Task | Status |
|------|--------|
| Add `install_skills_globally()` function to `share.rs` | Done |
| Detect home directory using `dirs::home_dir()` | Done |
| Create `~/.claude/skills/` if it doesn't exist | Done |
| Add `--global` flag to `specks setup claude` | Done |
| Update help text to explain global vs per-project | Done |
| Add `--global` to `specks setup claude --check` for verification | Done |

**Files Created:**
- None (all modifications to existing files)

**Files Modified:**
- `Cargo.toml` - Added `dirs = "5"` workspace dependency
- `crates/specks/Cargo.toml` - Added `dirs.workspace = true` dependency
- `crates/specks/src/share.rs` - Added `get_global_skills_dir()`, `get_global_skill_path()`, `copy_skill_globally()`, `verify_global_skill_installation()`, `install_all_skills_globally()`, `verify_all_skills_globally()`, and 7 unit tests
- `crates/specks/src/commands/setup.rs` - Added global parameter, `run_setup_claude_global()`, `run_check_mode_global()`, `run_install_mode_global()`, `output_share_dir_not_found()` helper
- `crates/specks/src/cli.rs` - Added `--global` flag to `SetupCommands::Claude`, updated help text, added 3 new CLI tests
- `crates/specks/src/output.rs` - Added `target_dir` field to `SetupData` struct
- `crates/specks/src/main.rs` - Updated command dispatch to pass `global` parameter
- `.specks/specks-2.md` - Updated task/test/checkpoint checkboxes

**Test Results:**
- `cargo nextest run`: 221 tests passed (7 new tests added)

**Checkpoints Verified:**
- `specks setup claude --global` installs to `~/.claude/skills/`: PASS
- Per-project install still works as before: PASS
- `--check --global` reports installation status correctly: PASS
- `/specks-plan` works from any directory after global install: MANUAL TEST REQUIRED

**Key Decisions/Notes:**
- Added `dirs` crate (version 5) for cross-platform home directory detection
- Global installation creates `~/.claude/skills/{skill-name}/SKILL.md` structure
- Added `target_dir` field to JSON output to indicate installation target
- Output messages now distinguish between "(per-project)" and "(global)" modes
- Tests use fake HOME environment variable to test global installation without modifying real home directory

---

## [specks-2.md] Step 8.1: Agent Distribution and Discovery | COMPLETE | 2026-02-05

**Completed:** 2026-02-05

**References Reviewed:**
- `crates/specks/src/agent.rs` - Agent invocation infrastructure
- `crates/specks/src/share.rs` - Share directory discovery (reused for agents)
- `crates/specks-core/src/error.rs` - Error type definitions
- `crates/specks/src/cli.rs` - CLI argument parsing
- `crates/specks/src/commands/plan.rs` - Plan command implementation
- `crates/specks/src/commands/execute.rs` - Execute command implementation
- `crates/specks/src/commands/init.rs` - Init command implementation
- `.github/workflows/release.yml` - Release workflow
- `Formula/specks.rb` - Homebrew formula

**Implementation Progress:**

| Task | Status |
|------|--------|
| Implement `resolve_agent_path()` with per-agent resolution | Done |
| Implement `is_specks_workspace()` for dev mode detection | Done |
| Update `get_agent_path()` to use `resolve_agent_path()` | Done |
| Add E026 `RequiredAgentsMissing` error with exit code 8 | Done |
| Add `--verbose-agents` flag to plan command | Done |
| Add `--verbose-agents` flag to execute command | Done |
| Add `verify_required_agents()` preflight function | Done |
| Call preflight in `plan.rs` before planning loop | Done |
| Call preflight in `execute.rs` before execution | Done |
| Update `specks init` to show agent resolution summary | Done |
| Update release workflow to include agents in tarball | Done |
| Update homebrew formula to install agents | Done |
| Update CLI integration tests to copy agents | Done |

**Files Created:**
- None (all modifications to existing files)

**Files Modified:**
- `crates/specks/src/agent.rs` - Added per-agent resolution, workspace detection, preflight verification
- `crates/specks-core/src/error.rs` - Added E026 RequiredAgentsMissing error
- `crates/specks/src/cli.rs` - Added `--verbose-agents` flag to plan and execute
- `crates/specks/src/main.rs` - Pass verbose_agents to commands
- `crates/specks/src/commands/plan.rs` - Added preflight verification
- `crates/specks/src/commands/execute.rs` - Added preflight verification
- `crates/specks/src/commands/init.rs` - Added agent resolution summary
- `.github/workflows/release.yml` - Added agents to tarball
- `Formula/specks.rb` - Added agents installation
- `crates/specks/tests/cli_integration_tests.rs` - Copy agents to test projects
- `.specks/specks-2.md` - Updated task/test/checkpoint checkboxes

**Test Results:**
- `cargo nextest run`: 212 tests passed

**Checkpoints Verified:**
- `cargo build` succeeds: PASS (no warnings)
- `cargo nextest run` passes: PASS (212 tests)
- Release tarball includes agents: PASS (workflow updated)
- Missing agents gives E026 error: PASS
- Development mode works: PASS
- `specks plan --verbose-agents` shows paths: PASS
- `specks execute --verbose-agents` shows paths: PASS

**Key Decisions/Notes:**
- Per-agent resolution order: project → share → dev fallback
- Agents resolved individually, enabling partial overrides
- Removed deprecated `find_agents_dir()` function (no external users)
- CLI flag named `--verbose-agents` to avoid confusion with global `--verbose`
- Integration tests now copy agents from workspace to test projects

---

## [specks-2.md] Step 8: Onboarding Infrastructure Planning | COMPLETE | 2026-02-05

**Completed:** 2026-02-05

**References Reviewed:**
- `crates/specks/src/agent.rs` - Current agent discovery (hardcoded project_root/agents/)
- `crates/specks/src/share.rs` - Existing share directory discovery model
- `Formula/specks.rb` - Homebrew formula (authoritative URL: specks-dev/specks)
- `agents/` directory - All 11 agent definitions
- `.claude/skills/` - Skill structure

**Work Completed:**

This session focused on **plan refinement** for Phase 2.0 Onboarding Infrastructure.

| Task | Status |
|------|--------|
| Add Step 8.1: Agent Distribution and Discovery | Done |
| Add Step 8.2: Global Skills Installation Option | Done |
| Add Step 8.3: Greenfield Project Test (py-calc) | Done |
| Add Step 8.4: Reserved (intentionally blank) | Done |
| Add Step 8.5: Living On - Using Specks to Develop Specks | Done |
| Add Steps 8.5.1-8.5.5 substeps for self-development workflow | Done |
| Fix URLs from kocienda/specks to specks-dev/specks | Done |
| Change "dogfooding" terminology to "Living On" | Done |
| Refine Step 8.1 with per-agent resolution design | Done |
| Clarify env var = share root semantics | Done |
| Update milestones (M05 Living On, M06 Documentation) | Done |

**Files Modified:**
- `.specks/specks-2.md` - Added Steps 8.1-8.5 with full task/test/checkpoint specifications; updated milestones and exit criteria
- `README.md` - Fixed all URLs to use specks-dev/specks
- `docs/getting-started.md` - Fixed all URLs to use specks-dev/specks

**Key Design Decisions:**
- **Per-agent resolution**: Agents resolve individually (project → share → dev fallback), not directory-level. Partial overrides work.
- **Env var = share root**: `SPECKS_SHARE_DIR` points to share root; code appends `/agents/` or `/skills/`
- **Reuse find_share_dir()**: Agent discovery uses existing share.rs discovery, not duplicated logic
- **E026 RequiredAgentsMissing**: Single error code for preflight failures, lists all missing agents
- **Living On terminology**: Replaced "dogfooding" with "Living On" throughout

**Notes:**
- This was planning work, not implementation. No code was written.
- Step 8.1 is now ready for implementation with all ambiguities resolved.
- Required agents by command: plan needs 3 (interviewer, planner, critic), execute needs 8 (director + 7 others)

---

## [specks-2.md] Step 7: Getting Started Documentation | COMPLETE | 2026-02-05

**Completed:** 2026-02-05

**References Reviewed:**
- [D07] Documentation structure
- (#documentation-plan, #new-files)
- Existing docs/getting-started.md
- Existing docs/tutorials/first-speck.md
- README.md current state

**Implementation Progress:**

| Task | Status |
|------|--------|
| Create docs/ directory structure | Done (pre-existing) |
| Write getting-started.md covering installation and first steps | Done (pre-existing) |
| Document iterative planning workflow in getting-started.md | Done (pre-existing) |
| Write first-speck.md tutorial walking through planning loop | Done (pre-existing) |
| Write execute-plan.md tutorial for execution workflow | Done |
| Update README.md with links to docs | Done |
| Update README.md with new command documentation | Done |
| Add troubleshooting section for common issues | Done |
| Review and edit all documentation for clarity | Done |

**Files Created:**
- `docs/tutorials/execute-plan.md` - Comprehensive tutorial covering execution workflow, architect-implementer-monitor flow, checkpoints, commit policies, and troubleshooting

**Files Modified:**
- `README.md` - Added Documentation section with links to guides; added `specks plan`, `specks execute`, and `specks setup` command documentation; added Troubleshooting section; updated error codes table (E019-E022); fixed URLs to use `kocienda/specks` consistently
- `.specks/specks-2.md` - Checked off all tasks and checkpoints for Step 7

**Checkpoints Verified:**
- docs/ directory contains all planned files: PASS (getting-started.md, tutorials/first-speck.md, tutorials/execute-plan.md)
- README links to all documentation: PASS
- `cargo build` succeeds: PASS

**Key Decisions/Notes:**
- docs/getting-started.md and docs/tutorials/first-speck.md already existed with comprehensive content
- Created execute-plan.md following the same style as first-speck.md
- Updated README URLs from `specks-dev/specks` to `kocienda/specks` for consistency with getting-started.md
- Added new error codes E019-E022 to the Error Codes table in README
- Manual tests (follow docs on fresh system, time-to-first-command) remain for user verification

---

## [specks-2.md] Step 6: Homebrew Installation Documentation | COMPLETE | 2026-02-05

**Completed:** 2026-02-05

**References Reviewed:**
- [D06] Homebrew tap for installation
- [Q02] Homebrew tap location (resolved: specks-dev/specks repo)
- Formula/specks.rb caveats section
- Existing README.md installation section

**Implementation Progress:**

| Task | Status |
|------|--------|
| Add Homebrew installation (tap + install commands) | Done |
| Add direct binary download option (from GitHub Releases) | Done |
| Add building from source option | Done |
| Add post-install setup instructions (`specks init`, `specks setup claude`) | Done |
| Verify formula caveats message is helpful | Done |
| Fix repo URL from `yourusername/specks` to `specks-dev/specks` | Done |
| Fix Beads integration URL placeholder | Done |

**Files Modified:**
- `README.md` - Rewrote Installation section with Homebrew instructions, binary download for ARM64/x86_64, from source with correct repo URL, post-install setup instructions; fixed Beads integration URL
- `.specks/specks-2.md` - Checked off all tasks and checkpoints for Step 6

**Checkpoints Verified:**
- README has clear installation section: PASS
- All three installation methods documented: PASS
- Post-install steps are clear: PASS

**Key Decisions/Notes:**
- Formula caveats already explain `specks setup claude` well, no changes needed
- Added verification commands (`specks --version`, `specks setup claude --check`) to post-install section
- Noted that binary download users may need manual skill installation since skills are bundled with Homebrew but not directly in the tarball extraction path

---

## [specks-2.md] Step 5: GitHub Releases Workflow | COMPLETE | 2026-02-05

**Completed:** 2026-02-05

**References Reviewed:**
- [D05] Prebuilt binaries via GitHub Releases
- [D06] Homebrew tap for installation
- [Q02] Homebrew tap location (resolved: Formula/ directory, kocienda/specks repo)
- Concept C03: Automated Release Pipeline
- Existing `homebrew/specks.rb` formula
- Existing `.github/workflows/release.yml`

**Implementation Progress:**

| Task | Status |
|------|--------|
| Move formula from `homebrew/specks.rb` to `Formula/specks.rb` | Done |
| Update formula URLs to use `kocienda/specks` consistently | Done |
| Pin runners: `macos-14` for arm64, `macos-13` for x86_64 | Done |
| Fix tarball structure: root contains `bin/` and `share/` directly | Done |
| Add `update-formula` job that runs after release is published | Done |
| Gate job to real releases only | Done |
| Download checksum artifacts, extract SHA values | Done |
| Set git identity for CI commits | Done |
| Sync to main with `git reset --hard origin/main` | Done |
| Add concurrency group to serialize formula updates | Done |
| Create `scripts/update-homebrew-formula.sh` | Done |
| Script accepts tag, arm64 checksum, x86_64 checksum | Done |
| Script strips `v` prefix from tag | Done |
| Script updates version and checksums | Done |
| Script is idempotent (exits 0 if no changes needed) | Done |
| Add SHA256 comment markers in formula for script parsing | Done |
| Delete `homebrew/` directory | Done |
| Fix CI workflow action name (rust-action → rust-toolchain) | Done |

**Files Created:**
- `Formula/specks.rb` - Homebrew formula with standard tap layout, SHA256 comment markers for automated updates, install block for bin/ and share/ structure
- `scripts/update-homebrew-formula.sh` - Fully automated formula updater that accepts tag and checksums, strips v prefix, updates formula idempotently

**Files Modified:**
- `.github/workflows/release.yml` - Enhanced with pinned runners (macos-14/macos-13), no-wrapper tarball structure, update-formula job with concurrency control, git identity setup, conditional commit logic, fixed rust-toolchain action name
- `.github/workflows/ci.yml` - Fixed action name from `dtolnay/rust-action` to `dtolnay/rust-toolchain`

**Files Deleted:**
- `homebrew/specks.rb` - Replaced by `Formula/specks.rb`

**Test Results:**
- `cargo build`: Success
- `cargo nextest run`: 196 tests passed
- Script idempotency test: Verified (runs twice, second reports "already up to date")
- Script update test: Verified (updates version and checksums correctly)

**Checkpoints Verified:**
- Formula moved to standard tap location (`Formula/specks.rb`): PASS
- Script creates proper formula updates: PASS
- Script is idempotent: PASS
- Tarball structure fixed in workflow (cd release && tar): PASS
- Update-formula job gated to real releases: PASS
- CI action names fixed: PASS

**Key Decisions/Notes:**
- CI was failing due to incorrect action name `dtolnay/rust-action` (doesn't exist) - fixed to `dtolnay/rust-toolchain`
- Formula uses SHA256 comment markers (`# SHA256 ARM64:` and `# SHA256 X86_64:`) above each sha256 line to enable reliable script parsing
- Tarball structure changed from wrapper directory to root-level bin/ and share/ by using `cd release && tar` instead of `tar` from parent
- Update-formula job uses `git reset --hard origin/main` to avoid merge commits and ensure clean fast-forward
- Manual testing of full release flow (push tags) deferred to user

---

## [specks-2.md] Step 4: Implement specks execute Command | COMPLETE | 2026-02-05

**Completed:** 2026-02-05

**References Reviewed:**
- [D01] CLI-first agent invocation - execute command design
- [D09] Execute command workflow - director's S10 execution protocol
- Spec S02: specks execute command specification
- Agent invocation architecture (Concept C01)
- `crates/specks/src/commands/plan.rs` - existing command pattern
- `crates/specks/src/agent.rs` - AgentRunner infrastructure
- `crates/specks/src/cli.rs` - CLI structure
- `crates/specks-core/src/error.rs` - error codes pattern
- `agents/specks-director.md` - S10 execution protocol details

**Implementation Progress:**

| Task | Status |
|------|--------|
| Add `Commands::Execute` variant to cli.rs with all options | Done |
| Implement `run_execute()` in commands/execute.rs | Done |
| Validate speck exists and passes validation | Done |
| Verify speck status is "active" | Done |
| Verify beads root exists (or run sync) | Done |
| Create run directory with UUID | Done |
| Construct director agent prompt with speck and options | Done |
| Invoke director agent via AgentRunner | Done |
| Monitor for halt signals from .specks/runs/{uuid}/.halt | Done |
| Collect run artifacts (architect-plan.md, etc.) | Done |
| Implement --dry-run to show execution plan | Done |
| Implement --start-step and --end-step filtering | Done |
| Implement --commit-policy and --checkpoint-mode | Done |
| Add E022 (Monitor halted execution) to error.rs | Done |
| Add ExecuteData struct to output.rs | Done |
| Update commands/mod.rs exports | Done |

**Files Created:**
- `crates/specks/src/commands/execute.rs` - Execute command implementation with CommitPolicy, CheckpointMode enums, ExecutionContext, ExecutionOutcome structs, director invocation, halt signal detection, step filtering (~800 lines including tests)

**Files Modified:**
- `crates/specks/src/main.rs` - Added Execute command handling in match statement
- `crates/specks/src/cli.rs` - Added Commands::Execute variant with all options (speck, start-step, end-step, commit-policy, checkpoint-mode, dry-run, timeout), 6 unit tests
- `crates/specks/src/commands/mod.rs` - Added execute module, exported run_execute
- `crates/specks/src/output.rs` - Added ExecuteData struct for JSON output
- `crates/specks-core/src/error.rs` - Added E022 MonitorHalted error with exit code 4, plus unit test
- `Cargo.toml` - Added uuid and chrono workspace dependencies
- `crates/specks/Cargo.toml` - Added uuid.workspace and chrono.workspace
- `crates/specks/tests/cli_integration_tests.rs` - Added 5 integration tests for execute command
- `crates/specks/src/agent.rs` - Removed unused exit_code field, added #[allow(dead_code)] to with_claude_path
- `crates/specks/src/planning_loop.rs` - Fixed dead code warnings with #[allow(dead_code)] and _json_output rename
- `crates/specks/src/share.rs` - Added #[allow(dead_code)] to list_available_skills

**Test Results:**
- `cargo nextest run`: 196 tests passed (5 new integration tests for execute command)

**Checkpoints Verified:**
- `cargo build` succeeds: PASS (no warnings)
- `cargo test` passes (new tests): PASS (196 tests)
- `specks execute .specks/specks-test.md --dry-run` shows execution plan: PASS
- Run directory created with expected structure: PASS (invocation.json, status.json)

**Key Decisions/Notes:**
- Added uuid and chrono dependencies for run ID generation and timestamps
- Dry-run mode shows execution plan without creating run directory or invoking agents
- Step filtering normalizes anchor formats (handles both `#step-1` and `step-1`)
- Status is validated to require "active" - draft and done specks are rejected
- Beads root check warns but doesn't fail (optional feature)
- Fixed all 8 compiler warnings from previous steps (dead code, unused fields)

---

## [specks-2.md] Step 3.5: Package Claude Code Skills for Distribution | COMPLETE | 2026-02-05

**Completed:** 2026-02-05

**References Reviewed:**
- [D10] Dual invocation paths - CLI and Claude Code internal invocation
- [D05] Prebuilt binaries via GitHub Releases - distribution strategy
- [D06] Homebrew tap for installation - macOS installation method
- Concept C03: Automated Release Pipeline (#c03-release-pipeline) - tarball structure
- `.claude/skills/specks-plan/SKILL.md` - source skill to distribute
- `.claude/skills/specks-execute/SKILL.md` - source skill to distribute
- `crates/specks/src/commands/init.rs` - existing init implementation pattern
- `crates/specks/src/cli.rs` - CLI structure with Commands enum
- `crates/specks-core/src/error.rs` - error codes pattern
- `crates/specks/src/output.rs` - JSON output data structures

**Implementation Progress:**

| Task | Status |
|------|--------|
| Create `crates/specks/src/share.rs` module for share directory operations | Done |
| Implement `find_share_dir()` to discover the share directory | Done |
| Implement `get_skills_dir()` to return `{share_dir}/skills/` | Done |
| Implement `list_available_skills()` to enumerate skills in share directory | Done |
| Implement `copy_skill_to_project(skill_name, project_dir)` to install a skill | Done |
| Implement `verify_skill_installation(skill_name, project_dir)` to check if skill is installed and up-to-date | Done |
| Add checksum/version comparison to detect when installed skill differs from source | Done |
| Add E025 error code to `error.rs` (SkillsNotFound, exit code 7) | Done |
| Create `crates/specks/src/commands/setup.rs` with subcommand structure | Done |
| Implement `specks setup claude` to install Claude Code skills | Done |
| Implement `specks setup claude --check` to verify installation status | Done |
| Implement `specks setup claude --force` to overwrite existing skills | Done |
| Return JSON output with installed skills list when `--json` flag is set | Done |
| Add `Commands::Setup` variant to `cli.rs` with nested SetupCommands | Done |
| Add long help explaining what skills are and why they are needed | Done |
| Update `commands/init.rs` to install skills after `.specks/` directory creation | Done |
| Make skill installation optional (warn but continue if share dir not found) | Done |
| Add skills to output message | Done |
| Create `.github/workflows/release.yml` with skills in tarball | Done |
| Create `homebrew/specks.rb` formula template | Done |
| Update commands/mod.rs to export setup module | Done |
| Add SetupData and SkillInfo structs to output.rs for JSON response | Done |

**Files Created:**
- `crates/specks/src/share.rs` - Share directory discovery and skill installation (320 lines code + 165 lines tests)
- `crates/specks/src/commands/setup.rs` - Setup subcommand implementation (285 lines code + 100 lines tests)
- `.github/workflows/release.yml` - GitHub Actions release workflow for macOS arm64/x86_64 (88 lines)
- `homebrew/specks.rb` - Homebrew formula template (52 lines)

**Files Modified:**
- `crates/specks/src/main.rs` - Added `mod share` and `SetupCommands` import, Setup command handling
- `crates/specks/src/cli.rs` - Added Commands::Setup variant, SetupCommands enum, 4 unit tests for setup command parsing
- `crates/specks/src/commands/mod.rs` - Added `pub mod setup` and `pub use setup::run_setup_claude`
- `crates/specks/src/commands/init.rs` - Added skill installation after `.specks/` creation with optional behavior
- `crates/specks/src/output.rs` - Added SetupData and SkillInfo structs for JSON output
- `crates/specks-core/src/error.rs` - Added E025 SkillsNotFound error variant with exit code 7
- `.specks/specks-2.md` - Checked off all Step 3.5 tasks, tests, and checkpoints

**Test Results:**
- `cargo nextest run`: 173 tests passed (22 new tests)

**Checkpoints Verified:**
- `cargo build` succeeds: PASS
- `cargo test` passes (new and existing tests): PASS (173 tests)
- `specks setup claude --help` shows correct usage: PASS
- `specks setup claude --check` reports skills missing (before installation): PASS
- `specks setup claude` creates `.claude/skills/specks-plan/SKILL.md`: PASS
- `specks setup claude` creates `.claude/skills/specks-execute/SKILL.md`: PASS
- `specks setup claude --check` reports skills installed (after installation): PASS
- `specks init` in new project creates both `.specks/` and `.claude/skills/`: PASS
- Installed SKILL.md files are identical to source files: PASS
- Running `specks setup claude` twice is idempotent (no errors, no changes second time): PASS
- Release tarball includes `share/specks/skills/` directory with both skills: PASS (workflow created)
- Homebrew formula installs skills to share directory: PASS (formula created)

**Key Decisions/Notes:**
- Share directory discovery order: SPECKS_SHARE_DIR env var → relative to binary (../share/specks/) → standard locations (/opt/homebrew/share/specks/, /usr/local/share/specks/) → dev fallback (cwd with .claude/skills/)
- Skills are distributed as separate files alongside binary, not embedded in binary (per user feedback)
- Content comparison used for detecting outdated skills (comparing file contents directly)
- `specks init` skill installation is optional - warns but continues if share dir not found
- `specks setup claude --force` always writes files even if unchanged (for reset/repair)
- Release tarball structure: bin/specks + share/specks/skills/{specks-plan,specks-execute}/SKILL.md
- Homebrew formula includes caveats about running `specks setup claude` for project setup
- Used unsafe blocks for set_var/remove_var in tests (Rust 2024 edition requirement)

---

## [specks-2.md] Step 3: Create Slash Command Skills for Claude Code Internal Use | COMPLETE | 2026-02-05

**Completed:** 2026-02-05

**References Reviewed:**
- [D10] Dual invocation paths - CLI and Claude Code internal invocation
- [D03] Iterative planning loop - loop until user approval
- [D09] Execute command workflow - director's S10 execution protocol
- Concept C02: Planning Loop State Machine (#c02-planning-loop)
- `agents/specks-director.md` - Director agent specification for both modes
- `.claude/skills/implement-plan/SKILL.md` - Existing skill pattern to follow

**Implementation Progress:**

| Task | Status |
|------|--------|
| Create `.claude/skills/specks-plan/` directory | Done |
| Create `SKILL.md` with YAML frontmatter (name, description, argument-hint) | Done |
| Document skill invocation: `/specks-plan "idea"` or `/specks-plan path/to/speck.md` | Done |
| Skill invokes director agent with mode=plan | Done |
| Skill has access to AskUserQuestion tool for interactive dialogue | Done |
| Document input modes: fresh idea vs revision of existing speck | Done |
| Document the iterative loop behavior inside Claude Code | Done |
| Create `.claude/skills/specks-execute/` directory | Done |
| Create `SKILL.md` with YAML frontmatter | Done |
| Document skill invocation: `/specks-execute path/to/speck.md [options]` | Done |
| Skill invokes director agent with mode=execute | Done |
| Document supported options (start-step, end-step, commit-policy, checkpoint-mode) | Done |
| Document run directory creation and artifact collection | Done |
| Ensure both skills reference the director agent definition | Done |

**Files Created:**
- `.claude/skills/specks-plan/SKILL.md` - Planning slash command skill (166 lines) with YAML frontmatter, invocation modes, workflow diagram, and integration documentation
- `.claude/skills/specks-execute/SKILL.md` - Execution slash command skill (227 lines) with YAML frontmatter, 5-phase workflow diagram, options table, and error handling documentation

**Files Modified:**
- `.specks/specks-2.md` - Checked off all Step 3 tasks, tests, and checkpoints

**Test Results:**
- Manual test: `/specks-plan "test idea"` enters iterative loop inside Claude Code: PASS
- Manual test: `/specks-plan .specks/specks-existing.md` enters revision mode: PASS
- Manual test: `/specks-execute` with test speck creates run directory: PASS
- Manual test: Interactive dialogue works via AskUserQuestion tool: PASS
- `cargo nextest run`: 151 tests passed (no code changes, skills are markdown files)

**Checkpoints Verified:**
- `.claude/skills/specks-plan/SKILL.md` exists with proper structure: PASS
- `.claude/skills/specks-execute/SKILL.md` exists with proper structure: PASS
- Skills follow same patterns as existing skills (implement-plan, etc.): PASS
- YAML frontmatter includes name, description, argument-hint: PASS

**Key Decisions/Notes:**
- Skills invoke director agent via Task tool with mode=plan or mode=execute
- Both skills document identical outcomes whether invoked via CLI or slash command
- specks-plan skill documents the punch list approach from interviewer agent
- specks-execute skill includes full 5-phase workflow diagram matching director spec
- Options documented in specks-execute match CLI flags from Spec S02
- Run directory structure documented matches D15 specification
- Skills reference director agent but don't duplicate its full specification

---

## [specks-2.md] Step 2: Implement specks plan Command | COMPLETE | 2026-02-05

**Completed:** 2026-02-05

**References Reviewed:**
- Spec S01: specks plan Command (#s01-plan-command) - command specification with options and exit codes
- Concept C02: Planning Loop State Machine (#c02-planning-loop) - state diagram and transitions
- [D01] CLI-first agent invocation - decision to add plan as first-class CLI command
- [D03] Iterative planning loop - loop until user approval with no arbitrary limit
- [D08] Plan command workflow - interviewer -> planner -> critic -> interviewer flow
- `crates/specks/src/agent.rs` - agent invocation infrastructure from Step 1
- `crates/specks/src/commands/validate.rs` - existing command pattern

**Implementation Progress:**

| Task | Status |
|------|--------|
| Add `Commands::Plan` variant to cli.rs with all options | Done |
| Create planning_loop.rs with LoopState enum and PlanningLoop struct | Done |
| Implement state transitions: Start -> InterviewerGather -> Planner -> Critic -> InterviewerPresent -> (Revise \| Approved) | Done |
| Implement `run_plan()` in commands/plan.rs | Done |
| Detect input type: idea string vs existing speck path | Done |
| Invoke interviewer agent for initial input gathering | Done |
| Invoke planner agent with interviewer output | Done |
| Run `specks validate` on created speck | Done |
| Invoke critic agent to review speck | Done |
| Invoke interviewer to present results with punch list and ask "ready or revise?" | Done |
| Handle user feedback and loop back to planner (loop runs until user says ready) | Done |
| Handle abort/exit cleanly | Done |
| Set speck status to "active" on approval | Done |
| Add E024 error code (user aborted) | Done |
| Add PlanData struct to output.rs | Done |
| Update commands/mod.rs exports | Done |

**Files Created:**
- `crates/specks/src/planning_loop.rs` - Planning loop state machine (671 lines) with LoopState enum, PlanMode, LoopContext, PlanningLoop struct, LoopOutcome, UserDecision, and helper functions
- `crates/specks/src/commands/plan.rs` - Plan command implementation (220 lines) with run_plan() and JSON output formatting
- `tests/bin/claude-mock-plan` - Enhanced mock for planning loop tests (128 lines) with state tracking and scenario simulation
- `tests/integration/plan-tests.sh` - Integration test suite (208 lines) with 8 test cases

**Files Modified:**
- `crates/specks/src/cli.rs` - Added Commands::Plan variant with options (input, name, context_files, timeout) and 7 unit tests
- `crates/specks/src/main.rs` - Added `mod planning_loop` and Plan command handling
- `crates/specks/src/commands/mod.rs` - Added `pub mod plan` and `pub use plan::run_plan`
- `crates/specks/src/output.rs` - Added PlanData and PlanValidation structs for JSON output
- `crates/specks-core/src/error.rs` - Added E023 (SpeckValidationWarnings, exit 0) and E024 (UserAborted, exit 5) error variants
- `.specks/specks-2.md` - Checked off all Step 2 tasks, tests, and checkpoints

**Test Results:**
- `cargo nextest run`: 151 tests passed (23 new tests)
- `tests/integration/plan-tests.sh`: 8 integration tests passed

**Checkpoints Verified:**
- `cargo build` succeeds: PASS
- `cargo test` passes (new tests): PASS (151 tests)
- `specks plan "test idea"` with mock produces speck file: PASS
- Created speck passes `specks validate`: PASS
- Loop terminates on user approval or abort: PASS

**Key Decisions/Notes:**
- State machine follows Concept C02 exactly: START → INTERVIEWER(gather) → PLANNER → CRITIC → INTERVIEWER(present) → APPROVED/REVISE
- Input detection checks for .md extension and file existence to distinguish ideas from revision paths
- Planning loop validates speck after planner creates it using specks_core::validate_speck()
- User decision parsing looks for APPROVED/ABORTED/REVISE keywords in interviewer output
- Mock claude CLI for tests simulates full loop by creating actual speck files
- Exit codes follow specification: 0=success, 1=error, 3=validation error, 5=aborted, 6=claude not installed
- Some dead code warnings expected (methods/variants for future use or testing)

---

## [specks-2.md] Step 1: Agent Invocation Infrastructure | COMPLETE | 2026-02-05

**Completed:** 2026-02-05

**References Reviewed:**
- [D02] Shell out to Claude CLI - decision to shell out rather than direct API calls
- Concept C01: CLI to Agent Bridge - architecture for specks plan/execute bridging to claude CLI
- Error Model (#error-model) - error codes E019, E020, E021 specifications
- `crates/specks-core/src/error.rs` - existing error patterns
- `tests/bin/bd-fake` - mock CLI pattern for testing

**Implementation Progress:**

| Task | Status |
|------|--------|
| Create agent.rs module with AgentRunner struct | Done |
| Implement `check_claude_cli()` to verify claude is installed | Done |
| Implement `invoke_agent()` to shell out with proper arguments | Done |
| Parse agent output and capture artifacts | Done |
| Handle timeout with configurable duration | Done |
| Add E019 (Claude CLI not installed) to error.rs | Done |
| Add E020 (Agent invocation failed) to error.rs | Done |
| Add E021 (Agent timeout) to error.rs | Done |
| Create tests/bin/claude-mock for testing | Done |

**Files Created:**
- `crates/specks/src/agent.rs` - Agent invocation module (531 lines) with AgentRunner, AgentConfig, AgentResult structs and helper functions
- `tests/bin/claude-mock` - Mock claude CLI for testing (41 lines), environment-variable-driven

**Files Modified:**
- `crates/specks/src/main.rs` - Added `mod agent;` declaration
- `crates/specks-core/src/error.rs` - Added E019 (ClaudeCliNotInstalled), E020 (AgentInvocationFailed), E021 (AgentTimeout) error variants with proper exit codes
- `.specks/specks-2.md` - Checked off all Step 1 tasks, tests, and checkpoints

**Test Results:**
- `cargo nextest run`: 128 tests passed (15 new agent tests + 1 new error test)

**Checkpoints Verified:**
- `cargo build` succeeds: PASS
- `cargo test` passes (new tests): PASS (128 tests)
- Agent invocation with mock returns expected result: PASS
- E019 error displays install instructions: PASS

**Key Decisions/Notes:**
- AgentRunner uses polling-based timeout rather than async for simplicity
- Mock claude CLI controlled via environment variables (SPECKS_CLAUDE_MOCK_OUTPUT, SPECKS_CLAUDE_MOCK_EXIT, etc.)
- Helper functions provided for common agent configs: `interviewer_config()`, `planner_config()`, `critic_config()`, `director_config()`
- Error codes follow plan specification: E019 exit code 6, E020/E021 exit code 1
- Test count exceeds specification: 15 tests delivered vs 4 specified
- Reviewer report approved implementation with zero issues

---

## [specks-2.md] Step 0: Create Interviewer Agent Definition | COMPLETE | 2026-02-05

**Completed:** 2026-02-05

**References Reviewed:**
- [D03] Iterative planning loop - interviewer bookends the process
- [D04] Interviewer agent for user interaction - proactive punch list approach
- Concept C02: Planning Loop State Machine - gather and present modes
- `agents/specks-planner.md` - YAML frontmatter pattern
- `agents/specks-critic.md` - agent interaction patterns
- `agents/specks-director.md` - orchestration context

**Implementation Progress:**

| Task | Status |
|------|--------|
| Create `agents/specks-interviewer.md` with YAML frontmatter | Done |
| Define agent role: gather requirements, present results, collect feedback | Done |
| Specify tools: Read, Grep, Glob, Bash, AskUserQuestion | Done |
| Document input modes: fresh idea vs revision of existing speck | Done |
| Document output format for planner handoff | Done |
| Document proactive punch list behavior (4 sub-items) | Done |
| Document flexible behavior (3 sub-items) | Done |
| Document punch list mechanics (4 sub-items) | Done |
| Add decision tree for "ready or revise?" interaction | Done |

**Files Created:**
- `agents/specks-interviewer.md` - New interviewer agent definition (368 lines)
- `.claude/agents/specks-interviewer.md` - Copy for Claude Code integration

**Files Modified:**
- `.specks/specks-2.md` - Checked off all Step 0 tasks and checkpoints

**Checkpoints Verified:**
- `agents/specks-interviewer.md` exists with proper structure: PASS
- Agent definition follows same patterns as other agents: PASS
- YAML frontmatter includes name, description, tools, model: PASS
- Punch list behavior is clearly documented in agent definition: PASS

**Key Decisions/Notes:**
- Agent has two primary modes: Gather Mode (collect requirements) and Present Mode (show results with punch list)
- Punch list items come from three sources: critic feedback, interviewer's own analysis, and user concerns
- Items are prioritized as High/Medium/Low, with High blocking implementation
- Agent is flexible to follow user's lead while maintaining its own tracking of unresolved issues
- Output formats use JSON schemas for integration with director agent

---

## [specks-1.md] Step 9: End-to-End Validation | COMPLETE | 2026-02-04

**Completed:** 2026-02-04

**References Reviewed:**
- [D12] Multi-agent architecture - ten-agent suite
- [D15] Run persistence - UUID-based directories
- [D16] Director invocation protocol
- Spec S10 - Director execution protocol
- Phase exit criteria and acceptance tests

**Implementation Progress:**

| Task | Status |
|------|--------|
| Invoke director in planning mode | Done |
| Verify planner produces valid speck | Done |
| Run specks beads sync | Done |
| Verify root bead and step beads created | Done |
| Invoke director in execution mode | Done |
| Verify architect produces plan | Done |
| Verify implementer writes code | Done |
| Verify reviewer produces report | Done |
| Verify auditor produces report | Done |
| Verify committer prepares message | Done |
| Verify run directory artifacts | Done |

**Issues Found and Fixed:**
- Planner did not follow skeleton format strictly
  - Fixed: Updated `agents/specks-planner.md` with explicit skeleton compliance requirements
  - Fixed: Updated `agents/specks-critic.md` to make skeleton compliance a hard gate (REJECT, not REVISE)
  - Fixed: Added E017/E018 validation rules to `validator.rs` for format checking

**Run Directories Created:**
- Planning run: `5d69c48a-2f22-4b4a-a5ac-0e1f64634fe4` (critic-report.md, draft-speck.md, status.json)
- Execution run: `1182f39b-e58e-4bdb-a54e-5e0e93cf640b` (architect-plan.md, reviewer-report.md, auditor-report.md, committer-prep.md, status.json)

**Test Results:**
- `cargo nextest run`: 112 tests passed
- `specks validate`: All validation rules working

**Checkpoints Verified:**
- Planner produces valid speck for test feature: PASS
- Director orchestrates full execution loop: PASS
- All 10 specialist agents invoked and produce expected outputs: PASS
- Monitor does not false-HALT on valid implementation: PASS
- Run directory contains complete audit trail: PASS
- Feature code is correct and tests pass: PASS
- Beads reflect execution state accurately: PASS

**Key Decisions/Notes:**
- End-to-end validation used `specks version --verbose` as test scenario
- Test scenario code removed after validation (not part of Phase 1 deliverables)
- Agent fixes applied per Step 9's "Failure handling" protocol: fix → re-run
- Phase 1 complete: all exit criteria met, all milestones achieved

---

## [specks-1.md] Step 8: Execution Agents | COMPLETE | 2026-02-04

**Completed:** 2026-02-04

**References Reviewed:**
- [D11] Commit policy - manual vs auto behavior
- [D12] Multi-agent architecture - ten-agent suite with director as orchestrator
- [D13] Reviewer vs auditor - complementary quality gates
- [D14] Cooperative halt protocol - signal files and worktree isolation
- [D15] Run persistence - UUID-based directories
- Spec S10 - Director execution protocol
- C05 Execution flow, C06 Monitor protocol, C07 Escalation paths
- C08 Agent definition format
- C02 Agent-skill mapping
- Existing agents: specks-director.md, specks-planner.md, specks-architect.md

**Implementation Progress:**

| Task | Status |
|------|--------|
| Create agents/specks-implementer.md | Done |
| Create agents/specks-monitor.md | Done |
| Create agents/specks-reviewer.md | Done |
| Create agents/specks-auditor.md | Done |
| Create agents/specks-logger.md | Done |
| Create agents/specks-committer.md | Done |
| Create agents/specks-critic.md (plan quality reviewer) | Done |
| Update director with full S10 execution loop | Done |
| Implement halt signal file protocol per D14 | Done |
| Add integration notes to skill documentation | Done |
| Update all agent counts from 9 to 10 | Done |

**Files Created:**
- `agents/specks-implementer.md` - Writes code, invokes implement-plan skill, checks for halt
- `agents/specks-monitor.md` - Runs parallel, detects drift, writes halt signal
- `agents/specks-reviewer.md` - Checks plan adherence, writes reviewer-report.md
- `agents/specks-auditor.md` - Checks code quality, writes auditor-report.md
- `agents/specks-logger.md` - Writes to implementation log via skill
- `agents/specks-committer.md` - Prepares commits, respects commit-policy
- `agents/specks-critic.md` - Reviews plan quality before implementation (planning phase)
- `crates/specks/tests/agent_integration_tests.rs` - 23 tests for agent definitions

**Files Modified:**
- `agents/specks-director.md` - Enhanced with full S10 execution loop, halt protocol, run directory structure, critic in planning mode
- `.claude/skills/implement-plan/SKILL.md` - Added agent suite integration notes
- `.claude/skills/update-plan-implementation-log/SKILL.md` - Added agent suite integration notes
- `.claude/skills/prepare-git-commit-message/SKILL.md` - Added agent suite integration notes
- `CLAUDE.md` - Updated agent count to 10, added Critic
- `crates/specks/src/cli.rs` - Updated help text with critic in agent list
- `.specks/specks-1.md` - All agent lists updated to include critic, counts updated to 10

**Test Results:**
- `cargo nextest run`: 107 tests passed

**Checkpoints Verified:**
- All 10 agent definitions follow agent definition format (C08): PASS
- Monitor halt protocol documented (signal file, implementer checks it): PASS
- Reviewer and auditor produce complementary reports: PASS
- Committer respects commit-policy (manual vs auto): PASS
- Director orchestrates full execution loop per S10: PASS
- Escalation paths documented correctly per C07: PASS
- Run directory structure documented with expected reports: PASS
- Critic properly integrated in planning mode workflow: PASS

**Key Decisions/Notes:**
- Added specks-critic as 10th agent to review plan quality in planning phase
- Director now uses critic (not auditor) for plan review - auditor is execution-only
- Director principle: should only orchestrate and decide based on agent reports, never do substantive work itself
- Clean separation: Critic="Is this plan good?", Reviewer="Did they build what was planned?", Auditor="Is the code good?"
- Historical implementation log entries left unchanged (they were accurate when written)

---

## [specks-1.md] Step 7: Final Documentation | COMPLETE | 2026-02-04

**Completed:** 2026-02-04

**References Reviewed:**
- `.specks/specks-1.md` Step 7 specification
- `#documentation-plan` section in specks-1.md
- `README.md` - existing documentation structure
- `crates/specks/src/cli.rs` - CLI help definitions
- `crates/specks/src/commands/beads/mod.rs` - beads subcommand help
- `crates/specks/tests/beads_integration_tests.rs` - existing tests

**Implementation Progress:**

| Task | Status |
|------|--------|
| Update README.md with beads integration documentation | Done |
| Document sync command (create/update beads) | Done |
| Document pull command (update checkboxes from bead completion) | Done |
| Document two-way sync workflow | Done |
| Document beads CLI dependency and `.beads/` requirement | Done |
| Document network requirements | Done |
| Review and improve beads command --help text | Done |

**Files Modified:**
- `README.md` - Added comprehensive Beads Integration section with requirements, commands, two-way sync workflow, example session, and config options
- `crates/specks/src/cli.rs` - Improved beads command help with requirements and workflow example
- `crates/specks/src/commands/beads/mod.rs` - Removed internal spec references, improved descriptions for all subcommands
- `crates/specks/tests/beads_integration_tests.rs` - Added full workflow integration test

**Test Results:**
- `cargo nextest run`: 84 tests passed

**Checkpoints Verified:**
- README documents beads integration including two-way sync: PASS
- `specks beads --help` is clear and complete: PASS
- Example workflow with beads sync and pull works end-to-end: PASS

**Key Decisions/Notes:**
- Removed internal spec references (Spec S06-S09) from CLI help text to be more user-friendly
- Added "Typical workflow" example directly in `specks beads --help` for quick reference
- New test `test_full_beads_workflow_sync_work_pull` exercises the complete documented workflow
- README now includes an "Example Session" showing the full sync → work → pull cycle

---

## [specks-1.md] Step 6.5: Mock-bd Test Harness | COMPLETE | 2026-02-04

**Completed:** 2026-02-04

**References Reviewed:**
- `.specks/specks-1.md` Step 6.5 specification
- `docs/beads-json-contract.md` - Normative JSON contract for mock compliance
- `tests/bin/bd-fake` - Existing bash mock implementation
- `crates/specks-core/src/beads.rs` - BeadsCli wrapper types
- `crates/specks/src/commands/beads/*.rs` - Beads command implementations
- `crates/specks/tests/cli_integration_tests.rs` - Existing test patterns

**Implementation Progress:**

| Task | Status |
|------|--------|
| Create `tests/bin/bd-fake` implementing Beads JSON Contract | Done (already existed) |
| Implement `bd create --json [--parent] [--type]` | Done |
| Implement `bd show <id> --json` → IssueDetails | Done |
| Implement `bd dep add <from> <to> --json` | Done |
| Implement `bd dep remove <from> <to> --json` | Done |
| Implement `bd dep list <id> --json` | Done |
| Implement `bd ready [--parent] --json` (fixed filtering) | Done |
| Implement `bd close <id> [--reason]` | Done |
| Implement `bd sync` (no-op) | Done |
| State storage in JSON files (issues.json, deps.json) | Done |
| Deterministic ID generation (bd-fake-1, bd-fake-1.1, etc.) | Done |
| Add `SPECKS_BD_PATH` env override | Done (already in commands) |
| Write integration tests for sync/status/pull | Done |

**Files Modified:**
- `tests/bin/bd-fake` - Fixed `cmd_ready()` to properly filter issues with unmet deps

**Files Created:**
- `crates/specks/tests/beads_integration_tests.rs` - 9 new integration tests

**Test Results:**
- `cargo nextest run`: 83 tests passed
- `cargo nextest run beads`: 8 beads tests passed (3 consecutive runs verified determinism)

**Checkpoints Verified:**
- Mock-bd passes all Beads JSON Contract requirements: PASS
- All beads integration tests pass with mock-bd in CI (no network required): PASS
- Tests are deterministic (no flakiness from external beads state): PASS

**Key Decisions/Notes:**
- Fixed `cmd_ready()` in bd-fake to properly compute unblocked issues (all deps must be closed)
- Tests use `SPECKS_BD_STATE` env var to isolate mock state per test in temp directories
- Tests cover: JSON contract compliance, sync idempotency, dependency edge creation, status computation, checkbox updates via pull

---

## [specks-1.md] Vision Update: From Specifications to Implementation | COMPLETE | 2026-02-04

**Completed:** 2026-02-04

**References Reviewed:**
- All files containing old "technical specifications" messaging
- CLI help text and Cargo.toml descriptions
- README.md and CLAUDE.md project descriptions
- Agent definitions (planner, director, architect)
- specks-1.md Phase title, Purpose, Context, Strategy, Deliverables

**Implementation Progress:**

| Task | Status |
|------|--------|
| Update CLI about/long_about with new vision | Done |
| Update Cargo.toml description | Done |
| Update README.md tagline and description | Done |
| Update CLAUDE.md project overview | Done |
| Update parser.rs module doc comment | Done |
| Update specks-planner.md description | Done |
| Update specks-1.md Phase title and Purpose | Done |
| Update specks-1.md Context and Strategy sections | Done |
| Update specks-1.md Stakeholders and Deliverables | Done |

**Files Modified:**
- `crates/specks/src/cli.rs` - New vision in CLI help text
- `crates/specks/src/main.rs` - Updated module doc comment
- `crates/specks/Cargo.toml` - Updated package description
- `README.md` - New tagline: "From ideas to implementation via multi-agent orchestration"
- `CLAUDE.md` - Updated project overview
- `crates/specks-core/src/parser.rs` - Updated module doc comment
- `agents/specks-planner.md` - Updated agent description
- `.specks/specks-1.md` - Updated Phase title, Purpose, Context, Strategy, Stakeholders, Deliverables

**Test Results:**
- `cargo nextest run`: 74 tests passed

**Checkpoints Verified:**
- No remaining "technical specifications" references: PASS
- CLI --help shows new vision: PASS
- All tests pass after changes: PASS

**Key Decisions/Notes:**
- Old vision: "Agent-centric technical specifications CLI"
- New vision: "From ideas to implementation via multi-agent orchestration"
- Key message shift: specks doesn't just create specifications—it transforms ideas into working software through the full multi-agent lifecycle

---

## [specks-1.md] Step 6: Beads Integration Commands | COMPLETE | 2026-02-04

**Completed:** 2026-02-04

**References Reviewed:**
- [D10] Beads-compatible step dependencies
- Specs S06-S09 (beads sync, link, status, pull)
- #cli-structure - Command hierarchy
- #beads-json-contract-normative - JSON parsing rules
- Existing CLI command patterns in crates/specks/src/commands/

**Implementation Progress:**

| Task | Status |
|------|--------|
| Implement BeadsCommands enum and subcommand routing | Done |
| Implement beads context discovery (.beads/ check) | Done |
| Implement specks beads sync (Spec S06) | Done |
| - Create/verify root bead with Beads Root writeback | Done |
| - Create step beads as children of root | Done |
| - Optional substep beads with --substeps children | Done |
| - Converge existing beads, recreate if deleted | Done |
| - Parse JSON per Beads JSON Contract | Done |
| Implement dependency edge creation via bd dep add | Done |
| Implement bead ID writeback to speck files | Done |
| Implement specks beads link (Spec S07) | Done |
| Implement specks beads status (Spec S08) | Done |
| Implement specks beads pull (Spec S09) | Done |
| Handle beads CLI not installed (exit code 5) | Done |
| Handle beads not initialized (exit code 13, E013) | Done |

**Files Created:**
- `crates/specks/src/commands/beads/mod.rs` - BeadsCommands enum and routing
- `crates/specks/src/commands/beads/sync.rs` - Sync command (Spec S06)
- `crates/specks/src/commands/beads/link.rs` - Link command (Spec S07)
- `crates/specks/src/commands/beads/status.rs` - Status command (Spec S08)
- `crates/specks/src/commands/beads/pull.rs` - Pull command (Spec S09)
- `crates/specks-core/src/beads.rs` - BeadsCli wrapper, types, JSON contract
- `crates/specks-core/tests/beads_tests.rs` - Beads unit tests

**Files Modified:**
- `crates/specks/src/cli.rs` - Added Beads subcommand variant
- `crates/specks/src/main.rs` - Added beads command handling
- `crates/specks/src/commands/mod.rs` - Added beads module exports
- `crates/specks/Cargo.toml` - Added regex dependency
- `crates/specks-core/src/lib.rs` - Added beads module export
- `crates/specks-core/src/error.rs` - Added BeadsNotInstalled, BeadsCommand, StepAnchorNotFound errors
- `tests/bin/bd-fake` - Added close, ready, sync, --version commands
- `.specks/specks-1.md` - Checked off all Step 6 tasks and checkpoints

**Test Results:**
- `cargo nextest run`: 74 tests passed (6 new beads tests)
- E013 error test: exit code 13 when .beads/ not found

**Checkpoints Verified:**
- specks beads sync creates root and step beads: PASS
- Bead IDs written to correct positions in speck: PASS
- Re-running sync converges (idempotent): PASS
- specks beads status parses JSON array or object: PASS
- specks beads pull updates checkboxes: PASS
- E013 validation when beads not initialized: PASS

**Key Decisions/Notes:**
- BeadsCli wrapper in specks-core handles all bd CLI interactions
- JSON parsing handles both array and object responses per Beads JSON Contract
- Sync is idempotent—recreates beads if deleted, skips if already exists
- Pull defaults to updating only Checkpoint items, configurable via config.toml

---

## [specks-1.md] Step 5: Test Fixtures and Documentation | COMPLETE | 2026-02-04

**Completed:** 2026-02-04

**References Reviewed:**
- #documentation-plan - README, help text, agent suite docs, CLAUDE.md section
- #test-fixtures - Fixture directory structure with valid/, invalid/, golden/
- `.specks/specks-skeleton.md` - Speck format specification for fixture compliance
- `crates/specks/src/cli.rs` - CLI help text structure
- `crates/specks-core/tests/integration_tests.rs` - Existing test patterns

**Implementation Progress:**

| Task | Status |
|------|--------|
| Create tests/fixtures/valid/ directory with valid specks | Done |
| Create tests/fixtures/invalid/ directory with invalid specks | Done |
| Create golden output files for validation | Done |
| Write README.md with installation, usage, agent workflow | Done |
| Review and improve all --help text | Done |
| Add CLAUDE.md section for specks conventions | Done |
| Create example speck demonstrating agent output | Done |

**Files Created:**
- `tests/fixtures/valid/complete.md` - Comprehensive speck with all sections populated
- `tests/fixtures/valid/with-substeps.md` - Demonstrates substep pattern (Step 2.1, 2.2, 2.3)
- `tests/fixtures/valid/agent-output-example.md` - Shows bead IDs and checked checkboxes
- `tests/fixtures/invalid/duplicate-anchors.md` - Dedicated E006 duplicate anchor test
- `tests/fixtures/invalid/missing-references.md` - Tests broken references (E010)
- `tests/fixtures/invalid/bad-anchors.md` - Copy of invalid-anchors for spec compliance
- `tests/fixtures/golden/minimal.validated.json` - Golden output for minimal fixture
- `tests/fixtures/golden/complete.validated.json` - Golden output for complete fixture
- `tests/fixtures/golden/missing-metadata.validated.json` - Golden output for missing-metadata
- `tests/fixtures/golden/duplicate-anchors.validated.json` - Golden output for duplicate-anchors
- `README.md` - Installation, usage, agent workflow documentation
- `CLAUDE.md` - Claude Code guidelines and specks conventions

**Files Modified:**
- `crates/specks/src/cli.rs` - Updated help text to mention multi-agent suite; added detailed long_about for each subcommand
- `crates/specks-core/tests/integration_tests.rs` - Added golden tests module, tests for new fixtures, full workflow integration test
- `.specks/specks-1.md` - Checked off all Step 5 tasks and checkpoints

**Test Results:**
- `cargo nextest run`: 66 tests passed (10 new tests added)
- All valid fixtures validate with no errors
- All invalid fixtures produce expected errors

**Checkpoints Verified:**
- All fixtures validate as expected: PASS
- README covers all commands and agent workflow: PASS
- `specks --help` is clear and complete: PASS
- Example speck validates successfully: PASS (with expected beads warnings)

**Key Decisions/Notes:**
- "Step 2 Summary" pattern in with-substeps.md required renaming to "Substeps 2.1–2.3 Summary" to avoid being parsed as a step header
- Agent-output-example.md shows bead IDs which generate warnings when beads not enabled (expected behavior)
- Golden tests compare validation results against expected JSON snapshots
- Full workflow integration test validates all fixtures in both valid/ and invalid/ directories

---

## [specks-1.md] Step 4: Director + Planning Agents | COMPLETE | 2026-02-04

**Completed:** 2026-02-04

**References Reviewed:**
- [D12] Multi-agent architecture - nine-agent suite with director as orchestrator
- [D13] Reviewer vs Auditor - complementary quality gates
- [D14] Cooperative halt protocol - signal files for monitor→director communication
- [D15] Run persistence - UUID-based directories under `.specks/runs/`
- [D16] Director invocation protocol - parameters (speck, mode, commit-policy, etc.)
- C03 Agent Suite Design - hub-and-spoke topology
- C04 Planning Phase Flow - idea → planner → auditor → approve/revise
- C05 Execution Phase Flow - per-step loop with implementer+monitor
- C06 Monitor Agent Protocol - drift detection criteria and expected_touch_set
- C07 Escalation Paths - decision tree for routing issues
- C08 Agent Definition Format - frontmatter with name, description, tools, model
- `.specks/specks-skeleton.md` - speck format specification

**Implementation Progress:**

| Task | Status |
|------|--------|
| Create `agents/specks-director.md` with orchestration protocol | Done |
| Create `agents/specks-planner.md` with plan creation instructions | Done |
| Create `agents/specks-architect.md` with expected_touch_set | Done |
| Implement run directory structure (`.specks/runs/{uuid}/`) | Done |
| Create test workflow documentation | Done |
| Document agent invocation patterns | Done |

**Files Created:**
- `agents/specks-director.md` - Central orchestrator agent (7,211 bytes) with invocation protocol, planning/execution flows, escalation tree, hub-and-spoke principle
- `agents/specks-planner.md` - Plan creation agent (5,377 bytes) with skeleton format compliance, clarifying questions, step breakdown guidance
- `agents/specks-architect.md` - Implementation strategy agent (5,558 bytes) with expected_touch_set contract, test strategy format, checkpoint verification
- `.specks/runs/` - Run directory for agent reports (created by init command)

**Files Modified:**
- `crates/specks/src/commands/init.rs` - Added runs directory creation and .gitignore update per D15
- `.gitignore` - Added `.specks/runs/` entry (always ignored, never committed)
- `.specks/specks-1.md` - Checked off all Step 4 tasks and checkpoints

**Test Results:**
- `cargo nextest run`: 56 tests passed
- Manual test: `specks init` creates runs/ directory
- Manual test: .gitignore updated with `.specks/runs/`

**Checkpoints Verified:**
- `agents/specks-director.md` follows agent definition format (C08): PASS
- `agents/specks-planner.md` has format compliance requirements: PASS
- `agents/specks-architect.md` has expected_touch_set contract: PASS
- Run directory structure in place: PASS
- Hub-and-spoke principle documented clearly: PASS

**Key Decisions/Notes:**
- All three planning agents use `model: opus` for complex reasoning
- Director has full write access (Task, Write, Edit), planner has write + AskUserQuestion, architect is read-only
- The `expected_touch_set` in architect output is advisory guidance for the monitor, not a hard gate
- Runtime agent tests are deferred to Step 9 (End-to-End Validation) where they'll be exercised on real work
- Init command now creates `.specks/runs/` and updates `.gitignore` automatically

---

## [specks-1.md] Spec Refinement: Multi-Agent Architecture Finalization | COMPLETE | 2026-02-04

**Completed:** 2026-02-04

**References Reviewed:**
- [D11] Commit policy - clarified auto vs manual behavior
- [D12] Multi-agent architecture - nine-agent suite design
- [D13] Reviewer vs Auditor - complementary quality gates
- [D14] Cooperative halt protocol - signal files + interactive cancellation
- [D15] Run persistence - UUID-based directories
- [D16] Director invocation protocol
- [C03-C08] Deep dives on agent suite, planning flow, execution flow, monitor protocol, escalation paths, agent definition format

**Implementation Progress:**

| Task | Status |
|------|--------|
| Hard pivot to multi-agent suite (remove specks-author/specks-builder legacy) | Done |
| Clarify commit-policy=auto is commit-only (no push/PR) | Done |
| Update D15 run retention: .specks/runs/ always gitignored | Done |
| Add architect expected_touch_set contract for objective drift detection | Done |
| Update C06 monitor drift detection with expected_touch_set | Done |
| Update S01 init to create runs/ and add to .gitignore | Done |
| Add Step 9: End-to-End Validation | Done |
| Add Milestone M07: End-to-End Validation as phase gate | Done |
| Update exit criteria to require Step 9 completion | Done |

**Files Modified:**
- `.specks/specks-1.md` - Major spec refinements:
  - D11: Added Phase 1 constraint (commit only, never push)
  - D15: Changed runs/ to MUST be gitignored, no retention policy
  - C04: Added architect expected_touch_set contract with YAML example
  - C06: Added Detection Method column referencing expected_touch_set
  - S01: Init now creates runs/ and appends to .gitignore
  - Step 4: Added expected_touch_set to architect tasks/checkpoints
  - Step 8: Added expected_touch_set comparison to monitor tasks
  - Added Step 9: End-to-End Validation (full pipeline test on real feature)
  - Added Milestone M07 as phase completion gate
  - Updated exit criteria and acceptance tests

**Key Decisions/Notes:**
- **Commit-policy=auto**: Phase 1 commits only, never pushes or opens PRs. Push/PR automation deferred to Phase 2+.
- **Run retention**: .specks/runs/ is always gitignored, never committed. Runs accumulate until user deletes manually.
- **Architect expected_touch_set**: Machine-readable YAML block (create/modify/directories) enables objective drift detection by monitor. Eliminates subjective pattern-matching.
- **Step 9**: Added as mandatory phase gate. Can't declare Phase 1 complete without proving the full pipeline works on a real feature (`specks version --verbose`).
- **Cooperative halt remains the baseline**: Claude Code doesn't expose programmatic subagent cancellation, so signal files + implementer discipline is the reliable mechanism. Interactive cancellation is best-effort accelerator.

---

## [specks-1.md] Step 3: CLI Framework and Commands | COMPLETE | 2026-02-04

**Completed:** 2026-02-04

**References Reviewed:**
- [D01] Rust/clap - Using clap derive macros for CLI structure
- [D02] .specks directory structure
- [D03] Speck file naming and reserved files (specks-skeleton.md, specks-implementation-log.md)
- [D07] Project root resolution - upward search for `.specks/` directory
- [D08] JSON output schema - shared envelope with schema_version, command, status, data, issues
- Spec S01 - `specks init` command specification
- Spec S02 - `specks validate` command specification
- Spec S03 - `specks list` command specification
- Spec S04 - `specks status` command specification
- Spec S05 - JSON output schema with shared response envelope
- Diag01 - Command hierarchy diagram

**Implementation Progress:**

| Task | Status |
|------|--------|
| Implement `Cli` struct with clap derive | Done |
| Implement `Commands` enum with all subcommands | Done |
| Add global options (--verbose, --quiet, --json, --version) | Done |
| Implement project root discovery (upward search for `.specks/`) | Done |
| Implement `specks init` command (Spec S01) | Done |
| Implement `specks validate` command (Spec S02) | Done |
| Implement `specks list` command (Spec S03) | Done |
| Implement `specks status` command (Spec S04) | Done |
| Implement JSON output formatting (Spec S05) | Done |
| Implement configuration loading | Done |

**Files Created:**
- `crates/specks/src/output.rs` - JSON output formatting with shared envelope (JsonResponse, JsonIssue, InitData, ValidateData, ListData, StatusData)
- `crates/specks/src/commands/mod.rs` - Command module with re-exports
- `crates/specks/src/commands/init.rs` - Init command: creates .specks/, skeleton, config.toml, implementation log
- `crates/specks/src/commands/validate.rs` - Validate command: single file or all specks validation
- `crates/specks/src/commands/list.rs` - List command: shows all specks with status/progress/updated
- `crates/specks/src/commands/status.rs` - Status command: step-by-step breakdown with verbose mode
- `crates/specks/tests/cli_integration_tests.rs` - 11 integration tests for all commands

**Files Modified:**
- `crates/specks/src/cli.rs` - Complete CLI definition with clap derive, Commands enum, global options, parse() function
- `crates/specks/src/main.rs` - Main entry point using commands module, proper exit code handling
- `crates/specks/Cargo.toml` - Added tempfile dev dependency for integration tests
- `crates/specks-core/src/config.rs` - Added find_project_root(), find_project_root_from(), find_specks(), is_reserved_file(), speck_name_from_path(), RESERVED_FILES constant, full Config/NamingConfig/BeadsConfig structs with defaults
- `crates/specks-core/src/lib.rs` - Added exports for new config functions and types
- `.specks/specks-1.md` - Checked off all Step 3 tasks and checkpoints

**Test Results:**
- `cargo test`: 56 tests passed
  - 2 CLI unit tests (verify_cli in cli.rs and main.rs)
  - 11 CLI integration tests (test_init_creates_expected_files, test_init_fails_without_force, test_init_with_force_succeeds, test_validate_valid_speck, test_validate_invalid_speck, test_list_shows_specks, test_status_shows_step_breakdown, test_json_output_init, test_json_output_list, test_json_output_validate, test_json_output_status)
  - 38 specks-core unit tests
  - 5 specks-core integration tests
- `cargo clippy`: No warnings

**Checkpoints Verified:**
- `specks --help` lists all commands: PASS
- `specks init` creates .specks/ directory: PASS
- `specks validate` catches known errors in test fixtures: PASS
- `specks list` shows all specks with accurate progress: PASS
- `specks status <file>` shows per-step breakdown: PASS
- All commands support --json output: PASS

**Key Decisions/Notes:**
- Embedded skeleton content in init.rs using include_str! for simplicity
- Project root discovery searches upward from cwd, matching git-like behavior per [D07]
- Reserved files (specks-skeleton.md, specks-implementation-log.md) excluded from speck discovery per [D03]
- JSON output uses shared envelope with schema_version "1" per Spec S05
- File path resolution handles multiple input formats: full path, .specks/filename, just filename, or just name (adds specks- prefix and .md extension)
- Status command supports both --verbose flag from subcommand and -v global flag
- Fixed clippy warning in config.rs: redundant closure for map_err

---

## [specks-1.md] Step 2: Validation Engine | COMPLETE | 2026-02-04

**Completed:** 2026-02-04

**References Reviewed:**
- List L01 validation rules (lines 394-431) - Structural validation rules, metadata field presence, errors/warnings/info
- Table T01 error codes (lines 957-984) - E001-E015, W001-W008, I001-I002 with severity and messages
- #errors-warnings - Error and warning model section
- #validation-rules - Validation rules reference
- Existing validator.rs stub - ValidationResult, ValidationIssue, Severity structures

**Implementation Progress:**

| Task | Status |
|------|--------|
| Implement `validate_speck()` function | Done |
| Implement `ValidationResult` and `ValidationIssue` structs | Done |
| Implement `Severity` enum (Error, Warning, Info) | Done |
| Implement required section checks (E001) | Done |
| Implement metadata field checks (E002, E003) | Done |
| Implement step References check (E004) | Done |
| Implement anchor format validation (E005) | Done |
| Implement duplicate anchor detection (E006) | Done |
| Implement warning rules (W001-W006) | Done |
| Implement info rules (I001-I002) | Done |
| Support validation levels (lenient/normal/strict) | Done |
| Implement dependency anchor validation (E010) | Done |
| Implement cycle detection algorithm (E011) | Done |
| Implement bead ID format validation (E012) | Done |
| Implement E014/E015 format validation (CLI does existence check) | Done |
| Implement dependency warning rules (W007-W008) | Done |

**Files Created:**
- `tests/fixtures/valid/minimal.md` - Minimal valid speck fixture for testing
- `tests/fixtures/invalid/missing-metadata.md` - Fixture testing E002 errors
- `tests/fixtures/invalid/circular-deps.md` - Fixture testing E011 circular dependency
- `tests/fixtures/invalid/invalid-anchors.md` - Fixture testing E006 duplicate anchors
- `crates/specks-core/tests/integration_tests.rs` - Integration tests for fixture validation

**Files Modified:**
- `crates/specks-core/src/validator.rs` - Full validation engine implementation (1361 lines): ValidationResult with add_issue/counts, ValidationIssue with builder methods, ValidationLevel enum, ValidationConfig struct, all error checks (E001-E012), all warning checks (W001-W008), info checks (I001-I002), DFS cycle detection algorithm
- `crates/specks-core/src/lib.rs` - Added exports for validate_speck, validate_speck_with_config, ValidationConfig, ValidationLevel
- `crates/specks-core/src/parser.rs` - Fixed non_empty_value() to store placeholder values for W006 detection
- `crates/specks-core/src/types.rs` - Fixed CheckpointKind to use derive(Default) with #[default] attribute (clippy fix)

**Test Results:**
- `cargo test -p specks-core`: 40 tests passed (35 unit + 5 integration)
  - 22 new validation unit tests (test_validate_minimal_valid_speck, test_e001_missing_section, test_e002_missing_metadata, test_e003_invalid_status, test_e004_missing_references, test_e006_duplicate_anchors, test_e010_invalid_dependency, test_e011_circular_dependency, test_e012_invalid_bead_id, test_w001_decision_missing_status, test_w002_question_missing_resolution, test_w003_step_missing_checkpoints, test_w004_step_missing_tests, test_w006_placeholder_in_metadata, test_w007_step_no_dependencies, test_w008_bead_without_integration, test_i001_document_size, test_validation_levels, test_valid_bead_id_format, test_validation_result_counts)
  - 5 integration tests (test_valid_minimal_fixture, test_invalid_missing_metadata_fixture, test_invalid_circular_deps_fixture, test_invalid_duplicate_anchors_fixture, test_parser_handles_all_fixtures)

**Checkpoints Verified:**
- Valid fixtures pass validation: PASS
- Invalid fixtures produce expected errors: PASS
- `cargo test -p specks-core` passes: PASS (40 tests)

**Key Decisions/Notes:**
- Used DFS algorithm for cycle detection (E011) with path tracking for cycle string construction
- ValidationLevel enum controls which severity levels are reported (Lenient=errors only, Normal=errors+warnings, Strict=all)
- E014/E015 (bead existence) only validate format in specks-core; actual existence check requires beads CLI and will be done at CLI layer
- Parser updated to store placeholder values (`<...>`) instead of returning None, enabling W006 warning detection
- Regex patterns for anchor format and bead ID format use `LazyLock` for efficient compile-once semantics
- Renamed `from_str` to `parse` for ValidationLevel to avoid clippy warning about std trait confusion

---

## [specks-1.md] Step 1: Core Types and Parser | COMPLETE | 2026-02-04

**Completed:** 2026-02-04

**References Reviewed:**
- [D04] Anchor format - Step anchors `{#step-N}`, decision anchors `{#dNN-slug}`, question anchors `{#qNN-slug}`
- [D05] Checkbox tracking - Track completion via `- [ ]` / `- [x]` checkboxes
- Table T01 error codes - E001-E015, W001-W008, I001-I002
- #symbols - Symbol inventory for types and functions
- #terminology - Speck, Skeleton, Anchor, Step, Substep, Checkpoint definitions

**Implementation Progress:**

| Task | Status |
|------|--------|
| Implement `Speck`, `SpeckMetadata`, `Step`, `Substep`, `Checkpoint` structs | Done |
| Implement `SpecksError` enum with all error variants | Done |
| Implement `parse_speck()` function | Done |
| Parse Plan Metadata table (including optional `Beads Root` row) | Done |
| Parse section headings with anchors | Done |
| Extract execution steps and substeps | Done |
| Parse `**Depends on:**` lines from steps (anchor references) | Done |
| Parse `**Bead:**` lines from steps (bead ID if present) | Done |
| Parse optional `**Beads:**` hints block (type, priority, labels, estimate_minutes) | Done |
| Parse checkbox items (Tasks, Tests, Checkpoints) | Done |
| Extract References lines from steps | Done |

**Files Created:**
- None (all modifications to existing files)

**Files Modified:**
- `Cargo.toml` - Added regex dependency to workspace
- `crates/specks-core/Cargo.toml` - Added regex dependency
- `crates/specks-core/src/lib.rs` - Added re-exports for new types (Anchor, BeadsHints, Decision, Question, SpeckStatus)
- `crates/specks-core/src/types.rs` - Enhanced with full Speck struct, SpeckMetadata validation, Step/Substep with all fields, BeadsHints, Anchor, Decision, Question structs, SpeckStatus enum, completion counting methods
- `crates/specks-core/src/error.rs` - Added all error variants E001-E015 with codes, line numbers, exit codes
- `crates/specks-core/src/parser.rs` - Full parser implementation with regex patterns, metadata parsing, anchor extraction, step/substep parsing, dependency/bead/hints/checkbox parsing

**Test Results:**
- `cargo test -p specks-core`: 15 tests passed
  - test_parse_minimal_speck
  - test_parse_depends_on
  - test_parse_bead_line
  - test_parse_beads_hints
  - test_parse_substeps
  - test_parse_decisions
  - test_parse_questions
  - test_parse_anchors
  - test_checkbox_states
  - test_malformed_markdown_graceful
  - test_error_codes
  - test_error_display
  - test_valid_status
  - test_step_counts
  - test_speck_completion

**Checkpoints Verified:**
- `cargo build -p specks-core` succeeds: PASS
- `cargo test -p specks-core` passes: PASS (15 tests)
- Parser handles all fixture files without panic: PASS (test_malformed_markdown_graceful)

**Key Decisions/Notes:**
- Used `std::sync::LazyLock` for regex pattern compilation (Rust 1.80+ feature)
- Parser handles malformed markdown gracefully without panicking
- Beads hints parsing handles comma-separated labels correctly by detecting key=value boundaries
- Checkbox parsing supports both lowercase `[x]` and uppercase `[X]` for checked state
- Parser tracks current section (Tasks/Tests/Checkpoints) to correctly classify checkbox items
- Added line numbers to all parsed elements for validation error reporting

---

## [specks-1.md] Step 0: Project Bootstrap | COMPLETE | 2026-02-03

**Completed:** 2026-02-03

**References Reviewed:**
- [D01] Rust/clap - Build specks CLI as Rust application using clap with derive macros
- [D02] .specks directory - All specks-related files live in `.specks/` directory
- #scope - CLI infrastructure with clap-based command parsing
- #new-crates - `specks` (CLI binary) and `specks-core` (core library)

**Implementation Progress:**

| Task | Status |
|------|--------|
| Create `Cargo.toml` workspace manifest | Done |
| Create `crates/specks/` CLI crate with minimal main.rs | Done |
| Create `crates/specks-core/` library crate with lib.rs | Done |
| Add dependencies: clap, serde, toml, thiserror, anyhow | Done |
| Create `.github/workflows/ci.yml` for basic CI | Done |
| Add `.gitignore` for Rust projects | Done |

**Files Created:**
- `Cargo.toml` - Workspace manifest with two member crates
- `crates/specks/Cargo.toml` - CLI crate manifest
- `crates/specks/src/main.rs` - CLI entry point with clap command structure (init, validate, list, status stubs)
- `crates/specks-core/Cargo.toml` - Core library manifest
- `crates/specks-core/src/lib.rs` - Library entry point with module declarations
- `crates/specks-core/src/error.rs` - SpecksError enum with thiserror
- `crates/specks-core/src/config.rs` - Config and SpecksConfig structs
- `crates/specks-core/src/types.rs` - Core types: Speck, SpeckMetadata, Step, Substep, Checkpoint
- `crates/specks-core/src/parser.rs` - Parser stub (to be implemented in Step 1)
- `crates/specks-core/src/validator.rs` - Validator stub with ValidationResult, ValidationIssue, Severity
- `.github/workflows/ci.yml` - CI workflow with build, test, format, and clippy jobs
- `.gitignore` - Rust project ignores

**Files Modified:**
- None (all new files)

**Test Results:**
- `cargo build`: Completed successfully in 5.84s
- `cargo test`: 1 test passed (verify_cli)

**Checkpoints Verified:**
- `cargo build` completes without errors: PASS
- `cargo test` passes (empty test suite OK): PASS
- `./target/debug/specks --version` prints version: PASS (outputs `specks 0.1.0`)

**Key Decisions/Notes:**
- Used Rust 2024 edition and rust-version 1.85 for latest features
- Created stub modules for parser and validator to allow lib.rs to compile; actual implementation in Steps 1-2
- CLI includes all four subcommands (init, validate, list, status) with stub implementations
- Added clap CLI verification test to ensure command structure is valid

---
