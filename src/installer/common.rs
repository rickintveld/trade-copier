use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

const MT5_INSTALLER_URL: &str = "https://download.mql5.com/cdn/web/metaquotes.ltd/mt5/mt5setup.exe";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub id: usize,
    pub name: String,
    pub address: String,
    pub multiplier: f64,
    pub path: PathBuf,
    pub created_at: String,
}

impl Instance {
    pub fn new(id: usize, name: String, address: String, multiplier: f64, path: PathBuf) -> Self {
        let created_at = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            name,
            address,
            multiplier,
            path,
            created_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstanceConfig {
    pub instances: Vec<Instance>,
}

impl InstanceConfig {
    pub fn new() -> Self {
        Self {
            instances: Vec::new(),
        }
    }

    pub fn load(config_path: &Path) -> Result<Self> {
        if !config_path.exists() {
            return Ok(Self::new());
        }

        let content = fs::read_to_string(config_path)
            .context("Failed to read instance config file")?;
        
        let config: InstanceConfig = serde_json::from_str(&content)
            .context("Failed to parse instance config file")?;
        
        Ok(config)
    }

    pub fn save(&self, config_path: &Path) -> Result<()> {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create config directory")?;
        }

        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize instance config")?;
        
        fs::write(config_path, content)
            .context("Failed to write instance config file")?;
        
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
            .context(format!("Instance '{}' not found", name))?;
        
        Ok(self.instances.remove(index))
    }

    pub fn get_instance(&self, name: &str) -> Option<&Instance> {
        self.instances.iter().find(|i| i.name == name)
    }

    pub fn next_id(&self) -> usize {
        self.instances.iter().map(|i| i.id).max().unwrap_or(0) + 1
    }
}

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
