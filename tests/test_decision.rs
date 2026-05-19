use automode::decision::{
    AntigravityHookResponse, CodexPermissionResponse, DecisionKind, HookResponse, LlmDecision,
    LogEntry, ToolCall,
};

fn approve_response(reason: &str) -> HookResponse {
    HookResponse::from(&LlmDecision {
        decision: DecisionKind::Approve,
        reason: reason.to_string(),
    })
}

fn reject_response(reason: &str) -> HookResponse {
    HookResponse::from(&LlmDecision {
        decision: DecisionKind::Reject,
        reason: reason.to_string(),
    })
}

fn codex_response(decision: DecisionKind, reason: &str) -> CodexPermissionResponse {
    CodexPermissionResponse::from(&LlmDecision {
        decision,
        reason: reason.to_string(),
    })
}

fn antigravity_response(decision: DecisionKind, reason: &str) -> AntigravityHookResponse {
    AntigravityHookResponse::from(&LlmDecision {
        decision,
        reason: reason.to_string(),
    })
}

#[test]
fn test_llm_decision_deserialize_approve() {
    let json = r#"{"decision":"approve","reason":"Read-only SELECT"}"#;
    let d: LlmDecision = serde_json::from_str(json).unwrap();
    assert_eq!(d.decision, DecisionKind::Approve);
    assert_eq!(d.reason, "Read-only SELECT");
}

#[test]
fn test_llm_decision_deserialize_reject() {
    let json = r#"{"decision":"reject","reason":"DROP TABLE is destructive"}"#;
    let d: LlmDecision = serde_json::from_str(json).unwrap();
    assert_eq!(d.decision, DecisionKind::Reject);
}

#[test]
fn test_hook_response_approve_serializes_correctly() {
    let r = approve_response("safe");
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains(r#""permissionDecision":"allow""#));
    assert!(json.contains(r#""hookEventName":"PreToolUse""#));
}

#[test]
fn test_hook_response_reject_includes_reason() {
    let r = reject_response("rm -rf is dangerous");
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains(r#""permissionDecision":"deny""#));
    assert!(json.contains("rm -rf is dangerous"));
}

#[test]
fn test_codex_permission_response_approve_serializes_correctly() {
    let r = codex_response(DecisionKind::Approve, "safe");
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains(r#""hookEventName":"PermissionRequest""#));
    assert!(json.contains(r#""behavior":"allow""#));
    assert!(!json.contains(r#""decision":"approve""#));
}

#[test]
fn test_codex_permission_response_reject_includes_message() {
    let r = codex_response(DecisionKind::Reject, "rm -rf is dangerous");
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains(r#""behavior":"deny""#));
    assert!(json.contains(r#""message":"rm -rf is dangerous""#));
}

#[test]
fn test_antigravity_response_approve_serializes_correctly() {
    let r = antigravity_response(DecisionKind::Approve, "safe");
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains(r#""decision":"allow""#));
    assert!(json.contains(r#""reason":"safe""#));
}

#[test]
fn test_antigravity_response_reject_serializes_correctly() {
    let r = antigravity_response(DecisionKind::Reject, "dangerous");
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains(r#""decision":"deny""#));
    assert!(json.contains(r#""reason":"dangerous""#));
}

#[test]
fn test_antigravity_tool_call_deserializes() {
    let payload = r#"{
      "toolCall": {
        "name": "run_command",
        "args": {
          "CommandLine": "npm test",
          "Cwd": "/workspace/project"
        }
      },
      "stepIdx": 19
    }"#;
    let call: ToolCall = serde_json::from_str(payload).unwrap();
    assert_eq!(call.hook_event_name.as_deref(), Some("AntigravityPreToolUse"));
    assert_eq!(call.tool, "run_command");
    assert_eq!(call.command_str(), "npm test");
}

#[test]
fn test_log_entry_formats_correctly() {
    let entry = LogEntry {
        timestamp: "2026-05-09T10:00:00Z".to_string(),
        tool: "Bash".to_string(),
        command: "ls -la".to_string(),
        decision: DecisionKind::Approve,
        reason: "Read-only file listing".to_string(),
    };
    let line = entry.to_log_line();
    assert!(line.contains("APPROVE"));
    assert!(line.contains("Bash"));
    assert!(line.contains("ls -la"));
}

use automode::llama_client::build_chat_messages;

#[test]
fn test_build_chat_messages_includes_policy_and_tool_call() {
    let messages = build_chat_messages(
        "Approve safe ops.",
        r#"{"tool":"Bash","input":{"command":"ls"}}"#,
    );
    assert_eq!(messages[0]["role"], "system");
    assert!(messages[0]["content"]
        .as_str()
        .unwrap()
        .contains("Approve safe ops."));
    assert_eq!(messages[1]["role"], "user");
    assert!(messages[1]["content"].as_str().unwrap().contains("ls"));
}
