use anyhow::Result;
use log::{info, warn, error};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, watch, RwLock, mpsc};
use tokio::task::JoinHandle;

use crate::database::Database;
use crate::types::{Trade, SlaveConfig};
use crate::worker;

/// Generate a unique worker key from id and name
fn worker_key(id: i64, name: &str) -> String {
    format!("{}-{}", id, name)
}

/// Commands for the worker manager
#[derive(Debug)]
pub enum WorkerCommand {
    /// Start a specific worker by ID
    Start(i64),
    /// Stop a specific worker by ID
    Stop(i64),
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
        info!("🔄 Syncing worker database state with running workers...");
        
        // Get all workers from database
        let all_workers = self.db.get_all_workers().await?;
        
        // Get currently running workers
        let running_workers = self.workers.read().await;
        
        let mut synced_count = 0;
        
        for worker_record in all_workers {
            // Check if worker is marked as active in database
            if worker_record.state == "active" {
                // Check if it's actually running
                let key = worker_key(worker_record.id, &worker_record.name);
                if !running_workers.contains_key(&key) {
                    // Worker is marked active but not running - update to inactive
                    info!(
                        "  ⚠️  Worker '{}' @ {} is marked active but not running, updating to inactive",
                        worker_record.name, worker_record.address
                    );
                    
                    if let Err(e) = self.db.update_worker_state(
                        &worker_record.address,
                        crate::database::WorkerState::Inactive,
                        Some("Application restart detected - worker was not running"),
                    ).await {
                        error!("[WORKER_MANAGER] Failed to update state for '{}': {}", worker_record.name, e);
                    }
                    
                    // Set mt5_connected to false
                    if let Err(e) = self.db.update_mt5_connected(&worker_record.address, false).await {
                        error!("[WORKER_MANAGER] Failed to update mt5_connected for '{}': {}", worker_record.name, e);
                    }
                    
                    synced_count += 1;
                }
            }
        }
        
        if synced_count > 0 {
            info!("✅ Synced {} worker(s) to inactive state", synced_count);
        } else {
            info!("✅ All worker states are in sync");
        }
        
