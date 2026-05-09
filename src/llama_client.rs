use anyhow::{anyhow, Result};
use serde_json::{json, Value};

pub fn build_chat_messages(policy: &str, tool_call_json: &str) -> Vec<Value> {
    vec![
        json!({"role": "system", "content": policy}),
        json!({"role": "user", "content": format!(
            "Evaluate this tool call and respond with JSON only:\n\n{}",
            tool_call_json
        )}),
    ]
}

/// Calls llama.cpp and returns the raw JSON string from the LLM content field.
/// Callers use parse_llm_json (in server.rs) to turn this into a LlmDecision.
pub async fn ask_llm(
    llama_port: u16,
    policy: &str,
    tool_call_json: &str,
) -> Result<String> {
    let client = reqwest::Client::new();
    let messages = build_chat_messages(policy, tool_call_json);

    // Strict JSON schema — llama.cpp converts this to a grammar that constrains
    // every token, so the model literally cannot produce malformed output.
    let body = json!({
        "model": "default",
        "messages": messages,
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "decision",
                "strict": true,
                "schema": {
                    "type": "object",
                    "properties": {
                        "decision": {"type": "string", "enum": ["approve", "reject"]},
                        "reason":   {"type": "string"}
                    },
                    "required": ["decision", "reason"],
                    "additionalProperties": false
                }
            }
        },
        "max_tokens": 256,
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

    json["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("unexpected llama.cpp response shape: {}", json))
}
