use serde::Serialize;
use std::sync::Arc;
use tauri::State;
use tokio::sync::{mpsc, Mutex};

use crate::database::Database;
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

// Get all instances
#[tauri::command]
pub async fn get_instances(state: State<'_, AppState>) -> Result<ApiResponse<serde_json::Value>, String> {
    match InstanceManager::new(state.db.clone()) {
        Ok(manager) => match manager.list_instances().await {
            Ok(instances) => Ok(ApiResponse {
                success: true,
                data: serde_json::to_value(instances).map_err(|e| e.to_string())?,
            }),
            Err(e) => Err(e.to_string()),
        },
        Err(e) => Err(format!("Failed to initialize instance manager: {}", e)),
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

// Delete instance
#[tauri::command]
pub async fn delete_instance(
    state: State<'_, AppState>,
    name: String,
    force: bool,
) -> Result<ApiResponse<serde_json::Value>, String> {
    match InstanceManager::new(state.db.clone()) {
        Ok(manager) => match manager.delete_instance(&name, force).await {
            Ok(()) => {
                // Stop only the specific worker for the deleted instance
                let tx = state.worker_command_tx.lock().await;
                if let Err(e) = tx.send(WorkerCommand::Stop(name.clone())).await {
                    eprintln!("[TAURI] Failed to send stop command: {}", e);
                }

                Ok(ApiResponse {
                    success: true,
                    data: serde_json::json!({
                        "message": format!("Instance '{}' deleted successfully. Worker stopped.", name)
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
    name: String,
    force: Option<bool>,
) -> Result<ApiResponse<serde_json::Value>, String> {
    let force = force.unwrap_or(false);
    
    match InstanceManager::new(state.db.clone()) {
        Ok(manager) => match manager.start_instance(&name, force).await {
            Ok(()) => {
                // Now send start command to worker manager
                let tx = state.worker_command_tx.lock().await;
                if let Err(e) = tx.send(WorkerCommand::Start(name.clone())).await {
                    return Err(format!("MT5 instance started but failed to start worker: {}", e));
                }

                Ok(ApiResponse {
                    success: true,
                    data: serde_json::json!({
                        "message": format!("Instance '{}' and worker started successfully", name)
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
    name: String,
) -> Result<ApiResponse<serde_json::Value>, String> {
    // First, stop the worker
    let tx = state.worker_command_tx.lock().await;
    if let Err(e) = tx.send(WorkerCommand::Stop(name.clone())).await {
        return Err(format!("Failed to send stop command: {}", e));
    }

    // Then, stop the MT5 instance (kill the Wine process)
    match InstanceManager::new(state.db.clone()) {
        Ok(manager) => {
            if let Err(e) = manager.stop_instance(&name).await {
                eprintln!("[TAURI] Warning: Failed to stop MT5 instance: {}", e);
            }
        }
        Err(e) => {
            eprintln!("[TAURI] Warning: Failed to initialize instance manager: {}", e);
        }
    }

    Ok(ApiResponse {
        success: true,
        data: serde_json::json!({
            "message": format!("Worker '{}' and MT5 instance stopped successfully", name)
        }),
    })
}
