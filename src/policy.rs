use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Yolo,
    Mild,
    Strict,
    Custom,
}

impl Mode {
    pub fn from_str(s: &str) -> Result<Mode> {
        match s {
            "yolo" => Ok(Mode::Yolo),
            "mild" => Ok(Mode::Mild),
            "strict" => Ok(Mode::Strict),
            "custom" => Ok(Mode::Custom),
            other => Err(anyhow!("unknown mode '{}' — use: yolo, mild, strict, custom", other)),
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Mode::Yolo => write!(f, "yolo"),
            Mode::Mild => write!(f, "mild"),
            Mode::Strict => write!(f, "strict"),
            Mode::Custom => write!(f, "custom"),
        }
    }
}

pub const STARTER_POLICY: &str = "# Automode Policy\n\n## Always approve\n- File reads: ls, cat, find, grep, head, tail\n";
