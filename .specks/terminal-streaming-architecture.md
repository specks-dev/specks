# Terminal Streaming Architecture for Specks CLI

## Executive Summary

This document provides a complete architectural design for implementing Claude Code-style terminal streaming in the Specks CLI. The design addresses the current failures (screen clearing, disappearing spinners, raw JSON display, no anchored layout) and provides a robust solution using `indicatif::MultiProgress` as the foundation.

## Problem Analysis

### Current Implementation Failures

The existing `streaming.rs` implementation suffers from several fundamental issues:

1. **Screen clearing on start**: Using `terminal::Clear(ClearType::FromCursorDown)` clears content that should persist
2. **Spinner disappearing**: Save/restore cursor positions (`cursor::SavePosition`/`cursor::RestorePosition`) do not work correctly with scrolling content
3. **Raw JSON output**: The streaming display shows raw stream-json chunks instead of formatted content
4. **No anchored layout**: Attempting to manually manage cursor positions fails when content exceeds one screen

### Root Cause

The current approach tries to manually manage terminal state using cursor positioning, which fundamentally does not work when content scrolls. Terminal scrolling invalidates saved cursor positions because the coordinate system shifts as lines scroll off the top.

## Architectural Approaches Evaluated

### Approach 1: Raw ANSI Scrolling Regions (DECSTBM)

**What it is**: VT100 escape sequence `ESC [ <top> ; <bottom> r` sets a scrolling region where content scrolls while areas outside remain fixed.

**Pros**:
- Native terminal support, very efficient
- Used by vim, tmux for fixed status bars
- No redrawing overhead

**Cons**:
- Not supported by crossterm (PR pending)
- Inconsistent support across terminals
- Complex cursor management required
- Must reset region before any output outside the region

**Verdict**: Too fragile for cross-platform CLI use. Would require raw escape sequences and extensive terminal compatibility testing.

### Approach 2: Alternate Screen Buffer

**What it is**: Switch to a completely separate screen buffer (like vim does), draw a TUI layout, switch back when done.

**Pros**:
- Clean separation from shell history
- Full control over layout
- Well-supported by crossterm (`EnterAlternateScreen`/`LeaveAlternateScreen`)

**Cons**:
- Output disappears when leaving alternate screen
- Users cannot scroll back to see agent output
- Disruptive UX (screen "blinks")
- Not how Claude Code works

**Verdict**: Inappropriate. Users need to see and scroll back through output after the agent finishes.

### Approach 3: Line-by-Line Redraw (indicatif pattern)

**What it is**: Reserve lines at the bottom by printing them, then use ANSI codes to move cursor up, overwrite those lines for updates. Content prints normally above.

**Pros**:
- Battle-tested by indicatif (millions of users)
- Cross-platform (Windows, macOS, Linux)
- Integrates with normal terminal scrolling
- Output persists in scrollback
- Already a dependency in Specks

**Cons**:
- Slight visual flicker possible (minimized in indicatif 0.17+)
- Must coordinate all output through the MultiProgress system

**Verdict**: **RECOMMENDED**. This is how Claude Code, npm, cargo, and most modern CLIs achieve anchored progress bars.

### Approach 4: Full TUI Framework (Ratatui)

**What it is**: Use ratatui to render a complete terminal UI with layouts, widgets, and managed regions.

**Pros**:
- Full control over every pixel
- Rich widget library
- Proper resize handling

**Cons**:
- Massive architectural change
- Output doesn't integrate with shell (alternate screen)
- Overkill for streaming text with a spinner
- Additional dependency

**Verdict**: Overkill. Specks needs streaming output with progress, not a full TUI.

## Recommended Architecture

### Overview

Use `indicatif::MultiProgress` to manage a fixed spinner at the bottom while streaming content above through `MultiProgress::println()`.

```
+------------------------------------------------------------------+
|  Content streams here, scrolls naturally with terminal           |
|  Each line printed via MultiProgress::println()                  |
|                                                                  |
|  The clarifier is analyzing the requirements.                    |
|  Based on the context, here are the ambiguities identified:      |
|  1. Authentication scope is unclear...                           |
|  2. Database choice not specified...                             |
|                                                                  |
+------------------------------------------------------------------+
| ⠼ Clarifier analyzing... [12.3s]          <- FIXED AT BOTTOM    |
+------------------------------------------------------------------+
```

