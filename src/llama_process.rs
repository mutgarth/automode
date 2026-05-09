use anyhow::{anyhow, Result};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

fn port_in_use(port: u16) -> bool {
    TcpStream::connect_timeout(
        &format!("127.0.0.1:{}", port).parse().unwrap(),
        Duration::from_millis(200),
    ).is_ok()
}

pub struct LlamaProcess {
    child: Option<Child>,
    bin: String,
    model: String,
    port: u16,
}

impl LlamaProcess {
    pub fn new(bin: &str, model: &str, port: u16) -> Self {
        Self {
            child: None,
            bin: bin.to_string(),
            model: model.to_string(),
            port,
        }
    }

    pub fn start(&mut self) -> Result<()> {
        if self.child.is_some() {
            return Ok(());
        }

        // If something is already listening on the port, assume it's a healthy
        // llama-server (e.g., orphaned from a previous run) and adopt it
        // instead of spawning a duplicate.
        if port_in_use(self.port) {
            info!("Port {} already in use — adopting existing llama-server", self.port);
            return Ok(());
        }

        info!("Starting llama-server on port {}", self.port);
        let child = Command::new(&self.bin)
            .args([
                "--model", &self.model,
                "--port", &self.port.to_string(),
                // Edit tool calls can include multi-KB old_string/new_string.
                // 8K ctx fits even very large diffs comfortably.
                "--ctx-size", "8192",
                "--n-predict", "256",
                "--log-disable",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            // Pipe stderr to a log file so we can debug LLM errors after the fact.
            .stderr(
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(format!(
                        "{}/logs/llama-server.log",
                        dirs::home_dir().unwrap().join(".automode").display()
                    ))
                    .map(Stdio::from)
                    .unwrap_or(Stdio::null()),
            )
            .spawn()
            .map_err(|e| anyhow!("failed to start llama-server '{}': {}", self.bin, e))?;
        self.child = Some(child);
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            info!("Stopping llama-server");
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    /// Check if the llama-server is responsive; restart up to 3 times if not.
    /// Uses port-in-use as the liveness signal so adopted processes work.
    pub async fn ensure_alive(&mut self) -> Result<()> {
        for attempt in 1..=3 {
            // Reap any exited child first
            if let Some(child) = self.child.as_mut() {
                if let Ok(Some(status)) = child.try_wait() {
                    warn!("llama-server exited ({}), attempt {}/3", status, attempt);
                    self.child = None;
                }
            }

            if port_in_use(self.port) {
                return Ok(());
            }

            self.start()?;
            sleep(Duration::from_secs(2)).await;
        }
        Err(anyhow!("llama-server failed to stay alive after 3 restart attempts"))
    }
}

impl Drop for LlamaProcess {
    fn drop(&mut self) {
        self.stop();
    }
}
