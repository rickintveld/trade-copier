use anyhow::{Context, Result};
use chrono::Utc;
use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

#[cfg(target_os = "windows")]
use std::process::Command;

use crate::config::{Config, Instance};

pub struct InstanceManager {
    config: Config,
}

impl InstanceManager {
    pub fn new() -> Result<Self> {
        let config = Config::load()?;
        Ok(Self { config })
    }

    pub fn create_instance(&mut self, name: String, path: Option<PathBuf>) -> Result<()> {
        // Generate path if not provided
        let instance_path = if let Some(p) = path {
            p
        } else {
            // Default to C:\MT5-{name} on Windows
            #[cfg(target_os = "windows")]
            {
                PathBuf::from(format!("C:\\MT5-{}", name))
            }
            #[cfg(not(target_os = "windows"))]
            {
                // For testing on non-Windows platforms
                let home = std::env::var("HOME")?;
                PathBuf::from(home).join(format!("MT5-{}", name))
            }
        };

        // Check if directory already exists
        if instance_path.exists() {
            anyhow::bail!("Directory '{}' already exists", instance_path.display());
        }

        // Create the directory
        fs::create_dir_all(&instance_path)
            .context(format!("Failed to create directory '{}'", instance_path.display()))?;

        println!("✓ Created directory: {}", instance_path.display());

        // Create instance metadata
        let instance = Instance {
            name: name.clone(),
            path: instance_path.clone(),
            port: None,
            created_at: Utc::now(),
        };

        // Add to config
        self.config.add_instance(instance)?;
        self.config.save()?;

        println!("✓ Instance '{}' created successfully", name);
        println!();
        println!("Next steps:");
        println!("1. Install MT5 to: {}", instance_path.display());
        println!("2. Run the MT5 installer and select this directory as the installation path");
        println!("3. Once installed, use 'mt5-manager start {}' to launch", name);

        Ok(())
    }

    pub fn delete_instance(&mut self, name: &str, force: bool) -> Result<()> {
        // Get instance
        let instance = self.config.get_instance(name)
            .ok_or_else(|| anyhow::anyhow!("Instance '{}' not found", name))?
            .clone();

        // Confirm deletion if not forced
        if !force {
            println!("Are you sure you want to delete instance '{}'?", name);
            println!("Path: {}", instance.path.display());
            println!("This will remove all files and data in this directory.");
            println!();
            println!("To confirm, run with --force flag:");
            println!("  mt5-manager delete {} --force", name);
            return Ok(());
        }

        // Check if directory exists
        if instance.path.exists() {
            // Remove directory and all contents
            fs::remove_dir_all(&instance.path)
                .context(format!("Failed to remove directory '{}'", instance.path.display()))?;
            println!("✓ Removed directory: {}", instance.path.display());
        } else {
            println!("⚠ Directory not found: {}", instance.path.display());
        }

        // Remove from config
        self.config.remove_instance(name)?;
        self.config.save()?;

        println!("✓ Instance '{}' deleted successfully", name);

        Ok(())
    }

    pub fn start_instance(&self, name: Option<&str>) -> Result<()> {
        let instances_to_start: Vec<Instance> = if let Some(n) = name {
            // Start single instance
            let instance = self.config.get_instance(n)
                .ok_or_else(|| anyhow::anyhow!("Instance '{}' not found", n))?
                .clone();
            vec![instance]
        } else {
            // Start all instances
            if self.config.instances.is_empty() {
                println!("No instances configured. Create one with 'mt5-manager create <name>'");
                return Ok(());
            }
            self.config.instances.clone()
        };

        println!("Starting {} instance(s)...", instances_to_start.len());
        println!();

        for instance in instances_to_start {
            self.launch_instance(&instance)?;
            
            // Add delay between launches (as per documentation)
            thread::sleep(Duration::from_secs(2));
        }

        println!();
        println!("✓ All instances launched successfully");

        Ok(())
    }

    fn launch_instance(&self, instance: &Instance) -> Result<()> {
        let exe_path = instance.path.join("terminal64.exe");

        if !exe_path.exists() {
            anyhow::bail!(
                "MT5 executable not found at '{}'. Please install MT5 to this directory first.",
                exe_path.display()
            );
        }

        println!("▶ Launching instance '{}'...", instance.name);
        println!("  Path: {}", exe_path.display());

        #[cfg(target_os = "windows")]
        {
            Command::new("cmd")
                .args(&["/C", "start", "", exe_path.to_str().unwrap()])
                .spawn()
                .context(format!("Failed to launch instance '{}'", instance.name))?;
        }

        #[cfg(not(target_os = "windows"))]
        {
            // For testing on non-Windows platforms, just print what would be executed
            println!("  [DRY RUN - not on Windows] Would execute: {}", exe_path.display());
        }

        Ok(())
    }

    pub fn list_instances(&self) -> Result<()> {
        if self.config.instances.is_empty() {
            println!("No instances configured.");
            println!();
            println!("Create a new instance with:");
            println!("  mt5-manager create <name>");
            return Ok(());
        }

        println!("MT5 Instances:");
        println!();
        println!("{:<20} {:<50} {:<12}", "NAME", "PATH", "STATUS");
        println!("{}", "-".repeat(82));

        for instance in &self.config.instances {
            let status = if instance.path.join("terminal64.exe").exists() {
                "Installed"
            } else {
                "Not installed"
            };

            println!(
                "{:<20} {:<50} {:<12}",
                instance.name,
                instance.path.display().to_string(),
                status
            );
        }

        println!();
        println!("Total: {} instance(s)", self.config.instances.len());

        Ok(())
    }
}
