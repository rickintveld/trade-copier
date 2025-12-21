use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub name: String,
    pub path: PathBuf,
    pub port: Option<u16>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub instances: Vec<Instance>,
}

impl Config {
    pub fn new() -> Self {
        Self {
            instances: Vec::new(),
        }
    }

    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        
        if !config_path.exists() {
            return Ok(Self::new());
        }

        let content = fs::read_to_string(&config_path)
            .context("Failed to read config file")?;
        
        let config: Config = serde_json::from_str(&content)
            .context("Failed to parse config file")?;
        
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create config directory")?;
        }

        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize config")?;
        
        fs::write(&config_path, content)
            .context("Failed to write config file")?;
        
        Ok(())
    }

    pub fn add_instance(&mut self, instance: Instance) -> Result<()> {
        if self.instances.iter().any(|i| i.name == instance.name) {
            anyhow::bail!("Instance '{}' already exists", instance.name);
        }
        self.instances.push(instance);
        Ok(())
    }

    pub fn remove_instance(&mut self, name: &str) -> Result<Instance> {
        let index = self.instances
            .iter()
            .position(|i| i.name == name)
            .ok_or_else(|| anyhow::anyhow!("Instance '{}' not found", name))?;
        
        Ok(self.instances.remove(index))
    }

    pub fn get_instance(&self, name: &str) -> Option<&Instance> {
        self.instances.iter().find(|i| i.name == name)
    }

    fn config_path() -> Result<PathBuf> {
        // On Windows, use %APPDATA%/mt5-manager/config.json
        // For development/testing on other platforms, use a local path
        #[cfg(target_os = "windows")]
        {
            let appdata = std::env::var("APPDATA")
                .context("APPDATA environment variable not found")?;
            Ok(PathBuf::from(appdata).join("mt5-manager").join("config.json"))
        }

        #[cfg(not(target_os = "windows"))]
        {
            // For testing on non-Windows platforms
            let home = std::env::var("HOME")
                .context("HOME environment variable not found")?;
            Ok(PathBuf::from(home).join(".mt5-manager").join("config.json"))
        }
    }
}
