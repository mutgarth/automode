use automode::decision::DecisionKind;
use automode::server::{yolo_response, parse_llm_json};

#[test]
fn test_yolo_mode_returns_approve() {
    let response = yolo_response();
    assert_eq!(response.decision, DecisionKind::Approve);
}

#[test]
fn test_parse_llm_response_malformed_defaults_to_reject() {
    let result = parse_llm_json("not valid json at all");
    assert_eq!(result.decision, DecisionKind::Reject);
    assert!(result.reason.contains("malformed") || result.reason.contains("parse"));
}

#[test]
fn test_parse_llm_response_unknown_decision_defaults_to_reject() {
    let result = parse_llm_json(r#"{"decision":"maybe","reason":"dunno"}"#);
    assert_eq!(result.decision, DecisionKind::Reject);
}
