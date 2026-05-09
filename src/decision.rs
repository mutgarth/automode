use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::io::Write;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DecisionKind {
    Approve,
    Reject,
}

impl std::fmt::Display for DecisionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecisionKind::Approve => write!(f, "APPROVE"),
            DecisionKind::Reject => write!(f, "REJECT"),
        }
    }
}

/// The JSON structure the LLM must return.
#[derive(Debug, Deserialize, Serialize)]
pub struct LlmDecision {
    pub decision: DecisionKind,
    pub reason: String,
}

/// The JSON structure written to stdout by hook.sh (read by Claude Code).
#[derive(Debug, Serialize)]
pub struct HookResponse {
    pub decision: DecisionKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl From<&LlmDecision> for HookResponse {
    fn from(d: &LlmDecision) -> Self {
        HookResponse {
            decision: d.decision.clone(),
            reason: if d.decision == DecisionKind::Reject {
                Some(d.reason.clone())
            } else {
                None
            },
        }
    }
}

/// One line in decisions.log.
pub struct LogEntry {
    pub timestamp: String,
    pub tool: String,
    pub command: String,
    pub decision: DecisionKind,
    pub reason: String,
}

impl LogEntry {
    pub fn to_log_line(&self) -> String {
        format!(
            "[{}] {} | {} | {} | {}",
            self.timestamp, self.decision, self.tool, self.command, self.reason
        )
    }
}

/// Append a decision to ~/.automode/logs/decisions.log
pub fn append_log(tool: &str, command: &str, d: &LlmDecision) -> Result<()> {
    let log_path = crate::config::log_path();
    std::fs::create_dir_all(log_path.parent().unwrap())?;
    let entry = LogEntry {
        timestamp: Utc::now().to_rfc3339(),
        tool: tool.to_string(),
        command: command.to_string(),
        decision: d.decision.clone(),
        reason: d.reason.clone(),
    };
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    writeln!(file, "{}", entry.to_log_line())?;
    Ok(())
}

/// The JSON body from Claude Code's PreToolUse hook.
/// Claude Code sends `tool_name` and `tool_input` (not `tool` and `input`).
#[derive(Debug, Deserialize)]
pub struct ToolCall {
    #[serde(rename = "tool_name", alias = "tool")]
    pub tool: String,
    #[serde(rename = "tool_input", alias = "input")]
    pub input: serde_json::Value,
}

impl ToolCall {
    /// Extract a human-readable command string for logging.
    pub fn command_str(&self) -> String {
        if let Some(cmd) = self.input.get("command").and_then(|v| v.as_str()) {
            cmd.to_string()
        } else {
            self.input.to_string()
        }
    }
}
