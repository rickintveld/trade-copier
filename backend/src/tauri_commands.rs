use log::{info, warn, error};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, State};
use tokio::sync::{mpsc, Mutex};

use crate::database::Database;
use crate::dependency_manager;
use crate::installer::InstanceManager;
use crate::worker_manager::WorkerCommand;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub worker_command_tx: Arc<Mutex<mpsc::Sender<WorkerCommand>>>,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: T,
}

// Health check command
#[tauri::command]
pub async fn health_check() -> Result<ApiResponse<serde_json::Value>, String> {
    Ok(ApiResponse {
        success: true,
        data: serde_json::json!({
            "status": "ok",
            "message": "Trade Copier is running"
        }),
    })
}

// Get all workers
#[tauri::command]
pub async fn get_workers(state: State<'_, AppState>) -> Result<ApiResponse<serde_json::Value>, String> {
    match state.db.get_all_workers().await {
        Ok(workers) => Ok(ApiResponse {
            success: true,
            data: serde_json::to_value(workers).map_err(|e| e.to_string())?,
        }),
        Err(e) => Err(e.to_string()),
    }
}

// Get trades with optional limit
#[tauri::command]
pub async fn get_trades(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<ApiResponse<serde_json::Value>, String> {
    match state.db.get_all_trades(limit).await {
        Ok(trades) => Ok(ApiResponse {
            success: true,
            data: serde_json::to_value(trades).map_err(|e| e.to_string())?,
        }),
        Err(e) => Err(e.to_string()),
    }
}

// Get errors with optional limit
#[tauri::command]
pub async fn get_errors(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<ApiResponse<serde_json::Value>, String> {
    match state.db.get_all_errors(limit).await {
        Ok(errors) => Ok(ApiResponse {
            success: true,
            data: serde_json::to_value(errors).map_err(|e| e.to_string())?,
        }),
        Err(e) => Err(e.to_string()),
    }
}

// Get system metrics
#[tauri::command]
pub async fn get_system_metrics(state: State<'_, AppState>) -> Result<ApiResponse<serde_json::Value>, String> {
    match state.db.get_system_metrics().await {
        Ok(metrics) => Ok(ApiResponse {
            success: true,
            data: serde_json::to_value(metrics).map_err(|e| e.to_string())?,
        }),
        Err(e) => Err(e.to_string()),
    }
}

// Get profit history
#[tauri::command]
pub async fn get_profit_history(
    state: State<'_, AppState>,
    worker_id: Option<i64>,
    limit: Option<i64>,
) -> Result<ApiResponse<serde_json::Value>, String> {
    match state.db.get_profit_history(worker_id, limit).await {
        Ok(profits) => Ok(ApiResponse {
            success: true,
            data: serde_json::to_value(profits).map_err(|e| e.to_string())?,
        }),
        Err(e) => Err(e.to_string()),
    }
}

// Create new instance
#[tauri::command]
pub async fn create_instance(
    state: State<'_, AppState>,
    name: String,
    address: String,
    multiplier: f64,
    symbol_prefix: Option<String>,
) -> Result<ApiResponse<serde_json::Value>, String> {
    // Validate input using SlaveConfig
    let config = crate::types::SlaveConfig {
        name: name.clone(),
        address: address.clone(),
        multiplier,
        symbol_prefix: symbol_prefix.clone().unwrap_or_default(),
    };
    
    if let Err(e) = config.validate() {
        return Err(format!("Invalid configuration: {}", e));
    }
    
    match InstanceManager::new(state.db.clone()) {
        Ok(manager) => {
            let worker_tx = state.worker_command_tx.lock().await.clone();
            let prefix = symbol_prefix.unwrap_or_default();
            match manager
                .create_instance(name, address, multiplier, prefix, Some(worker_tx))
                .await
            {
                Ok(instance) => Ok(ApiResponse {
                    success: true,
                    data: serde_json::to_value(instance).map_err(|e| e.to_string())?,
                }),
                Err(e) => Err(e.to_string()),
            }
        }
        Err(e) => Err(format!("Failed to initialize instance manager: {}", e)),
    }
}

// Update instance
#[tauri::command]
pub async fn update_instance(
    state: State<'_, AppState>,
    id: i64,
    name: String,
    address: String,
    multiplier: f64,
    symbol_prefix: Option<String>,
) -> Result<ApiResponse<serde_json::Value>, String> {
    // Validate input using SlaveConfig
    let config = crate::types::SlaveConfig {
        name: name.clone(),
        address: address.clone(),
        multiplier,
        symbol_prefix: symbol_prefix.clone().unwrap_or_default(),
    };
    
    if let Err(e) = config.validate() {
        return Err(format!("Invalid configuration: {}", e));
    }
    
    // Get current worker info to check if it's running
    let worker = state.db.get_worker_by_id(id).await
        .map_err(|e| format!("Failed to get worker: {}", e))?
        .ok_or_else(|| format!("Worker with ID {} not found", id))?;
    
    let was_active = worker.state == "active";
    
    // Stop the worker if it's currently running
    if was_active {
        let tx = state.worker_command_tx.lock().await;
        if let Err(e) = tx.send(WorkerCommand::Stop(id)).await {
            error!("[TAURI] Failed to send stop command for update: {}", e);
        }
        drop(tx);
        // Give the worker time to shut down
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
    
    // Also stop the MT5 instance if it was running
    if was_active {
        match InstanceManager::new(state.db.clone()) {
            Ok(manager) => {
                if let Err(e) = manager.stop_instance(&worker).await {
                    warn!("[TAURI] Failed to stop MT5 instance during update: {}", e);
                }
            }
            Err(e) => {
                warn!("[TAURI] Failed to initialize instance manager during update: {}", e);
            }
        }
        
        // Poll until the TCP port is actually free (max ~10s)
        info!("[TAURI] Waiting for port to be released...");
        let mut port_free = false;
        for attempt in 1..=20 {
            match tokio::net::TcpListener::bind(&address).await {
                Ok(listener) => {
                    drop(listener);
                    info!("[TAURI] Port {} is now free (after {}ms)", address, attempt * 500);
                    port_free = true;
                    break;
                }
                Err(_) => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
            }
        }
        if !port_free {
            warn!("[TAURI] Port {} still in use after 10s, proceeding anyway", address);
        }
    }
    
    // Update the worker settings in the database
    let prefix = symbol_prefix.unwrap_or_default();
    state.db.update_worker_settings(id, &name, &address, multiplier, &prefix).await
        .map_err(|e| format!("Failed to update worker: {}", e))?;
    
    // Restart the worker if it was previously active
    if was_active {
        // Clear error state before restarting
        if let Err(e) = state.db.update_worker_state(
            &address,
            crate::database::WorkerState::Inactive,
            None,
        ).await {
            error!("[TAURI] Failed to clear error state: {}", e);
        }
        
        // Start the MT5 instance (use force=false since we already cleanly stopped)
        let updated_worker = state.db.get_worker_by_id(id).await
            .map_err(|e| format!("Failed to get updated worker: {}", e))?
            .ok_or_else(|| format!("Updated worker with ID {} not found", id))?;
        
        match InstanceManager::new(state.db.clone()) {
            Ok(manager) => {
                if let Err(e) = manager.start_instance(&updated_worker, false).await {
                    warn!("[TAURI] Failed to restart MT5 instance after update: {}", e);
                }
            }
            Err(e) => {
                warn!("[TAURI] Failed to initialize instance manager for restart: {}", e);
            }
        }
        
        // Start the worker
        let tx = state.worker_command_tx.lock().await;
        if let Err(e) = tx.send(WorkerCommand::Start(id)).await {
            return Err(format!("Worker updated but failed to restart: {}", e));
        }
    }
    
    Ok(ApiResponse {
        success: true,
        data: serde_json::json!({
            "message": format!("Worker '{}' updated successfully{}", name, if was_active { " and restarted" } else { "" })
        }),
    })
}

// Delete instance
#[tauri::command]
pub async fn delete_instance(
    state: State<'_, AppState>,
    id: i64,
    force: bool,
) -> Result<ApiResponse<serde_json::Value>, String> {
    // Get worker info from database
    let worker = state.db.get_worker_by_id(id).await
        .map_err(|e| format!("Failed to get worker: {}", e))?
        .ok_or_else(|| format!("Worker with ID {} not found", id))?;
    
    match InstanceManager::new(state.db.clone()) {
        Ok(manager) => match manager.delete_instance(&worker, force).await {
            Ok(()) => {
                // Stop only the specific worker for the deleted instance
                let tx = state.worker_command_tx.lock().await;
                if let Err(e) = tx.send(WorkerCommand::Stop(worker.id)).await {
                    error!("[TAURI] Failed to send stop command: {}", e);
                }

                Ok(ApiResponse {
                    success: true,
                    data: serde_json::json!({
                        "message": format!("Instance '{}' deleted successfully. Worker stopped.", worker.name)
                    }),
                })
            }
            Err(e) => Err(e.to_string()),
        },
        Err(e) => Err(format!("Failed to initialize instance manager: {}", e)),
    }
}

// Start instance
#[tauri::command]
pub async fn start_instance(
    state: State<'_, AppState>,
    id: i64,
    force: Option<bool>,
) -> Result<ApiResponse<serde_json::Value>, String> {
    let force = force.unwrap_or(false);
    
    // Get worker info from database
    let worker = state.db.get_worker_by_id(id).await
        .map_err(|e| format!("Failed to get worker: {}", e))?
        .ok_or_else(|| format!("Worker with ID {} not found", id))?;
    
    // If force is true, stop the existing worker first to release the TCP port
    if force {
        let tx = state.worker_command_tx.lock().await;
        if let Err(e) = tx.send(WorkerCommand::Stop(worker.id)).await {
            error!("[TAURI] Failed to send stop command before force start: {}", e);
        } else {
            // Give the worker time to shut down and release the port
            drop(tx); // Release the lock before sleeping
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    }
    
    match InstanceManager::new(state.db.clone()) {
        Ok(manager) => match manager.start_instance(&worker, force).await {
            Ok(()) => {
                // Clear any previous error state
                if let Err(e) = state.db.update_worker_state(
                    &worker.address,
                    crate::database::WorkerState::Inactive,
                    None,
                ).await {
                    error!("[TAURI] Failed to clear error state: {}", e);
                }
                
                // Now send start command to worker manager
                let tx = state.worker_command_tx.lock().await;
                if let Err(e) = tx.send(WorkerCommand::Start(worker.id)).await {
                    return Err(format!("MT5 instance started but failed to start worker: {}", e));
                }

                Ok(ApiResponse {
                    success: true,
                    data: serde_json::json!({
                        "message": format!("Instance '{}' and worker started successfully", worker.name)
                    }),
                })
            }
            Err(e) => Err(e.to_string()),
        },
        Err(e) => Err(format!("Failed to initialize instance manager: {}", e)),
    }
}

// Stop instance
#[tauri::command]
pub async fn stop_instance(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<serde_json::Value>, String> {
    // Get worker info from database
    let worker = state.db.get_worker_by_id(id).await
        .map_err(|e| format!("Failed to get worker: {}", e))?
        .ok_or_else(|| format!("Worker with ID {} not found", id))?;
    
    // First, stop the worker
    let tx = state.worker_command_tx.lock().await;
    if let Err(e) = tx.send(WorkerCommand::Stop(worker.id)).await {
        return Err(format!("Failed to send stop command: {}", e));
    }

    // Then, stop the MT5 instance (kill the Wine process)
    match InstanceManager::new(state.db.clone()) {
        Ok(manager) => {
            if let Err(e) = manager.stop_instance(&worker).await {
                warn!("[TAURI] Failed to stop MT5 instance: {}", e);
            }
        }
        Err(e) => {
            warn!("[TAURI] Failed to initialize instance manager: {}", e);
        }
    }

    Ok(ApiResponse {
        success: true,
        data: serde_json::json!({
            "message": format!("Worker '{}' and MT5 instance stopped successfully", worker.name)
        }),
    })
}

// Get dependency status
#[tauri::command]
pub async fn get_dependency_status(state: State<'_, AppState>) -> Result<ApiResponse<serde_json::Value>, String> {
    match dependency_manager::get_dependency_status(state.db.clone()).await {
        Ok(Some(status)) => Ok(ApiResponse {
            success: true,
            data: serde_json::to_value(status).map_err(|e| e.to_string())?,
        }),
        Ok(None) => Ok(ApiResponse {
            success: true,
            data: serde_json::json!(null),
        }),
        Err(e) => Err(e.to_string()),
    }
}

// Check dependencies
#[tauri::command]
pub async fn check_dependencies(
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<ApiResponse<serde_json::Value>, String> {
    match dependency_manager::check_dependencies(state.db.clone(), Some(&app_handle)).await {
        Ok(status) => Ok(ApiResponse {
            success: true,
            data: serde_json::to_value(status).map_err(|e| e.to_string())?,
        }),
        Err(e) => Err(e.to_string()),
    }
}

// Install dependencies
#[tauri::command]
pub async fn install_dependencies(
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<ApiResponse<serde_json::Value>, String> {
    match dependency_manager::install_dependencies(state.db.clone(), &app_handle).await {
        Ok(status) => Ok(ApiResponse {
            success: true,
            data: serde_json::to_value(status).map_err(|e| e.to_string())?,
        }),
        Err(e) => Err(e.to_string()),
    }
}

// Get feature toggles
#[tauri::command]
pub async fn get_feature_toggles(state: State<'_, AppState>) -> Result<ApiResponse<serde_json::Value>, String> {
    match state.db.get_feature_toggles().await {
        Ok(toggles) => Ok(ApiResponse {
            success: true,
            data: serde_json::to_value(toggles).map_err(|e| e.to_string())?,
        }),
        Err(e) => Err(e.to_string()),
    }
}

// Update a feature toggle
#[tauri::command]
pub async fn update_feature_toggle(
    state: State<'_, AppState>,
    key: String,
    enabled: bool,
) -> Result<ApiResponse<serde_json::Value>, String> {
    match state.db.update_feature_toggle(&key, enabled).await {
        Ok(()) => Ok(ApiResponse {
            success: true,
            data: serde_json::json!({
                "message": format!("Feature '{}' updated to {}", key, enabled)
            }),
        }),
        Err(e) => Err(e.to_string()),
    }
}

// Fetch economic calendar from FTMO API (bypasses CORS)
#[tauri::command]
pub async fn fetch_economic_calendar(
    date_from: String,
    date_to: String,
) -> Result<ApiResponse<serde_json::Value>, String> {
    let url = format!(
        "https://gw2.ftmo.com/public-api/v1/economic-calendar?dateFrom={}&dateTo={}&timezone=Europe/Amsterdam&forceUnrestricted=false",
        urlencoding::encode(&date_from),
        urlencoding::encode(&date_to)
    );

    let client = reqwest::Client::new();
    match client.get(&url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(data) => Ok(ApiResponse {
                        success: true,
                        data,
                    }),
                    Err(e) => Err(format!("Failed to parse response: {}", e)),
                }
            } else {
                Err(format!("API request failed with status: {}", response.status()))
            }
        }
        Err(e) => Err(format!("Failed to fetch economic calendar: {}", e)),
    }
}
