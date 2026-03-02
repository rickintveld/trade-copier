use anyhow::{bail, Context, Result};
use log::{info, warn};
use std::process::Command;
use std::io::Write;

/// Check if Homebrew is installed on macOS
#[cfg(target_os = "macos")]
pub fn is_homebrew_installed() -> bool {
    // Check common Homebrew installation paths
    // This is more reliable than 'which brew' in production builds
    // where PATH may not include Homebrew directories
    let brew_paths = vec![
        "/opt/homebrew/bin/brew",  // Apple Silicon
        "/usr/local/bin/brew",      // Intel Mac
    ];
    
    // First check if brew exists in standard locations
    for path in &brew_paths {
        if std::path::Path::new(path).exists() {
            return true;
        }
    }
    
    // Fallback to 'which' command (works in dev mode)
    Command::new("which")
        .arg("brew")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Install Homebrew on macOS
/// This requires user interaction and sudo permissions
#[cfg(target_os = "macos")]
pub fn install_homebrew() -> Result<()> {
    if is_homebrew_installed() {
        info!("[INSTALLER] Homebrew is already installed");
        return Ok(());
    }

    info!("[INSTALLER] Homebrew is not installed. Installing Homebrew...");
    info!("[INSTALLER] This will require user interaction and may ask for your password.");
    
    // Homebrew installation script
    let install_script = r#"/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)""#;
    
    let status = Command::new("bash")
        .arg("-c")
        .arg(install_script)
        .status()
        .context("Failed to execute Homebrew installation script")?;
    
    if !status.success() {
        bail!("Homebrew installation failed");
    }
    
    info!("[INSTALLER] Homebrew installed successfully");
    
    // Check if we need to add Homebrew to PATH (Apple Silicon)
    if cfg!(target_arch = "aarch64") {
        info!("[INSTALLER] Detected Apple Silicon. Checking Homebrew PATH...");
        
        // Try to run brew after installation
        if !is_homebrew_installed() {
            info!("[INSTALLER] Adding Homebrew to PATH for this session...");
            info!("[INSTALLER] Note: You may need to restart your terminal for permanent PATH changes");
            
            // For the current process, we can't modify PATH for subsequent commands
            // but we can inform the user
            info!("[INSTALLER] Please run: eval \"$(/opt/homebrew/bin/brew shellenv)\"");
        }
    }
    
    Ok(())
}

/// Check if Wine is installed on macOS
#[cfg(target_os = "macos")]
pub fn is_wine_installed() -> bool {
    // Check common Wine installation paths
    let wine_paths = vec![
        "/opt/homebrew/bin/wine",  // Homebrew on Apple Silicon
        "/usr/local/bin/wine",      // Homebrew on Intel
        "/opt/local/bin/wine",      // MacPorts
    ];
    
    // First check PATH
    if let Ok(output) = Command::new("which").arg("wine").output() {
        if output.status.success() {
            return true;
        }
    }
    
    // Then check common paths
    for path in &wine_paths {
        if std::path::Path::new(path).exists() {
            return true;
        }
    }
    
    false
}

/// Install Wine on macOS using Homebrew
#[cfg(target_os = "macos")]
pub fn install_wine() -> Result<()> {
    if is_wine_installed() {
        info!("[INSTALLER] Wine is already installed");
        return Ok(());
    }
    
    info!("[INSTALLER] Wine is not installed. Installing Wine via Homebrew...");
    
    // Ensure Homebrew is installed first
    if !is_homebrew_installed() {
        info!("[INSTALLER] Homebrew is required to install Wine");
        install_homebrew()?;
    }
    
    // Determine the correct brew path
    let brew_path = if cfg!(target_arch = "aarch64") {
        "/opt/homebrew/bin/brew"
    } else {
        "/usr/local/bin/brew"
    };
    
    // Check if brew is accessible
    let brew_cmd = if std::path::Path::new(brew_path).exists() {
        brew_path
    } else if Command::new("which").arg("brew").output().is_ok() {
        "brew"
    } else {
        bail!("Homebrew not found. Please ensure Homebrew is installed and in PATH");
    };
    
    info!("[INSTALLER] Installing wine-stable (this may take several minutes)...");
    
    let status = Command::new(brew_cmd)
        .args(["install", "--cask", "wine-stable"])
        .status()
        .context("Failed to execute brew install command")?;
    
    if !status.success() {
        bail!("Wine installation failed. Please try manually: brew install --cask wine-stable");
    }
    
    info!("[INSTALLER] Wine installed successfully");
    
    // Verify installation
    if !is_wine_installed() {
        warn!("[INSTALLER] Wine was installed but not found in expected locations");
        warn!("[INSTALLER] You may need to restart your terminal");
    }
    
    Ok(())
}

/// Ensure Wine is installed on macOS (install if not present)
/// Interactive version - asks user for confirmation before installing
#[cfg(target_os = "macos")]
#[allow(dead_code)]
pub fn ensure_wine_installed() -> Result<()> {
    if is_wine_installed() {
        return Ok(());
    }
    
    info!("[INSTALLER] ═══════════════════════════════════════════════════════");
    info!("[INSTALLER] Wine is required but not installed");
    info!("[INSTALLER] ═══════════════════════════════════════════════════════");
    
    // Ask user for confirmation
    print!("[INSTALLER] Do you want to install Wine now? This will also install Homebrew if needed. (y/n): ");
    std::io::stdout().flush().context("Failed to flush stdout")?;
    
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .context("Failed to read user input")?;
    
    let input = input.trim().to_lowercase();
    
    if input == "y" || input == "yes" {
        install_wine()?;
        info!("[INSTALLER] ═══════════════════════════════════════════════════════");
        info!("[INSTALLER] Wine installation complete!");
        info!("[INSTALLER] ═══════════════════════════════════════════════════════");
        Ok(())
    } else {
        bail!("Wine installation cancelled by user. Please install Wine manually: brew install --cask wine-stable");
    }
}

/// Ensure Wine is installed on macOS - automatic version (no prompts)
/// This is suitable for GUI applications where stdin is not available
#[cfg(target_os = "macos")]
pub fn ensure_wine_installed_auto() -> Result<()> {
    if is_wine_installed() {
        return Ok(());
    }
    
    info!("[INSTALLER] ═══════════════════════════════════════════════════════");
    info!("[INSTALLER] Wine is required but not installed");
    info!("[INSTALLER] Automatically installing Wine and Homebrew (if needed)...");
    info!("[INSTALLER] ═══════════════════════════════════════════════════════");
    
    install_wine()?;
    
    info!("[INSTALLER] ═══════════════════════════════════════════════════════");
    info!("[INSTALLER] Wine installation complete!");
    info!("[INSTALLER] ═══════════════════════════════════════════════════════");
    
    Ok(())
}

// Windows package manager functions
#[cfg(target_os = "windows")]
pub fn is_chocolatey_installed() -> bool {
    Command::new("where")
        .arg("choco")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
pub fn install_chocolatey() -> Result<()> {
    if is_chocolatey_installed() {
        info!("[INSTALLER] Chocolatey is already installed");
        return Ok(());
    }
    
    info!("[INSTALLER] Chocolatey is not installed. Installing Chocolatey...");
    info!("[INSTALLER] This requires administrator privileges");
    
    // Chocolatey installation command (requires PowerShell with admin rights)
    let install_script = "Set-ExecutionPolicy Bypass -Scope Process -Force; [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072; iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))";
    
    let status = Command::new("powershell")
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-Command")
        .arg(install_script)
        .status()
        .context("Failed to execute Chocolatey installation script")?;
    
    if !status.success() {
        bail!("Chocolatey installation failed. Please run PowerShell as Administrator and try again.");
    }
    
    info!("[INSTALLER] Chocolatey installed successfully");
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn ensure_chocolatey_installed() -> Result<()> {
    if is_chocolatey_installed() {
        return Ok(());
    }
    
    info!("[INSTALLER] ═══════════════════════════════════════════════════════");
    info!("[INSTALLER] Chocolatey package manager is not installed");
    info!("[INSTALLER] ═══════════════════════════════════════════════════════");
    
    print!("[INSTALLER] Do you want to install Chocolatey now? This requires administrator privileges. (y/n): ");
    std::io::stdout().flush().context("Failed to flush stdout")?;
    
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .context("Failed to read user input")?;
    
    let input = input.trim().to_lowercase();
    
    if input == "y" || input == "yes" {
        install_chocolatey()?;
        info!("[INSTALLER] ═══════════════════════════════════════════════════════");
        info!("[INSTALLER] Chocolatey installation complete!");
        info!("[INSTALLER] ═══════════════════════════════════════════════════════");
        Ok(())
    } else {
        bail!("Chocolatey installation cancelled by user");
    }
}
