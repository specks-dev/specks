## Phase 1.0: Resolve Specks Binary Deployment Gap {#phase-binary-deployment}

**Purpose:** Document and resolve the gap between locally-built specks binary (which includes `specks beads close`) and the outdated released binary in the user's PATH.

---

### Plan Metadata {#plan-metadata}

| Field | Value |
|-------|-------|
| Owner | specks team |
| Status | draft |
| Target branch | main |
| Tracking issue/PR | - |
| Last updated | 2026-02-08 |

---

### Phase Overview {#phase-overview}

#### Context {#context}

The `specks beads close` command was implemented and merged to main, but the user's installed binary (in PATH) is from an older release that predates this feature. Running `specks beads close` from the shell produces an "unrecognized subcommand" error because the PATH binary is outdated, not because the command is missing.

The local build at `./target/release/specks` has the correct implementation. This is a deployment/release gap, not a code bug.

#### Strategy {#strategy}

- Confirm the close command exists in the local build
- Document how to use the local build while PATH binary is outdated
- Release a new version to update the installed binary
- Verify the installed binary has the close command after release

#### Stakeholders / Primary Customers {#stakeholders}

1. Users running specks from PATH who need `specks beads close`
2. Implementer workflow that uses `specks beads close` via committer-agent

#### Success Criteria (Measurable) {#success-criteria}

- Running `specks beads close --help` from PATH binary shows help text (not "unrecognized subcommand")
- The released version number is greater than the currently installed version
- Committer-agent can successfully call `specks beads close`

#### Scope {#scope}

1. Verify local build has working close command
2. Document workaround for using local build
3. Release new version with close command
4. Update installed binary

#### Non-goals (Explicitly out of scope) {#non-goals}

- Implementing the close command (already done)
- Changing the close command behavior
- Automated release pipeline setup

#### Dependencies / Prerequisites {#dependencies}

- Local build must be up to date (`cargo build --release`)
- User must have release permissions (if publishing to registry)

#### Constraints {#constraints}

- Release process depends on project's release workflow
- User's PATH configuration is outside project control

#### Assumptions {#assumptions}

- The close command implementation at `crates/specks/src/commands/beads/close.rs` is complete and correct
- Local build reflects current main branch
- User can update their PATH binary via release or manual installation

---

### 1.0.0 Design Decisions {#design-decisions}

#### [D01] Local Build Confirms Implementation Exists (DECIDED) {#d01-local-build}

**Decision:** The `specks beads close` command is fully implemented; the error is from an outdated installed binary.

**Rationale:**
- `crates/specks/src/commands/beads/close.rs` exists with complete implementation
- Running `./target/release/specks beads close --help` works correctly
- The PATH binary predates the close command merge

**Implications:**
- No code changes required
- Resolution is deployment/release, not development

#### [D02] Release New Version to Resolve Gap (DECIDED) {#d02-release}

**Decision:** Release a new version of specks to make `beads close` available to users.

**Rationale:**
- Users should not need to run from source to access features
- Released binaries should match main branch functionality
- This enables implementer workflow to work out of the box

**Implications:**
- Requires version bump
- Requires release publication
- Users must update their installed binary

---

### 1.0.1 Specification {#specification}

#### 1.0.1.1 Version Information {#version-info}

**Current state:**
- Installed binary: Older version missing `beads close` subcommand
- Local build: Current main branch with `beads close` implemented

**Target state:**
- Installed binary: New release with `beads close` available

#### 1.0.1.2 Verification Commands {#verification-commands}

**Verify local build:**
```bash
cargo build --release
./target/release/specks beads close --help
```

**Verify installed binary (before fix):**
```bash
specks beads close --help
# Expected: "unrecognized subcommand 'close'"
```

**Verify installed binary (after fix):**
```bash
specks beads close --help
# Expected: Help text for close command
```

---

### 1.0.2 Symbol Inventory {#symbol-inventory}

#### 1.0.2.1 Files Involved {#files-involved}

| File | Status |
|------|--------|
| `crates/specks/src/commands/beads/close.rs` | Exists, complete |
| `crates/specks/src/commands/beads/mod.rs` | Exports close command |
| `Cargo.toml` | May need version bump for release |

---

### 1.0.3 Execution Steps {#execution-steps}

#### Step 0: Verify Local Build Has Close Command {#step-0}

**Commit:** N/A (verification only, no code changes)

**References:** [D01] Local build confirms implementation exists, (#verification-commands, #version-info)

**Artifacts:**
- Confirmation that local build works correctly

**Tasks:**
- [ ] Run `cargo build --release`
- [ ] Run `./target/release/specks beads close --help`
- [ ] Confirm help text appears (not "unrecognized subcommand")

**Tests:**
- [ ] Integration test: Local build responds to `beads close --help`

**Checkpoint:**
- [ ] `./target/release/specks beads close --help` shows help text
- [ ] Exit code is 0

**Rollback:**
- N/A - verification step only

---

#### Step 1: Document Workaround for Using Local Build {#step-1}

**Depends on:** #step-0

**Commit:** `docs: add workaround for using local build while PATH binary is outdated`

**References:** [D01] Local build confirms implementation exists, (#context, #verification-commands)

**Artifacts:**
- Updated documentation with workaround instructions

**Tasks:**
- [ ] Add section to README or CLAUDE.md explaining how to use local build
- [ ] Document `cargo run --` as alternative to `specks` command
- [ ] Document `./target/release/specks` as alternative

**Tests:**
- [ ] Manual test: Workaround instructions are clear and actionable

**Checkpoint:**
- [ ] Documentation contains workaround for PATH binary mismatch
- [ ] Both `cargo run --` and `./target/release/specks` approaches are documented

**Rollback:**
- Revert documentation commit

**Commit after all checkpoints pass.**

---

#### Step 2: Release New Version {#step-2}

**Depends on:** #step-0

**Commit:** `chore: release vX.Y.Z with beads close command`

**References:** [D02] Release new version to resolve gap, (#version-info)

**Artifacts:**
- New release with `beads close` command available
- Updated version in Cargo.toml

**Tasks:**
- [ ] Determine next version number
- [ ] Update version in `Cargo.toml`
- [ ] Create release (git tag, cargo publish, or install script)
- [ ] Update installed binary via release mechanism

**Tests:**
- [ ] Integration test: `specks --version` shows new version
- [ ] Integration test: `specks beads close --help` works from PATH

**Checkpoint:**
- [ ] `specks --version` shows released version number
- [ ] `specks beads close --help` shows help text (not error)

**Rollback:**
- Yank release if critical issues found
- Revert version bump commit

**Commit after all checkpoints pass.**

---

### 1.0.4 Deliverables and Checkpoints {#deliverables}

**Deliverable:** Released specks binary with working `specks beads close` command available in user's PATH.

#### Phase Exit Criteria ("Done means...") {#exit-criteria}

- [ ] `specks beads close --help` works from PATH binary (not "unrecognized subcommand")
- [ ] Released version includes the close command
- [ ] Workaround documentation exists for users who cannot immediately update

**Acceptance tests:**
- [ ] Integration test: Run `specks beads close bd-test --reason "test"` successfully
- [ ] Manual test: Committer-agent workflow completes without close command errors

#### Roadmap / Follow-ons (Explicitly Not Required for Phase Close) {#roadmap}

- [ ] Set up automated release pipeline to prevent future gaps
- [ ] Add version check warning when local build differs from installed

| Checkpoint | Verification |
|------------|--------------|
| Close command available | `specks beads close --help` returns 0 |
| Version updated | `specks --version` shows new release |

**Commit after all checkpoints pass.**
