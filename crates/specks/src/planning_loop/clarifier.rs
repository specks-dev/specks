//! Clarifier agent invocation and output parsing
//!
//! Per [D21] and [D24], the clarifier agent runs in EVERY iteration of the planning loop:
//! - First iteration: analyzes the user's idea and generates clarifying questions
//! - Subsequent iterations: analyzes critic feedback and generates revision questions
//!
//! The clarifier produces JSON output that is parsed into `ClarifierOutput` and passed
//! to the presentation layer (CLI prompts or interviewer agent).

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use specks_core::SpecksError;

use crate::agent::{AgentRunner, clarifier_config};

use super::cli_present::{Priority, PunchListItem};

/// Input to the clarifier agent.
///
/// The clarifier operates in two modes:
/// - `Idea`: First iteration, analyzing the user's original idea
/// - `CriticFeedback`: Subsequent iterations, analyzing what needs revision
#[derive(Debug, Clone)]
pub enum ClarifierInput {
    /// First iteration: the user's original idea
    Idea {
        /// The user's idea string
        idea: String,
        /// Additional context files
        context_files: Vec<String>,
    },
    /// Subsequent iterations: the critic's feedback
    CriticFeedback {
        /// The critic's report highlighting issues
        critic_feedback: String,
        /// Path to the current draft speck
        speck_path: String,
        /// Parsed punch list items from critic feedback (structured issues)
        critic_issues: Vec<PunchListItem>,
    },
}

impl ClarifierInput {
    /// Get the mode string for this input
    pub fn mode(&self) -> &'static str {
        match self {
            ClarifierInput::Idea { .. } => "idea",
            ClarifierInput::CriticFeedback { .. } => "revision",
        }
    }

    /// Build the prompt for the clarifier agent
    pub fn to_prompt(&self) -> String {
        match self {
            ClarifierInput::Idea {
                idea,
                context_files,
            } => {
                let mut prompt = format!(
                    r#"Analyze this idea and generate clarifying questions.

Input mode: idea

The user's idea:
{}

"#,
                    idea
                );

                if !context_files.is_empty() {
                    prompt.push_str("Additional context:\n");
                    for ctx in context_files {
                        prompt.push_str(ctx);
                        prompt.push_str("\n\n");
                    }
                }

                prompt.push_str(
                    r#"
Explore the codebase for relevant patterns and context before generating questions.

Return your response as JSON matching this format:
{
  "mode": "idea",
  "analysis": {
    "understood_intent": "...",
    "relevant_context": ["file - reason"],
    "identified_ambiguities": ["..."]
  },
  "questions": [
    {
      "question": "...",
      "options": ["option1", "option2"],
      "why_asking": "...",
      "default": "option1"
    }
  ],
  "assumptions_if_no_answer": ["..."]
}

Return ONLY valid JSON, no markdown code blocks or other text."#,
                );

                prompt
            }
            ClarifierInput::CriticFeedback {
                critic_feedback,
                speck_path,
                critic_issues,
            } => {
                let mut prompt = format!(
                    r#"Analyze the critic's feedback and generate questions about what to revise.

Input mode: revision

"#
                );

                // Add structured issues if available
                if !critic_issues.is_empty() {
                    prompt.push_str("Issues to address:\n");
                    for (i, issue) in critic_issues.iter().enumerate() {
                        let priority_label = match issue.priority {
                            Priority::High => "HIGH",
                            Priority::Medium => "MEDIUM",
                            Priority::Low => "LOW",
                        };
                        prompt.push_str(&format!(
                            "{}. [{}] {}\n",
                            i + 1,
                            priority_label,
                            issue.description
                        ));
                    }
                    prompt.push_str("\nFor each issue, generate a question with options for how to fix it.\n\n");
                }

                prompt.push_str(&format!(
                    r#"Full critic's feedback:
{}

Current speck path: {}

Read the speck file and the critic's feedback. Generate questions about:
1. How to address each issue (focus on the structured issues above if present)
2. Priorities for which issues to fix first
3. Any trade-offs the user should decide

Return your response as JSON matching this format:
{{
  "mode": "revision",
  "analysis": {{
    "understood_intent": "...",
    "relevant_context": ["file - reason"],
    "identified_ambiguities": ["..."]
  }},
  "questions": [
    {{
      "question": "...",
      "options": ["option1", "option2"],
      "why_asking": "...",
      "default": "option1"
    }}
  ],
  "assumptions_if_no_answer": ["..."]
}}

Return ONLY valid JSON, no markdown code blocks or other text."#,
                    critic_feedback, speck_path
                ));

                prompt
            }
        }
    }
}

