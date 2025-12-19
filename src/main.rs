mod router;
mod types;
mod worker;

use anyhow::Result;
use std::fs;
use tokio::sync::broadcast;
use types::{Config, Trade};

const BROADCAST_CHANNEL_SIZE: usize = 1024;
const CONFIG_PATH: &str = "config/slaves.yaml";

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Trade Copier Starting...");

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

    // Create broadcast channel
    let (tx, _rx) = broadcast::channel::<Trade>(BROADCAST_CHANNEL_SIZE);

    // Spawn workers for each slave
    let mut worker_handles = vec![];
    for slave in config.slaves {
        let rx = tx.subscribe();
        let handle = tokio::spawn(async move {
            if let Err(e) = worker::run_worker(slave.clone(), rx).await {
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

    println!("✅ Trade Copier is running");
    println!("📡 Router listening on port 5000");
    println!("⏳ Press Ctrl+C to stop\n");

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;
    println!("\n🛑 Shutting down...");

    // Cleanup (tasks will be aborted when handles are dropped)
    router_handle.abort();
    for handle in worker_handles {
        handle.abort();
    }

    println!("👋 Trade Copier stopped");
    Ok(())
}