### Component Architecture

```
                    ┌─────────────────────────────────────────┐
                    │           StreamingDisplay              │
                    │  (public API for agent invocation)      │
                    └──────────────────┬──────────────────────┘
                                       │
                                       │ owns
                                       ▼
                    ┌─────────────────────────────────────────┐
                    │         indicatif::MultiProgress        │
                    │  - Manages terminal output coordination │
                    │  - Prevents interleaved output          │
                    └──────────────────┬──────────────────────┘
                                       │
              ┌────────────────────────┼────────────────────────┐
              │                        │                        │
              ▼                        ▼                        ▼
     ┌────────────────┐    ┌────────────────────┐    ┌────────────────┐
     │  ProgressBar   │    │   println() calls  │    │  ContentBuffer │
     │  (spinner)     │    │   (content lines)  │    │  (line assembly)│
     │  - Fixed style │    │   - Prints ABOVE   │    │  - Word wrap    │
     │  - Elapsed time│    │     the spinner    │    │  - Token concat │
     └────────────────┘    └────────────────────┘    └────────────────┘
```

### Data Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        AGENT INVOCATION FLOW                             │
└─────────────────────────────────────────────────────────────────────────┘

   invoke_agent_streaming()
           │
           ▼
   ┌───────────────┐     spawn      ┌─────────────────────────────────────┐
   │ claude CLI    │────────────────│  stdout pipe (stream-json NDJSON)   │
   │ subprocess    │                └─────────────────────────────────────┘
   └───────────────┘                                    │
                                                        │ BufReader::lines()
                                                        ▼
                                        ┌───────────────────────────────┐
                                        │  parse_stream_json_event()    │
                                        │  - Extract text deltas        │
                                        │  - Skip metadata events       │
                                        └───────────────────────────────┘
                                                        │
                                                        │ Some(text)
                                                        ▼
                                        ┌───────────────────────────────┐
                                        │      ContentBuffer            │
                                        │  - Accumulates tokens         │
                                        │  - Detects line breaks        │
                                        │  - Handles word wrap          │
                                        └───────────────────────────────┘
                                                        │
                                                        │ complete lines
                                                        ▼
                                        ┌───────────────────────────────┐
                                        │   MultiProgress::println()    │
                                        │   Prints ABOVE the spinner    │
                                        └───────────────────────────────┘

   Meanwhile (100ms tick):
   ┌───────────────────────────────────────────────────────────────────────┐
   │  ProgressBar::tick() → updates spinner frame and elapsed time         │
   │  Remains fixed at bottom, content scrolls above                       │
   └───────────────────────────────────────────────────────────────────────┘
