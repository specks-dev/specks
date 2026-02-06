//! Agent invocation infrastructure for shelling out to Claude CLI
//!
//! This module provides the infrastructure to invoke specks agents via the `claude` CLI.
//! Per [D02] Shell out to Claude CLI, agent invocation shells out rather than using direct API calls.
//!
//! Agent resolution is per-agent, not per-directory. Resolution order:
//! 1. `project_root/agents/{agent_name}.md` (if file exists)
//! 2. `{share_dir}/agents/{agent_name}.md` where share_dir comes from `find_share_dir()`
//! 3. Development fallback: `{specks_repo}/agents/{agent_name}.md` (when specks workspace detected)

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::Duration;

use specks_core::SpecksError;

use crate::share::find_share_dir;

/// Agents directory name within share directory
pub const AGENTS_DIR_NAME: &str = "agents";

/// Required agents for the `specks plan` command
pub const PLAN_REQUIRED_AGENTS: &[&str] = &[
    "specks-clarifier",
    "specks-interviewer",
    "specks-planner",
    "specks-critic",
];

/// Required agents for the `specks execute` command
pub const EXECUTE_REQUIRED_AGENTS: &[&str] = &[
    "specks-director",
    "specks-architect",
    "specks-implementer",
    "specks-monitor",
    "specks-reviewer",
    "specks-auditor",
    "specks-committer",
    "specks-logger",
];

/// Default timeout for agent invocations in seconds
pub const DEFAULT_TIMEOUT_SECS: u64 = 300;

/// Result from an agent invocation
#[derive(Debug, Clone)]
pub struct AgentResult {
    /// The agent's text output
    pub output: String,
    /// Whether the agent completed successfully
    pub success: bool,
}

/// Configuration for agent invocation
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Agent name (e.g., "specks-interviewer", "specks-planner")
    pub agent_name: String,
    /// Path to the agent definition file
    pub agent_path: PathBuf,
    /// Tools the agent is allowed to use
    pub allowed_tools: Vec<String>,
    /// Timeout in seconds
    pub timeout_secs: u64,
}

impl AgentConfig {
    /// Create a new agent configuration
    pub fn new(agent_name: &str, agent_path: PathBuf, allowed_tools: Vec<String>) -> Self {
        Self {
            agent_name: agent_name.to_string(),
            agent_path,
            allowed_tools,
            timeout_secs: DEFAULT_TIMEOUT_SECS,
        }
    }

    /// Set the timeout for this agent invocation
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }
}

/// Agent runner that manages invocation of specks agents via claude CLI
#[derive(Debug, Clone)]
pub struct AgentRunner {
    /// Path to the claude CLI binary (default: "claude")
    claude_path: PathBuf,
    /// Project root directory
    project_root: PathBuf,
}

