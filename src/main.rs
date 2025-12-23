mod api;
mod database;
mod router;
mod types;
mod worker;

use anyhow::Result;
use std::fs;
use std::sync::Arc;
use tokio::sync::{broadcast, watch};
use database::Database;
use types::{Config, Trade};

const BROADCAST_CHANNEL_SIZE: usize = 1024;
const CONFIG_PATH: &str = "config/slaves.yaml";
const DB_PATH: &str = "trade_copier.db";

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Trade Copier Starting...");

    // Initialize database
    let db = Arc::new(Database::new(DB_PATH).await?);
    println!("💾 Database initialized at {}", DB_PATH);

    // Load configuration
    let config_content = fs::read_to_string(CONFIG_PATH)?;
    let config: Config = serde_yaml::from_str(&config_content)?;

    println!("📋 Loaded {} slave(s) from config", config.slaves.len());
    for slave in &config.slaves {
        println!(
            "  - {} @ {} (multiplier: {}x)",
            slave.name, slave.address, slave.multiplier
        );
    }

    // Create broadcast channel for trades
    let (tx, _rx) = broadcast::channel::<Trade>(BROADCAST_CHANNEL_SIZE);
    
    // Create shutdown signal channel
    let (shutdown_tx, _shutdown_rx) = watch::channel(false);

    // Spawn workers for each slave
    let mut worker_handles = vec![];
    let slaves = config.slaves.clone();
    for slave in config.slaves {
        let rx = tx.subscribe();
        let db_clone = db.clone();
        let shutdown_rx = shutdown_tx.subscribe();
        let handle = tokio::spawn(async move {
            if let Err(e) = worker::run_worker(slave.clone(), rx, db_clone, shutdown_rx).await {
                eprintln!("[WORKER:{}] Error: {}", slave.name, e);
            }
        });
        worker_handles.push(handle);
    }

    // Spawn router
    let router_handle = tokio::spawn(async move {
        if let Err(e) = router::run_router(tx).await {
            eprintln!("[ROUTER] Error: {}", e);
        }
    });
    
    // Spawn API server
    let api_db_clone = db.clone();
    let api_handle = tokio::spawn(async move {
        if let Err(e) = api::run_api(api_db_clone).await {
            eprintln!("[API] Error: {}", e);
        }
    });

    println!("✅ Trade Copier is running");
    println!("📡 TCP Router listening on port 5000");
    println!("🌐 HTTP API listening on port 8081");
    println!("⏳ Press Ctrl+C to stop\n");

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;
    println!("\n🛑 Shutting down...");

    // Send shutdown signal to all workers
    let _ = shutdown_tx.send(true);
    
    // Abort router and API server (don't need graceful shutdown)
    router_handle.abort();
    api_handle.abort();
    
    // Wait for all workers to finish gracefully (with timeout)
    let shutdown_timeout = tokio::time::Duration::from_secs(5);
    for (handle, slave) in worker_handles.into_iter().zip(slaves.iter()) {
        match tokio::time::timeout(shutdown_timeout, handle).await {
            Ok(Ok(())) => println!("[WORKER:{}] Shutdown complete", slave.name),
            Ok(Err(e)) => eprintln!("[WORKER:{}] Join error: {}", slave.name, e),
            Err(_) => {
                eprintln!("[WORKER:{}] Shutdown timeout, forcing abort", slave.name);
                // Worker will be aborted when handle is dropped
            }
        }
    }

    // Update any remaining workers to deactivated state
    for slave in &slaves {
        if let Err(e) = db.update_worker_state(
            &slave.address,
            database::WorkerState::Deactivated,
            None,
        ).await {
            eprintln!("[MAIN] Failed to update final state for {}: {}", slave.name, e);
        }
    }

    println!("👋 Trade Copier stopped");
    Ok(())
}
