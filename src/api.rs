use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tower_http::cors::CorsLayer;

use crate::database::Database;
use crate::installer::InstanceManager;
use crate::worker_manager::WorkerCommand;

const API_PORT: u16 = 3000;

#[derive(Clone)]
struct AppState {
    db: Arc<Database>,
    worker_command_tx: mpsc::Sender<WorkerCommand>,
}

#[derive(Debug, Deserialize)]
struct PaginationParams {
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_limit() -> i64 {
    100
}

#[derive(Debug, Deserialize)]
struct CreateInstanceRequest {
    name: String,
    address: String,
    multiplier: f64,
}

#[derive(Debug, Deserialize)]
struct DeleteInstanceParams {
    #[serde(default)]
    force: bool,
}

#[derive(Debug, Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: T,
}

#[derive(Debug, Serialize)]
struct ApiError {
    success: bool,
    error: String,
}

pub async fn run_api(db: Arc<Database>, worker_command_tx: mpsc::Sender<WorkerCommand>) -> Result<()> {
    let state = AppState { db, worker_command_tx };

    let app = Router::new()
        .route("/api/workers", get(get_workers))
        .route("/api/trades", get(get_trades))
        .route("/api/errors", get(get_errors))
        .route("/api/system/metrics", get(get_system_metrics))
        .route("/api/health", get(health_check))
        .route("/api/instances", get(get_instances).post(create_instance))
        .route("/api/instances/:name", delete(delete_instance))
        .route("/api/instances/:name/start", post(start_instance))
        .route("/api/instances/:name/stop", post(stop_instance))
        .route("/api/instances/start-all", post(start_all_instances))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", API_PORT)).await?;
    println!("[API] HTTP server listening on port {}", API_PORT);

    axum::serve(listener, app).await?;
    Ok(())
}

async fn health_check() -> impl IntoResponse {
    Json(ApiResponse {
        success: true,
        data: serde_json::json!({
            "status": "ok",
            "message": "Trade Copier API is running"
        }),
    })
}

async fn get_workers(State(state): State<AppState>) -> impl IntoResponse {
    match state.db.get_all_workers().await {
        Ok(workers) => Json(ApiResponse {
            success: true,
            data: workers,
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                success: false,
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

async fn get_trades(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> impl IntoResponse {
    match state.db.get_all_trades(Some(params.limit)).await {
        Ok(trades) => Json(ApiResponse {
            success: true,
            data: trades,
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                success: false,
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

async fn get_errors(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> impl IntoResponse {
    match state.db.get_all_errors(Some(params.limit)).await {
        Ok(errors) => Json(ApiResponse {
            success: true,
            data: errors,
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                success: false,
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

async fn get_system_metrics(State(state): State<AppState>) -> impl IntoResponse {
    match state.db.get_system_metrics().await {
        Ok(metrics) => Json(ApiResponse {
            success: true,
            data: metrics,
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                success: false,
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

// Instance Management Endpoints

async fn get_instances(State(state): State<AppState>) -> impl IntoResponse {
    match InstanceManager::new(state.db.clone()) {
        Ok(manager) => match manager.list_instances().await {
            Ok(instances) => Json(ApiResponse {
                success: true,
                data: instances,
            })
            .into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    success: false,
                    error: e.to_string(),
                }),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                success: false,
                error: format!("Failed to initialize instance manager: {}", e),
            }),
        )
            .into_response(),
    }
}

async fn create_instance(
    State(state): State<AppState>,
    Json(req): Json<CreateInstanceRequest>,
) -> impl IntoResponse {
    match InstanceManager::new(state.db.clone()) {
        Ok(manager) => match manager
            .create_instance(req.name, req.address, req.multiplier, Some(state.worker_command_tx.clone()))
            .await
        {
            Ok(instance) => (
                StatusCode::CREATED,
                Json(ApiResponse {
                    success: true,
                    data: instance,
                }),
            )
                .into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    success: false,
                    error: e.to_string(),
                }),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                success: false,
                error: format!("Failed to initialize instance manager: {}", e),
            }),
        )
            .into_response(),
    }
}

async fn delete_instance(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(params): Query<DeleteInstanceParams>,
) -> impl IntoResponse {
    match InstanceManager::new(state.db.clone()) {
        Ok(manager) => match manager.delete_instance(&name, params.force).await {
            Ok(()) => {
                // Trigger worker reload to reflect the deletion
                if let Err(e) = state.worker_command_tx.send(WorkerCommand::Reload).await {
                    eprintln!("[API] Failed to send reload command: {}", e);
                }
                
                Json(ApiResponse {
                    success: true,
                    data: serde_json::json!({
                        "message": format!("Instance '{}' deleted successfully. Workers reloading...", name)
                    }),
                })
                .into_response()
            }
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    success: false,
                    error: e.to_string(),
                }),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                success: false,
                error: format!("Failed to initialize instance manager: {}", e),
            }),
        )
            .into_response(),
    }
}

async fn start_instance(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match InstanceManager::new(state.db.clone()) {
        Ok(manager) => match manager.start_instance(&name).await {
            Ok(()) => Json(ApiResponse {
                success: true,
                data: serde_json::json!({
                    "message": format!("Instance '{}' started successfully", name)
                }),
            })
            .into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    success: false,
                    error: e.to_string(),
                }),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                success: false,
                error: format!("Failed to initialize instance manager: {}", e),
            }),
        )
            .into_response(),
    }
}

async fn stop_instance(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    // Send stop command to worker manager
    if let Err(e) = state.worker_command_tx.send(WorkerCommand::Stop(name.clone())).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                success: false,
                error: format!("Failed to send stop command: {}", e),
            }),
        )
            .into_response();
    }
    
    Json(ApiResponse {
        success: true,
        data: serde_json::json!({
            "message": format!("Worker '{}' stop command sent successfully", name)
        }),
    })
    .into_response()
}

async fn start_all_instances(State(state): State<AppState>) -> impl IntoResponse {
    match InstanceManager::new(state.db.clone()) {
        Ok(manager) => match manager.start_all_instances().await {
            Ok(()) => {
                // Trigger worker reload to restart all workers
                if let Err(e) = state.worker_command_tx.send(WorkerCommand::Reload).await {
                    eprintln!("[API] Failed to send reload command: {}", e);
                }
                
                Json(ApiResponse {
                    success: true,
                    data: serde_json::json!({
                        "message": "All instances started successfully. Workers reloading..."
                    }),
                })
                .into_response()
            }
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    success: false,
                    error: e.to_string(),
                }),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                success: false,
                error: format!("Failed to initialize instance manager: {}", e),
            }),
        )
            .into_response(),
    }
}