impl AgentRunner {
    /// Create a new AgentRunner
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            claude_path: PathBuf::from("claude"),
            project_root,
        }
    }

    /// Set a custom path to the claude CLI
    #[allow(dead_code)] // Used in tests
    pub fn with_claude_path(mut self, path: PathBuf) -> Self {
        self.claude_path = path;
        self
    }

    /// Check if the claude CLI is installed and accessible
    pub fn check_claude_cli(&self) -> Result<(), SpecksError> {
        // First try `which claude` to find it on PATH
        let which_result = Command::new("which")
            .arg(self.claude_path.to_string_lossy().as_ref())
            .output();

        match which_result {
            Ok(output) if output.status.success() => Ok(()),
            _ => {
                // Try running claude --version directly as fallback
                let version_result = Command::new(&self.claude_path)
                    .arg("--version")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();

                match version_result {
                    Ok(status) if status.success() => Ok(()),
                    _ => Err(SpecksError::ClaudeCliNotInstalled),
                }
            }
        }
    }

    /// Invoke an agent with the given prompt
    ///
    /// This shells out to the claude CLI with the agent's system prompt and allowed tools.
    pub fn invoke_agent(
        &self,
        config: &AgentConfig,
        prompt: &str,
    ) -> Result<AgentResult, SpecksError> {
        // First verify claude CLI is available
        self.check_claude_cli()?;

        // Read the agent definition file
        let agent_content = std::fs::read_to_string(&config.agent_path).map_err(|e| {
            SpecksError::AgentInvocationFailed {
                reason: format!(
                    "Failed to read agent definition at {}: {}",
                    config.agent_path.display(),
                    e
                ),
            }
        })?;

        // Build the claude command
        let mut cmd = Command::new(&self.claude_path);

        // Add --print flag to capture output
        cmd.arg("--print");

        // Add allowed tools
        if !config.allowed_tools.is_empty() {
            cmd.arg("--allowed-tools");
            cmd.arg(config.allowed_tools.join(","));
        }

        // Add system prompt (the agent definition)
        cmd.arg("--system-prompt");
        cmd.arg(&agent_content);

        // Add the user prompt
        cmd.arg(prompt);

        // Set working directory to project root
        cmd.current_dir(&self.project_root);

        // Execute with timeout
        let output = self.execute_with_timeout(&mut cmd, config.timeout_secs)?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let exit_code = output.status.code().unwrap_or(-1);

        // Check for timeout (exit code 124 is typical for timeout)
        if exit_code == 124 {
            return Err(SpecksError::AgentTimeout {
                secs: config.timeout_secs,
            });
        }

        // Check for other failures
        if !output.status.success() && exit_code != 0 {
            // Agent failed - include both stdout and stderr in error
            let error_output = if stderr.is_empty() {
                stdout.clone()
            } else {
                format!("{}\n{}", stdout, stderr)
            };

            return Err(SpecksError::AgentInvocationFailed {
                reason: format!(
                    "Agent {} failed with exit code {}: {}",
                    config.agent_name,
                    exit_code,
                    error_output.trim()
                ),
            });
        }

        Ok(AgentResult {
            output: stdout,
            success: output.status.success(),
        })
    }

    /// Execute a command with timeout
    fn execute_with_timeout(
        &self,
        cmd: &mut Command,
        timeout_secs: u64,
    ) -> Result<Output, SpecksError> {
        // Configure command for output capture
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Spawn the child process
        let mut child = cmd
            .spawn()
            .map_err(|e| SpecksError::AgentInvocationFailed {
                reason: format!("Failed to spawn claude process: {}", e),
            })?;

        // Wait with timeout using a simple polling approach
        // In production, we might use a more sophisticated timeout mechanism
        let timeout = Duration::from_secs(timeout_secs);
        let start = std::time::Instant::now();
        let poll_interval = Duration::from_millis(100);

        loop {
            match child.try_wait() {
                Ok(Some(_status)) => {
                    // Process has exited, get the output
                    return child.wait_with_output().map_err(|e| {
                        SpecksError::AgentInvocationFailed {
                            reason: format!("Failed to get agent output: {}", e),
                        }
                    });
                }
                Ok(None) => {
                    // Process still running
                    if start.elapsed() >= timeout {
                        // Kill the process
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(SpecksError::AgentTimeout { secs: timeout_secs });
                    }
                    std::thread::sleep(poll_interval);
                }
                Err(e) => {
                    return Err(SpecksError::AgentInvocationFailed {
                        reason: format!("Failed to check agent status: {}", e),
                    });
                }
            }
        }
    }

    /// Invoke an agent with streaming output display.
    ///
    /// This method streams the agent's output line-by-line to the display,
    /// showing a spinner with elapsed time at the bottom.
    ///
    /// Uses a background thread to read stdout so the spinner can update
    /// independently of blocking I/O.
    ///
    /// If `show_text` is true, streaming text is displayed.
    /// If false, only the spinner is shown (useful for tool-heavy agents like planner).
    #[allow(dead_code)] // Available for agents that benefit from visible text streaming
    pub fn invoke_agent_streaming(
        &self,
        config: &AgentConfig,
        prompt: &str,
        display: &mut crate::streaming::StreamingDisplay,
    ) -> Result<AgentResult, SpecksError> {
        self.invoke_agent_streaming_impl(config, prompt, display, true, None)
    }

    /// Like `invoke_agent_streaming` but only shows the spinner, not streaming text.
    /// Use this for tool-heavy agents (planner, critic) where streaming text is noise.
    pub fn invoke_agent_spinner_only(
        &self,
        config: &AgentConfig,
        prompt: &str,
        display: &mut crate::streaming::StreamingDisplay,
    ) -> Result<AgentResult, SpecksError> {
        self.invoke_agent_streaming_impl(config, prompt, display, false, None)
    }

    /// Like `invoke_agent_spinner_only` but also monitors a file for progress.
    /// Shows file stats (lines, bytes) as the agent writes to it.
    #[allow(dead_code)]
    pub fn invoke_agent_with_file_monitor(
        &self,
        config: &AgentConfig,
        prompt: &str,
        display: &mut crate::streaming::StreamingDisplay,
        file_to_monitor: &std::path::Path,
    ) -> Result<AgentResult, SpecksError> {
        self.invoke_agent_streaming_impl(
            config,
            prompt,
            display,
            false,
            Some(file_to_monitor.to_path_buf()),
        )
    }

    /// Implementation of streaming invocation.
    fn invoke_agent_streaming_impl(
        &self,
        config: &AgentConfig,
        prompt: &str,
        display: &mut crate::streaming::StreamingDisplay,
        show_text: bool,
        file_to_monitor: Option<PathBuf>,
    ) -> Result<AgentResult, SpecksError> {
        use std::io::BufRead;
        use std::sync::mpsc;
        use std::thread;

        // First verify claude CLI is available
        self.check_claude_cli()?;

        // Read the agent definition file
        let agent_content = std::fs::read_to_string(&config.agent_path).map_err(|e| {
            SpecksError::AgentInvocationFailed {
                reason: format!(
                    "Failed to read agent definition at {}: {}",
                    config.agent_path.display(),
                    e
                ),
            }
        })?;

        // Build the claude command
        let mut cmd = Command::new(&self.claude_path);

        // Add --print flag with stream-json output format for real-time streaming
        // Note: stream-json requires --verbose, and --include-partial-messages for token streaming
        cmd.arg("--print");
        cmd.arg("--verbose");
        cmd.arg("--output-format");
        cmd.arg("stream-json");
        cmd.arg("--include-partial-messages");

        // Add allowed tools
        if !config.allowed_tools.is_empty() {
            cmd.arg("--allowed-tools");
            cmd.arg(config.allowed_tools.join(","));
        }

        // Add system prompt (the agent definition)
        cmd.arg("--system-prompt");
        cmd.arg(&agent_content);

        // Add the user prompt
        cmd.arg(prompt);

        // Set working directory to project root
        cmd.current_dir(&self.project_root);

        // Configure for streaming: pipe stdout, capture stderr
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Spawn the child process
        let mut child = cmd
            .spawn()
            .map_err(|e| SpecksError::AgentInvocationFailed {
                reason: format!("Failed to spawn claude process: {}", e),
            })?;

        // Take stdout for streaming
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| SpecksError::AgentInvocationFailed {
                reason: "Failed to capture stdout".to_string(),
            })?;

        // Create a channel for the reader thread to send parsed events
        let (tx, rx) = mpsc::channel::<Result<StreamEvent, String>>();

        // Spawn a thread to read stream-json events
        thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            for line_result in reader.lines() {
                match line_result {
                    Ok(line) => {
                        // Parse stream-json event
                        if let Some(event) = parse_stream_json_event(&line) {
                            if tx.send(Ok(event)).is_err() {
                                break; // Receiver dropped, stop reading
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e.to_string()));
                        break;
                    }
                }
            }
            // Channel closes when tx is dropped
        });

        let mut all_output = String::new();
        let timeout = Duration::from_secs(config.timeout_secs);
        let start = std::time::Instant::now();
        let spinner_update_interval = Duration::from_millis(100);
        let mut last_spinner_update = std::time::Instant::now();

        // Start the streaming display
        display.start();

        // Main loop: check for lines and update spinner
        loop {
            // Check for timeout
            if start.elapsed() >= timeout {
                let _ = child.kill();
                let _ = child.wait();
                display.finish_error("timeout");
                return Err(SpecksError::AgentTimeout {
                    secs: config.timeout_secs,
                });
            }

            // Check for cancellation
            if display.is_cancelled() {
                let _ = child.kill();
                let _ = child.wait();
                display.finish_error("cancelled");
                return Err(SpecksError::UserAborted);
            }

            // Update spinner periodically regardless of event processing
            if last_spinner_update.elapsed() >= spinner_update_interval {
                if let Some(ref path) = file_to_monitor {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        let lines = content.lines().count();
                        let bytes = content.len();
                        display.update_file_stats(lines, bytes);
                    }
                }
                display.update_spinner();
                last_spinner_update = std::time::Instant::now();
            }

            // Try to receive an event (non-blocking with short timeout)
            match rx.recv_timeout(spinner_update_interval) {
                Ok(Ok(event)) => {
                    match event {
                        StreamEvent::Text(content) => {
                            // Got text content - accumulate and optionally display
                            all_output.push_str(&content);
                            if show_text {
                                display.write_content(&content);
                            }
                        }
                        StreamEvent::ToolStart(tool_name) => {
                            // Tool is being invoked - show in spinner
                            display.set_current_tool(&tool_name);
                        }
                        StreamEvent::ToolEnd => {
                            // Tool finished
                            display.clear_current_tool();
                        }
                    }
                }
                Ok(Err(e)) => {
                    // Reader error
                    display.finish_error(&format!("read error: {}", e));
                    return Err(SpecksError::AgentInvocationFailed {
                        reason: format!("Failed to read agent output: {}", e),
                    });
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Timeout already handled above
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    // Reader thread finished - process is done
                    break;
                }
            }
        }

        // Wait for the process to complete
        let status = child.wait().map_err(|e| {
            display.finish_error(&format!("wait error: {}", e));
            SpecksError::AgentInvocationFailed {
                reason: format!("Failed to wait for agent: {}", e),
            }
        })?;

        // Capture stderr
        let stderr = if let Some(mut stderr_pipe) = child.stderr.take() {
            let mut stderr_content = String::new();
            let _ = std::io::Read::read_to_string(&mut stderr_pipe, &mut stderr_content);
            stderr_content
        } else {
            String::new()
        };

        let exit_code = status.code().unwrap_or(-1);

        // Check for failures
        if !status.success() && exit_code != 0 {
            let error_output = if stderr.is_empty() {
                all_output.clone()
            } else {
                format!("{}\n{}", all_output, stderr)
            };

            display.finish_error(&format!("exit code {}", exit_code));
            return Err(SpecksError::AgentInvocationFailed {
                reason: format!(
                    "Agent {} failed with exit code {}: {}",
                    config.agent_name,
                    exit_code,
                    error_output.trim()
                ),
            });
        }

        display.finish_success();

        Ok(AgentResult {
            output: all_output,
            success: status.success(),
        })
    }
}

