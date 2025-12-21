use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

pub fn check_wine_installed() -> Result<()> {
    let output = Command::new("which")
        .arg("wine")
        .output()
        .context("Failed to check for Wine installation")?;

    if !output.status.success() {
        bail!("Wine is not installed. Install it with: brew install --cask wine-stable");
    }

    Ok(())
}

pub fn create_wine_prefix(prefix_path: &Path) -> Result<()> {
    println!("Creating Wine prefix at {}...", prefix_path.display());

    if prefix_path.exists() {
        bail!("Wine prefix already exists at {}", prefix_path.display());
    }

    let status = Command::new("winecfg")
        .env("WINEPREFIX", prefix_path)
        .status()
        .context("Failed to create Wine prefix")?;

    if !status.success() {
        bail!("Wine prefix creation failed");
    }

    println!("Wine prefix created successfully");
    Ok(())
}

pub fn install_mt5(prefix_path: &Path, installer_path: &Path) -> Result<()> {
    if !installer_path.exists() {
        bail!("MT5 installer not found at {}", installer_path.display());
    }

    println!("Installing MT5 from {}...", installer_path.display());
    println!("This may take a few minutes. Follow the installation wizard.");

    let status = Command::new("wine")
        .env("WINEPREFIX", prefix_path)
        .arg(installer_path)
        .status()
        .context("Failed to run MT5 installer")?;

    if !status.success() {
        bail!("MT5 installation failed");
    }

    println!("MT5 installed successfully");
    Ok(())
}

pub fn launch_mt5(prefix_path: &Path, mt5_executable: &Path) -> Result<()> {
    if !mt5_executable.exists() {
        bail!(
            "MT5 executable not found at {}. Make sure MT5 is installed.",
            mt5_executable.display()
        );
    }

    println!("Launching MT5 instance at {}...", prefix_path.display());

    let child = Command::new("wine")
        .env("WINEPREFIX", prefix_path)
        .arg(mt5_executable)
        .spawn()
        .context("Failed to launch MT5")?;

    println!("MT5 launched with PID {}", child.id());
    Ok(())
}
