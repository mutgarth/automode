use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::policy::Mode;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub port: u16,
    pub mode: Mode,
    pub model_path: String,
    pub llama_server_bin: String,
    pub llama_server_port: u16,
    pub log_level: String,
}

impl Default for Config {
    fn default() -> Self {
        let base = automode_dir();
        Self {
            port: 7878,
            mode: Mode::Mild,
            model_path: base.join("models/bonsai.gguf").to_string_lossy().into(),
            llama_server_bin: base.join("llama-server").to_string_lossy().into(),
            llama_server_port: 8080,
            log_level: "info".to_string(),
        }
    }
}

pub fn automode_dir() -> PathBuf {
    dirs::home_dir()
        .expect("could not find home directory")
        .join(".automode")
}

pub fn config_path() -> PathBuf {
    automode_dir().join("config.toml")
}

pub fn pid_path() -> PathBuf {
    automode_dir().join("automode.pid")
}

pub fn log_path() -> PathBuf {
    automode_dir().join("logs/decisions.log")
}

pub fn policy_path() -> PathBuf {
    automode_dir().join("policy.md")
}

pub fn hook_path() -> PathBuf {
    automode_dir().join("hook.sh")
}

pub fn load() -> Result<Config> {
    let path = config_path();
    if !path.exists() {
        return Ok(Config::default());
    }
    let s = std::fs::read_to_string(&path)?;
    load_from_str(&s)
}

pub fn save(cfg: &Config) -> Result<()> {
    let path = config_path();
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, save_to_string(cfg)?)?;
    Ok(())
}

pub fn load_from_str(s: &str) -> Result<Config> {
    Ok(toml::from_str(s)?)
}

pub fn save_to_string(cfg: &Config) -> Result<String> {
    Ok(toml::to_string_pretty(cfg)?)
}
