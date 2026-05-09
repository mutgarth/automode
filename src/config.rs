use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::policy::Mode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub mode: Mode,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            mode: Mode::Mild,
        }
    }
}

fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("automode")
}

pub fn pid_path() -> PathBuf {
    config_dir().join("automode.pid")
}

pub fn log_path() -> PathBuf {
    config_dir().join("decisions.log")
}

pub fn policy_path() -> PathBuf {
    config_dir().join("policy.toml")
}

fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn load() -> Result<Config> {
    let path = config_path();
    if !path.exists() {
        return Ok(Config::default());
    }
    let content = std::fs::read_to_string(&path)?;
    let cfg: Config = toml::from_str(&content)?;
    Ok(cfg)
}

pub fn save(cfg: &Config) -> Result<()> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)?;
    let content = toml::to_string(cfg)?;
    std::fs::write(config_path(), content)?;
    Ok(())
}
