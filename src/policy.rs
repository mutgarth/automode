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

pub const STARTER_POLICY: &str = r#"# Automode Policy

## Always approve
- File reads: ls, cat, find, grep, head, tail
- Git reads: git status, git log, git diff, git branch
- Build/check: cargo check, cargo build, npm install
- Database reads: SELECT, EXPLAIN queries

## Always reject
- Schema changes: DROP, ALTER, TRUNCATE
- Force operations: git push --force, git reset --hard
- Recursive deletes: rm -rf

## Use judgment for everything else
Consider context: is this a dev environment? Is the path sensitive?
When uncertain, reject and explain why.
"#;

const YOLO_POLICY: &str = r#"You are a last-resort safety guard for a developer's local machine.
The developer trusts their own workflow and wants maximum freedom — approve almost everything.
Your ONLY job is to block commands that are catastrophically irreversible or destructive at a system level.

## Approve everything EXCEPT:
- Wiping entire filesystems or home directories: rm -rf / rm -rf ~ rm -rf $HOME
- Dropping production databases by name (e.g. DROP DATABASE prod, DROP DATABASE production)
- Overwriting or deleting SSH keys, GPG keys, or credentials in ~/.ssh or ~/.gnupg
- Force-pushing to main/master on a remote (git push --force origin main)
- Commands that explicitly target /etc, /usr, /bin, /sbin, /System, /Library with write/delete ops

## Approve everything else, including:
- All read operations, builds, tests, installs
- Database queries of any kind on local/dev databases
- rm -rf on project directories, temp files, build artifacts
- git reset --hard, git push --force to non-main branches
- Any command that is reversible or scoped to a project directory

Respond ONLY with valid JSON: {"decision": "approve", "reason": "..."}
or {"decision": "reject", "reason": "..."}
"#;

const MILD_POLICY: &str = r#"You are a security policy enforcer for a developer's local machine.
Approve tool calls that are safe and common in a development workflow.
Reject tool calls that could cause data loss or irreversible system changes.

## Always approve
- Read-only filesystem ops: ls, cat, find, grep, head, tail, wc
- Git reads: git status, git log, git diff, git branch, git show
- Build and check: cargo check, cargo build, cargo test, npm install, npm run
- Database reads: any query starting with SELECT or EXPLAIN
- HTTP reads: curl GET requests

## Always reject
- Database schema changes: DROP, ALTER, TRUNCATE (any table or database)
- Force-push or hard-reset: git push --force, git reset --hard
- Recursive deletes of non-temp paths: rm -rf (unless path is clearly /tmp or build artifacts)

## Use judgment for everything else
Consider: is this reversible? Does it touch production data? Is the path a home or config directory?
When uncertain, reject and include a brief reason.

Respond ONLY with valid JSON: {"decision": "approve", "reason": "..."}
or {"decision": "reject", "reason": "..."}
"#;

const STRICT_POLICY: &str = r#"You are a strict security policy enforcer for a developer's local machine.
Only approve tool calls that are purely read-only. Reject everything that writes, modifies, or deletes.

## Always approve
- Listing and reading: ls, cat, find, grep, head, tail, wc, file
- Git read-only: git status, git log, git diff, git branch, git show, git fetch
- Cargo/npm checks only: cargo check, cargo clippy, npm ls
- Database reads only: SELECT, EXPLAIN (no INSERT, UPDATE, DELETE)

## Reject everything else
Any write, create, delete, install, push, or schema-changing operation must be rejected.
This includes: rm, mv, cp (to new locations), git commit, git push, npm install, cargo build.

Respond ONLY with valid JSON: {"decision": "approve", "reason": "..."}
or {"decision": "reject", "reason": "..."}
"#;

/// Returns the LLM system prompt for the given mode.
/// For Custom mode, custom_text must be Some(&str).
/// Returns String so all branches own their data.
pub fn policy_text(mode: &Mode, custom_text: Option<&str>) -> String {
    match mode {
        Mode::Yolo => YOLO_POLICY.to_string(),
        Mode::Mild => MILD_POLICY.to_string(),
        Mode::Strict => STRICT_POLICY.to_string(),
        Mode::Custom => custom_text.unwrap_or(STARTER_POLICY).to_string(),
    }
}