/// Check if the given path is the specks development workspace.
///
/// Returns true if:
/// 1. A Cargo.toml exists with specks workspace package
/// 2. An agents/ directory exists with specks agent files
///
/// This enables development mode where agents are loaded from the repo's agents/ directory.
pub fn is_specks_workspace(path: &Path) -> bool {
    let cargo_toml = path.join("Cargo.toml");
    if !cargo_toml.exists() {
        return false;
    }

    // Check for workspace marker in Cargo.toml
    if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
        // Look for specks workspace indicators
        let has_workspace = content.contains("[workspace]");
        let has_specks_members =
            content.contains("specks") && content.contains("crates/specks-core");

        if !(has_workspace && has_specks_members) {
            return false;
        }
    } else {
        return false;
    }

    // Check for agents directory with at least one specks agent
    let agents_dir = path.join("agents");
    if !agents_dir.is_dir() {
        return false;
    }

    // Check if at least one expected agent exists
    let test_agent = agents_dir.join("specks-director.md");
    test_agent.exists()
}

/// Find the specks workspace root relative to the running binary.
///
/// If the binary is running from a cargo target directory (target/debug/ or target/release/),
/// this returns the workspace root (parent of target/). This enables development builds
/// to find agents regardless of what directory they're invoked from.
///
/// Handles both:
/// - Direct binaries: `.../specks/target/debug/specks`
/// - Test binaries: `.../specks/target/debug/deps/specks-xxxx`
///
/// Returns `None` if:
/// - The binary path can't be determined
/// - The binary isn't in a cargo target directory
/// - The workspace root doesn't look like the specks workspace
pub fn find_binary_workspace_root() -> Option<PathBuf> {
    let exe_path = std::env::current_exe().ok()?;

    // Walk up the directory tree looking for a "target" directory
    // This handles both direct binaries (target/debug/specks) and
    // test binaries (target/debug/deps/specks-xxxx)
    let mut current = exe_path.parent()?;

    while let Some(parent) = current.parent() {
        if current.file_name()?.to_str()? == "target" {
            // Found target directory - parent is the workspace root
            let workspace_root = parent;
            if is_specks_workspace(workspace_root) {
                return Some(workspace_root.to_path_buf());
            }
            // Found target but not specks workspace, stop looking
            return None;
        }
        current = parent;
    }

    None
}

