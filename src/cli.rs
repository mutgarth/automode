use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "automode", about = "Claude Code auto-approval daemon")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Interactive onboarding — installs the Claude Code hook and selects a mode
    Setup,
    /// Start the automode daemon and llama.cpp server in the background
    Start,
    /// Stop the running daemon
    Stop,
    /// Show mode, uptime, total decisions, and last 5 log entries
    Status,
    /// Switch operating mode: yolo | mild | strict | custom
    Mode { name: String },
    /// Tail the decisions log
    Logs,
    /// Local dev setup: self-install binary, download llama-server + model, then run setup
    Dev,
    /// Internal: run the HTTP server (do not invoke directly)
    #[command(hide = true)]
    Serve,
}
