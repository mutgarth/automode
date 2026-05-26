use anyhow::Result;
use std::process::Command;

use crate::config::{self, Config, Target};
use crate::setup;

const AUTOMODE_REPO: &str = "https://github.com/mutgarth/automode";

pub async fn run(target: Option<&str>) -> Result<()> {
    println!("\nautomode update");
    println!("──────────────────────────────────────");

    let base = config::automode_dir();
    std::fs::create_dir_all(&base)?;
    std::fs::create_dir_all(base.join("logs"))?;
    std::fs::create_dir_all(base.join("models"))?;

    install_current_binary()?;

    let mut cfg = config::load()?;
    let had_target_key = config_has_target_key();
    let target = match target {
        Some(target) => Target::from_str(target)?,
        None if had_target_key => cfg.target.clone(),
        // Legacy configs predate multi-host support. Migrating to "all" lets
        // existing installs pick up newer integrations without rerunning setup.
        None => Target::All,
    };

    cfg.target = target.clone();
    save_preserving_existing_values(&cfg)?;
    println!(
        "✓ Config migrated/preserved → {}",
        config::config_path().display()
    );

    setup::install_target_hooks(&target)?;

    println!("✓ Update complete. Restart running sessions so hooks are reloaded.");
    println!("  If automode is already running, run `automode stop && automode start`.");
    Ok(())
}

fn install_current_binary() -> Result<()> {
    let self_path = std::env::current_exe()?;
    let dest = config::automode_dir().join("automode");

    if self_path == dest {
        download_latest_binary(&dest)?;
        return Ok(());
    }

    std::fs::copy(&self_path, &dest)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))?;
    }
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("xattr")
            .args(["-c", dest.to_str().unwrap()])
            .status();
        let _ = Command::new("codesign")
            .args(["--force", "--sign", "-", dest.to_str().unwrap()])
            .status();
    }

    println!("✓ Binary updated → {}", dest.display());
    Ok(())
}

fn download_latest_binary(dest: &std::path::Path) -> Result<()> {
    let platform = automode_platform_string()?;
    let url = format!(
        "{}/releases/latest/download/automode-{}",
        AUTOMODE_REPO, platform
    );
    let tmp = dest.with_extension("update");

    println!("→ Downloading latest automode binary for {}...", platform);
    curl_download(&url, &tmp)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
    }
    sign_macos_binary(&tmp);
    std::fs::rename(&tmp, dest)?;
    println!("✓ Binary updated → {}", dest.display());
    Ok(())
}

fn curl_download(url: &str, dest: &std::path::Path) -> Result<()> {
    let status = Command::new("curl")
        .args(["-fsSL", "--progress-bar", "-o", dest.to_str().unwrap(), url])
        .status()
        .map_err(|e| anyhow::anyhow!("curl not found — please install curl: {}", e))?;

    if !status.success() {
        let _ = std::fs::remove_file(dest);
        return Err(anyhow::anyhow!("download failed for {}", url));
    }
    Ok(())
}

fn automode_platform_string() -> Result<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Ok("macos-arm64"),
        ("macos", "x86_64") => Ok("macos-x86_64"),
        ("linux", "x86_64") => Ok("linux-x86_64"),
        (os, arch) => Err(anyhow::anyhow!("unsupported platform: {}-{}", os, arch)),
    }
}

fn sign_macos_binary(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("xattr")
            .args(["-c", path.to_str().unwrap()])
            .status();
        let _ = Command::new("codesign")
            .args(["--force", "--sign", "-", path.to_str().unwrap()])
            .status();
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
    }
}

fn config_has_target_key() -> bool {
    let path = config::config_path();
    let Ok(s) = std::fs::read_to_string(path) else {
        return false;
    };

    s.lines()
        .map(str::trim)
        .any(|line| line.starts_with("target") && line.contains('='))
}

fn save_preserving_existing_values(cfg: &Config) -> Result<()> {
    config::save(cfg)
}
