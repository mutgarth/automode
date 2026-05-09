mod cli;
mod config;
mod decision;
mod llama_client;
mod llama_process;
mod policy;
mod server;
mod setup;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Setup => setup::run().await,
        Commands::Start => start_daemon().await,
        Commands::Stop => stop_daemon().await,
        Commands::Status => print_status().await,
        Commands::Mode { name } => switch_mode(&name).await,
        Commands::Logs => tail_logs().await,
        Commands::Serve => server::run().await,
    }
}

async fn start_daemon() -> Result<()> {
    use std::process::{Command, Stdio};
    let exe = std::env::current_exe()?;
    let child = Command::new(exe)
        .arg("serve")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    std::fs::write(config::pid_path(), child.id().to_string())?;
    println!("automode started (pid {})", child.id());
    Ok(())
}

async fn stop_daemon() -> Result<()> {
    let pid_path = config::pid_path();
    let pid_str = std::fs::read_to_string(&pid_path)
        .map_err(|_| anyhow::anyhow!("automode is not running (no PID file)"))?;
    let pid: u32 = pid_str.trim().parse()?;
    #[cfg(unix)]
    unsafe { libc::kill(pid as i32, libc::SIGTERM); }
    std::fs::remove_file(&pid_path).ok();
    println!("automode stopped (pid {})", pid);
    Ok(())
}

async fn print_status() -> Result<()> {
    let cfg = config::load()?;
    let pid_path = config::pid_path();
    let running = std::fs::read_to_string(&pid_path).ok()
        .map(|p| format!("running (pid {})", p.trim()))
        .unwrap_or_else(|| "stopped".to_string());
    println!("status : {}", running);
    println!("mode   : {}", cfg.mode);
    tail_logs().await
}

async fn switch_mode(name: &str) -> Result<()> {
    let mode = policy::Mode::from_str(name)?;
    let mut cfg = config::load()?;
    cfg.mode = mode.clone();
    config::save(&cfg)?;

    if mode == policy::Mode::Custom {
        let policy_path = config::policy_path();
        if !policy_path.exists() {
            std::fs::write(&policy_path, policy::STARTER_POLICY)?;
            println!("Created {}", policy_path.display());
        }
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
        std::process::Command::new(editor).arg(&policy_path).status()?;
    }
    println!("Mode set to: {}", name);
    Ok(())
}

async fn tail_logs() -> Result<()> {
    let log_path = config::log_path();
    if !log_path.exists() {
        println!("No decisions logged yet.");
        return Ok(());
    }
    let content = std::fs::read_to_string(&log_path)?;
    let lines: Vec<&str> = content.lines().collect();
    let start = lines.len().saturating_sub(5);
    for line in &lines[start..] {
        println!("{}", line);
    }
    Ok(())
}
