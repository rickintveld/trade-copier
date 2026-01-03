use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use crate::database::{Database, WorkerState};

pub struct MacInstanceManager {}

impl MacInstanceManager {
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }

    pub async fn create_instance(
        &self,
        name: String,
        address: String,
        multiplier: f64,
        symbol_prefix: String,
        installer_path: &Path,
        db: Arc<Database>,
    ) -> Result<()> {
        // Ensure Wine is installed (will install automatically if not present)
        if let Err(e) = super::package_manager::ensure_wine_installed_auto() {
            let error_msg = format!("Wine installation failed: {}", e);
            eprintln!("[INSTALLER] {}", error_msg);
            let _ = db.insert_worker_error(&address, crate::database::ErrorSeverity::Critical, &error_msg).await;
            return Err(e);
        }

        // Generate prefix path using worker count + 1 as ID
        let workers = db.get_worker_configs().await?;
        let id = workers.len() + 1;
        let prefix_path = self.generate_prefix_path(id)?;

        println!("[INSTALLER] Creating Wine prefix at {:?}", prefix_path);

        // Create Wine prefix
        if let Err(e) = create_wine_prefix(&prefix_path) {
            let error_msg = format!("Failed to create Wine prefix: {}", e);
            eprintln!("[INSTALLER] {}", error_msg);
            let _ = db.insert_worker_error(&address, crate::database::ErrorSeverity::Critical, &error_msg).await;
            return Err(e);
        }

        // Install MT5
        println!("[INSTALLER] Installing MT5 from {:?}", installer_path);
        if let Err(e) = install_mt5(&prefix_path, installer_path) {
            let error_msg = format!("MT5 installation failed: {}", e);
            eprintln!("[INSTALLER] {}", error_msg);
            eprintln!("[INSTALLER] The instance was created but MT5 installation incomplete.");
            eprintln!("[INSTALLER] You can delete it with: DELETE /api/instances/{}", name);
            // Log to worker_errors table
            let _ = db.insert_worker_error(&address, crate::database::ErrorSeverity::Critical, &error_msg).await;
            return Err(e);
        }
        
        // Copy Expert Advisors after successful installation
        if let Err(e) = super::common::copy_expert_advisors(&prefix_path) {
            let error_msg = format!("Failed to copy Expert Advisors: {}. You can manually copy them from ./src/mql5/Trading Rocket/", e);
            eprintln!("[INSTALLER] Warning: {}", error_msg);
            let _ = db.insert_worker_error(&address, crate::database::ErrorSeverity::Warning, &error_msg).await;
        }
        
        // Copy Default.tpl template with worker port configuration
        if let Err(e) = super::common::copy_default_template(&prefix_path, &address) {
            let error_msg = format!("Failed to copy Default.tpl template: {}. You can manually copy it from ./src/mql5/Profiles/Templates/", e);
            eprintln!("[INSTALLER] Warning: {}", error_msg);
            let _ = db.insert_worker_error(&address, crate::database::ErrorSeverity::Warning, &error_msg).await;
        }

        // Save worker to database with wine_prefix
        let prefix_str = prefix_path.to_string_lossy().to_string();
        db.upsert_worker(
            &name,
            &address,
            multiplier,
            WorkerState::Inactive,
            None,
            Some(&prefix_str),
            Some(&symbol_prefix),
        ).await?;

        println!("[INSTALLER] Instance '{}' created successfully!", name);
        println!("[INSTALLER]   ID: {}", id);
        println!("[INSTALLER]   Prefix: {:?}", prefix_path);
        println!("[INSTALLER]   Address: {}", address);
        println!("[INSTALLER]   Multiplier: {}", multiplier);
        println!("[INSTALLER] NOTE: Worker will be activated automatically after installation completes");

        Ok(())
    }

    pub async fn delete_instance(&self, name: &str, force: bool, db: Arc<Database>) -> Result<()> {
        // Get worker from database
        let worker = db.get_worker_by_name(name).await?
            .context(format!("Instance '{}' not found", name))?;

        if !force {
            println!("[INSTALLER] Warning: This will delete instance '{}' and all its data", name);
            println!("[INSTALLER] Use force=true to confirm deletion");
            bail!("Deletion cancelled - use force=true to confirm");
        }

        // Stop the Wine process if it's running
        if let Some(wine_prefix) = &worker.wine_prefix {
            let prefix_path = PathBuf::from(wine_prefix);
            
            // Kill Wine process
            if let Err(e) = crate::worker::kill_wine_process(&prefix_path).await {
                eprintln!("[INSTALLER] Warning: Failed to kill Wine process: {}", e);
            }
            
            // Delete Wine prefix directory if it exists
            if prefix_path.exists() {
                fs::remove_dir_all(&prefix_path)?;
                println!("[INSTALLER] Removed Wine prefix: {:?}", prefix_path);
            }
        }

        // Remove from database
        db.delete_worker(name).await?;

        println!("[INSTALLER] Instance '{}' deleted successfully", name);
        println!("[INSTALLER] NOTE: Workers will be reloaded automatically");

        Ok(())
    }

    pub async fn stop_instance(&self, name: &str, db: Arc<Database>) -> Result<()> {
        // Get worker from database
        let worker = db.get_worker_by_name(name).await?
            .context(format!("Instance '{}' not found", name))?;
        
        let wine_prefix = worker.wine_prefix
            .context("Instance does not have a wine_prefix configured")?;
        let prefix_path = PathBuf::from(wine_prefix);
        
        // Kill the Wine process
        crate::worker::kill_wine_process(&prefix_path).await?;
        
        println!("[INSTALLER] Instance '{}' stopped", name);
        
        Ok(())
    }

    pub async fn start_instance(&self, name: &str, db: Arc<Database>) -> Result<()> {
        // Ensure Wine is installed (will install automatically if not present)
        if let Err(e) = super::package_manager::ensure_wine_installed_auto() {
            let error_msg = format!("Wine installation failed: {}", e);
            eprintln!("[INSTALLER] {}", error_msg);
            // Try to get worker address for error logging
            if let Ok(Some(worker)) = db.get_worker_by_name(name).await {
                let _ = db.insert_worker_error(&worker.address, crate::database::ErrorSeverity::Critical, &error_msg).await;
            }
            return Err(e);
        }

        // Get worker from database
        let worker = db.get_worker_by_name(name).await?
            .context(format!("Instance '{}' not found", name))?;
        
        let wine_prefix = worker.wine_prefix.clone()
            .context("Instance does not have a wine_prefix configured")?;
        let prefix_path = PathBuf::from(&wine_prefix);
        
        // Copy Expert Advisors before starting
        if let Err(e) = super::common::copy_expert_advisors(&prefix_path) {
            let error_msg = format!("Failed to copy Expert Advisors: {}", e);
            eprintln!("[INSTALLER] Warning: {}", error_msg);
            let _ = db.insert_worker_error(&worker.address, crate::database::ErrorSeverity::Warning, &error_msg).await;
        }
        
        // Copy Default.tpl template with worker port configuration before starting
        if let Err(e) = super::common::copy_default_template(&prefix_path, &worker.address) {
            let error_msg = format!("Failed to copy Default.tpl template: {}", e);
            eprintln!("[INSTALLER] Warning: {}", error_msg);
            let _ = db.insert_worker_error(&worker.address, crate::database::ErrorSeverity::Warning, &error_msg).await;
        }

        let mt5_exe = self.mt5_executable(&prefix_path);
        if let Err(e) = launch_mt5(&prefix_path, &mt5_exe, None).await {
            let error_msg = format!("Failed to launch MT5: {}", e);
            eprintln!("[INSTALLER] {}", error_msg);
            let _ = db.insert_worker_error(&worker.address, crate::database::ErrorSeverity::Critical, &error_msg).await;
            return Err(e);
        }

        println!("[INSTALLER] Instance '{}' started", name);

        Ok(())
    }

    pub async fn list_instances(&self, db: Arc<Database>) -> Result<Vec<crate::database::WorkerRecord>> {
        db.get_all_workers().await
    }

    fn generate_prefix_path(&self, id: usize) -> Result<PathBuf> {
        let app_data_dir = dirs::data_local_dir()
            .context("Could not find local data directory")?
            .join("trade-copier");
        
        // Ensure the app data directory exists
        fs::create_dir_all(&app_data_dir)
            .context("Failed to create app data directory")?;
        
        Ok(app_data_dir.join(format!("wine-mt5-instance{}", id)))
    }

    fn mt5_executable(&self, prefix_path: &Path) -> PathBuf {
        prefix_path.join("drive_c/Program Files/MetaTrader 5/terminal64.exe")
    }
}

