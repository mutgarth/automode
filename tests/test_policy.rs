use automode::policy::{Mode, policy_text};

#[test]
fn test_mode_from_str_valid() {
    assert_eq!(Mode::from_str("yolo").unwrap(), Mode::Yolo);
    assert_eq!(Mode::from_str("mild").unwrap(), Mode::Mild);
    assert_eq!(Mode::from_str("strict").unwrap(), Mode::Strict);
    assert_eq!(Mode::from_str("custom").unwrap(), Mode::Custom);
}

#[test]
fn test_mode_from_str_invalid() {
    assert!(Mode::from_str("unknown").is_err());
}

#[test]
fn test_policy_text_mild_contains_select() {
    let text = policy_text(&Mode::Mild, None);
    assert!(text.contains("SELECT"));
}

#[test]
fn test_policy_text_strict_contains_read_only() {
    let text = policy_text(&Mode::Strict, None);
    assert!(text.contains("read-only"));
}

#[test]
fn test_policy_text_custom_uses_provided_text() {
    let custom_text = "Only allow ls commands.";
    let text = policy_text(&Mode::Custom, Some(custom_text));
    assert_eq!(text.as_str(), custom_text);
}
