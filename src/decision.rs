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

/// Inner struct for Claude Code's PreToolUse hook output format.
#[derive(Debug, Serialize)]
pub struct HookSpecificOutput {
    #[serde(rename = "hookEventName")]
    pub hook_event_name: &'static str,
    #[serde(rename = "permissionDecision")]
    pub permission_decision: &'static str,
    #[serde(
        rename = "permissionDecisionReason",
        skip_serializing_if = "Option::is_none"
    )]
    pub permission_decision_reason: Option<String>,
}

/// The JSON structure written to stdout by hook.sh (read by Claude Code).
/// Uses the official `hookSpecificOutput` shape required for PreToolUse hooks.
#[derive(Debug, Serialize)]
pub struct HookResponse {
    #[serde(rename = "hookSpecificOutput")]
    pub hook_specific_output: HookSpecificOutput,
    /// Mirror the decision at the top level for backward-compat readers (tests, curl).
    pub decision: DecisionKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl From<&LlmDecision> for HookResponse {
    fn from(d: &LlmDecision) -> Self {
        let permission_decision = match d.decision {
            DecisionKind::Approve => "allow",
            DecisionKind::Reject => "deny",
        };
        HookResponse {
            hook_specific_output: HookSpecificOutput {
                hook_event_name: "PreToolUse",
                permission_decision,
                permission_decision_reason: Some(d.reason.clone()),
            },
            decision: d.decision.clone(),
            reason: if d.decision == DecisionKind::Reject {
                Some(d.reason.clone())
            } else {
                None
            },
        }
    }
}

/// Inner struct for Codex's PermissionRequest hook output format.
#[derive(Debug, Serialize)]
pub struct CodexPermissionDecision {
    pub behavior: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CodexPermissionHookSpecificOutput {
    #[serde(rename = "hookEventName")]
    pub hook_event_name: &'static str,
    pub decision: CodexPermissionDecision,
}

/// The JSON structure written to stdout by codex-hook.sh.
/// Codex's hook parser is strict, so this intentionally omits Claude-only
/// top-level compatibility fields.
#[derive(Debug, Serialize)]
pub struct CodexPermissionResponse {
    #[serde(rename = "hookSpecificOutput")]
    pub hook_specific_output: CodexPermissionHookSpecificOutput,
}

impl From<&LlmDecision> for CodexPermissionResponse {
    fn from(d: &LlmDecision) -> Self {
        let behavior = match d.decision {
            DecisionKind::Approve => "allow",
            DecisionKind::Reject => "deny",
        };
        CodexPermissionResponse {
            hook_specific_output: CodexPermissionHookSpecificOutput {
                hook_event_name: "PermissionRequest",
                decision: CodexPermissionDecision {
                    behavior,
                    message: if d.decision == DecisionKind::Reject {
                        Some(d.reason.clone())
                    } else {
                        None
                    },
                },
            },
        }
    }
}

/// The JSON structure written to stdout by antigravity-hook.sh.
#[derive(Debug, Serialize)]
pub struct AntigravityHookResponse {
    pub decision: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl From<&LlmDecision> for AntigravityHookResponse {
    fn from(d: &LlmDecision) -> Self {
        let decision = match d.decision {
            DecisionKind::Approve => "allow",
            DecisionKind::Reject => "deny",
        };
        AntigravityHookResponse {
            decision,
            reason: Some(d.reason.clone()),
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
#[derive(Debug)]
pub struct ToolCall {
    pub hook_event_name: Option<String>,
    pub tool: String,
    pub input: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct AntigravityToolCallPayload {
    #[serde(rename = "toolCall")]
    tool_call: AntigravityToolCall,
}

#[derive(Debug, Deserialize)]
struct AntigravityToolCall {
    name: String,
    args: serde_json::Value,
}

impl<'de> Deserialize<'de> for ToolCall {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;

        if value.get("toolCall").is_some() {
            let payload = AntigravityToolCallPayload::deserialize(value)
                .map_err(serde::de::Error::custom)?;
            return Ok(ToolCall {
                hook_event_name: Some("AntigravityPreToolUse".to_string()),
                tool: payload.tool_call.name,
                input: payload.tool_call.args,
            });
        }

        #[derive(Deserialize)]
        struct StandardToolCall {
            #[serde(default)]
            hook_event_name: Option<String>,
            #[serde(rename = "tool_name", alias = "tool")]
            tool: String,
            #[serde(rename = "tool_input", alias = "input")]
            input: serde_json::Value,
        }

        let standard = StandardToolCall::deserialize(value).map_err(serde::de::Error::custom)?;
        Ok(ToolCall {
            hook_event_name: standard.hook_event_name,
            tool: standard.tool,
            input: standard.input,
        })
    }
}

impl ToolCall {
    /// Extract a human-readable command string for logging.
    pub fn command_str(&self) -> String {
        if let Some(cmd) = self.input.get("command").and_then(|v| v.as_str()) {
            cmd.to_string()
        } else if let Some(cmd) = self.input.get("CommandLine").and_then(|v| v.as_str()) {
            cmd.to_string()
        } else {
            self.input.to_string()
        }
    }
}