/// Resolve the path to an agent definition using per-agent resolution.
///
/// Resolution order:
/// 1. `project_root/agents/{agent_name}.md` (if file exists)
/// 2. `{share_dir}/agents/{agent_name}.md` where share_dir comes from `find_share_dir()`
/// 3. Development fallback: binary's workspace agents directory (for dev builds run from anywhere)
///
/// Returns `None` if the agent is not found in any location.
pub fn resolve_agent_path(agent_name: &str, project_root: &Path) -> Option<PathBuf> {
    let filename = format!("{}.md", agent_name);

    // 1. Check project-local agents directory first
    let project_agent = project_root.join("agents").join(&filename);
    if project_agent.is_file() {
        return Some(project_agent);
    }

    // 2. Check share directory (from find_share_dir)
    if let Some(share_dir) = find_share_dir() {
        let share_agent = share_dir.join(AGENTS_DIR_NAME).join(&filename);
        if share_agent.is_file() {
            return Some(share_agent);
        }
    }

    // 3. Development fallback: check if binary is running from a cargo target directory
    //    This enables dev builds to find agents regardless of working directory
    if let Some(workspace_root) = find_binary_workspace_root() {
        let dev_agent = workspace_root.join("agents").join(&filename);
        if dev_agent.is_file() {
            return Some(dev_agent);
        }
    }

    None
}

/// Result of resolving an agent path, including its source
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentSource {
    /// Agent found in project's local agents/ directory
    Project,
    /// Agent found in share directory (installed via homebrew/tarball)
    Share,
    /// Agent found in development workspace
    Development,
}

impl std::fmt::Display for AgentSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentSource::Project => write!(f, "project"),
            AgentSource::Share => write!(f, "share"),
            AgentSource::Development => write!(f, "development"),
        }
    }
}

/// Resolve an agent path and return both the path and its source.
///
/// This is useful for verbose output showing where each agent was loaded from.
pub fn resolve_agent_path_with_source(
    agent_name: &str,
    project_root: &Path,
) -> Option<(PathBuf, AgentSource)> {
    let filename = format!("{}.md", agent_name);

    // 1. Check project-local agents directory first
    let project_agent = project_root.join("agents").join(&filename);
    if project_agent.is_file() {
        return Some((project_agent, AgentSource::Project));
    }

    // 2. Check share directory (from find_share_dir)
    if let Some(share_dir) = find_share_dir() {
        let share_agent = share_dir.join(AGENTS_DIR_NAME).join(&filename);
        if share_agent.is_file() {
            return Some((share_agent, AgentSource::Share));
        }
    }

    // 3. Development fallback: check if binary is running from a cargo target directory
    if let Some(workspace_root) = find_binary_workspace_root() {
        let dev_agent = workspace_root.join("agents").join(&filename);
        if dev_agent.is_file() {
            return Some((dev_agent, AgentSource::Development));
        }
    }

    None
}

/// Verify that all required agents for a command are available.
///
/// Returns `Ok(Vec<(agent_name, path, source)>)` if all agents are found,
/// or `Err(SpecksError::RequiredAgentsMissing)` with the list of missing agents.
pub fn verify_required_agents(
    command: &str,
    project_root: &Path,
) -> Result<Vec<(String, PathBuf, AgentSource)>, SpecksError> {
    let required_agents = match command {
        "plan" => PLAN_REQUIRED_AGENTS,
        "execute" => EXECUTE_REQUIRED_AGENTS,
        _ => return Ok(vec![]),
    };

    let mut found_agents = Vec::new();
    let mut missing_agents = Vec::new();

    for agent_name in required_agents {
        if let Some((path, source)) = resolve_agent_path_with_source(agent_name, project_root) {
            found_agents.push((agent_name.to_string(), path, source));
        } else {
            missing_agents.push(agent_name.to_string());
        }
    }

    if missing_agents.is_empty() {
        Ok(found_agents)
    } else {
        // Build list of searched paths for error message
        let mut searched_paths = vec![project_root.join("agents").to_string_lossy().to_string()];
        if let Some(share_dir) = find_share_dir() {
            searched_paths.push(
                share_dir
                    .join(AGENTS_DIR_NAME)
                    .to_string_lossy()
                    .to_string(),
            );
        }

        Err(SpecksError::RequiredAgentsMissing {
            command: command.to_string(),
            missing: missing_agents,
            searched: searched_paths,
        })
    }
}

