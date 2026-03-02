use anyhow::{bail, Context, Result};
use log::{info, warn, error};
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
use std::process::Command;

use crate::database::{Database, WorkerState};
use std::sync::Arc;

pub struct WindowsInstanceManager {}

impl WindowsInstanceManager {
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
        // Generate instance path
        let instance_path = self.generate_instance_path(&name)?;

        info!("[INSTALLER] Creating directory at {:?}", instance_path);

        // Create the directory
        fs::create_dir_all(&instance_path)
            .context(format!("Failed to create directory '{:?}'", instance_path))?;

        info!("[INSTALLER] Installing MT5 from {:?}", installer_path);

        #[cfg(target_os = "windows")]
        {
            // Run MT5 installer on Windows
            if let Err(e) = self.install_mt5_windows(&instance_path, installer_path) {
                let error_msg = format!("MT5 installation failed: {}", e);
                error!("[INSTALLER] {}", error_msg);
                error!("[INSTALLER] The instance was created but MT5 installation incomplete.");
                error!("[INSTALLER] You can delete it with: DELETE /api/instances/{}", name);
                // Log to worker_errors table
                let _ = db.insert_worker_error(&address, crate::database::ErrorSeverity::Critical, &error_msg).await;
                return Err(e);
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            info!("[INSTALLER] [DRY RUN - not on Windows] Would install MT5 to {:?}", instance_path);
        }

        // Copy Expert Advisors after successful installation
        if let Err(e) = super::common::copy_expert_advisors(&instance_path) {
            let error_msg = format!("Failed to copy Expert Advisors: {}. You can manually copy them from ./src/mql5/Trading Rocket/", e);
            warn!("[INSTALLER] {}", error_msg);
            let _ = db.insert_worker_error(&address, crate::database::ErrorSeverity::Warning, &error_msg).await;
        }
        
        // Copy Default.tpl template with worker port configuration
        if let Err(e) = super::common::copy_default_template(&instance_path, &address) {
            let error_msg = format!("Failed to copy Default.tpl template: {}. You can manually copy it from ./src/mql5/Profiles/Templates/", e);
            warn!("[INSTALLER] {}", error_msg);
            let _ = db.insert_worker_error(&address, crate::database::ErrorSeverity::Warning, &error_msg).await;
        }

        // Save worker to database with instance path (Windows doesn't use Wine)
        let path_str = instance_path.to_string_lossy().to_string();
        db.upsert_worker(crate::database::WorkerUpsertConfig {
            name: name.clone(),
            address: address.clone(),
            multiplier,
            state: WorkerState::Inactive,
            error: None,
            wine_prefix: Some(path_str),
            symbol_prefix,
        }).await?;

        info!("[INSTALLER] Instance '{}' created successfully!", name);
        info!("[INSTALLER]   Path: {:?}", instance_path);
        info!("[INSTALLER]   Address: {}", address);
        info!("[INSTALLER]   Multiplier: {}", multiplier);
        info!("[INSTALLER] NOTE: Worker will be activated automatically after installation completes");

        Ok(())
    }

    pub async fn delete_instance(&self, worker: &crate::database::WorkerRecord, force: bool, db: Arc<Database>) -> Result<()> {
        if !force {
            info!("[INSTALLER] Warning: This will delete instance '{}' and all its data", worker.name);
            info!("[INSTALLER] Use force=true to confirm deletion");
            bail!("Deletion cancelled - use force=true to confirm");
        }

        // Stop the MT5 process for this specific instance
        #[cfg(target_os = "windows")]
        {
            // Find and kill only processes running from this instance's directory
            // Use wmic to find PIDs of terminal64.exe with specific path
            if let Some(instance_prefix) = &worker.wine_prefix {
                let instance_path = PathBuf::from(instance_prefix);
                let exe_path = instance_path.join("terminal64.exe");
                let exe_path_str = exe_path.to_string_lossy().replace("\\", "\\\\");
                
                // Query for processes with this specific executable path
                let query = format!("process where ExecutablePath='{}' get ProcessId", exe_path_str);
                let output = Command::new("wmic")
                    .args(&["process", "where", &format!("ExecutablePath='{}'", exe_path_str), "get", "ProcessId"])
                    .output();
                
                if let Ok(result) = output {
                    let stdout = String::from_utf8_lossy(&result.stdout);
                    // Parse PIDs from output and kill each one
                    for line in stdout.lines().skip(1) { // Skip header
                        if let Ok(pid) = line.trim().parse::<u32>() {
                            let _ = Command::new("taskkill")
                                .args(&["/F", "/PID", &pid.to_string()])
                                .output();
                            info!("[INSTALLER] Killed MT5 process (PID: {}) for instance '{}'", pid, worker.name);
                        }
                    }
                }
            }
        }

        // Delete instance directory if it exists
        if let Some(wine_prefix) = &worker.wine_prefix {
            let instance_path = PathBuf::from(wine_prefix);
            if instance_path.exists() {
                fs::remove_dir_all(&instance_path)?;
                info!("[INSTALLER] Removed directory: {:?}", instance_path);
            }
        }

        // Remove from database using ID
        db.delete_worker_by_id(worker.id).await?;

        info!("[INSTALLER] Instance '{}' deleted successfully", worker.name);
        info!("[INSTALLER] NOTE: Workers will be reloaded automatically");

        Ok(())
    }