/// Get the path to Wine executable, checking common installation locations
fn get_wine_path() -> Result<PathBuf> {
    // Common Wine installation paths on macOS
    let wine_paths = vec![
        "/opt/homebrew/bin/wine",  // Homebrew on Apple Silicon
        "/usr/local/bin/wine",      // Homebrew on Intel
        "/opt/local/bin/wine",      // MacPorts
    ];
    
    // First, try to find wine in PATH using which
    if let Ok(output) = Command::new("which").arg("wine").output() {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path_str.is_empty() {
                return Ok(PathBuf::from(path_str));
            }
        }
    }
    
    // If not in PATH, check common installation locations
    for wine_path in &wine_paths {
        let path = Path::new(wine_path);
        if path.exists() {
            println!("[INSTALLER] Found Wine at {}", wine_path);
            return Ok(path.to_path_buf());
        }
    }
    
    bail!("Wine is not installed. Install it with: brew install --cask wine-stable")
}

/// Get the path to wineserver executable
pub fn get_wineserver_path() -> Result<PathBuf> {
    let wine_path = get_wine_path()?;
    // wineserver is usually in the same directory as wine
    let wine_dir = wine_path.parent()
        .context("Could not determine Wine directory")?;
    let wineserver = wine_dir.join("wineserver");
    
    if wineserver.exists() {
        Ok(wineserver)
    } else {
        bail!("wineserver not found at {:?}", wineserver)
    }
}

