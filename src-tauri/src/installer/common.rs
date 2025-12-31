use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

const MT5_INSTALLER_URL: &str = "https://download.mql5.com/cdn/web/metaquotes.ltd/mt5/mt5setup.exe";


/// Download MT5 installer to a temporary file
pub async fn download_mt5_installer() -> Result<PathBuf> {
    println!("[INSTALLER] Downloading MT5 installer from {}", MT5_INSTALLER_URL);
    
    let response = reqwest::get(MT5_INSTALLER_URL)
        .await
        .context("Failed to download MT5 installer")?;
    
    if !response.status().is_success() {
        anyhow::bail!("Failed to download MT5 installer: HTTP {}", response.status());
    }

    // Create temporary file
    let temp_file = tempfile::Builder::new()
        .prefix("mt5setup")
        .suffix(".exe")
        .tempfile()
        .context("Failed to create temporary file")?;
    
    let temp_path = temp_file.path().to_path_buf();
    
    // Download content
    let bytes = response.bytes()
        .await
        .context("Failed to read MT5 installer bytes")?;
    
    // Write to temp file
    let mut file = tokio::fs::File::create(&temp_path)
        .await
        .context("Failed to create temp file for MT5 installer")?;
    
    file.write_all(&bytes)
        .await
        .context("Failed to write MT5 installer to temp file")?;
    
    file.flush()
        .await
        .context("Failed to flush MT5 installer file")?;
    
    println!("[INSTALLER] Downloaded MT5 installer to {:?}", temp_path);
    
    // Keep the temp file alive by forgetting the TempFile handle
    // The caller is responsible for cleanup
    std::mem::forget(temp_file);
    
    Ok(temp_path)
}

/// Copy Expert Advisors from ./mql5/Trading Rocket/ to the MT5 MQL5/Experts directory
pub fn copy_expert_advisors(wine_prefix: &Path) -> Result<()> {
    println!("[INSTALLER] Copying Expert Advisors to MT5 directory");
    
    // Try multiple possible locations for the source directory
    let mut source_dir: Option<PathBuf> = None;
    
    // 1. Try relative to current working directory (development mode - new location)
    let dev_path = PathBuf::from("./src/mql5/Trading Rocket");
    if dev_path.exists() {
        source_dir = Some(dev_path);
    } else {
        // 2. Try old location for backwards compatibility
        let old_dev_path = PathBuf::from("./mql5/Trading Rocket");
        if old_dev_path.exists() {
            source_dir = Some(old_dev_path);
        } else {
            // 3. Try relative to executable directory (production mode)
            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(exe_dir) = exe_path.parent() {
                    // On macOS, the executable is in Contents/MacOS/, so we need to go up to Resources/
                    let resource_path = exe_dir.parent()
                        .and_then(|p| Some(p.join("Resources/mql5/Trading Rocket")));
                    
                    if let Some(ref path) = resource_path {
                        if path.exists() {
                            source_dir = Some(path.clone());
                        }
                    }
                    
                    // 4. Also try directly next to executable (for non-bundled builds)
                    if source_dir.is_none() {
                        let exe_relative = exe_dir.join("mql5/Trading Rocket");
                        if exe_relative.exists() {
                            source_dir = Some(exe_relative);
                        }
                    }
                }
            }
        }
    }
    
    let source_dir = source_dir.ok_or_else(|| {
        anyhow::anyhow!(
            "Source directory not found: ./src/mql5/Trading Rocket\n".to_owned() +
            "  Tried current directory, old location (./mql5/), and executable resource locations"
        )
    })?;
    
    println!("[INSTALLER] Using source directory: {:?}", source_dir);
    
    // Destination directory in MT5 installation
    let dest_dir = wine_prefix.join("drive_c/Program Files/MetaTrader 5/MQL5/Experts/Trading Rocket");
    
    // Create destination directory if it doesn't exist
    if !dest_dir.exists() {
        fs::create_dir_all(&dest_dir)
            .context(format!("Failed to create destination directory: {:?}", dest_dir))?;
        println!("[INSTALLER] Created directory: {:?}", dest_dir);
    }
    
    // Copy all files from source to destination
    let entries = fs::read_dir(&source_dir)
        .context(format!("Failed to read source directory: {:?}", source_dir))?;
    
    let mut copied_count = 0;
    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        
        // Skip directories and hidden files like .DS_Store
        if path.is_file() {
            let file_name = path.file_name()
                .context("Failed to get file name")?;
            
            // Skip .DS_Store and other hidden files
            if file_name.to_string_lossy().starts_with('.') {
                continue;
            }
            
            let dest_path = dest_dir.join(file_name);
            fs::copy(&path, &dest_path)
                .context(format!("Failed to copy {:?} to {:?}", path, dest_path))?;
            
            println!("[INSTALLER]   Copied: {:?}", file_name);
            copied_count += 1;
        }
    }
    
    println!("[INSTALLER] Successfully copied {} Expert Advisor files", copied_count);
    Ok(())
}
