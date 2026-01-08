pub mod common;
pub mod package_manager;

#[cfg(target_os = "macos")]
pub mod mac;

#[cfg(target_os = "windows")]
pub mod windows;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use crate::database::Database;

/// Platform-agnostic instance manager
pub struct InstanceManager {
    #[cfg(target_os = "macos")]
    inner: mac::MacInstanceManager,
    
    #[cfg(target_os = "windows")]
    inner: windows::WindowsInstanceManager,
    
    db: Arc<Database>,
}

impl InstanceManager {
    pub fn new(db: Arc<Database>) -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            Ok(Self {
                inner: mac::MacInstanceManager::new()?,
                db,
            })
        }
        
        #[cfg(target_os = "windows")]
        {
            Ok(Self {
                inner: windows::WindowsInstanceManager::new()?,
                db,
            })
        }
        
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            anyhow::bail!("Unsupported platform. Only macOS and Windows are supported.");
        }
    }

    /// Create a new MT5 instance
    /// Now creates DB record immediately (state: installing) and spawns background task
    pub async fn create_instance(
        &self,
        name: String,
        address: String,
        multiplier: f64,
        symbol_prefix: String,
        reload_tx: Option<mpsc::Sender<crate::worker_manager::WorkerCommand>>,
    ) -> Result<crate::database::WorkerRecord> {
        use crate::database::{WorkerState};
        
        // 1) Create worker in DB with 'installing' state so API can return quickly
        self.db
            .create_worker_with_state(&name, &address, multiplier, WorkerState::Installing)
            .await?;
        
        // 2) Spawn background task to perform heavy install work
        let db = self.db.clone();
        
        #[cfg(target_os = "macos")]
        let inner = mac::MacInstanceManager::new();
        
        #[cfg(target_os = "windows")]
        let inner = windows::WindowsInstanceManager::new();
        
        let name_bg = name.clone();
        let address_bg = address.clone();
        let symbol_prefix_bg = symbol_prefix.clone();
        tokio::spawn(async move {
            if let Err(e) = async {
                let inner = inner?; // Unpack Result from new()
                // Download installer
                let installer_path = common::download_mt5_installer().await?;
                // Run platform-specific creation (may install MT5)
                let _instance = inner.create_instance(name_bg.clone(), address_bg.clone(), multiplier, symbol_prefix_bg.clone(), &installer_path, db.clone()).await?;
                // Clean up
                if let Err(e) = std::fs::remove_file(&installer_path) {
                    eprintln!("[INSTALLER] Warning: Failed to remove installer: {}", e);
                }
                // Mark as inactive after installation completes - worker will activate when application starts
                db.update_worker_state(&address_bg, WorkerState::Inactive, None).await?;
                
                // Trigger worker reload if callback provided
                if let Some(tx) = reload_tx {
                    if let Err(e) = tx.send(crate::worker_manager::WorkerCommand::Reload).await {
                        eprintln!("[INSTALLER] Failed to trigger worker reload: {}", e);
                    } else {
                        println!("[INSTALLER] Triggered worker reload for new instance '{}'", name_bg);
                    }
                }
                
                anyhow::Ok(())
            }.await {
                let error_msg = format!("installation failed: {}", e);
                eprintln!("[INSTALLER] Background install for '{}' failed: {}", name_bg, e);
                
                // Update worker state
                let _ = db.update_worker_state(&address_bg, WorkerState::Error, Some(&error_msg)).await;
                
                // Log to worker_errors table for visibility in UI
                use crate::database::ErrorSeverity;
                if let Err(log_err) = db.insert_worker_error(&address_bg, ErrorSeverity::Critical, &error_msg).await {
                    eprintln!("[INSTALLER] Failed to log installation error to database: {}", log_err);
                }
            }
        });
        
        // 3) Return the worker record from database
        // Consumers should poll GET /api/instances or check worker state for progress.
        let worker = self.db.get_worker_by_name(&name).await?
            .ok_or_else(|| anyhow::anyhow!("Worker not found after creation"))?;
        Ok(worker)
    }

    /// Delete an existing MT5 instance
    pub async fn delete_instance(&self, name: &str, force: bool) -> Result<()> {
        // Delete instance files and database entry
        self.inner.delete_instance(name, force, self.db.clone()).await?;
        Ok(())
    }

    /// Start a specific MT5 instance
    pub async fn start_instance(&self, name: &str, force: bool) -> Result<()> {
        self.inner.start_instance(name, force, self.db.clone()).await
    }

    /// Stop a specific MT5 instance
    pub async fn stop_instance(&self, name: &str) -> Result<()> {
        self.inner.stop_instance(name, self.db.clone()).await
    }

    /// List all MT5 instances
    pub async fn list_instances(&self) -> Result<Vec<crate::database::WorkerRecord>> {
        self.inner.list_instances(self.db.clone()).await
    }
}
