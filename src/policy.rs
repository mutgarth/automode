use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

pub const STARTER_POLICY: &str = r#"# automode custom policy
# Define rules to approve or deny tool calls.

[[rules]]
tool = "Bash"
action = "approve"
comment = "Allow all bash commands"
"#;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Yolo,
    Mild,
    Strict,
    Custom,
}

impl Mode {
    pub fn from_str(s: &str) -> Result<Mode> {
        match s.to_lowercase().as_str() {
            "yolo" => Ok(Mode::Yolo),
            "mild" => Ok(Mode::Mild),
            "strict" => Ok(Mode::Strict),
            "custom" => Ok(Mode::Custom),
            other => Err(anyhow!("Unknown mode: {}. Valid modes: yolo, mild, strict, custom", other)),
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