/// Output from the clarifier agent.
///
/// This is parsed from the agent's JSON response.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClarifierOutput {
    /// The mode that was analyzed ("idea" or "revision")
    pub mode: String,

    /// Analysis of the input
    pub analysis: ClarifierAnalysis,

    /// Generated questions (can be empty if idea was detailed enough)
    pub questions: Vec<ClarifierQuestion>,

    /// What the clarifier will assume if questions are skipped
    pub assumptions_if_no_answer: Vec<String>,
}

impl ClarifierOutput {
    /// Create an empty output for when clarifier is not invoked
    #[allow(dead_code)] // Will be used in Step 8.3.6
    pub fn empty(mode: &str) -> Self {
        Self {
            mode: mode.to_string(),
            analysis: ClarifierAnalysis::default(),
            questions: Vec::new(),
            assumptions_if_no_answer: Vec::new(),
        }
    }

    /// Check if there are no questions to ask
    pub fn has_no_questions(&self) -> bool {
        self.questions.is_empty()
    }
}

/// Analysis section of clarifier output
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClarifierAnalysis {
    /// Clarifier's understanding of what the user wants
    pub understood_intent: String,

    /// Files/patterns found in codebase that are relevant
    pub relevant_context: Vec<String>,

    /// Ambiguities or gaps that need clarification
    pub identified_ambiguities: Vec<String>,
}

/// A single question generated by the clarifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarifierQuestion {
    /// The question to ask the user
    pub question: String,

    /// Available options (2-4 choices)
    pub options: Vec<String>,

    /// Explanation of why this question matters
    pub why_asking: String,

    /// Default option if user doesn't answer
    pub default: String,
}

/// Invoke the clarifier agent and parse its output.
///
/// # Arguments
///
/// * `input` - The clarifier input (idea or critic feedback)
/// * `runner` - The agent runner to use for invocation
/// * `project_root` - The project root path
/// * `timeout_secs` - Timeout for the agent invocation
///
/// # Returns
///
/// The parsed clarifier output, or an error if invocation or parsing fails.
pub fn invoke_clarifier(
    input: &ClarifierInput,
    runner: &AgentRunner,
    project_root: &Path,
    timeout_secs: u64,
) -> Result<ClarifierOutput, SpecksError> {
    // Build the prompt
    let prompt = input.to_prompt();

    // Get clarifier config
    let config = clarifier_config(project_root).with_timeout(timeout_secs);

    // Invoke the agent
    let result = runner.invoke_agent(&config, &prompt)?;

    // Parse the JSON output
    parse_clarifier_output(&result.output, input.mode())
}

/// Invoke the clarifier agent with streaming output.
///
/// This version shows agent output in real-time with a spinner at the bottom.
/// Use this for interactive CLI mode to provide better user feedback.
///
/// # Arguments
///
/// * `input` - The clarifier input (idea or critic feedback)
/// * `runner` - The agent runner to use for invocation
/// * `project_root` - The project root path
/// * `timeout_secs` - Timeout for the agent invocation
/// * `display` - The streaming display to use for output
///
/// # Returns
///
/// The parsed clarifier output, or an error if invocation or parsing fails.
pub fn invoke_clarifier_streaming(
    input: &ClarifierInput,
    runner: &AgentRunner,
    project_root: &Path,
    timeout_secs: u64,
    display: &mut crate::streaming::StreamingDisplay,
) -> Result<ClarifierOutput, SpecksError> {
    // Build the prompt
    let prompt = input.to_prompt();

    // Get clarifier config
    let config = clarifier_config(project_root).with_timeout(timeout_secs);

    // Invoke the agent with spinner only (JSON output is illegible when streamed)
    let result = runner.invoke_agent_spinner_only(&config, &prompt, display)?;

    // Parse the JSON output
    parse_clarifier_output(&result.output, input.mode())
}