        Ok(())
    }
    

    /// Start a specific worker by ID (load from database and spawn)
    pub async fn start_worker(&self, worker_id: i64) -> Result<()> {
        // Load worker configuration from database
        let workers = self.db.get_worker_configs().await?;
        let worker_cfg = workers.iter()
            .find(|w| w.id == worker_id)
            .ok_or_else(|| anyhow::anyhow!("Worker with ID {} not found in database", worker_id))?;
        
        // Check if worker is already running
        let key = worker_key(worker_cfg.id, &worker_cfg.name);
        {
            let workers = self.workers.read().await;
            if workers.contains_key(&key) {
                return Err(anyhow::anyhow!("Worker '{}' (ID: {}) is already running", worker_cfg.name, worker_id));
            }
        }
        
        info!("[WORKER_MANAGER] Starting worker [{}] '{}' @ {}", worker_cfg.id, worker_cfg.name, worker_cfg.address);
        
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
        let wine_prefix = worker_cfg.wine_prefix.as_ref().map(std::path::PathBuf::from);
        
        let handle = tokio::spawn(async move {
            if let Err(e) = worker::run_worker(slave.clone(), rx, db_clone, worker_shutdown_rx, wine_prefix).await {
                error!("[WORKER:{}] Error: {}", slave.name, e);
            }
        });
        
        let worker_handle = WorkerHandle {
            _address: worker_cfg.address.clone(),
            handle,
            shutdown_tx: worker_shutdown_tx,
        };
        
        let key = worker_key(worker_cfg.id, &worker_cfg.name);
        let mut workers = self.workers.write().await;
        workers.insert(key, worker_handle);
    }

    /// Stop a specific worker by ID (renamed to stop_worker_internal to avoid conflict)
    async fn stop_worker_internal(&self, worker_id: i64) -> Result<()> {
        // First, get worker info from database to construct the key
        let worker_record = self.db.get_worker_by_id(worker_id).await?
            .ok_or_else(|| anyhow::anyhow!("Worker with ID {} not found", worker_id))?;
        
        let key = worker_key(worker_record.id, &worker_record.name);
        let mut workers = self.workers.write().await;
        
        if let Some(worker) = workers.remove(&key) {
            info!("[WORKER_MANAGER] Stopping worker [{}] '{}'", worker_id, worker_record.name);
            
            // Send shutdown signal
            let _ = worker.shutdown_tx.send(true);
            
            // Wait for worker to finish with timeout
            let shutdown_timeout = tokio::time::Duration::from_secs(5);
            match tokio::time::timeout(shutdown_timeout, worker.handle).await {
                Ok(Ok(())) => {
                    info!("[WORKER_MANAGER] Worker [{}] '{}' stopped successfully", worker_id, worker_record.name);
                    Ok(())
                }
                Ok(Err(e)) => {
                    error!("[WORKER_MANAGER] Worker [{}] '{}' error: {}", worker_id, worker_record.name, e);
                    Err(anyhow::anyhow!("Worker task error: {}", e))
                }
                Err(_) => {
                    error!("[WORKER_MANAGER] Worker [{}] '{}' stop timeout", worker_id, worker_record.name);
                    Err(anyhow::anyhow!("Worker stop timeout"))
                }
            }
        } else {
            // Worker not found in running workers map
            // Check if it exists in database and update its state to Inactive
            info!("[WORKER_MANAGER] Worker [{}] not running, checking database...", worker_id);
            
            match self.db.get_worker_by_id(worker_id).await {
                Ok(Some(worker_rec)) => {
                    // Worker exists in database, update state to Inactive
                    if let Err(e) = self.db.update_worker_state(
                        &worker_rec.address,
                        crate::database::WorkerState::Inactive,
                        None,
                    ).await {
                        error!("[WORKER_MANAGER] Failed to update state for [{}] '{}': {}", worker_id, worker_rec.name, e);
                        return Err(anyhow::anyhow!("Failed to update worker state: {}", e));
                    }
                    
                    // Set mt5_connected to false
                    if let Err(e) = self.db.update_mt5_connected(&worker_rec.address, false).await {
                        error!("[WORKER_MANAGER] Failed to update mt5_connected for [{}] '{}': {}", worker_id, worker_rec.name, e);
                    }
                    
                    info!("[WORKER_MANAGER] Worker [{}] '{}' state updated to Inactive", worker_id, worker_rec.name);
                    Ok(())
                }
                Ok(None) => {
                    Err(anyhow::anyhow!("Worker with ID {} not found", worker_id))
                }
                Err(e) => {
                    error!("[WORKER_MANAGER] Database error checking worker [{}]: {}", worker_id, e);
                    Err(anyhow::anyhow!("Database error: {}", e))
                }
            }
        }
    }

    /// Gracefully shut down all running workers, kill Wine processes, and update DB state
    pub async fn shutdown_all(&self) {
        info!("[WORKER_MANAGER] Shutting down all workers...");

        // 1. Drain all running workers and send shutdown signals
        let mut handles = Vec::new();
        {
            let mut workers = self.workers.write().await;
            for (key, worker) in workers.drain() {
                info!("[WORKER_MANAGER] Sending shutdown signal to '{}'", key);
                let _ = worker.shutdown_tx.send(true);
                handles.push((key, worker.handle));
            }
        }

        // 2. Wait for all worker tasks to finish (with timeout)
        for (key, handle) in handles {
            match tokio::time::timeout(Duration::from_secs(5), handle).await {
                Ok(Ok(())) => info!("[WORKER_MANAGER] Worker '{}' stopped cleanly", key),
                Ok(Err(e)) => error!("[WORKER_MANAGER] Worker '{}' task error: {}", key, e),
                Err(_) => warn!("[WORKER_MANAGER] Worker '{}' stop timed out, aborting", key),
            }
        }

        // 3. Kill any remaining Wine processes
        match self.db.get_worker_configs().await {
            Ok(configs) => {
                for cfg in configs {
                    if let Some(prefix) = &cfg.wine_prefix {
                        let path = std::path::PathBuf::from(prefix);
                        if let Err(e) = crate::worker::kill_wine_process(&path).await {
                            warn!("[WORKER_MANAGER] Failed to kill Wine for '{}': {}", cfg.name, e);
                        }
                    }
                }
            }
            Err(e) => error!("[WORKER_MANAGER] Failed to load worker configs for Wine cleanup: {}", e),
        }

        // 4. Mark all workers as inactive in the database
        match self.db.get_all_workers().await {
            Ok(workers) => {
                for w in workers.iter().filter(|w| w.state == "active") {
                    if let Err(e) = self.db.update_worker_state(
                        &w.address,
                        crate::database::WorkerState::Inactive,
                        Some("Application shutdown"),
                    ).await {
                        error!("[WORKER_MANAGER] Failed to deactivate '{}': {}", w.name, e);
                    }
                    if let Err(e) = self.db.update_mt5_connected(&w.address, false).await {
                        error!("[WORKER_MANAGER] Failed to update mt5_connected for '{}': {}", w.name, e);
                    }
                }
            }
            Err(e) => error!("[WORKER_MANAGER] Failed to load workers for DB cleanup: {}", e),
        }

        info!("[WORKER_MANAGER] All workers shut down");
    }

    /// Run the worker manager event loop
    pub async fn run(self: Arc<Self>, mut command_rx: mpsc::Receiver<WorkerCommand>) {
        info!("[WORKER_MANAGER] Event loop started");
        
        while let Some(command) = command_rx.recv().await {
            match command {
                WorkerCommand::Start(worker_id) => {
                    if let Err(e) = self.start_worker(worker_id).await {
                        error!("[WORKER_MANAGER] Failed to start worker [{}]: {}", worker_id, e);
                    }
                }
                WorkerCommand::Stop(worker_id) => {
                    if let Err(e) = self.stop_worker_internal(worker_id).await {
                        error!("[WORKER_MANAGER] Failed to stop worker [{}]: {}", worker_id, e);
                    }
                }
            }
        }
        
        info!("[WORKER_MANAGER] Event loop stopped");
    }
}