/// Get the path to winecfg executable
fn get_winecfg_path() -> Result<PathBuf> {
    let wine_path = get_wine_path()?;
    // winecfg is usually in the same directory as wine
    let wine_dir = wine_path.parent()
        .context("Could not determine Wine directory")?;
    let winecfg = wine_dir.join("winecfg");
    
    if winecfg.exists() {
        Ok(winecfg)
    } else {
        bail!("winecfg not found at {:?}", winecfg)
    }
}


fn create_wine_prefix(prefix_path: &Path) -> Result<()> {
    if prefix_path.exists() {
        bail!("Wine prefix already exists at {:?}", prefix_path);
    }

    let winecfg = get_winecfg_path()?;
    let status = Command::new(winecfg)
        .env("WINEPREFIX", prefix_path)
        .status()
        .context("Failed to create Wine prefix")?;

    if !status.success() {
        bail!("Wine prefix creation failed");
    }

    println!("[INSTALLER] Wine prefix created successfully");
    Ok(())
}

fn install_mt5(prefix_path: &Path, installer_path: &Path) -> Result<()> {
    if !installer_path.exists() {
        bail!("MT5 installer not found at {:?}", installer_path);
    }

    println!("[INSTALLER] Installing MT5... This may take a few minutes.");

    // Get Wine executable path
    let wine = get_wine_path()?;
    
    // Spawn the installer without waiting for it to exit
    // (Wine keeps running even after installation completes)
    let mut child = Command::new(wine)
        .env("WINEPREFIX", prefix_path)
        .arg(installer_path)
        .spawn()
        .context("Failed to run MT5 installer")?;

    println!("[INSTALLER] Installer process started (PID: {}), waiting for installation to complete...", child.id());

    // Poll for the MT5 executable to appear
    let mt5_exe = prefix_path.join("drive_c/Program Files/MetaTrader 5/terminal64.exe");
    let max_wait_secs = 300; // 5 minutes timeout
    let poll_interval = std::time::Duration::from_secs(2);
    let start = std::time::Instant::now();
    
    loop {
        // Check if executable exists
        if mt5_exe.exists() {
            println!("[INSTALLER] MT5 executable detected at {:?}", mt5_exe);
            println!("[INSTALLER] MT5 installed successfully");
            
            // Let the installer process finish naturally
            return Ok(());
        }
        
        // Check for timeout
        if start.elapsed().as_secs() > max_wait_secs {
            let _ = child.kill();
            bail!(
                "MT5 installation timeout: executable not found at {:?} after {} seconds",
                mt5_exe,
                max_wait_secs
            );
        }
        
        // Check if the installer process has exited unexpectedly
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process exited - check if executable was created
                if mt5_exe.exists() {
                    println!("[INSTALLER] MT5 installed successfully");
                    return Ok(());
                } else {
                    bail!(
                        "MT5 installation failed: installer exited with code {} but executable not found at {:?}",
                        status.code().map(|c| c.to_string()).unwrap_or_else(|| "unknown".to_string()),
                        mt5_exe
                    );
                }
            }
            Ok(None) => {
                // Process still running - continue polling
                std::thread::sleep(poll_interval);
            }
            Err(e) => {
                let _ = child.kill();
                bail!("Error checking installer process status: {}", e);
            }
        }
    }
}

async fn launch_mt5(prefix_path: &Path, mt5_executable: &Path, _db: Option<Arc<Database>>) -> Result<()> {
    if !mt5_executable.exists() {
        bail!(
            "MT5 executable not found at {:?}. Make sure MT5 is installed.",
            mt5_executable
        );
    }

    // Get Wine executable path
    let wine = get_wine_path()?;
    
    Command::new(wine)
        .env("WINEPREFIX", prefix_path)
        .arg(mt5_executable)
        .spawn()
        .context("Failed to launch MT5")?;

    println!("[INSTALLER] MT5 launched for prefix {:?}", prefix_path);

    Ok(())
}