/// Parse clarifier output from JSON string.
///
/// Handles common issues like prose before JSON, markdown code blocks, etc.
fn parse_clarifier_output(
    output: &str,
    expected_mode: &str,
) -> Result<ClarifierOutput, SpecksError> {
    // Extract the JSON object from the output (handles prose, markdown, etc.)
    let cleaned = extract_json_from_output(output);

    // Try to parse as JSON
    match serde_json::from_str::<ClarifierOutput>(&cleaned) {
        Ok(parsed) => Ok(parsed),
        Err(parse_err) => {
            // If JSON parsing fails, create a fallback output with the raw response
            // This allows the planning loop to continue even if clarifier output is malformed
            eprintln!(
                "Warning: Failed to parse clarifier JSON output: {}",
                parse_err
            );
            eprintln!("Raw output: {}", output);

            // Return an empty clarifier output to allow the loop to continue
            Ok(ClarifierOutput {
                mode: expected_mode.to_string(),
                analysis: ClarifierAnalysis {
                    understood_intent: "Unable to parse clarifier response".to_string(),
                    relevant_context: Vec::new(),
                    identified_ambiguities: Vec::new(),
                },
                questions: Vec::new(),
                assumptions_if_no_answer: vec![
                    "Proceeding with default assumptions due to parse error".to_string(),
                ],
            })
        }
    }
}

/// Extract JSON object from agent output.
///
/// Agents sometimes output prose before/after the JSON object.
/// This function finds and extracts just the JSON.
fn extract_json_from_output(text: &str) -> String {
    let trimmed = text.trim();

    // First, try to strip markdown code blocks
    let without_markdown = strip_markdown_code_block(trimmed);

    // If it already looks like valid JSON, return it
    if without_markdown.starts_with('{') && without_markdown.ends_with('}') {
        return without_markdown;
    }

    // Otherwise, find the JSON object in the text
    // Look for the first '{' that starts a JSON object with "mode" key
    if let Some(start) = trimmed.find("{\"mode\"") {
        // Find the matching closing brace
        if let Some(json_str) = extract_balanced_braces(&trimmed[start..]) {
            return json_str;
        }
    }

    // Fallback: find any JSON object
    if let Some(start) = trimmed.find('{') {
        if let Some(json_str) = extract_balanced_braces(&trimmed[start..]) {
            return json_str;
        }
    }

    // If we can't find JSON, return original (will fail parsing with good error)
    without_markdown
}

/// Extract a balanced JSON object starting from the beginning of the string.
fn extract_balanced_braces(text: &str) -> Option<String> {
    if !text.starts_with('{') {
        return None;
    }

    let mut depth = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for (i, c) in text.char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }

        match c {
            '\\' if in_string => escape_next = true,
            '"' => in_string = !in_string,
            '{' if !in_string => depth += 1,
            '}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(text[..=i].to_string());
                }
            }
            _ => {}
        }
    }

    None
}

/// Strip markdown code block markers from JSON output.
fn strip_markdown_code_block(text: &str) -> String {
    let trimmed = text.trim();

    // Check for ```json or ``` at start
    let without_start = if trimmed.starts_with("```json") {
        trimmed.strip_prefix("```json").unwrap_or(trimmed).trim()
    } else if trimmed.starts_with("```") {
        trimmed.strip_prefix("```").unwrap_or(trimmed).trim()
    } else {
        trimmed
    };

    // Check for ``` at end
    let without_end = if without_start.ends_with("```") {
        without_start
            .strip_suffix("```")
            .unwrap_or(without_start)
            .trim()
    } else {
        without_start
    };

    without_end.to_string()
}

/// Enriched requirements combining idea, clarifier analysis, and user answers.
///
/// This is the output of the clarifier + presentation phase, ready for the planner.
/// Will be fully utilized in Step 8.3.6 when CLI gather presents clarifier questions.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)] // Fields used in Step 8.3.6
pub struct EnrichedRequirements {
    /// The original idea string
    pub original_idea: String,

