use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Select};
use serde_json::Value;
use tracing::info;

use crate::config::{self, antigravity_hook_path, codex_hook_path, hook_path, Config, Target};
use crate::policy::{Mode, STARTER_POLICY};

const MODES: &[(&str, &str)] = &[
    ("yolo", "approve everything, no questions asked"),
    ("mild", "approves reads/queries, blocks destructive ops"),
    ("strict", "approves only read-only operations"),
    ("custom", "write your own policy in policy.md"),
];

pub async fn run(target: &str) -> Result<()> {
    let target = Target::from_str(target)?;

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
    cfg.target = target.clone();
    cfg.mode = mode.clone();
    config::save(&cfg)?;

    // Create custom policy template if needed
    if mode == Mode::Custom {
        let policy_path = config::policy_path();
        if !policy_path.exists() {
            std::fs::write(&policy_path, STARTER_POLICY)?;
        }
    }

    match target {
        Target::Claude => {
            install_claude_hook()?;
            patch_claude_settings()?;
            println!("───────────────────────────────────────");
            println!("✓ Hook installed in ~/.claude/settings.json");
        }
        Target::Codex => {
            install_codex_hook()?;
            patch_codex_hooks()?;
            println!("───────────────────────────────────────");
            println!("✓ Hook installed in ~/.codex/hooks.json");
        }
        Target::Antigravity => {
            install_antigravity_hook()?;
            patch_antigravity_hooks()?;
            println!("───────────────────────────────────────");
            println!("✓ Hook installed in ~/.gemini/config/hooks.json");
        }
        Target::Both => {
            install_claude_hook()?;
            patch_claude_settings()?;
            install_codex_hook()?;
            patch_codex_hooks()?;
            println!("───────────────────────────────────────");
            println!("✓ Hook installed in ~/.claude/settings.json");
            println!("✓ Hook installed in ~/.codex/hooks.json");
        }
        Target::All => {
            install_claude_hook()?;
            patch_claude_settings()?;
            install_codex_hook()?;
            patch_codex_hooks()?;
            install_antigravity_hook()?;
            patch_antigravity_hooks()?;
            println!("───────────────────────────────────────");
            println!("✓ Hook installed in ~/.claude/settings.json");
            println!("✓ Hook installed in ~/.codex/hooks.json");
            println!("✓ Hook installed in ~/.gemini/config/hooks.json");
        }
    }

    println!("✓ Service configured. Run `automode start` to begin.");
    Ok(())
}

fn install_claude_hook() -> Result<()> {
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

fn install_codex_hook() -> Result<()> {
    let hook_src = include_str!("../scripts/codex-hook.sh");
    let dest = codex_hook_path();
    std::fs::write(&dest, hook_src)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))?;
    }
    info!("codex-hook.sh installed to {}", dest.display());
    Ok(())
}

fn install_antigravity_hook() -> Result<()> {
    let hook_src = include_str!("../scripts/antigravity-hook.sh");
    let dest = antigravity_hook_path();
    std::fs::write(&dest, hook_src)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))?;
    }
    info!("antigravity-hook.sh installed to {}", dest.display());
    Ok(())
}

fn patch_claude_settings() -> Result<()> {
    let settings_path = dirs::home_dir().unwrap().join(".claude/settings.json");

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

    let pre_tool_use = settings["hooks"]["PreToolUse"].as_array_mut().unwrap();

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

fn patch_codex_hooks() -> Result<()> {
    let hooks_path = dirs::home_dir().unwrap().join(".codex/hooks.json");

    let hook_path_str = codex_hook_path().to_string_lossy().to_string();

    let hook_entry = serde_json::json!({
        "matcher": ".*",
        "hooks": [{
            "type": "command",
            "command": hook_path_str,
            "timeout": 30,
            "statusMessage": "automode deciding"
        }]
    });

    let mut hooks_file: Value = if hooks_path.exists() {
        let content = std::fs::read_to_string(&hooks_path)?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if hooks_file.get("hooks").is_none() {
        hooks_file["hooks"] = serde_json::json!({});
    }
    if hooks_file["hooks"].get("PermissionRequest").is_none() {
        hooks_file["hooks"]["PermissionRequest"] = serde_json::json!([]);
    }

    let permission_request = hooks_file["hooks"]["PermissionRequest"]
        .as_array_mut()
        .unwrap();

    let already_present = permission_request.iter().any(|e| {
        e["hooks"]
            .as_array()
            .map(|hooks| {
                hooks.iter().any(|h| {
                    h["command"]
                        .as_str()
                        .map(|c| c.contains("automode") && c.contains("codex-hook"))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    });

    if !already_present {
        permission_request.push(hook_entry);
    }

    std::fs::create_dir_all(hooks_path.parent().unwrap())?;
    std::fs::write(&hooks_path, serde_json::to_string_pretty(&hooks_file)?)?;
    Ok(())
}

fn patch_antigravity_hooks() -> Result<()> {
    let hooks_path = dirs::home_dir()
        .unwrap()
        .join(".gemini/config/hooks.json");

    let hook_path_str = antigravity_hook_path().to_string_lossy().to_string();

    let hook_entry = serde_json::json!({
        "matcher": "run_command|write_to_file|replace_file_content|multi_replace_file_content|list_dir|view_file",
        "hooks": [{
            "type": "command",
            "command": hook_path_str,
            "timeout": 30
        }]
    });

    let mut hooks_file: Value = if hooks_path.exists() {
        let content = std::fs::read_to_string(&hooks_path)?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if hooks_file.get("automode").is_none() {
        hooks_file["automode"] = serde_json::json!({});
    }
    if hooks_file["automode"].get("PreToolUse").is_none() {
        hooks_file["automode"]["PreToolUse"] = serde_json::json!([]);
    }

    let pre_tool_use = hooks_file["automode"]["PreToolUse"]
        .as_array_mut()
        .unwrap();

    let already_present = pre_tool_use.iter().any(|e| {
        e["hooks"]
            .as_array()
            .map(|hooks| {
                hooks.iter().any(|h| {
                    h["command"]
                        .as_str()
                        .map(|c| c.contains("automode") && c.contains("antigravity-hook"))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    });

    if !already_present {
        pre_tool_use.push(hook_entry);
    }

    std::fs::create_dir_all(hooks_path.parent().unwrap())?;
    std::fs::write(&hooks_path, serde_json::to_string_pretty(&hooks_file)?)?;
    Ok(())
}
