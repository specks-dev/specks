##### Step 8.3.7.5: Complete Full-Featured Terminal Streaming with Anchored Spinner {#step-8-3-7-5}

**Depends on:** #step-8-3-6-5

**Commit:** `feat(cli): complete terminal streaming with indicatif MultiProgress for anchored spinner`

**References:** [D25] Streaming agent output with fixed spinner, (#d25-streaming-spinner, #terminal-streaming-architecture)

**Problem Statement:**

Step 8.3.6.5 attempted streaming but has fundamental failures:

1. **Screen clearing on start** - `Clear(ClearType::FromCursorDown)` clears content that should persist
2. **Spinner disappearing** - Save/restore cursor positions don't work with scrolling content
3. **Raw JSON chunks displayed** - Token-per-line output instead of flowing text
4. **No anchored layout** - Cursor positioning fails when content exceeds one screen

**Root Cause:**

The current approach uses cursor save/restore positioning (`cursor::SavePosition`/`cursor::RestorePosition`), which **fundamentally breaks when content scrolls**. Saved cursor coordinates become invalid as lines scroll off the top of the terminal.

**Solution: indicatif MultiProgress**

Replace manual cursor positioning with `indicatif::MultiProgress`, the battle-tested pattern used by Claude Code, cargo, npm, and docker. This provides:

- Automatic coordination of output with progress bars
- Content prints ABOVE the spinner via `mp.println()`
- Spinner stays fixed at bottom, auto-animates via `enable_steady_tick()`
- Cross-platform support (Windows, macOS, Linux)
- Output persists in terminal scrollback after completion

**Required UX:**

```
+------------------------------------------------------------------+
|  Content streams here via MultiProgress::println()               |
|  Scrolls naturally with normal terminal behavior                 |
|                                                                  |
|  The clarifier is analyzing the requirements.                    |
|  Based on the context, here are the ambiguities identified:      |
|  1. Authentication scope is unclear...                           |
|  2. Database choice not specified...                             |
+------------------------------------------------------------------+
| ⠼ Clarifier analyzing... [12.3s]          <- FIXED AT BOTTOM    |
+------------------------------------------------------------------+
```

**Architecture Document:** `.specks/terminal-streaming-architecture.md`

**Dependencies:**

The `indicatif` crate is already a transitive dependency via `inquire`. Make it explicit:

```toml
# In crates/specks/Cargo.toml
indicatif = "0.17"
```

**Artifacts:**
- Rewritten `crates/specks/src/streaming.rs` - MultiProgress-based streaming display
- Updated `crates/specks/src/agent.rs` - Simplified streaming invocation
- Updated `crates/specks/Cargo.toml` - Explicit indicatif dependency

**Tasks:**

*Phase 1: Core StreamingDisplay Rewrite*

- [ ] Add `indicatif = "0.17"` to `crates/specks/Cargo.toml` (make explicit)
- [ ] Rewrite `StreamingDisplay` struct to use `indicatif::MultiProgress`:
  ```rust
  pub struct StreamingDisplay {
      message: String,
      start_time: Instant,
      multi_progress: Option<MultiProgress>,  // None if non-TTY
      spinner: Option<ProgressBar>,           // None if non-TTY
      line_buffer: String,                    // Accumulates partial tokens
      is_tty: bool,
      cancelled: Arc<AtomicBool>,
      term_width: u16,
  }
  ```
- [ ] Implement `start()`:
  - Create `MultiProgress::new()`
  - Create spinner with `mp.add(ProgressBar::new_spinner())`
  - Set style with braille frames: `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`
  - Enable auto-tick: `spinner.enable_steady_tick(Duration::from_millis(100))`
  - Non-TTY: just `println!` the message
- [ ] Implement `write_content()`:
  - Accumulate tokens into `line_buffer`
  - On newline: extract complete line and call `mp.println(&line)`
  - `mp.println()` automatically prints ABOVE the spinner
- [ ] Implement `flush_buffer()`:
  - Print any remaining partial line on finish
- [ ] Implement `finish_success()`:
  - Flush buffer
  - Call `spinner.finish_and_clear()`
  - Print success message: `✓ {message} [{elapsed}]` (green)
- [ ] Implement `finish_error()`:
  - Flush buffer
  - Call `spinner.finish_and_clear()`
  - Print error message: `✗ {message} [{elapsed}]: {error}` (red)
- [ ] Remove all cursor save/restore logic
- [ ] Remove all `Clear(ClearType::*)` calls
- [ ] Remove `at_line_start` tracking (no longer needed)

*Phase 2: Agent Runner Integration*

- [ ] Simplify `invoke_agent_streaming()` in `agent.rs`:
  - Remove manual spinner tick calls (indicatif handles it)
  - Keep thread-based stdout reading (unchanged)
  - Keep 100ms receive timeout (for cancellation checks only, not spinner)
  - The main loop just calls `display.write_content(&content)` on receive
- [ ] Verify cancellation still works via `display.is_cancelled()`
- [ ] Verify timeout detection still works

*Phase 3: Word Wrapping and Polish*

- [ ] Add word wrapping for long lines:
  ```rust
  fn wrap_line(&self, line: &str) -> Vec<String> {
      // Split at word boundaries to fit term_width
  }
  ```
- [ ] Get terminal width: `crossterm::terminal::size().map(|(w, _)| w).unwrap_or(80)`
- [ ] Handle terminal resize gracefully (re-query width on demand, not critical)

*Phase 4: Cleanup*

- [ ] Remove unused imports from streaming.rs (cursor::SavePosition, etc.)
- [ ] Update doc comments to reflect MultiProgress approach
- [ ] Ensure Drop impl calls `spinner.finish_and_clear()` for unexpected drops

**Tests:**

Unit tests:
- [ ] `StreamingDisplay::new()` initializes correctly
- [ ] `write_content()` buffers partial tokens until newline
- [ ] `write_content()` with multiple newlines flushes multiple lines
- [ ] Non-TTY mode prints directly without MultiProgress
- [ ] `finish_success()` outputs correct format
- [ ] `finish_error()` outputs correct format
- [ ] Word wrap splits long lines correctly

Integration tests:
- [ ] Streaming invocation with mock agent captures all output
- [ ] Spinner remains visible during streaming (visual verification)

**Manual Test Script:**

```bash
# Test 1: Basic streaming
specks plan "create a python command-line calculator"
# VERIFY:
# - Content appears line-by-line as agent generates it
# - Text flows naturally (not token-per-line)
# - Spinner stays FIXED at the bottom with elapsed time
# - Spinner animates (braille pattern cycles)
# - On completion, spinner transforms to green checkmark

# Test 2: Non-TTY mode
specks plan "test idea" | cat
# VERIFY:
# - Output appears without ANSI codes
# - No spinner (just message at start)
# - Content prints line-by-line

# Test 3: Long running agent
specks plan "design a complex microservices architecture with multiple databases"
# VERIFY:
# - Content scrolls naturally as it exceeds screen height
# - Spinner NEVER scrolls off screen
# - Can scroll up to see previous content after completion

# Test 4: Ctrl+C handling
specks plan "test idea"
# Press Ctrl+C during agent execution
# VERIFY:
# - Spinner clears cleanly
# - Terminal state is restored
# - Exit message appears

# Test 5: Terminal resize (optional)
# Resize terminal during streaming
# VERIFY:
# - No crash
# - Output continues (wrapping may be suboptimal until next line)
```

**Checkpoint:**

- [ ] `cargo build` succeeds
- [ ] `cargo nextest run` passes
- [ ] `indicatif` is explicit dependency in Cargo.toml
- [ ] **Manual test: Content streams in real-time** (not all-at-once dump)
- [ ] **Manual test: Spinner stays fixed at bottom** (never scrolls away)
- [ ] **Manual test: Spinner animates continuously** (braille pattern cycles)
- [ ] **Manual test: Text flows naturally** (not token-per-line)
- [ ] **Manual test: Ctrl+C exits cleanly** (no terminal corruption)
- [ ] No cursor save/restore calls in streaming.rs
- [ ] No Clear(ClearType::*) calls in streaming.rs

**Acceptance Criteria (from user):**

> "I want to see the streaming content coming back from the agents we're running, and I want a pulsing/spinning feedback widget in a fixed location under this content as it streams in. I WILL NOT SETTLE for anything less."

This step is complete when the terminal experience matches Claude Code's behavior:
1. Content streams token-by-token, rendered as flowing text
2. Spinner/timer anchored at bottom, never scrolls
3. Spinner pulses/animates while content streams above
4. Clean transitions on success/error/cancel

**Rollback:**

- Revert streaming.rs to pre-indicatif version
- Remove indicatif explicit dependency

**Commit after all checkpoints pass.**
