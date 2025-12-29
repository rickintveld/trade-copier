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
}

/// Manages the lifecycle of worker tasks
pub struct WorkerManager {
    db: Arc<Database>,
    trade_tx: broadcast::Sender<Trade>,
    workers: Arc<RwLock<HashMap<String, WorkerHandle>>>,
    command_tx: mpsc::Sender<WorkerCommand>,
}

struct WorkerHandle {
    address: String,
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
        };
        
        // Parse wine prefix path if present
        let wine_prefix = worker_cfg.wine_prefix.as_ref().map(|p| std::path::PathBuf::from(p));
        
        let handle = tokio::spawn(async move {
            if let Err(e) = worker::run_worker(slave.clone(), rx, db_clone, worker_shutdown_rx, wine_prefix).await {
                eprintln!("[WORKER:{}] Error: {}", slave.name, e);
            }
        });
        
        let worker_handle = WorkerHandle {
            address: worker_cfg.address.clone(),
            handle,
            shutdown_tx: worker_shutdown_tx,
        };
        
        let mut workers = self.workers.write().await;
        workers.insert(worker_cfg.name, worker_handle);
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
    pub async fn shutdown(&self) {
        println!("[WORKER_MANAGER] Shutting down...");
        self.stop_all_workers().await;
        
        // Update final worker states in database
        let workers = self.workers.read().await;
        for (name, worker) in workers.iter() {
            if let Err(e) = self.db.update_worker_state(
                &worker.address,
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
            }
        }
        
        println!("[WORKER_MANAGER] Event loop stopped");
    }
}
