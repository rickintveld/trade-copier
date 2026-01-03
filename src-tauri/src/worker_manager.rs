use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, watch, RwLock, mpsc};
use tokio::task::JoinHandle;

use crate::database::Database;
use crate::types::{Trade, SlaveConfig};
use crate::worker;

/// Commands for the worker manager
#[derive(Debug)]
pub enum WorkerCommand {
    /// Reload all workers from database
    Reload,
    /// Start a specific worker by name
    Start(String),
    /// Stop a specific worker by name
    Stop(String),
}

/// Manages the lifecycle of worker tasks
pub struct WorkerManager {
    db: Arc<Database>,
    trade_tx: broadcast::Sender<Trade>,
    workers: Arc<RwLock<HashMap<String, WorkerHandle>>>,
    command_tx: mpsc::Sender<WorkerCommand>,
}

struct WorkerHandle {
    _address: String,
    handle: JoinHandle<()>,
    shutdown_tx: watch::Sender<bool>,
}

impl WorkerManager {
    pub fn new(
        db: Arc<Database>,
        trade_tx: broadcast::Sender<Trade>,
    ) -> (Self, mpsc::Receiver<WorkerCommand>) {
        let (command_tx, command_rx) = mpsc::channel(32);
        
        let manager = Self {
            db,
            trade_tx,
            workers: Arc::new(RwLock::new(HashMap::new())),
            command_tx,
        };
        
        (manager, command_rx)
    }

    /// Get a sender for worker commands
    pub fn command_sender(&self) -> mpsc::Sender<WorkerCommand> {
        self.command_tx.clone()
    }

    /// Sync database state with actual running workers on startup
    /// Updates any workers marked as "active" in the database but not actually running
    pub async fn sync_database_state(&self) -> Result<()> {
        println!("🔄 Syncing worker database state with running workers...");
        
        // Get all workers from database
        let all_workers = self.db.get_all_workers().await?;
        
        // Get currently running workers
        let running_workers = self.workers.read().await;
        
        let mut synced_count = 0;
        
        for worker_record in all_workers {
            // Check if worker is marked as active in database
            if worker_record.state == "active" {
                // Check if it's actually running
                if !running_workers.contains_key(&worker_record.name) {
                    // Worker is marked active but not running - update to inactive
                    println!(
                        "  ⚠️  Worker '{}' @ {} is marked active but not running, updating to inactive",
                        worker_record.name, worker_record.address
                    );
                    
                    if let Err(e) = self.db.update_worker_state(
                        &worker_record.address,
                        crate::database::WorkerState::Inactive,
                        Some("Application restart detected - worker was not running"),
                    ).await {
                        eprintln!("[WORKER_MANAGER] Failed to update state for '{}': {}", worker_record.name, e);
                    }
                    
                    // Set mt5_connected to false
                    if let Err(e) = self.db.update_mt5_connected(&worker_record.address, false).await {
                        eprintln!("[WORKER_MANAGER] Failed to update mt5_connected for '{}': {}", worker_record.name, e);
                    }
                    
                    synced_count += 1;
                }
            }
        }
        
        if synced_count > 0 {
            println!("✅ Synced {} worker(s) to inactive state", synced_count);
        } else {
            println!("✅ All worker states are in sync");
        }
        
        Ok(())
    }
    
    /// Load and start all workers from the database
    pub async fn load_workers(&self) -> Result<()> {
        let workers = self.db.get_worker_configs().await?;
        
        println!("📋 Loading {} worker(s) from database", workers.len());
        
        for worker_cfg in workers {
            println!(
                "  - {} @ {} (multiplier: {}x)",
                worker_cfg.name, worker_cfg.address, worker_cfg.multiplier
            );
            
            self.spawn_worker(worker_cfg).await;
        }
        
        Ok(())
    }

    /// Reload all workers: stop existing ones and start fresh from database
    pub async fn reload_workers(&self) -> Result<()> {
        println!("🔄 Reloading workers...");
        
        // Stop all existing workers
        self.stop_all_workers().await;
        
        // Load and start workers from database
        self.load_workers().await?;
        
        println!("✅ Workers reloaded successfully");
        Ok(())
    }

    /// Start a specific worker by name (load from database and spawn)
    pub async fn start_worker(&self, name: &str) -> Result<()> {
        // Check if worker is already running
        {
            let workers = self.workers.read().await;
            if workers.contains_key(name) {
                return Err(anyhow::anyhow!("Worker '{}' is already running", name));
            }
        }
        
        // Load worker configuration from database
        let workers = self.db.get_worker_configs().await?;
        let worker_cfg = workers.iter()
            .find(|w| w.name == name)
            .ok_or_else(|| anyhow::anyhow!("Worker '{}' not found in database", name))?;
        
        println!("[WORKER_MANAGER] Starting worker '{}' @ {}", worker_cfg.name, worker_cfg.address);
        
        self.spawn_worker(worker_cfg.clone()).await;
        
        Ok(())
    }
    
    /// Spawn a single worker
    async fn spawn_worker(&self, worker_cfg: crate::database::WorkerConfig) {
        let rx = self.trade_tx.subscribe();
        let db_clone = self.db.clone();
        
        // Create a dedicated shutdown channel for this worker
        let (worker_shutdown_tx, worker_shutdown_rx) = watch::channel(false);
        
        // Convert WorkerConfig to SlaveConfig
        let slave = SlaveConfig {
            name: worker_cfg.name.clone(),
            address: worker_cfg.address.clone(),
            multiplier: worker_cfg.multiplier,
            symbol_prefix: worker_cfg.symbol_prefix.clone(),
        };
        
        // Parse wine prefix path if present
        let wine_prefix = worker_cfg.wine_prefix.as_ref().map(|p| std::path::PathBuf::from(p));
        
        let handle = tokio::spawn(async move {
            if let Err(e) = worker::run_worker(slave.clone(), rx, db_clone, worker_shutdown_rx, wine_prefix).await {
                eprintln!("[WORKER:{}] Error: {}", slave.name, e);
            }
        });
        
        let worker_handle = WorkerHandle {
            _address: worker_cfg.address.clone(),
            handle,
            shutdown_tx: worker_shutdown_tx,
        };
        
        let mut workers = self.workers.write().await;
        workers.insert(worker_cfg.name, worker_handle);
    }

