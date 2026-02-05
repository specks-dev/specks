//! Agent invocation infrastructure for shelling out to Claude CLI
//!
//! This module provides the infrastructure to invoke specks agents via the `claude` CLI.
//! Per [D02] Shell out to Claude CLI, agent invocation shells out rather than using direct API calls.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::Duration;

use specks_core::SpecksError;

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
            cmd.arg("--allowedTools");
            cmd.arg(config.allowed_tools.join(","));
        }

        // Add system prompt (the agent definition)
        cmd.arg("--systemPrompt");
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
}

/// Find the agents directory relative to the project root
pub fn find_agents_dir(project_root: &Path) -> PathBuf {
    project_root.join("agents")
}

/// Get the path to a specific agent definition
pub fn get_agent_path(project_root: &Path, agent_name: &str) -> PathBuf {
    find_agents_dir(project_root).join(format!("{}.md", agent_name))
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
    fn test_find_agents_dir() {
        let project_root = PathBuf::from("/project");
        let agents_dir = find_agents_dir(&project_root);
        assert_eq!(agents_dir, PathBuf::from("/project/agents"));
    }

    #[test]
    fn test_get_agent_path() {
        let project_root = PathBuf::from("/project");
        let path = get_agent_path(&project_root, "specks-interviewer");
        assert_eq!(path, PathBuf::from("/project/agents/specks-interviewer.md"));
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
}
