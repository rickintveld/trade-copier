use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use super::common::{Instance, InstanceConfig};
use crate::database::{Database, WorkerState};

pub struct MacInstanceManager {
    config_path: PathBuf,
}

impl MacInstanceManager {
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        let config_dir = home.join(".mt5-manager");
        let config_path = config_dir.join("instances.json");

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .context("Failed to create config directory")?;
        }

        Ok(Self { config_path })
    }

    pub async fn create_instance(
        &self,
        name: String,
        address: String,
        multiplier: f64,
        installer_path: &Path,
        db: Arc<Database>,
    ) -> Result<Instance> {
        // Check Wine is installed
        check_wine_installed()?;

        let mut config = InstanceConfig::load(&self.config_path)?;

        // Check if instance name already exists
        if config.get_instance(&name).is_some() {
            bail!("Instance '{}' already exists", name);
        }

        // Generate instance ID and prefix path
        let id = config.next_id();
        let prefix_path = self.generate_prefix_path(id)?;

        println!("[INSTALLER] Creating Wine prefix at {:?}", prefix_path);

        // Create Wine prefix
        create_wine_prefix(&prefix_path)?;

        // Install MT5
        println!("[INSTALLER] Installing MT5 from {:?}", installer_path);
        if let Err(e) = install_mt5(&prefix_path, installer_path) {
            eprintln!("[INSTALLER] MT5 installation failed: {}", e);
            eprintln!("[INSTALLER] The instance was created but MT5 installation incomplete.");
            eprintln!("[INSTALLER] You can delete it with: DELETE /api/instances/{}", name);
            return Err(e);
        }

        // Create instance metadata
        let instance = Instance::new(id, name.clone(), address.clone(), multiplier, prefix_path.clone());

        // Save instance to local config
        config.add_instance(instance.clone())?;
        config.save(&self.config_path)?;
        
        // Save worker to database with wine_prefix
        let prefix_str = prefix_path.to_string_lossy().to_string();
        db.upsert_worker(
            &name,
            &address,
            multiplier,
            WorkerState::Inactive,
            None,
            Some(&prefix_str),
        ).await?;

        println!("[INSTALLER] Instance '{}' created successfully!", name);
        println!("[INSTALLER]   ID: {}", id);
        println!("[INSTALLER]   Prefix: {:?}", prefix_path);
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

        // Delete Wine prefix directory
        if instance.path.exists() {
            fs::remove_dir_all(&instance.path)?;
            println!("[INSTALLER] Removed Wine prefix: {:?}", instance.path);
        }

        println!("[INSTALLER] Instance '{}' deleted successfully", name);
        println!("[INSTALLER] NOTE: Workers will be reloaded automatically");

        Ok(())
    }

    pub async fn start_instance(&self, name: &str) -> Result<()> {
        check_wine_installed()?;

        let config = InstanceConfig::load(&self.config_path)?;
        let instance = config.get_instance(name)
            .context(format!("Instance '{}' not found", name))?;

        let mt5_exe = self.mt5_executable(&instance.path);
        launch_mt5(&instance.path, &mt5_exe, None).await?;

        println!("[INSTALLER] Instance '{}' started", name);

        Ok(())
    }

    pub async fn start_all_instances(&self, db: Option<Arc<Database>>) -> Result<()> {
        check_wine_installed()?;

        let config = InstanceConfig::load(&self.config_path)?;

        if config.instances.is_empty() {
            println!("[INSTALLER] No instances found");
            return Ok(());
        }

        println!("[INSTALLER] Starting {} instance(s)...", config.instances.len());
        for instance in &config.instances {
            let mt5_exe = self.mt5_executable(&instance.path);
            launch_mt5(&instance.path, &mt5_exe, db.clone()).await?;
            println!("[INSTALLER] Started instance '{}'", instance.name);
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

    fn generate_prefix_path(&self, id: usize) -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        Ok(home.join(format!(".wine-mt5-instance{}", id)))
    }

    fn mt5_executable(&self, prefix_path: &Path) -> PathBuf {
        prefix_path.join("drive_c/Program Files/MetaTrader 5/terminal64.exe")
    }
}

fn check_wine_installed() -> Result<()> {
    let output = Command::new("which")
        .arg("wine")
        .output()
        .context("Failed to check for Wine installation")?;

    if !output.status.success() {
        bail!("Wine is not installed. Install it with: brew install --cask wine-stable");
    }

    Ok(())
}

fn create_wine_prefix(prefix_path: &Path) -> Result<()> {
    if prefix_path.exists() {
        bail!("Wine prefix already exists at {:?}", prefix_path);
    }

    let status = Command::new("winecfg")
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

    // Spawn the installer without waiting for it to exit
    // (Wine keeps running even after installation completes)
    let mut child = Command::new("wine")
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

    Command::new("wine")
        .env("WINEPREFIX", prefix_path)
        .arg(mt5_executable)
        .spawn()
        .context("Failed to launch MT5")?;

    println!("[INSTALLER] MT5 launched for prefix {:?}", prefix_path);

    Ok(())
}
