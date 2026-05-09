use automode::decision::{DecisionKind, HookResponse, LlmDecision, LogEntry};

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
    let r = HookResponse { decision: DecisionKind::Approve, reason: None };
    let json = serde_json::to_string(&r).unwrap();
    assert_eq!(json, r#"{"decision":"approve"}"#);
}

#[test]
fn test_hook_response_reject_includes_reason() {
    let r = HookResponse {
        decision: DecisionKind::Reject,
        reason: Some("rm -rf is dangerous".to_string()),
    };
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains("reject"));
    assert!(json.contains("rm -rf is dangerous"));
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
    let messages = build_chat_messages("Approve safe ops.", r#"{"tool":"Bash","input":{"command":"ls"}}"#);
    assert_eq!(messages[0]["role"], "system");
    assert!(messages[0]["content"].as_str().unwrap().contains("Approve safe ops."));
    assert_eq!(messages[1]["role"], "user");
    assert!(messages[1]["content"].as_str().unwrap().contains("ls"));
}
