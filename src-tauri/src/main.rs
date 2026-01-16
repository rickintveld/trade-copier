// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod database;
mod dependency_manager;
mod ea_sync;
mod installer;
mod router;
mod tauri_commands;
mod types;
mod worker;
mod worker_manager;

use anyhow::Result;
use log::{info, error, warn};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, Mutex};

use database::Database;
use tauri_commands::AppState;
use types::Trade;

// Increased to 8192 to handle high-frequency trading without dropping messages
// This allows buffering ~8K trades before lagging workers cause drops
const BROADCAST_CHANNEL_SIZE: usize = 8192;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging - only in debug mode to avoid console output in release
    #[cfg(debug_assertions)]
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("Trade Copier starting...");

    // Sync Expert Advisors to master MT5 installation on startup
    // This ensures the latest EA version is always available in the master account
    if let Err(e) = ea_sync::sync_expert_advisors_to_master() {
        // Log warning but don't fail startup - user may not have master MT5 installed yet
        warn!("Failed to sync Expert Advisors to master MT5: {}", e);
        warn!("You can continue, but make sure to install the EAs manually or restart after installing MT5");
    }

    // Initialize database - use Tauri app data directory
    let app_data_dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get local data directory"))?
        .join("trade-copier");
    std::fs::create_dir_all(&app_data_dir)?;
    let db_path = app_data_dir.join("trade_copier.db");
    let db_path_str = db_path.to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid database path"))?;
    let db = Arc::new(Database::new(db_path_str).await?);
    info!("Database initialized at {:?}", db_path);

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
        warn!("Failed to sync worker database state: {}", e);
    }
    
    // Spawn worker manager event loop
    let worker_manager_clone = worker_manager.clone();
    tokio::spawn(async move {
        worker_manager_clone.run(command_rx).await;
    });

    // Spawn router
    let router_db = db.clone();
    tokio::spawn(async move {
        if let Err(e) = router::run_router(tx, router_db).await {
            error!("Router error: {}", e);
        }
    });

    // Spawn system metrics collector (runs every 30 seconds)
    let metrics_db = db.clone();
    let metrics_start_time = start_time;
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

            // Update system metrics (provider_connected defaults to false, will be updated by router)
            if let Err(e) = metrics_db
                .upsert_system_metrics("online", 5000, true, total_workers, active_workers, uptime_seconds, false)
                .await
            {
                error!("Failed to update system metrics: {}", e);
            }
        }
    });

    info!("Trade Copier backend is running");
    info!("TCP Router listening on port 5000");

    // Build and run Tauri app
    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            tauri_commands::health_check,
            tauri_commands::get_workers,
            tauri_commands::get_trades,
            tauri_commands::get_errors,
            tauri_commands::get_system_metrics,
            tauri_commands::get_account_balance_history,
            tauri_commands::get_instances,
            tauri_commands::create_instance,
            tauri_commands::delete_instance,
            tauri_commands::start_instance,
            tauri_commands::stop_instance,
            tauri_commands::get_dependency_status,
            tauri_commands::check_dependencies,
            tauri_commands::install_dependencies,
        ])
        .setup(move |app| {
            info!("Tauri app initialized");
            
            // Check dependencies on startup
            let db_clone = db.clone();
            let app_handle = app.handle();
            tokio::spawn(async move {
                if let Err(e) = dependency_manager::check_dependencies(db_clone, Some(&app_handle)).await {
                    error!("Failed to check dependencies on startup: {}", e);
                }
            });
            
            Ok(())
        })
        .on_window_event(|event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event.event() {
                info!("Window close requested, shutting down...");
            }
        })
        .build(tauri::generate_context!())
        .expect("Failed to build Tauri application - this is a critical error")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                info!("Trade Copier stopped");
            }
        });

    Ok(())
}
