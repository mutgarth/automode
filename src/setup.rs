use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Select};
use serde_json::Value;
use tracing::info;

use crate::config::{self, hook_path, Config};
use crate::policy::{Mode, STARTER_POLICY};

const MODES: &[(&str, &str)] = &[
    ("yolo",   "approve everything, no questions asked"),
    ("mild",   "approves reads/queries, blocks destructive ops"),
    ("strict", "approves only read-only operations"),
    ("custom", "write your own policy in policy.md"),
];

pub async fn run() -> Result<()> {
    println!("\nWelcome to automode");
    println!("───────────────────────────────────────");

    let labels: Vec<String> = MODES
        .iter()
        .map(|(name, desc)| format!("{:<8} — {}", name, desc))
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a mode")
        .items(&labels)
        .default(1) // mild
        .interact()?;

    let (mode_name, _) = MODES[selection];
    let mode = Mode::from_str(mode_name)?;

    // Create directory structure
    let base = config::automode_dir();
    std::fs::create_dir_all(&base)?;
    std::fs::create_dir_all(base.join("logs"))?;
    std::fs::create_dir_all(base.join("models"))?;

    // Save config
    let mut cfg = Config::default();
    cfg.mode = mode.clone();
    config::save(&cfg)?;

    // Create custom policy template if needed
    if mode == Mode::Custom {
        let policy_path = config::policy_path();
        if !policy_path.exists() {
            std::fs::write(&policy_path, STARTER_POLICY)?;
        }
    }

    // Install hook.sh (embedded at compile time)
    install_hook()?;

    // Patch Claude Code settings.json
    patch_claude_settings()?;

    println!("───────────────────────────────────────");
    println!("✓ Hook installed in ~/.claude/settings.json");
    println!("✓ Service configured. Run `automode start` to begin.");
    Ok(())
}

fn install_hook() -> Result<()> {
    let hook_src = include_str!("../scripts/hook.sh");
    let dest = hook_path();
    std::fs::write(&dest, hook_src)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))?;
    }
    info!("hook.sh installed to {}", dest.display());
    Ok(())
}

fn patch_claude_settings() -> Result<()> {
    let settings_path = dirs::home_dir()
        .unwrap()
        .join(".claude/settings.json");

    let hook_path_str = hook_path().to_string_lossy().to_string();

    let hook_entry = serde_json::json!({
        "matcher": ".*",
        "hooks": [{"type": "command", "command": hook_path_str}]
    });

    let mut settings: Value = if settings_path.exists() {
        let content = std::fs::read_to_string(&settings_path)?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Ensure hooks.PreToolUse array exists
    if settings.get("hooks").is_none() {
        settings["hooks"] = serde_json::json!({});
    }
    if settings["hooks"].get("PreToolUse").is_none() {
        settings["hooks"]["PreToolUse"] = serde_json::json!([]);
    }

    let pre_tool_use = settings["hooks"]["PreToolUse"]
        .as_array_mut()
        .unwrap();

    // Avoid duplicate entries
    let already_present = pre_tool_use.iter().any(|e| {
        e["hooks"]
            .as_array()
            .and_then(|h| h.first())
            .and_then(|h| h["command"].as_str())
            .map(|c| c.contains("automode"))
            .unwrap_or(false)
    });

    if !already_present {
        pre_tool_use.push(hook_entry);
    }

    std::fs::create_dir_all(settings_path.parent().unwrap())?;
    std::fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)?;
    Ok(())
}
