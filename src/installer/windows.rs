use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
use std::process::Command;

use super::common::{Instance, InstanceConfig};
use crate::database::{Database, WorkerState};
use std::sync::Arc;

pub struct WindowsInstanceManager {
    config_path: PathBuf,
}

impl WindowsInstanceManager {
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "windows")]
        {
            let appdata = std::env::var("APPDATA")
                .context("APPDATA environment variable not found")?;
            let config_path = PathBuf::from(appdata)
                .join("mt5-manager")
                .join("config.json");
            
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent)
                    .context("Failed to create config directory")?;
            }
            
            Ok(Self { config_path })
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            let home = std::env::var("HOME")
                .context("HOME environment variable not found")?;
            let config_path = PathBuf::from(home)
                .join(".mt5-manager")
                .join("config.json");
            
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent)
                    .context("Failed to create config directory")?;
            }
            
            Ok(Self { config_path })
        }
    }

    pub async fn create_instance(
        &self,
        name: String,
        address: String,
        multiplier: f64,
        installer_path: &Path,
        db: Arc<Database>,
    ) -> Result<Instance> {
        let mut config = InstanceConfig::load(&self.config_path)?;

        // Check if instance name already exists
        if config.get_instance(&name).is_some() {
            bail!("Instance '{}' already exists", name);
        }

        // Generate instance ID and path
        let id = config.next_id();
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

        // Create instance metadata
        let instance = Instance::new(id, name.clone(), address.clone(), multiplier, instance_path.clone());

        // Save instance to local config
        config.add_instance(instance.clone())?;
        config.save(&self.config_path)?;
        
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
        println!("[INSTALLER]   ID: {}", id);
        println!("[INSTALLER]   Path: {:?}", instance_path);
        println!("[INSTALLER]   Address: {}", address);
        println!("[INSTALLER]   Multiplier: {}", multiplier);
        println!("[INSTALLER] NOTE: Worker will be activated automatically after installation completes");

        Ok(instance)
    }

    pub async fn delete_instance(&self, name: &str, force: bool) -> Result<()> {
        let mut config = InstanceConfig::load(&self.config_path)?;

        // Get instance
        let instance = config.get_instance(name)
            .context(format!("Instance '{}' not found", name))?
            .clone();

        if !force {
            println!("[INSTALLER] Warning: This will delete instance '{}' and all its data", name);
            println!("[INSTALLER] Use force=true to confirm deletion");
            bail!("Deletion cancelled - use force=true to confirm");
        }

        // Remove from local config first
        config.remove_instance(name)?;
        config.save(&self.config_path)?;

        // Delete instance directory
        if instance.path.exists() {
            fs::remove_dir_all(&instance.path)?;
            println!("[INSTALLER] Removed directory: {:?}", instance.path);
        }

        println!("[INSTALLER] Instance '{}' deleted successfully", name);
        println!("[INSTALLER] NOTE: Workers will be reloaded automatically");

        Ok(())
    }

    pub async fn start_instance(&self, name: &str) -> Result<()> {
        let config = InstanceConfig::load(&self.config_path)?;
        let instance = config.get_instance(name)
            .context(format!("Instance '{}' not found", name))?;

        let exe_path = instance.path.join("terminal64.exe");

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

    pub async fn start_all_instances(&self, _db: Option<std::sync::Arc<crate::database::Database>>) -> Result<()> {
        let config = InstanceConfig::load(&self.config_path)?;

        if config.instances.is_empty() {
            println!("[INSTALLER] No instances found");
            return Ok(());
        }

        println!("[INSTALLER] Starting {} instance(s)...", config.instances.len());
        for instance in &config.instances {
            self.start_instance(&instance.name).await?;
            // Small delay between launches
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }

        println!("[INSTALLER] All instances started");

        Ok(())
    }

    pub async fn list_instances(&self) -> Result<Vec<Instance>> {
        let config = InstanceConfig::load(&self.config_path)?;
        Ok(config.instances.clone())
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