    /// Stop a specific worker by name (renamed to stop_worker_internal to avoid conflict)
    async fn stop_worker_internal(&self, name: &str) -> Result<()> {
        let mut workers = self.workers.write().await;
        
        if let Some(worker) = workers.remove(name) {
            println!("[WORKER_MANAGER] Stopping worker '{}'", name);
            
            // Send shutdown signal
            let _ = worker.shutdown_tx.send(true);
            
            // Wait for worker to finish with timeout
            let shutdown_timeout = tokio::time::Duration::from_secs(5);
            match tokio::time::timeout(shutdown_timeout, worker.handle).await {
                Ok(Ok(())) => {
                    println!("[WORKER_MANAGER] Worker '{}' stopped successfully", name);
                    Ok(())
                }
                Ok(Err(e)) => {
                    eprintln!("[WORKER_MANAGER] Worker '{}' error: {}", name, e);
                    Err(anyhow::anyhow!("Worker task error: {}", e))
                }
                Err(_) => {
                    eprintln!("[WORKER_MANAGER] Worker '{}' stop timeout", name);
                    Err(anyhow::anyhow!("Worker stop timeout"))
                }
            }
        } else {
            // Worker not found in running workers map
            // Check if it exists in database and update its state to Inactive
            println!("[WORKER_MANAGER] Worker '{}' not running, checking database...", name);
            
            match self.db.get_worker_by_name(name).await {
                Ok(Some(worker_record)) => {
                    // Worker exists in database, update state to Inactive
                    if let Err(e) = self.db.update_worker_state(
                        &worker_record.address,
                        crate::database::WorkerState::Inactive,
                        None,
                    ).await {
                        eprintln!("[WORKER_MANAGER] Failed to update state for '{}': {}", name, e);
                        return Err(anyhow::anyhow!("Failed to update worker state: {}", e));
                    }
                    
                    // Set mt5_connected to false
                    if let Err(e) = self.db.update_mt5_connected(&worker_record.address, false).await {
                        eprintln!("[WORKER_MANAGER] Failed to update mt5_connected for '{}': {}", name, e);
                    }
                    
                    println!("[WORKER_MANAGER] Worker '{}' state updated to Inactive", name);
                    Ok(())
                }
                Ok(None) => {
                    Err(anyhow::anyhow!("Worker '{}' not found", name))
                }
                Err(e) => {
                    eprintln!("[WORKER_MANAGER] Database error checking worker '{}': {}", name, e);
                    Err(anyhow::anyhow!("Database error: {}", e))
                }
            }
        }
    }
    
    /// Stop all workers
    async fn stop_all_workers(&self) {
        let mut workers = self.workers.write().await;
        
        println!("[WORKER_MANAGER] Stopping {} worker(s)...", workers.len());
        
        // Send shutdown signals to all workers
        for (name, worker) in workers.iter() {
            let _ = worker.shutdown_tx.send(true);
            println!("[WORKER_MANAGER] Sent shutdown signal to '{}'", name);
        }
        
        // Wait for all workers to finish
        let shutdown_timeout = tokio::time::Duration::from_secs(5);
        for (name, worker) in workers.drain() {
            match tokio::time::timeout(shutdown_timeout, worker.handle).await {
                Ok(Ok(())) => println!("[WORKER_MANAGER] Worker '{}' stopped", name),
                Ok(Err(e)) => eprintln!("[WORKER_MANAGER] Worker '{}' error: {}", name, e),
                Err(_) => eprintln!("[WORKER_MANAGER] Worker '{}' timeout", name),
            }
        }
    }

    /// Shutdown the manager and all workers
    pub async fn _shutdown(&self) {
        println!("[WORKER_MANAGER] Shutting down...");
        self.stop_all_workers().await;
        
        // Update final worker states in database
        let workers = self.workers.read().await;
        for (name, worker) in workers.iter() {
            if let Err(e) = self.db.update_worker_state(
                &worker._address,
                crate::database::WorkerState::Inactive,
                None,
            ).await {
                eprintln!("[WORKER_MANAGER] Failed to update final state for '{}': {}", name, e);
            }
        }
    }

    /// Run the worker manager event loop
    pub async fn run(self: Arc<Self>, mut command_rx: mpsc::Receiver<WorkerCommand>) {
        println!("[WORKER_MANAGER] Event loop started");
        
        while let Some(command) = command_rx.recv().await {
            match command {
                WorkerCommand::Reload => {
                    if let Err(e) = self.reload_workers().await {
                        eprintln!("[WORKER_MANAGER] Failed to reload workers: {}", e);
                    }
                }
                WorkerCommand::Start(name) => {
                    if let Err(e) = self.start_worker(&name).await {
                        eprintln!("[WORKER_MANAGER] Failed to start worker '{}': {}", name, e);
                    }
                }
                WorkerCommand::Stop(name) => {
                    if let Err(e) = self.stop_worker_internal(&name).await {
                        eprintln!("[WORKER_MANAGER] Failed to stop worker '{}': {}", name, e);
                    }
                }
            }
        }
        
        println!("[WORKER_MANAGER] Event loop stopped");
    }
}