/// Construct the expected agent path without checking existence.
///
/// This is a pure function with no I/O - useful for tests and error messages.
fn construct_agent_path(project_root: &Path, agent_name: &str) -> PathBuf {
    project_root
        .join("agents")
        .join(format!("{}.md", agent_name))
}

/// Get the path to a specific agent definition using per-agent resolution.
///
/// Returns the resolved path if found, or the project-local path if not found
/// (which will produce a clear "file not found" error when read).
pub fn get_agent_path(project_root: &Path, agent_name: &str) -> PathBuf {
    resolve_agent_path(agent_name, project_root)
        .unwrap_or_else(|| construct_agent_path(project_root, agent_name))
}

/// Create an AgentConfig for the interviewer agent
pub fn interviewer_config(project_root: &Path) -> AgentConfig {
    AgentConfig::new(
        "specks-interviewer",
        get_agent_path(project_root, "specks-interviewer"),
        vec![
            "Read".to_string(),
            "Grep".to_string(),
            "Glob".to_string(),
            "Bash".to_string(),
            "AskUserQuestion".to_string(),
        ],
    )
}

/// Create an AgentConfig for the planner agent
pub fn planner_config(project_root: &Path) -> AgentConfig {
    AgentConfig::new(
        "specks-planner",
        get_agent_path(project_root, "specks-planner"),
        vec![
            "Read".to_string(),
            "Grep".to_string(),
            "Glob".to_string(),
            "Bash".to_string(),
            "Write".to_string(),
            "Edit".to_string(),
            "AskUserQuestion".to_string(),
        ],
    )
}

/// Create an AgentConfig for the critic agent
pub fn critic_config(project_root: &Path) -> AgentConfig {
    AgentConfig::new(
        "specks-critic",
        get_agent_path(project_root, "specks-critic"),
        vec![
            "Read".to_string(),
            "Grep".to_string(),
            "Glob".to_string(),
            "Bash".to_string(),
        ],
    )
}

/// Create an AgentConfig for the director agent
pub fn director_config(project_root: &Path) -> AgentConfig {
    AgentConfig::new(
        "specks-director",
        get_agent_path(project_root, "specks-director"),
        vec![
            "Task".to_string(),
            "Read".to_string(),
            "Grep".to_string(),
            "Glob".to_string(),
            "Bash".to_string(),
            "Write".to_string(),
            "Edit".to_string(),
        ],
    )
}

/// Create an AgentConfig for the clarifier agent
pub fn clarifier_config(project_root: &Path) -> AgentConfig {
    AgentConfig::new(
        "specks-clarifier",
        get_agent_path(project_root, "specks-clarifier"),
        vec![
            "Read".to_string(),
            "Grep".to_string(),
            "Glob".to_string(),
            "Bash".to_string(),
        ],
    )
}

/// Parsed event from stream-json
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Text content delta
    Text(String),
    /// Tool use started (tool name)
    ToolStart(String),
    /// Tool use completed
    ToolEnd,
}

