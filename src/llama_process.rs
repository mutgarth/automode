use anyhow::{anyhow, Result};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

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
        info!("Starting llama-server on port {}", self.port);
        let child = Command::new(&self.bin)
            .args([
                "--model", &self.model,
                "--port", &self.port.to_string(),
                "--ctx-size", "512",
                "--n-predict", "128",
                "--log-disable",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
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

    /// Check if the process is still alive; restart up to 3 times if not.
    pub async fn ensure_alive(&mut self) -> Result<()> {
        for attempt in 1..=3 {
            match &mut self.child {
                None => {
                    self.start()?;
                    sleep(Duration::from_secs(2)).await;
                    return Ok(());
                }
                Some(child) => {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            warn!("llama-server exited ({}), restarting attempt {}/3", status, attempt);
                            self.child = None;
                            self.start()?;
                            sleep(Duration::from_secs(2)).await;
                        }
                        Ok(None) => return Ok(()), // still running
                        Err(e) => {
                            error!("could not check llama-server status: {}", e);
                            return Err(anyhow!("llama-server status check failed: {}", e));
                        }
                    }
                }
            }
        }
        Err(anyhow!("llama-server failed to stay alive after 3 restart attempts"))
    }
}

impl Drop for LlamaProcess {
    fn drop(&mut self) {
        self.stop();
    }
}
