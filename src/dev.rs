use anyhow::{anyhow, Result};
use std::process::Command;

use crate::config;
use crate::setup;

const MODEL_URL: &str =
    "https://huggingface.co/prism-ml/Bonsai-8B-gguf/resolve/main/Bonsai-8B-Q1_0.gguf";

const LLAMA_API: &str =
    "https://api.github.com/repos/ggerganov/llama.cpp/releases/latest";

const LLAMA_REPO: &str = "https://github.com/ggerganov/llama.cpp";

pub async fn run() -> Result<()> {
    println!("\nautomode dev setup");
    println!("──────────────────────────────────────");

    // 1. Create directory structure
    let base = config::automode_dir();
    std::fs::create_dir_all(&base)?;
    std::fs::create_dir_all(base.join("models"))?;
    std::fs::create_dir_all(base.join("logs"))?;

    // 2. Self-install: copy this running binary to ~/.automode/automode
    let self_path = std::env::current_exe()?;
    let dest = base.join("automode");
    if self_path != dest {
        std::fs::copy(&self_path, &dest)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))?;
        }
        println!("✓ Binary installed → {}", dest.display());
    } else {
        println!("✓ Binary already in place.");
    }

    // 3. Download llama-server if missing
    let llama_dest = base.join("llama-server");
    if llama_dest.exists() {
        println!("✓ llama-server already present, skipping.");
    } else {
        download_llama_server(&llama_dest).await?;
    }

    // 4. Download model if missing
    let model_dest = base.join("models/bonsai.gguf");
    if model_dest.exists() {
        println!("✓ Model already present, skipping.");
    } else {
        download_model(&model_dest)?;
    }

    println!("──────────────────────────────────────");
    println!("Running setup...\n");

    // 5. Run interactive setup
    setup::run().await
}

async fn download_llama_server(dest: &std::path::Path) -> Result<()> {
    let (os, arch) = detect_platform()?;
    let llama_platform = llama_platform_string(&os, &arch)?;

    println!("→ Fetching latest llama.cpp release tag...");
    let tag = fetch_llama_tag().await?;
    println!("→ Downloading llama-server {} for {}...", tag, llama_platform);

    let zip_url = format!(
        "{}/releases/download/{}/llama-{}-bin-{}.zip",
        LLAMA_REPO, tag, tag, llama_platform
    );

    let tmp = dest.parent().unwrap().join("llama-server.zip");

    curl_download(&zip_url, &tmp, true)?;

    // Extract llama-server from the zip
    let status = Command::new("unzip")
        .args(["-j", tmp.to_str().unwrap(), "*/llama-server", "-d", dest.parent().unwrap().to_str().unwrap()])
        .status()?;

    if !status.success() {
        // Try without the glob prefix (some releases have a flat structure)
        Command::new("unzip")
            .args(["-j", tmp.to_str().unwrap(), "llama-server", "-d", dest.parent().unwrap().to_str().unwrap()])
            .status()?;
    }

    std::fs::remove_file(&tmp).ok();

    if !dest.exists() {
        return Err(anyhow!(
            "llama-server not found in zip — check the release at {}",
            zip_url
        ));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dest, std::fs::Permissions::from_mode(0o755))?;
    }

    println!("✓ llama-server installed → {}", dest.display());
    Ok(())
}

fn download_model(dest: &std::path::Path) -> Result<()> {
    println!("→ Downloading Bonsai-8B-Q1_0.gguf (~1.16 GB)...");
    curl_download(MODEL_URL, dest, true)?;
    println!("✓ Model installed → {}", dest.display());
    Ok(())
}

/// Run curl to download a URL to a local path.
fn curl_download(url: &str, dest: &std::path::Path, progress: bool) -> Result<()> {
    let mut cmd = Command::new("curl");
    cmd.args(["-fsSL", "-H", "User-Agent: automode/dev"]);
    if progress {
        cmd.arg("--progress-bar");
    }
    cmd.args(["-o", dest.to_str().unwrap(), url]);

    let status = cmd.status()
        .map_err(|e| anyhow!("curl not found — please install curl: {}", e))?;

    if !status.success() {
        return Err(anyhow!("download failed for {}", url));
    }
    Ok(())
}

async fn fetch_llama_tag() -> Result<String> {
    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .get(LLAMA_API)
        .header("User-Agent", "automode/dev")
        .send()
        .await
        .map_err(|e| anyhow!("GitHub API unreachable: {}", e))?
        .json()
        .await
        .map_err(|e| anyhow!("failed to parse GitHub API response: {}", e))?;

    resp["tag_name"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("tag_name missing from GitHub API response — possible rate limit"))
}

fn detect_platform() -> Result<(String, String)> {
    let os = std::env::consts::OS.to_string();
    let arch = std::env::consts::ARCH.to_string();
    Ok((os, arch))
}

fn llama_platform_string(os: &str, arch: &str) -> Result<String> {
    match (os, arch) {
        ("macos", "aarch64") => Ok("macos-arm64".to_string()),
        ("macos", "x86_64")  => Ok("macos-x86_64".to_string()),
        ("linux", "x86_64")  => Ok("ubuntu-x64".to_string()),
        _ => Err(anyhow!("unsupported platform: {}-{}", os, arch)),
    }
}
