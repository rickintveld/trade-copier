use anyhow::Result;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

use crate::database::Database;

const API_PORT: u16 = 3000;

#[derive(Clone)]
struct AppState {
    db: Arc<Database>,
}

#[derive(Debug, Deserialize)]
struct PaginationParams {
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_limit() -> i64 {
    100
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

pub async fn run_api(db: Arc<Database>) -> Result<()> {
    let state = AppState { db };

    let app = Router::new()
        .route("/api/workers", get(get_workers))
        .route("/api/trades", get(get_trades))
        .route("/api/errors", get(get_errors))
        .route("/api/health", get(health_check))
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
