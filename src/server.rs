use anyhow::Result;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::config::{self, Config};
use crate::decision::{append_log, DecisionKind, HookResponse, LlmDecision, ToolCall};
use crate::llama_client::ask_llm;
use crate::llama_process::LlamaProcess;
use crate::policy::{policy_text, Mode};

pub struct AppState {
    pub config: Config,
    pub llama: Mutex<LlamaProcess>,
    pub custom_policy: Option<String>,
}

pub fn yolo_response() -> HookResponse {
    HookResponse { decision: DecisionKind::Approve, reason: None }
}

pub fn parse_llm_json(raw: &str) -> LlmDecision {
    match serde_json::from_str::<LlmDecision>(raw) {
        Ok(d) if d.decision == DecisionKind::Approve || d.decision == DecisionKind::Reject => d,
        Ok(_) => {
            warn!("LLM returned unknown decision value");
            LlmDecision {
                decision: DecisionKind::Reject,
                reason: "unknown decision value from LLM".to_string(),
            }
        }
        Err(e) => {
            warn!("LLM returned malformed JSON: {}", e);
            LlmDecision {
                decision: DecisionKind::Reject,
                reason: format!("malformed JSON from LLM: {}", e),
            }
        }
    }
}

async fn decide(
    State(state): State<Arc<AppState>>,
    body: String,
) -> Result<Json<HookResponse>, StatusCode> {
    // Yolo: skip LLM entirely
    if state.config.mode == Mode::Yolo {
        return Ok(Json(yolo_response()));
    }

    let tool_call: ToolCall = serde_json::from_str(&body).map_err(|e| {
        error!("failed to parse tool call JSON: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    let policy = match state.config.mode {
        Mode::Custom => policy_text(&Mode::Custom, state.custom_policy.as_deref()),
        ref m => policy_text(m, None),
    };

    // Ensure llama.cpp is alive
    {
        let mut llama = state.llama.lock().await;
        llama.ensure_alive().await.map_err(|e| {
            error!("llama-server unavailable: {}", e);
            StatusCode::SERVICE_UNAVAILABLE
        })?;
    }

    let tool_call_json = serde_json::to_string(&serde_json::json!({
        "tool": tool_call.tool,
        "input": tool_call.input
    }))
    .unwrap();

    let llm_decision = match ask_llm(state.config.llama_server_port, &policy, &tool_call_json).await {
        Ok(raw) => parse_llm_json(&raw),
        Err(e) => {
            error!("LLM error: {}", e);
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    };

    let command_str = tool_call.command_str();
    if let Err(e) = append_log(&tool_call.tool, &command_str, &llm_decision) {
        warn!("failed to write decision log: {}", e);
    }

    info!("{} | {} | {}", llm_decision.decision, tool_call.tool, command_str);
    Ok(Json(HookResponse::from(&llm_decision)))
}

pub async fn run() -> Result<()> {
    let cfg = config::load()?;
    let port = cfg.port;

    let custom_policy = if cfg.mode == Mode::Custom {
        let p = config::policy_path();
        if p.exists() {
            Some(std::fs::read_to_string(&p)?)
        } else {
            None
        }
    } else {
        None
    };

    let llama = LlamaProcess::new(&cfg.llama_server_bin, &cfg.model_path, cfg.llama_server_port);

    let state = Arc::new(AppState {
        config: cfg,
        llama: Mutex::new(llama),
        custom_policy,
    });

    // Start llama.cpp immediately (unless yolo)
    if state.config.mode != Mode::Yolo {
        let mut llama = state.llama.lock().await;
        if let Err(e) = llama.start() {
            error!("warning: could not start llama-server at startup: {}", e);
        }
        drop(llama);
        // give llama.cpp 2s to initialize
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    let app = Router::new()
        .route("/decide", post(decide))
        .with_state(state);

    let addr = format!("127.0.0.1:{}", port);
    info!("automode listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
