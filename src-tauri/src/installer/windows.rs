use anyhow::{bail, Context, Result};
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
        installer_path: &Path,
        db: Arc<Database>,
    ) -> Result<()> {
        // Generate instance path
        let instance_path = self.generate_instance_path(&name)?;

        println!("[INSTALLER] Creating directory at {:?}", instance_path);

        // Create the directory
        fs::create_dir_all(&instance_path)
            .context(format!("Failed to create directory '{:?}'", instance_path))?;

        println!("[INSTALLER] Installing MT5 from {:?}", installer_path);

        #[cfg(target_os = "windows")]
        {
            // Run MT5 installer on Windows
            if let Err(e) = self.install_mt5_windows(&instance_path, installer_path) {
                eprintln!("[INSTALLER] MT5 installation failed: {}", e);
                return Err(e);
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            println!("[INSTALLER] [DRY RUN - not on Windows] Would install MT5 to {:?}", instance_path);
        }

        // Save worker to database with instance path (Windows doesn't use Wine)
        let path_str = instance_path.to_string_lossy().to_string();
        db.upsert_worker(
            &name,
            &address,
            multiplier,
            WorkerState::Inactive,
            None,
            Some(&path_str),
        ).await?;

        println!("[INSTALLER] Instance '{}' created successfully!", name);
        println!("[INSTALLER]   Path: {:?}", instance_path);
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

        // Delete instance directory if it exists
        if let Some(wine_prefix) = &worker.wine_prefix {
            let instance_path = PathBuf::from(wine_prefix);
            if instance_path.exists() {
                fs::remove_dir_all(&instance_path)?;
                println!("[INSTALLER] Removed directory: {:?}", instance_path);
            }
        }

        // Remove from database
        db.delete_worker(name).await?;

        println!("[INSTALLER] Instance '{}' deleted successfully", name);
        println!("[INSTALLER] NOTE: Workers will be reloaded automatically");

        Ok(())
    }

    pub async fn start_instance(&self, name: &str, db: Arc<Database>) -> Result<()> {
        // Get worker from database
        let worker = db.get_worker_by_name(name).await?
            .context(format!("Instance '{}' not found", name))?;
        
        let wine_prefix = worker.wine_prefix
            .context("Instance does not have a path configured")?;
        let instance_path = PathBuf::from(wine_prefix);

        let exe_path = instance_path.join("terminal64.exe");

        if !exe_path.exists() {
            bail!(
                "MT5 executable not found at '{:?}'. Please install MT5 to this directory first.",
                exe_path
            );
        }

        #[cfg(target_os = "windows")]
        {
            Command::new("cmd")
                .args(&["/C", "start", "", exe_path.to_str().unwrap()])
                .spawn()
                .context(format!("Failed to launch instance '{}'", name))?;
        }

        #[cfg(not(target_os = "windows"))]
        {
            println!("[INSTALLER] [DRY RUN - not on Windows] Would execute: {:?}", exe_path);
        }

        println!("[INSTALLER] Instance '{}' started", name);

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

        println!("[INSTALLER] MT5 installed successfully");
        Ok(())
    }
}
