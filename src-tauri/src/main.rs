// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod database;
mod installer;
mod router;
mod tauri_commands;
mod types;
mod worker;
mod worker_manager;

use anyhow::Result;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, Mutex};

use database::Database;
use tauri_commands::AppState;
use types::Trade;

const BROADCAST_CHANNEL_SIZE: usize = 1024;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Trade Copier Starting...");

    // Initialize database - use Tauri app data directory
    let app_data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("trade-copier");
    std::fs::create_dir_all(&app_data_dir)?;
    let db_path = app_data_dir.join("trade_copier.db");
    let db = Arc::new(Database::new(db_path.to_str().unwrap()).await?);
    println!("💾 Database initialized at {:?}", db_path);

    // Track start time for uptime calculation
    let start_time = Instant::now();

    // Create broadcast channel for trades
    let (tx, _rx) = broadcast::channel::<Trade>(BROADCAST_CHANNEL_SIZE);

    // Create worker manager
    let (worker_manager, command_rx) = worker_manager::WorkerManager::new(db.clone(), tx.clone());
    let worker_manager = Arc::new(worker_manager);

    // Get worker command sender for Tauri commands
    let worker_command_tx = Arc::new(Mutex::new(worker_manager.command_sender()));

    // Create app state for Tauri
    let app_state = AppState {
        db: db.clone(),
        worker_command_tx: worker_command_tx.clone(),
    };

    // Sync database state with actual running workers on startup
    // This handles the case where the app crashed and workers are marked active but not running
    if let Err(e) = worker_manager.sync_database_state().await {
        eprintln!("[STARTUP] Failed to sync worker database state: {}", e);
    }
    
    // Spawn worker manager event loop
    let worker_manager_clone = worker_manager.clone();
    tokio::spawn(async move {
        worker_manager_clone.run(command_rx).await;
    });

    // Spawn router
    tokio::spawn(async move {
        if let Err(e) = router::run_router(tx).await {
            eprintln!("[ROUTER] Error: {}", e);
        }
    });

    // Spawn system metrics collector (runs every 30 seconds)
    let metrics_db = db.clone();
    let metrics_start_time = start_time.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;

            let uptime_seconds = metrics_start_time.elapsed().as_secs();

            // Get worker counts from database
            let (total_workers, active_workers) = match metrics_db.get_all_workers().await {
                Ok(workers) => {
                    let total = workers.len() as i32;
                    let active = workers.iter().filter(|w| w.state == "active").count() as i32;
                    (total, active)
                }
                Err(_) => (0, 0),
            };

            // Update system metrics
            if let Err(e) = metrics_db
                .upsert_system_metrics("online", 5000, true, total_workers, active_workers, uptime_seconds)
                .await
            {
                eprintln!("[METRICS] Failed to update system metrics: {}", e);
            }
        }
    });

    println!("✅ Trade Copier backend is running");
    println!("📡 TCP Router listening on port 5000");

    // Build and run Tauri app
    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            tauri_commands::health_check,
            tauri_commands::get_workers,
            tauri_commands::get_trades,
            tauri_commands::get_errors,
            tauri_commands::get_system_metrics,
            tauri_commands::get_instances,
            tauri_commands::create_instance,
            tauri_commands::delete_instance,
            tauri_commands::start_instance,
            tauri_commands::stop_instance,
        ])
        .setup(|_app| {
            println!("🌐 Tauri app initialized");
            Ok(())
        })
        .on_window_event(|event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event.event() {
                println!("🛑 Window close requested, shutting down...");
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                println!("👋 Trade Copier stopped");
            }
        });

    Ok(())
}