```

## Detailed Design

### 1. StreamingDisplay Struct (Revised)

```rust
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::io::{IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Braille spinner animation frames
const SPINNER_FRAMES: &str = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏";

/// Streaming display with anchored spinner at bottom
pub struct StreamingDisplay {
    /// The message shown in the spinner
    message: String,
    /// When the display started
    start_time: Instant,
    /// Multi-progress coordinator (None if non-TTY)
    multi_progress: Option<MultiProgress>,
    /// The spinner progress bar (None if non-TTY)
    spinner: Option<ProgressBar>,
    /// Buffer for accumulating partial lines
    line_buffer: String,
    /// Whether stdout is a TTY
    is_tty: bool,
    /// Cancellation flag
    cancelled: Arc<AtomicBool>,
    /// Terminal width for word wrapping
    term_width: u16,
}

impl StreamingDisplay {
    pub fn new(message: &str) -> Self {
        let is_tty = std::io::stdout().is_terminal();
        let term_width = crossterm::terminal::size().map(|(w, _)| w).unwrap_or(80);

        Self {
            message: message.to_string(),
            start_time: Instant::now(),
            multi_progress: None,
            spinner: None,
            line_buffer: String::new(),
            is_tty,
            cancelled: Arc::new(AtomicBool::new(false)),
            term_width,
        }
    }

    /// Initialize and show the display
    pub fn start(&mut self) {
        if !self.is_tty {
            // Non-TTY: print message and return
            println!("{}", self.message);
            return;
        }

        // Create MultiProgress for coordinated output
        let mp = MultiProgress::new();

        // Create the spinner with our custom style
        let spinner = mp.add(ProgressBar::new_spinner());
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg} [{elapsed:.dim}]")
                .expect("valid template")
                .tick_chars(SPINNER_FRAMES),
        );
        spinner.set_message(self.message.clone());
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));

        self.multi_progress = Some(mp);
        self.spinner = Some(spinner);
    }

    /// Write streaming content (may be partial tokens)
    pub fn write_content(&mut self, content: &str) {
        if content.is_empty() {
            return;
        }

        // Accumulate content into line buffer
        self.line_buffer.push_str(content);

        // Extract and print complete lines
        while let Some(newline_pos) = self.line_buffer.find('\n') {
            let line = self.line_buffer[..newline_pos].to_string();
            self.line_buffer = self.line_buffer[newline_pos + 1..].to_string();
            self.print_line(&line);
        }
    }

    /// Print a complete line above the spinner
    fn print_line(&self, line: &str) {
        if let Some(ref mp) = self.multi_progress {
            // Word wrap long lines
            for wrapped_line in self.wrap_line(line) {
                let _ = mp.println(&wrapped_line);
            }
        } else {
            // Non-TTY: direct print
            println!("{}", line);
        }
    }

    /// Word wrap a line to terminal width
    fn wrap_line(&self, line: &str) -> Vec<String> {
        let width = self.term_width as usize;
        if width == 0 || line.len() <= width {
            return vec![line.to_string()];
        }

        let mut result = Vec::new();
        let mut current = String::new();

        for word in line.split_whitespace() {
            if current.is_empty() {
                current = word.to_string();
            } else if current.len() + 1 + word.len() <= width {
                current.push(' ');
                current.push_str(word);
            } else {
                result.push(current);
                current = word.to_string();
            }
        }

        if !current.is_empty() {
            result.push(current);
        }

        if result.is_empty() {
            result.push(String::new());
        }

        result
    }

    /// Flush any remaining buffered content
    fn flush_buffer(&mut self) {
        if !self.line_buffer.is_empty() {
            let remaining = std::mem::take(&mut self.line_buffer);
            self.print_line(&remaining);
        }
    }

    /// Update spinner (called periodically if manual tick needed)
    pub fn tick(&self) {
        if let Some(ref spinner) = self.spinner {
            spinner.tick();
        }
    }

    /// Finish with success status
    pub fn finish_success(&mut self) {
        self.flush_buffer();
        let elapsed = self.format_elapsed();

        if let Some(ref spinner) = self.spinner {
            spinner.finish_and_clear();
        }
        if let Some(ref mp) = self.multi_progress {
            let _ = mp.clear();
        }

        // Print final success line
        if self.is_tty {
            use owo_colors::OwoColorize;
            println!("{} {} [{}]", "✓".green(), self.message.green(), elapsed);
        } else {
            println!("✓ {} [{}]", self.message, elapsed);
        }
    }

    /// Finish with error status
    pub fn finish_error(&mut self, error_msg: &str) {
        self.flush_buffer();
        let elapsed = self.format_elapsed();

        if let Some(ref spinner) = self.spinner {
            spinner.finish_and_clear();
        }
        if let Some(ref mp) = self.multi_progress {
            let _ = mp.clear();
        }

        // Print final error line
        if self.is_tty {
            use owo_colors::OwoColorize;
            println!(
                "{} {} [{}]: {}",
                "✗".red(),
                self.message.red(),
                elapsed,
                error_msg.red()
            );
        } else {
            println!("✗ {} [{}]: {}", self.message, elapsed, error_msg);
        }
    }

    /// Check if cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Get cancellation flag for Ctrl+C handler
    pub fn cancellation_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancelled)
    }

    /// Format elapsed time
    fn format_elapsed(&self) -> String {
        let secs = self.start_time.elapsed().as_secs_f64();
        format!("{:.1}s", secs)
    }
}