    /// The clarifier's analysis and questions
    pub clarifier_output: Option<ClarifierOutput>,

    /// User's answers to clarifier questions (keyed by question text)
    pub user_answers: HashMap<String, String>,

    /// Critic feedback from previous iteration (if revision)
    pub critic_feedback: Option<String>,
}

impl EnrichedRequirements {
    /// Create new enriched requirements for an idea
    pub fn new(idea: String) -> Self {
        Self {
            original_idea: idea,
            clarifier_output: None,
            user_answers: HashMap::new(),
            critic_feedback: None,
        }
    }

    /// Create enriched requirements for a revision iteration
    pub fn for_revision(idea: String, critic_feedback: String) -> Self {
        Self {
            original_idea: idea,
            clarifier_output: None,
            user_answers: HashMap::new(),
            critic_feedback: Some(critic_feedback),
        }
    }

    /// Set the clarifier output
    pub fn with_clarifier_output(mut self, output: ClarifierOutput) -> Self {
        self.clarifier_output = Some(output);
        self
    }

    /// Add a user answer
    #[allow(dead_code)] // Will be used in Step 8.3.6
    pub fn add_answer(&mut self, question: &str, answer: String) {
        self.user_answers.insert(question.to_string(), answer);
    }

    /// Format these requirements as a prompt for the planner agent.
    ///
    /// The planner receives:
    /// 1. The original idea
    /// 2. Clarifier's analysis (what it understood, relevant context)
    /// 3. User's answers to questions
    /// 4. Critic feedback (if revision iteration)
    #[allow(dead_code)] // Will be used in Step 8.3.6
    pub fn to_planner_prompt(&self) -> String {
        let mut prompt = String::new();

        // Section 1: Original Idea
        prompt.push_str("## Original Idea\n\n");
        prompt.push_str(&self.original_idea);
        prompt.push_str("\n\n");

        // Section 2: Clarifier Analysis (if available)
        if let Some(ref output) = self.clarifier_output {
            prompt.push_str("## Clarifier Analysis\n\n");

            prompt.push_str("### Understood Intent\n");
            prompt.push_str(&output.analysis.understood_intent);
            prompt.push_str("\n\n");

            if !output.analysis.relevant_context.is_empty() {
                prompt.push_str("### Relevant Context\n");
                for ctx in &output.analysis.relevant_context {
                    prompt.push_str("- ");
                    prompt.push_str(ctx);
                    prompt.push('\n');
                }
                prompt.push('\n');
            }

            if !output.analysis.identified_ambiguities.is_empty() {
                prompt.push_str("### Identified Ambiguities\n");
                for ambiguity in &output.analysis.identified_ambiguities {
                    prompt.push_str("- ");
                    prompt.push_str(ambiguity);
                    prompt.push('\n');
                }
                prompt.push('\n');
            }
        }

        // Section 3: User Answers (if any)
        if !self.user_answers.is_empty() {
            prompt.push_str("## User Decisions\n\n");
            for (question, answer) in &self.user_answers {
                prompt.push_str("**Q:** ");
                prompt.push_str(question);
                prompt.push_str("\n**A:** ");
                prompt.push_str(answer);
                prompt.push_str("\n\n");
            }
        }

        // Section 4: Assumptions (if clarifier questions were skipped)
        if let Some(ref output) = self.clarifier_output {
            // Show assumptions for questions that weren't answered
            let unanswered: Vec<_> = output
                .questions
                .iter()
                .filter(|q| !self.user_answers.contains_key(&q.question))
                .collect();

            if !unanswered.is_empty() {
                prompt.push_str("## Assumed Defaults\n\n");
                for q in unanswered {
                    prompt.push_str("- ");
                    prompt.push_str(&q.question);
                    prompt.push_str(" → ");
                    prompt.push_str(&q.default);
                    prompt.push('\n');
                }
                prompt.push('\n');
            }
        }

        // Section 5: Critic Feedback (if revision iteration)
        if let Some(ref feedback) = self.critic_feedback {
            prompt.push_str("## Previous Critic Feedback\n\n");
            prompt.push_str(feedback);
            prompt.push_str("\n\n");
        }

        prompt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clarifier_input_idea_mode() {
        let input = ClarifierInput::Idea {
            idea: "add a greeting command".to_string(),
            context_files: vec![],
        };
        assert_eq!(input.mode(), "idea");
    }

    #[test]
    fn test_clarifier_input_revision_mode() {
        let input = ClarifierInput::CriticFeedback {
            critic_feedback: "needs more detail".to_string(),
            speck_path: ".specks/specks-test.md".to_string(),
            critic_issues: vec![],
        };
        assert_eq!(input.mode(), "revision");
    }

    #[test]
    fn test_clarifier_input_idea_prompt() {
        let input = ClarifierInput::Idea {
            idea: "add a greeting command".to_string(),
            context_files: vec!["context 1".to_string()],
        };
        let prompt = input.to_prompt();

        assert!(prompt.contains("add a greeting command"));
        assert!(prompt.contains("Input mode: idea"));
        assert!(prompt.contains("context 1"));
        assert!(prompt.contains("JSON"));
    }

    #[test]
    fn test_clarifier_input_revision_prompt() {
        let input = ClarifierInput::CriticFeedback {
            critic_feedback: "step 3 is vague".to_string(),
            speck_path: ".specks/specks-1.md".to_string(),
            critic_issues: vec![],
        };
        let prompt = input.to_prompt();

        assert!(prompt.contains("step 3 is vague"));
        assert!(prompt.contains(".specks/specks-1.md"));
        assert!(prompt.contains("Input mode: revision"));
    }

    #[test]
    fn test_clarifier_input_revision_prompt_with_issues() {
        let input = ClarifierInput::CriticFeedback {
            critic_feedback: "step 3 is vague".to_string(),
            speck_path: ".specks/specks-1.md".to_string(),
            critic_issues: vec![
                PunchListItem::new(Priority::High, "Missing error handling"),
                PunchListItem::new(Priority::Medium, "Vague test strategy"),
                PunchListItem::new(Priority::Low, "Could add more comments"),
            ],
        };
        let prompt = input.to_prompt();

        // Should contain structured issues section
        assert!(prompt.contains("Issues to address:"));
        assert!(prompt.contains("[HIGH] Missing error handling"));
        assert!(prompt.contains("[MEDIUM] Vague test strategy"));
        assert!(prompt.contains("[LOW] Could add more comments"));
        assert!(prompt.contains("For each issue, generate a question with options for how to fix it."));

        // Should still contain the full feedback
        assert!(prompt.contains("step 3 is vague"));
        assert!(prompt.contains(".specks/specks-1.md"));
    }

    #[test]
    fn test_clarifier_output_parsing() {
        let json = r#"{
            "mode": "idea",
            "analysis": {
                "understood_intent": "Create a greeting command",
                "relevant_context": ["src/cli.rs - existing commands"],
                "identified_ambiguities": ["CLI or library?"]
            },
            "questions": [
                {
                    "question": "Should this be CLI only?",
                    "options": ["CLI only", "Library", "Both"],
                    "why_asking": "Affects design",
                    "default": "CLI only"
                }
            ],
            "assumptions_if_no_answer": ["Will assume CLI only"]
        }"#;

        let output: ClarifierOutput = serde_json::from_str(json).unwrap();

        assert_eq!(output.mode, "idea");
        assert_eq!(
            output.analysis.understood_intent,
            "Create a greeting command"
        );
        assert_eq!(output.analysis.relevant_context.len(), 1);
        assert_eq!(output.analysis.identified_ambiguities.len(), 1);
        assert_eq!(output.questions.len(), 1);
        assert_eq!(output.questions[0].question, "Should this be CLI only?");
        assert_eq!(output.questions[0].options.len(), 3);
        assert_eq!(output.questions[0].default, "CLI only");
        assert_eq!(output.assumptions_if_no_answer.len(), 1);
    }

