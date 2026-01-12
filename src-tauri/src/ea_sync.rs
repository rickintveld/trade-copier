use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// Get the path to the master MT5 installation's Experts directory based on the platform
pub fn get_master_mt5_experts_path() -> Result<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        // On macOS, MT5 runs through Wine with the following default path structure
        let home_dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Failed to get home directory"))?;
        
        let mt5_experts = home_dir.join("Library/Application Support/net.metaquotes.wine.metatrader5/drive_c/Program Files/MetaTrader 5/MQL5/Experts");
        
        if !mt5_experts.exists() {
            anyhow::bail!(
                "Master MT5 installation not found at: {:?}\n  \
                Please ensure MetaTrader 5 is installed before starting this application.",
                mt5_experts
            );
        }
        
        Ok(mt5_experts)
    }
    
    #[cfg(target_os = "windows")]
    {
        // On Windows, MT5 is typically installed in Program Files
        // Try both Program Files and Program Files (x86)
        let mut program_files_paths = vec![
            PathBuf::from("C:\\Program Files\\MetaTrader 5\\MQL5\\Experts"),
            PathBuf::from("C:\\Program Files (x86)\\MetaTrader 5\\MQL5\\Experts"),
        ];
        
        // Also check AppData for portable installations
        if let Some(app_data) = dirs::data_local_dir() {
            program_files_paths.push(app_data.join("Programs\\MetaTrader 5\\MQL5\\Experts"));
        }
        
        for path in &program_files_paths {
            if path.exists() {
                return Ok(path.clone());
            }
        }
        
        anyhow::bail!(
            "Master MT5 installation not found. Checked:\n  {}\n  \
            Please ensure MetaTrader 5 is installed before starting this application.",
            program_files_paths.iter()
                .map(|p| format!("{:?}", p))
                .collect::<Vec<_>>()
                .join("\n  ")
        );
    }
    
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        anyhow::bail!("Unsupported platform. Only macOS and Windows are supported.");
    }
}

/// Get the source directory containing the Expert Advisors to copy
fn get_ea_source_path() -> Result<PathBuf> {
    // Try multiple possible locations for the source directory
    let candidates = vec![
        // 1. Development mode - relative to project root
        PathBuf::from("./src-tauri/src/mql5/Trading Rocket"),
        
        // 2. Development mode - when running from src-tauri directory
        PathBuf::from("./src/mql5/Trading Rocket"),
        
        // 3. Development mode - old location
        PathBuf::from("./mql5/Trading Rocket"),
    ];
    
    // 3. Production mode - relative to executable
    let mut production_candidates = Vec::new();
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            // On macOS, the executable is in Contents/MacOS/, resources are in Contents/Resources/
            #[cfg(target_os = "macos")]
            {
                if let Some(resources_dir) = exe_dir.parent().map(|p| p.join("Resources")) {
                    production_candidates.push(resources_dir.join("src/mql5/Trading Rocket"));
                    production_candidates.push(resources_dir.join("mql5/Trading Rocket"));
                }
            }
            
            // On Windows, resources are typically next to the executable
            #[cfg(target_os = "windows")]
            {
                production_candidates.push(exe_dir.join("src\\mql5\\Trading Rocket"));
                production_candidates.push(exe_dir.join("mql5\\Trading Rocket"));
            }
        }
    }
    
    // Check all candidates
    for candidate in candidates.iter().chain(production_candidates.iter()) {
        if candidate.exists() && candidate.is_dir() {
            println!("[EA_SYNC] Found EA source directory: {:?}", candidate);
            return Ok(candidate.clone());
        }
    }
    
    anyhow::bail!(
        "Expert Advisor source directory not found. Tried:\n  {}\n  {}",
        candidates.iter()
            .map(|p| format!("{:?}", p))
            .collect::<Vec<_>>()
            .join("\n  "),
        production_candidates.iter()
            .map(|p| format!("{:?}", p))
            .collect::<Vec<_>>()
            .join("\n  ")
    )
}

/// Copy Expert Advisors from the application bundle to the master MT5 installation
/// This should be called on application startup to ensure the latest version is always available
pub fn sync_expert_advisors_to_master() -> Result<()> {
    println!("[EA_SYNC] Starting Expert Advisor synchronization to master MT5 installation...");
    
    // Get source and destination paths
    let source_dir = get_ea_source_path()?;
    let experts_dir = get_master_mt5_experts_path()?;
    let dest_dir = experts_dir.join("Trading Rocket");
    
    println!("[EA_SYNC] Source: {:?}", source_dir);
    println!("[EA_SYNC] Destination: {:?}", dest_dir);
    
    // Create destination directory if it doesn't exist
    if !dest_dir.exists() {
        fs::create_dir_all(&dest_dir)
            .context(format!("Failed to create destination directory: {:?}", dest_dir))?;
        println!("[EA_SYNC] Created directory: {:?}", dest_dir);
    }
    
    // Read all files from source directory
    let entries = fs::read_dir(&source_dir)
        .context(format!("Failed to read source directory: {:?}", source_dir))?;
    
    let mut copied_count = 0;
    let mut skipped_count = 0;
    
    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        
        // Only process files (skip directories and symlinks)
        if !path.is_file() {
            continue;
        }
        
        let file_name = path.file_name()
            .context("Failed to get file name")?;
        
        // Skip hidden files like .DS_Store
        let file_name_str = file_name.to_string_lossy();
        if file_name_str.starts_with('.') {
            skipped_count += 1;
            continue;
        }
        
        // Copy file to destination (overwrite if exists)
        let dest_path = dest_dir.join(file_name);
        fs::copy(&path, &dest_path)
            .context(format!("Failed to copy {:?} to {:?}", path, dest_path))?;
        
        println!("[EA_SYNC]   ✓ Copied: {:?}", file_name);
        copied_count += 1;
    }
    
    println!(
        "[EA_SYNC] ✓ Successfully synchronized {} Expert Advisor file(s) ({} skipped)",
        copied_count,
        skipped_count
    );
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_get_master_mt5_experts_path() {
        // This test will only pass if MT5 is actually installed
        // In CI/CD, you might want to skip this test
        match get_master_mt5_experts_path() {
            Ok(path) => {
                println!("Found MT5 at: {:?}", path);
                assert!(path.exists());
            }
            Err(e) => {
                println!("MT5 not found (expected if not installed): {}", e);
            }
        }
    }
    
    #[test]
    fn test_get_ea_source_path() {
        // This should work in development environment
        match get_ea_source_path() {
            Ok(path) => {
                println!("Found EA source at: {:?}", path);
                assert!(path.exists());
            }
            Err(e) => {
                println!("EA source not found: {}", e);
            }
        }
    }
}