impl Drop for StreamingDisplay {
    fn drop(&mut self) {
        // Ensure spinner is cleared if we're dropped unexpectedly
        if let Some(ref spinner) = self.spinner {
            spinner.finish_and_clear();
        }
    }
}
```

### 2. JSON Stream Parser (Improved)

The current `parse_stream_json_event` function is mostly correct but needs to handle more event types gracefully:

```rust
/// Parse a stream-json event from Claude CLI and extract displayable content.
///
/// Stream-json format is NDJSON with events like:
/// - content_block_delta with text: displayable content
/// - tool_use events: could format nicely (future enhancement)
/// - result: skip (we already displayed content)
fn parse_stream_json_event(line: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(line).ok()?;

    // Check for stream_event type
    if json.get("type").and_then(|t| t.as_str()) != Some("stream_event") {
        return None;
    }

    let event = json.get("event")?;
    let event_type = event.get("type").and_then(|t| t.as_str())?;

    match event_type {
        "content_block_delta" => {
            // Extract text from the delta
            event
                .get("delta")
                .and_then(|d| d.get("text"))
                .and_then(|t| t.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
        }
        // Future: could handle tool_use events to show "Using tool: X..."
        _ => None,
    }
}
```

### 3. Agent Runner Integration

The `invoke_agent_streaming` method in `agent.rs` needs minimal changes. The key fix is using the new `StreamingDisplay` that properly uses `MultiProgress`:

```rust
pub fn invoke_agent_streaming(
    &self,
    config: &AgentConfig,
    prompt: &str,
    display: &mut StreamingDisplay,
) -> Result<AgentResult, SpecksError> {
    use std::io::BufRead;
    use std::sync::mpsc;
    use std::thread;

    // ... existing setup code ...

    // Spawn reader thread (unchanged)
    let (tx, rx) = mpsc::channel::<Result<String, String>>();
    thread::spawn(move || {
        let reader = std::io::BufReader::new(stdout);
        for line_result in reader.lines() {
            match line_result {
                Ok(line) => {
                    if let Some(content) = parse_stream_json_event(&line) {
                        if tx.send(Ok(content)).is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(e.to_string()));
                    break;
                }
            }
        }
    });

    let mut all_output = String::new();
    let timeout = Duration::from_secs(config.timeout_secs);
    let start = std::time::Instant::now();

    // Start the display (creates MultiProgress + spinner)
    display.start();

    // Main loop - spinner auto-ticks via enable_steady_tick()
    loop {
        if start.elapsed() >= timeout {
            let _ = child.kill();
            display.finish_error("timeout");
            return Err(SpecksError::AgentTimeout { secs: config.timeout_secs });
        }

        if display.is_cancelled() {
            let _ = child.kill();
            display.finish_error("cancelled");
            return Err(SpecksError::UserAborted);
        }

        // Non-blocking receive with 100ms timeout
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(content)) => {
                all_output.push_str(&content);
                display.write_content(&content);
            }
            Ok(Err(e)) => {
                display.finish_error(&format!("read error: {}", e));
                return Err(SpecksError::AgentInvocationFailed {
                    reason: format!("Failed to read agent output: {}", e),
                });
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // No content yet - spinner is auto-ticking
                continue;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // Reader finished
                break;
            }
        }
    }

    // ... existing completion code ...
}
```

### 4. Handling Edge Cases

#### Terminal Resize

```rust
impl StreamingDisplay {
    /// Update terminal width (call on SIGWINCH if needed)
    pub fn handle_resize(&mut self) {
        if let Ok((width, _)) = crossterm::terminal::size() {
            self.term_width = width;
        }
    }
}
```

For most use cases, resize handling during streaming is not critical. The content will still display, just potentially with suboptimal wrapping until the next line.

#### Non-TTY (Pipes, CI)

The design already handles non-TTY by:
1. Skipping MultiProgress creation entirely
2. Using direct `println!` for output
3. Printing simple status messages without ANSI codes

#### Concurrent Agents (Future)

The architecture supports multiple concurrent agents by:
1. Creating multiple `ProgressBar` instances within the same `MultiProgress`
2. Each agent's spinner appears stacked at the bottom
3. All `println()` calls coordinate through the shared `MultiProgress`

```rust
// Future API sketch for concurrent agents:
pub struct MultiAgentDisplay {
    multi_progress: MultiProgress,
    agents: HashMap<String, ProgressBar>,
}

