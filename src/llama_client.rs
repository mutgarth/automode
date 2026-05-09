use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use crate::decision::LlmDecision;

pub fn build_chat_messages(policy: &str, tool_call_json: &str) -> Vec<Value> {
    vec![
        json!({"role": "system", "content": policy}),
        json!({"role": "user", "content": format!(
            "Evaluate this tool call and respond with JSON only:\n\n{}",
            tool_call_json
        )}),
    ]
}

pub async fn ask_llm(
    llama_port: u16,
    policy: &str,
    tool_call_json: &str,
) -> Result<LlmDecision> {
    let client = reqwest::Client::new();
    let messages = build_chat_messages(policy, tool_call_json);

    let body = json!({
        "model": "default",
        "messages": messages,
        "response_format": {"type": "json_object"},
        "max_tokens": 120,
        "temperature": 0.0
    });

    let resp = client
        .post(format!("http://localhost:{}/v1/chat/completions", llama_port))
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| anyhow!("llama.cpp unreachable: {}", e))?;

    let json: Value = resp.json().await
        .map_err(|e| anyhow!("failed to parse llama.cpp response: {}", e))?;

    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow!("unexpected llama.cpp response shape: {}", json))?;

    serde_json::from_str::<LlmDecision>(content)
        .map_err(|e| anyhow!("LLM returned malformed JSON: {} — raw: {}", e, content))
}