    #[test]
    fn test_clarifier_output_empty_questions() {
        let json = r#"{
            "mode": "idea",
            "analysis": {
                "understood_intent": "Detailed idea",
                "relevant_context": [],
                "identified_ambiguities": []
            },
            "questions": [],
            "assumptions_if_no_answer": []
        }"#;

        let output: ClarifierOutput = serde_json::from_str(json).unwrap();

        assert!(output.has_no_questions());
    }

    #[test]
    fn test_clarifier_output_has_questions() {
        let json = r#"{
            "mode": "idea",
            "analysis": {
                "understood_intent": "Vague idea",
                "relevant_context": [],
                "identified_ambiguities": ["many things unclear"]
            },
            "questions": [
                {
                    "question": "What scope?",
                    "options": ["Full", "Minimal"],
                    "why_asking": "Affects size",
                    "default": "Full"
                }
            ],
            "assumptions_if_no_answer": []
        }"#;

        let output: ClarifierOutput = serde_json::from_str(json).unwrap();

        assert!(!output.has_no_questions());
    }

    #[test]
    fn test_strip_markdown_code_block() {
        let with_json_block = "```json\n{\"key\": \"value\"}\n```";
        assert_eq!(
            strip_markdown_code_block(with_json_block),
            r#"{"key": "value"}"#
        );

        let with_plain_block = "```\n{\"key\": \"value\"}\n```";
        assert_eq!(
            strip_markdown_code_block(with_plain_block),
            r#"{"key": "value"}"#
        );

        let without_block = r#"{"key": "value"}"#;
        assert_eq!(
            strip_markdown_code_block(without_block),
            r#"{"key": "value"}"#
        );
    }

    #[test]
    fn test_extract_json_from_prose() {
        let with_prose = r#"I'll explore the codebase first to understand context.

{"mode": "idea", "analysis": {"understood_intent": "test", "relevant_context": [], "identified_ambiguities": []}, "questions": [], "assumptions_if_no_answer": []}"#;

        let extracted = extract_json_from_output(with_prose);
        assert!(extracted.starts_with("{\"mode\""));

        // Verify it parses
        let parsed: ClarifierOutput = serde_json::from_str(&extracted).unwrap();
        assert_eq!(parsed.mode, "idea");
    }

    #[test]
    fn test_extract_json_with_nested_braces() {
        let nested = r#"Some prose here {"mode": "idea", "analysis": {"understood_intent": "test {with braces}", "relevant_context": [], "identified_ambiguities": []}, "questions": [], "assumptions_if_no_answer": []}"#;

        let extracted = extract_json_from_output(nested);
        let parsed: ClarifierOutput = serde_json::from_str(&extracted).unwrap();
        assert_eq!(parsed.analysis.understood_intent, "test {with braces}");
    }

    #[test]
    fn test_parse_clarifier_output_with_markdown() {
        let markdown_wrapped = r#"```json
{
    "mode": "idea",
    "analysis": {
        "understood_intent": "test",
        "relevant_context": [],
        "identified_ambiguities": []
    },
    "questions": [],
    "assumptions_if_no_answer": []
}
```"#;

        let output = parse_clarifier_output(markdown_wrapped, "idea").unwrap();
        assert_eq!(output.mode, "idea");
        assert_eq!(output.analysis.understood_intent, "test");
    }

    #[test]
    fn test_parse_clarifier_output_fallback_on_error() {
        let invalid_json = "This is not JSON at all";

        let output = parse_clarifier_output(invalid_json, "idea").unwrap();

        // Should return fallback output, not error
        assert_eq!(output.mode, "idea");
        assert!(output.questions.is_empty());
        assert!(
            output
                .analysis
                .understood_intent
                .contains("Unable to parse")
        );
    }

    #[test]
    fn test_clarifier_question_struct() {
        let question = ClarifierQuestion {
            question: "What scope?".to_string(),
            options: vec!["Full".to_string(), "Minimal".to_string()],
            why_asking: "Affects implementation size".to_string(),
            default: "Full".to_string(),
        };

        assert_eq!(question.question, "What scope?");
        assert_eq!(question.options.len(), 2);
        assert_eq!(question.why_asking, "Affects implementation size");
        assert_eq!(question.default, "Full");
    }

    #[test]
    fn test_enriched_requirements_new() {
        let req = EnrichedRequirements::new("add a feature".to_string());

        assert_eq!(req.original_idea, "add a feature");
        assert!(req.clarifier_output.is_none());
        assert!(req.user_answers.is_empty());
        assert!(req.critic_feedback.is_none());
    }

    #[test]
    fn test_enriched_requirements_for_revision() {
        let req = EnrichedRequirements::for_revision(
            "add a feature".to_string(),
            "step 3 needs work".to_string(),
        );

        assert_eq!(req.original_idea, "add a feature");
        assert_eq!(req.critic_feedback, Some("step 3 needs work".to_string()));
    }

    #[test]
    fn test_enriched_requirements_with_clarifier_output() {
        let output = ClarifierOutput::empty("idea");
        let req = EnrichedRequirements::new("test".to_string()).with_clarifier_output(output);

        assert!(req.clarifier_output.is_some());
    }

    #[test]
    fn test_enriched_requirements_add_answer() {
        let mut req = EnrichedRequirements::new("test".to_string());
        req.add_answer("What scope?", "Full".to_string());

        assert_eq!(
            req.user_answers.get("What scope?"),
            Some(&"Full".to_string())
        );
    }

    #[test]
    fn test_enriched_requirements_to_planner_prompt() {
        let output = ClarifierOutput {
            mode: "idea".to_string(),
            analysis: ClarifierAnalysis {
                understood_intent: "Create a greeting command".to_string(),
                relevant_context: vec!["src/cli.rs".to_string()],
                identified_ambiguities: vec!["output format unclear".to_string()],
            },
            questions: vec![ClarifierQuestion {
                question: "What scope?".to_string(),
                options: vec!["Full".to_string(), "Minimal".to_string()],
                why_asking: "Affects size".to_string(),
                default: "Full".to_string(),
            }],
            assumptions_if_no_answer: vec![],
        };

        let mut req = EnrichedRequirements::new("add greeting command".to_string())
            .with_clarifier_output(output);
        req.add_answer("What scope?", "Minimal".to_string());

        let prompt = req.to_planner_prompt();

        assert!(prompt.contains("## Original Idea"));
        assert!(prompt.contains("add greeting command"));
        assert!(prompt.contains("## Clarifier Analysis"));
        assert!(prompt.contains("Create a greeting command"));
        assert!(prompt.contains("src/cli.rs"));
        assert!(prompt.contains("output format unclear"));
        assert!(prompt.contains("## User Decisions"));
        assert!(prompt.contains("What scope?"));
        assert!(prompt.contains("Minimal"));
    }

    #[test]
    fn test_enriched_requirements_to_planner_prompt_with_unanswered() {
        let output = ClarifierOutput {
            mode: "idea".to_string(),
            analysis: ClarifierAnalysis::default(),
            questions: vec![ClarifierQuestion {
                question: "What scope?".to_string(),
                options: vec!["Full".to_string(), "Minimal".to_string()],
                why_asking: "Affects size".to_string(),
                default: "Full".to_string(),
            }],
            assumptions_if_no_answer: vec![],
        };

        // Don't answer the question
        let req = EnrichedRequirements::new("test".to_string()).with_clarifier_output(output);

        let prompt = req.to_planner_prompt();

        // Should show assumed default
        assert!(prompt.contains("## Assumed Defaults"));
        assert!(prompt.contains("What scope?"));
        assert!(prompt.contains("Full"));
    }

    #[test]
    fn test_enriched_requirements_to_planner_prompt_with_critic_feedback() {
        let mut req = EnrichedRequirements::new("test".to_string());
        req.critic_feedback = Some("Step 3 is vague".to_string());

        let prompt = req.to_planner_prompt();

        assert!(prompt.contains("## Previous Critic Feedback"));
        assert!(prompt.contains("Step 3 is vague"));
    }

    #[test]
    fn test_clarifier_output_empty() {
        let output = ClarifierOutput::empty("idea");

        assert_eq!(output.mode, "idea");
        assert!(output.questions.is_empty());
        assert!(output.analysis.understood_intent.is_empty());
    }
}
