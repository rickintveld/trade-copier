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

/// Copy Expert Advisors from ./mql5/Trading Rocket/ to the MT5 MQL5/Experts directory
pub fn copy_expert_advisors(wine_prefix: &Path) -> Result<()> {
    println!("[INSTALLER] Copying Expert Advisors to MT5 directory");
    
    // Source directory (relative to current working directory)
    let source_dir = PathBuf::from("./mql5/Trading Rocket");
    if !source_dir.exists() {
        anyhow::bail!("Source directory not found: {:?}", source_dir);
    }
    
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