    pub async fn start_instance(&self, worker: &crate::database::WorkerRecord, force: bool, db: Arc<Database>) -> Result<()> {
        let wine_prefix = worker.wine_prefix.as_ref()
            .context("Instance does not have a path configured")?;
        let instance_path = PathBuf::from(wine_prefix);

        // If force is true, kill any existing MT5 processes
        if force {
            info!("[INSTALLER] Force start requested, killing existing MT5 processes...");
            
            #[cfg(target_os = "windows")]
            {
                let output = Command::new("taskkill")
                    .args(&["/F", "/IM", "terminal64.exe"])
                    .output();
                
                if let Ok(result) = output {
                    if result.status.success() {
                        info!("[INSTALLER] Killed existing MT5 processes");
                    }
                }
            }
            
            // Give process time to fully terminate
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        }

        let exe_path = instance_path.join("terminal64.exe");

        if !exe_path.exists() {
            bail!(
                "MT5 executable not found at '{:?}'. Please install MT5 to this directory first.",
                exe_path
            );
        }

        // Copy Expert Advisors before starting
        if let Err(e) = super::common::copy_expert_advisors(&instance_path) {
            let error_msg = format!("Failed to copy Expert Advisors: {}", e);
            warn!("[INSTALLER] {}", error_msg);
            let _ = db.insert_worker_error(&worker.address, crate::database::ErrorSeverity::Warning, &error_msg).await;
        }
        
        // Copy Default.tpl template with worker port configuration before starting
        if let Err(e) = super::common::copy_default_template(&instance_path, &worker.address) {
            let error_msg = format!("Failed to copy Default.tpl template: {}", e);
            warn!("[INSTALLER] {}", error_msg);
            let _ = db.insert_worker_error(&worker.address, crate::database::ErrorSeverity::Warning, &error_msg).await;
        }

        #[cfg(target_os = "windows")]
        {
            let exe_path_str = exe_path.to_str()
                .context("Invalid UTF-8 in executable path")?;
            if let Err(e) = Command::new("cmd")
                .args(&["/C", "start", "", exe_path_str])
                .spawn()
                .context(format!("Failed to launch instance '{}'", worker.name)) {
                let error_msg = format!("Failed to launch MT5: {}", e);
                error!("[INSTALLER] {}", error_msg);
                let _ = db.insert_worker_error(&worker.address, crate::database::ErrorSeverity::Critical, &error_msg).await;
                return Err(e);
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            info!("[INSTALLER] [DRY RUN - not on Windows] Would execute: {:?}", exe_path);
        }

        info!("[INSTALLER] Instance '{}' started", worker.name);

        Ok(())
    }

    pub async fn stop_instance(&self, worker: &crate::database::WorkerRecord, _db: Arc<Database>) -> Result<()> {
        let wine_prefix = worker.wine_prefix.as_ref()
            .context("Instance does not have a path configured")?;
        let instance_path = PathBuf::from(wine_prefix);

        let exe_path = instance_path.join("terminal64.exe");
        let exe_name = "terminal64.exe";

        #[cfg(target_os = "windows")]
        {
            // Use taskkill to terminate the process
            let output = Command::new("taskkill")
                .args(&["/F", "/IM", exe_name])
                .output()
                .context("Failed to execute taskkill")?;

            if output.status.success() {
                info!("[INSTALLER] Successfully stopped MT5 process for instance '{}'", worker.name);
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!("[INSTALLER] Failed to stop MT5 process: {}", stderr);
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            info!("[INSTALLER] [DRY RUN - not on Windows] Would kill process for: {:?}", exe_path);
        }

        info!("[INSTALLER] Instance '{}' stopped", worker.name);

        Ok(())
    }

    pub async fn list_instances(&self, db: Arc<Database>) -> Result<Vec<crate::database::WorkerRecord>> {
        db.get_all_workers().await
    }

    fn generate_instance_path(&self, name: &str) -> Result<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            Ok(PathBuf::from(format!("C:\\MT5-{}", name)))
        }

        #[cfg(not(target_os = "windows"))]
        {
            let home = std::env::var("HOME")?;
            Ok(PathBuf::from(home).join(format!("MT5-{}", name)))
        }
    }

    #[cfg(target_os = "windows")]
    fn install_mt5_windows(&self, _install_path: &Path, installer_path: &Path) -> Result<()> {
        // Run the installer
        let status = Command::new(installer_path)
            .status()
            .context("Failed to run MT5 installer")?;

        if !status.success() {
            bail!("MT5 installation failed");
        }

        info!("[INSTALLER] MT5 installed successfully");
        Ok(())
    }
}