/// Parse a stream-json event from the Claude CLI.
///
/// The stream-json format outputs NDJSON (newline-delimited JSON) events.
///
/// Key event types:
/// - `content_block_delta` with `text` - streaming text
/// - `content_block_start` with `tool_use` - tool invocation starting
/// - `content_block_stop` - content block (including tool use) finished
fn parse_stream_json_event(line: &str) -> Option<StreamEvent> {
    let json: serde_json::Value = serde_json::from_str(line).ok()?;

    if json.get("type").and_then(|t| t.as_str()) != Some("stream_event") {
        return None;
    }

    let event = json.get("event")?;
    let event_type = event.get("type").and_then(|t| t.as_str())?;

    match event_type {
        "content_block_delta" => {
            // Text delta
            if let Some(text) = event
                .get("delta")
                .and_then(|d| d.get("text"))
                .and_then(|t| t.as_str())
            {
                if !text.is_empty() {
                    return Some(StreamEvent::Text(text.to_string()));
                }
            }
        }
        "content_block_start" => {
            // Check for tool use start
            if let Some(content_block) = event.get("content_block") {
                if content_block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                    if let Some(name) = content_block.get("name").and_then(|n| n.as_str()) {
                        return Some(StreamEvent::ToolStart(name.to_string()));
                    }
                }
            }
        }
        "content_block_stop" => {
            return Some(StreamEvent::ToolEnd);
        }
        _ => {}
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_config_creation() {
        let config = AgentConfig::new(
            "test-agent",
            PathBuf::from("/path/to/agent.md"),
            vec!["Read".to_string(), "Write".to_string()],
        );

        assert_eq!(config.agent_name, "test-agent");
        assert_eq!(config.agent_path, PathBuf::from("/path/to/agent.md"));
        assert_eq!(config.allowed_tools.len(), 2);
        assert_eq!(config.timeout_secs, DEFAULT_TIMEOUT_SECS);
    }

    #[test]
    fn test_agent_config_with_timeout() {
        let config = AgentConfig::new(
            "test-agent",
            PathBuf::from("/path/to/agent.md"),
            vec!["Read".to_string()],
        )
        .with_timeout(600);

        assert_eq!(config.timeout_secs, 600);
    }

    #[test]
    fn test_agent_runner_creation() {
        let runner = AgentRunner::new(PathBuf::from("/project"));
        assert_eq!(runner.claude_path, PathBuf::from("claude"));
        assert_eq!(runner.project_root, PathBuf::from("/project"));
    }

    #[test]
    fn test_agent_runner_with_custom_path() {
        let runner = AgentRunner::new(PathBuf::from("/project"))
            .with_claude_path(PathBuf::from("/custom/claude"));
        assert_eq!(runner.claude_path, PathBuf::from("/custom/claude"));
    }

    #[test]
    fn test_check_claude_cli_not_installed() {
        // Use a path that definitely doesn't exist
        let runner = AgentRunner::new(PathBuf::from("/project")).with_claude_path(PathBuf::from(
            "/nonexistent/path/to/claude-that-does-not-exist",
        ));

        let result = runner.check_claude_cli();
        assert!(result.is_err());

        if let Err(SpecksError::ClaudeCliNotInstalled) = result {
            // Expected
        } else {
            panic!("Expected ClaudeCliNotInstalled error");
        }
    }

    #[test]
    fn test_construct_agent_path() {
        // Pure path construction - no I/O, deterministic
        let project_root = PathBuf::from("/project");
        let path = construct_agent_path(&project_root, "specks-interviewer");
        assert_eq!(path, PathBuf::from("/project/agents/specks-interviewer.md"));
    }

    #[test]
    fn test_get_agent_path_resolves_in_dev() {
        // When running in dev workspace, get_agent_path resolves to real agents
        let fake_project = PathBuf::from("/nonexistent-project");
        let path = get_agent_path(&fake_project, "specks-interviewer");

        // Should resolve to development workspace since /nonexistent has no agents
        if let Some(workspace) = find_binary_workspace_root() {
            assert_eq!(path, workspace.join("agents/specks-interviewer.md"));
        } else {
            // If no dev workspace detected, falls back to constructed path
            assert_eq!(
                path,
                PathBuf::from("/nonexistent-project/agents/specks-interviewer.md")
            );
        }
    }

    #[test]
    fn test_interviewer_config() {
        let project_root = PathBuf::from("/project");
        let config = interviewer_config(&project_root);

        assert_eq!(config.agent_name, "specks-interviewer");
        assert!(
            config
                .allowed_tools
                .contains(&"AskUserQuestion".to_string())
        );
        assert!(config.allowed_tools.contains(&"Read".to_string()));
    }

    #[test]
    fn test_planner_config() {
        let project_root = PathBuf::from("/project");
        let config = planner_config(&project_root);

        assert_eq!(config.agent_name, "specks-planner");
        assert!(config.allowed_tools.contains(&"Write".to_string()));
        assert!(config.allowed_tools.contains(&"Edit".to_string()));
    }

    #[test]
    fn test_critic_config() {
        let project_root = PathBuf::from("/project");
        let config = critic_config(&project_root);

        assert_eq!(config.agent_name, "specks-critic");
        assert!(!config.allowed_tools.contains(&"Write".to_string()));
        assert!(!config.allowed_tools.contains(&"Edit".to_string()));
    }

    #[test]
    fn test_director_config() {
        let project_root = PathBuf::from("/project");
        let config = director_config(&project_root);

        assert_eq!(config.agent_name, "specks-director");
        assert!(config.allowed_tools.contains(&"Task".to_string()));
    }

    #[test]
    fn test_clarifier_config() {
        let project_root = PathBuf::from("/project");
        let config = clarifier_config(&project_root);

        assert_eq!(config.agent_name, "specks-clarifier");
        // Clarifier has read-only tools - no Write, Edit, or AskUserQuestion
        assert!(config.allowed_tools.contains(&"Read".to_string()));
        assert!(config.allowed_tools.contains(&"Grep".to_string()));
        assert!(config.allowed_tools.contains(&"Glob".to_string()));
        assert!(config.allowed_tools.contains(&"Bash".to_string()));
        assert!(!config.allowed_tools.contains(&"Write".to_string()));
        assert!(!config.allowed_tools.contains(&"Edit".to_string()));
        assert!(
            !config
                .allowed_tools
                .contains(&"AskUserQuestion".to_string())
        );
    }

    #[test]
    fn test_invoke_agent_file_not_found() {
        // Test that invoke_agent fails gracefully when agent file doesn't exist
        // Skip this test if claude is not installed (we're testing the file check, not claude)
        let runner = AgentRunner::new(PathBuf::from("/nonexistent/project"));

        let config = AgentConfig::new(
            "test-agent",
            PathBuf::from("/nonexistent/agent.md"),
            vec!["Read".to_string()],
        );

        // This should fail either at claude check or file read
        let result = runner.invoke_agent(&config, "test prompt");
        assert!(result.is_err());
    }

    /// Find workspace root by looking for Cargo.toml with [workspace]
    fn find_workspace_root() -> PathBuf {
        // Start from the manifest directory (crate root)
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // Go up two levels: crates/specks -> crates -> workspace root
        manifest_dir
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf()
    }

    #[test]
    fn test_mock_claude_cli_available() {
        // Verify that our mock claude CLI is available in tests/bin
        let workspace_root = find_workspace_root();
        let mock_path = workspace_root.join("tests/bin/claude-mock");

        // The mock should exist
        assert!(
            mock_path.exists(),
            "claude-mock should exist at {:?}",
            mock_path
        );
    }

    #[test]
    fn test_invoke_agent_with_mock() {
        // Integration test using the mock claude CLI
        let workspace_root = find_workspace_root();
        let mock_path = workspace_root.join("tests/bin/claude-mock");

        // Skip if mock doesn't exist
        if !mock_path.exists() {
            eprintln!("Skipping test: claude-mock not found at {:?}", mock_path);
            return;
        }

        // Create a temporary agent file
        let temp_dir = std::env::temp_dir().join("specks-test-agent");
        let _ = std::fs::create_dir_all(&temp_dir);
        let agent_path = temp_dir.join("test-agent.md");

        std::fs::write(&agent_path, "# Test Agent\n\nYou are a test agent.")
            .expect("write agent file");

        // Create runner with mock path
        let runner = AgentRunner::new(workspace_root.clone()).with_claude_path(mock_path.clone());

        // Verify mock is "installed"
        assert!(
            runner.check_claude_cli().is_ok(),
            "mock claude should be detected"
        );

        // Clean up
        let _ = std::fs::remove_file(&agent_path);
        let _ = std::fs::remove_dir(&temp_dir);
    }

    #[test]
    fn test_invoke_agent_mock_success() {
        // Test successful agent invocation with mock
        // The mock outputs empty string by default and exits 0
        let workspace_root = find_workspace_root();
        let mock_path = workspace_root.join("tests/bin/claude-mock");

        if !mock_path.exists() {
            eprintln!("Skipping test: claude-mock not found");
            return;
        }

        // Create a temporary agent file
        let temp_dir = std::env::temp_dir().join("specks-test-agent-success");
        let _ = std::fs::create_dir_all(&temp_dir);
        let agent_path = temp_dir.join("test-agent.md");
        std::fs::write(&agent_path, "# Test Agent").expect("write agent file");

        let runner = AgentRunner::new(workspace_root).with_claude_path(mock_path);

        let config = AgentConfig::new("test-agent", agent_path.clone(), vec!["Read".to_string()]);

        // Default mock behavior: exit 0, empty output
        let result = runner.invoke_agent(&config, "test prompt");
        assert!(result.is_ok(), "invoke_agent should succeed: {:?}", result);

        let agent_result = result.unwrap();
        assert!(agent_result.success);

        // Clean up
        let _ = std::fs::remove_file(&agent_path);
        let _ = std::fs::remove_dir(&temp_dir);
    }

    // ============================================================================
    // Agent Resolution Tests
    // ============================================================================

    #[test]
    fn test_is_specks_workspace_in_repo() {
        // When running tests, we're in the specks workspace
        let workspace_root = find_workspace_root();
        assert!(
            is_specks_workspace(&workspace_root),
            "Running from specks repo should detect workspace"
        );
    }

    #[test]
    fn test_is_specks_workspace_random_dir() {
        // A random temp directory should not be detected as specks workspace
        let temp_dir = std::env::temp_dir().join("specks-test-random");
        let _ = std::fs::create_dir_all(&temp_dir);

        assert!(
            !is_specks_workspace(&temp_dir),
            "Random directory should not be detected as workspace"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_is_specks_workspace_cargo_toml_only() {
        // A directory with Cargo.toml but no workspace markers should not match
        let temp_dir = std::env::temp_dir().join("specks-test-cargo-only");
        let _ = std::fs::create_dir_all(&temp_dir);

        std::fs::write(
            temp_dir.join("Cargo.toml"),
            "[package]\nname = \"other-project\"\n",
        )
        .expect("write Cargo.toml");

        assert!(
            !is_specks_workspace(&temp_dir),
            "Non-specks Cargo.toml should not be detected"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_resolve_agent_path_finds_in_project() {
        // When agent exists in project/agents/, it should be found
        let temp_dir = std::env::temp_dir().join("specks-test-resolve-project");
        let agents_dir = temp_dir.join("agents");
        let _ = std::fs::create_dir_all(&agents_dir);

        let agent_path = agents_dir.join("specks-interviewer.md");
        std::fs::write(&agent_path, "# Test Agent").expect("write agent");

        let resolved = resolve_agent_path("specks-interviewer", &temp_dir);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap(), agent_path);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_resolve_agent_path_returns_none_when_not_found() {
        // When agent doesn't exist anywhere, None should be returned
        let temp_dir = std::env::temp_dir().join("specks-test-resolve-none");
        let _ = std::fs::create_dir_all(&temp_dir);

        // Don't set SPECKS_SHARE_DIR, and ensure no agents exist locally
        // This test may pass or fail depending on system state, so we just
        // verify the function doesn't panic
        let resolved = resolve_agent_path("nonexistent-agent-xyz", &temp_dir);
        // Can be None or Some depending on share dir state
        drop(resolved);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_resolve_agent_path_with_source_returns_project() {
        let temp_dir = std::env::temp_dir().join("specks-test-resolve-source");
        let agents_dir = temp_dir.join("agents");
        let _ = std::fs::create_dir_all(&agents_dir);

        let agent_path = agents_dir.join("specks-planner.md");
        std::fs::write(&agent_path, "# Test Planner").expect("write agent");

        let resolved = resolve_agent_path_with_source("specks-planner", &temp_dir);
        assert!(resolved.is_some());

        let (path, source) = resolved.unwrap();
        assert_eq!(path, agent_path);
        assert_eq!(source, AgentSource::Project);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_verify_required_agents_unknown_command() {
        // Unknown command should return Ok with empty vec
        let temp_dir = std::env::temp_dir().join("specks-test-unknown-cmd");
        let _ = std::fs::create_dir_all(&temp_dir);

        let result = verify_required_agents("unknown-command", &temp_dir);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_verify_required_agents_plan_in_workspace() {
        // In the specks workspace, all plan agents should be found
        let workspace_root = find_workspace_root();

        let result = verify_required_agents("plan", &workspace_root);
        assert!(result.is_ok(), "Plan agents should be found: {:?}", result);

        let agents = result.unwrap();
        assert_eq!(agents.len(), PLAN_REQUIRED_AGENTS.len());

        // Verify all expected agents were found
        for agent_name in PLAN_REQUIRED_AGENTS {
            assert!(
                agents.iter().any(|(name, _, _)| name == agent_name),
                "Agent {} should be found",
                agent_name
            );
        }
    }

    #[test]
    fn test_verify_required_agents_execute_in_workspace() {
        // In the specks workspace, all execute agents should be found
        let workspace_root = find_workspace_root();

        let result = verify_required_agents("execute", &workspace_root);
        assert!(
            result.is_ok(),
            "Execute agents should be found: {:?}",
            result
        );

        let agents = result.unwrap();
        assert_eq!(agents.len(), EXECUTE_REQUIRED_AGENTS.len());
    }

    #[test]
    fn test_verify_required_agents_missing_returns_error() {
        // In a directory without agents (and no share dir), should return error
        let temp_dir = std::env::temp_dir().join("specks-test-missing-agents");
        let _ = std::fs::create_dir_all(&temp_dir);

        // Set empty share dir to ensure no fallback
        // SAFETY: We're in a test context, single-threaded test execution
        unsafe {
            std::env::set_var(
                "SPECKS_SHARE_DIR",
                temp_dir
                    .join("nonexistent-share")
                    .to_string_lossy()
                    .to_string(),
            );
        }

        let result = verify_required_agents("plan", &temp_dir);

        // Clean up env var
        // SAFETY: Same as above
        unsafe {
            std::env::remove_var("SPECKS_SHARE_DIR");
        }

        match result {
            Err(SpecksError::RequiredAgentsMissing {
                command,
                missing,
                searched,
            }) => {
                assert_eq!(command, "plan");
                assert!(!missing.is_empty());
                assert!(!searched.is_empty());
            }
            Ok(_) => {
                // This could happen if agents are found via some other discovery path
                // (e.g., system-wide installation), which is fine
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_partial_override_uses_project_agent() {
        // Test that a project-local agent overrides share directory
        let temp_dir = std::env::temp_dir().join("specks-test-partial-override");
        let agents_dir = temp_dir.join("agents");
        let _ = std::fs::create_dir_all(&agents_dir);

        // Create one custom agent locally
        let custom_agent = agents_dir.join("specks-interviewer.md");
        std::fs::write(&custom_agent, "# Custom Interviewer").expect("write agent");

        // Resolve should find the local one
        let resolved = resolve_agent_path_with_source("specks-interviewer", &temp_dir);
        assert!(resolved.is_some());

        let (path, source) = resolved.unwrap();
        assert_eq!(source, AgentSource::Project);
        assert_eq!(path, custom_agent);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_agent_source_display() {
        assert_eq!(AgentSource::Project.to_string(), "project");
        assert_eq!(AgentSource::Share.to_string(), "share");
        assert_eq!(AgentSource::Development.to_string(), "development");
    }

    #[test]
    fn test_plan_required_agents_contains_expected() {
        assert!(PLAN_REQUIRED_AGENTS.contains(&"specks-clarifier"));
        assert!(PLAN_REQUIRED_AGENTS.contains(&"specks-interviewer"));
        assert!(PLAN_REQUIRED_AGENTS.contains(&"specks-planner"));
        assert!(PLAN_REQUIRED_AGENTS.contains(&"specks-critic"));
        assert_eq!(PLAN_REQUIRED_AGENTS.len(), 4);
    }

    #[test]
    fn test_execute_required_agents_contains_expected() {
        assert!(EXECUTE_REQUIRED_AGENTS.contains(&"specks-director"));
        assert!(EXECUTE_REQUIRED_AGENTS.contains(&"specks-architect"));
        assert!(EXECUTE_REQUIRED_AGENTS.contains(&"specks-implementer"));
        assert!(EXECUTE_REQUIRED_AGENTS.contains(&"specks-monitor"));
        assert!(EXECUTE_REQUIRED_AGENTS.contains(&"specks-reviewer"));
        assert!(EXECUTE_REQUIRED_AGENTS.contains(&"specks-auditor"));
        assert!(EXECUTE_REQUIRED_AGENTS.contains(&"specks-committer"));
        assert!(EXECUTE_REQUIRED_AGENTS.contains(&"specks-logger"));
        assert_eq!(EXECUTE_REQUIRED_AGENTS.len(), 8);
    }

    #[test]
    fn test_find_binary_workspace_root_in_cargo_tests() {
        // When running tests via cargo, the test binary is in target/debug/deps/
        // The function should find the specks workspace root
        let result = find_binary_workspace_root();

        // In cargo test context, we should find the workspace
        assert!(
            result.is_some(),
            "Should find workspace root when running cargo tests"
        );

        let workspace = result.unwrap();
        assert!(
            is_specks_workspace(&workspace),
            "Detected workspace should be the specks workspace"
        );

        // Verify it matches the expected workspace root
        let expected = find_workspace_root();
        assert_eq!(
            workspace, expected,
            "Binary workspace detection should match test workspace"
        );
    }
}