impl MultiAgentDisplay {
    pub fn add_agent(&mut self, name: &str, message: &str) -> AgentHandle {
        let pb = self.multi_progress.add(ProgressBar::new_spinner());
        pb.set_style(/* ... */);
        pb.set_message(message);
        pb.enable_steady_tick(Duration::from_millis(100));
        self.agents.insert(name.to_string(), pb);
        AgentHandle { name: name.to_string() }
    }

    pub fn write_content(&self, handle: &AgentHandle, content: &str) {
        // Content goes above all spinners
        let _ = self.multi_progress.println(content);
    }
}
```

## ANSI Sequences Reference

For debugging or understanding, here are the key sequences `indicatif` uses internally:

| Sequence | Purpose |
|----------|---------|
| `\x1b[?25l` | Hide cursor |
| `\x1b[?25h` | Show cursor |
| `\x1b[{n}A` | Move cursor up n lines |
| `\x1b[{n}B` | Move cursor down n lines |
| `\x1b[2K` | Clear entire line |
| `\x1b[K` | Clear from cursor to end of line |

`indicatif` tracks how many lines it has reserved at the bottom and uses cursor movement to update them without disrupting content above.

## Implementation Phases

### Phase 1: Core Streaming Display Rewrite
**Effort: 2-3 hours**

Tasks:
- [ ] Replace current `StreamingDisplay` with `indicatif::MultiProgress` based implementation
- [ ] Remove cursor save/restore logic
- [ ] Implement `write_content` using `MultiProgress::println()`
- [ ] Implement line buffering for token-by-token streaming
- [ ] Test basic functionality manually

### Phase 2: Integration and Polish
**Effort: 1-2 hours**

Tasks:
- [ ] Update `invoke_agent_streaming` to use new display
- [ ] Verify spinner remains visible during streaming
- [ ] Test non-TTY fallback (pipe output)
- [ ] Add word wrapping for long lines
- [ ] Verify Ctrl+C handling

### Phase 3: Testing and Documentation
**Effort: 1 hour**

Tasks:
- [ ] Update unit tests for new implementation
- [ ] Add integration test with mock Claude CLI
- [ ] Document public API
- [ ] Update any relevant speck documentation

## Appendix: Why Not [X]?

### Why not just fix the cursor positioning?

Cursor positioning fundamentally breaks when content scrolls. The saved position becomes invalid as soon as any content scrolls off the top of the terminal. This is not fixable without using scrolling regions (DECSTBM), which are not portable.

### Why not use ratatui?

Ratatui is designed for full TUI applications that take over the screen. Specks needs output that integrates with the shell's normal scrollback. Ratatui would prevent users from scrolling back through agent output after completion.

### Why indicatif specifically?

1. Already a dependency in Specks
2. Battle-tested with millions of downloads
3. Cross-platform (Windows, macOS, Linux)
4. Specifically designed for this exact use case (progress + output coordination)
5. Active maintenance and community

### Why not raw DECSTBM?

While DECSTBM would be more efficient, it:
1. Is not exposed by crossterm
2. Has inconsistent terminal support
3. Requires careful state management
4. Would need extensive compatibility testing

The `indicatif` approach achieves the same visual result with better portability.

## References

- [indicatif documentation](https://docs.rs/indicatif)
- [indicatif MultiProgress](https://docs.rs/indicatif/latest/indicatif/struct.MultiProgress.html)
- [crossterm terminal module](https://docs.rs/crossterm/latest/crossterm/terminal/index.html)
- [DECSTBM specification](https://vt100.net/docs/vt510-rm/DECSTBM.html)
- [status-line crate](https://github.com/pkolaczk/status-line) - alternative approach using background threads
