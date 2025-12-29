mod api;
mod database;
mod installer;
mod router;
mod types;
mod worker;
mod worker_manager;

use anyhow::Result;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, watch};
use database::Database;
use types::Trade;

const BROADCAST_CHANNEL_SIZE: usize = 1024;
const DB_PATH: &str = "trade_copier.db";

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Trade Copier Starting...");

    // Initialize database
    let db = Arc::new(Database::new(DB_PATH).await?);
    println!("💾 Database initialized at {}", DB_PATH);
    
    // Track start time for uptime calculation
    let start_time = Instant::now();

    // Create broadcast channel for trades
    let (tx, _rx) = broadcast::channel::<Trade>(BROADCAST_CHANNEL_SIZE);
    
    // Create shutdown signal channel
    let (shutdown_tx, _shutdown_rx) = watch::channel(false);

    // Create worker manager
    let (worker_manager, command_rx) = worker_manager::WorkerManager::new(
        db.clone(),
        tx.clone(),
    );
    let worker_manager = Arc::new(worker_manager);
    
    // Load and start all workers
    worker_manager.load_workers().await?;
    
    // Get worker command sender for API
    let worker_command_tx = worker_manager.command_sender();
    
    // Spawn worker manager event loop
    let worker_manager_clone = worker_manager.clone();
    let worker_manager_handle = tokio::spawn(async move {
        worker_manager_clone.run(command_rx).await;
    });

    // Spawn router
    let router_handle = tokio::spawn(async move {
        if let Err(e) = router::run_router(tx).await {
            eprintln!("[ROUTER] Error: {}", e);
        }
    });
    
    // Spawn API server with worker command sender
    let api_db_clone = db.clone();
    let api_handle = tokio::spawn(async move {
        if let Err(e) = api::run_api(api_db_clone, worker_command_tx).await {
            eprintln!("[API] Error: {}", e);
        }
    });
    
    // Spawn system metrics collector (runs every 30 seconds)
    let metrics_db = db.clone();
    let metrics_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            
            let uptime_seconds = start_time.elapsed().as_secs();
            
            // Get worker counts from database
            let (total_workers, active_workers) = match metrics_db.get_all_workers().await {
                Ok(workers) => {
                    let total = workers.len() as i32;
                    let active = workers.iter().filter(|w| w.state == "activated").count() as i32;
                    (total, active)
                }
                Err(_) => (0, 0),
            };
            
            // Update system metrics
            if let Err(e) = metrics_db.upsert_system_metrics(
                "online",
                5000,
                true,
                total_workers,
                active_workers,
                uptime_seconds,
            ).await {
                eprintln!("[METRICS] Failed to update system metrics: {}", e);
            }
        }
    });

    println!("✅ Trade Copier is running");
    println!("📡 TCP Router listening on port 5000");
    println!("🌐 HTTP API listening on port 3000");
    println!("⏳ Press Ctrl+C to stop\n");

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;
    println!("\n🛑 Shutting down...");

    // Send shutdown signal
    let _ = shutdown_tx.send(true);
    
    // Abort router, API server, metrics collector, and worker manager (don't need graceful shutdown)
    router_handle.abort();
    api_handle.abort();
    metrics_handle.abort();
    worker_manager_handle.abort();
    
    // Shutdown worker manager and all workers
    worker_manager.shutdown().await;
    
    // Update final system metrics showing offline status
    let uptime_seconds = start_time.elapsed().as_secs();
    let (total_workers, _) = match db.get_all_workers().await {
        Ok(workers) => (workers.len() as i32, 0),
        Err(_) => (0, 0),
    };
    
    if let Err(e) = db.upsert_system_metrics(
        "offline",
        5000,
        false,
        total_workers,
        0,
        uptime_seconds,
    ).await {
        eprintln!("[MAIN] Failed to update shutdown metrics: {}", e);
    }

    println!("👋 Trade Copier stopped");
    Ok(())
}
