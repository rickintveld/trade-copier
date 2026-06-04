use anyhow::Result;
use log::{info, error};
use tokio::sync::{broadcast, watch};
use tokio::io::{AsyncBufReadExt, BufReader};
use std::sync::Arc;
use crate::types::Trade;
use crate::database::Database;

const ROUTER_PORT: u16 = 5000;

pub async fn run_router(
    tx: broadcast::Sender<Trade>,
    db: Arc<Database>,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Result<()> {
    let address = format!("127.0.0.1:{}", ROUTER_PORT);
    let listener = crate::port_utils::bind_with_retry(&address).await?;
    info!("[ROUTER] Listening on port {} (TCP)", ROUTER_PORT);

    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    info!("[ROUTER] Received shutdown signal, stopping...");
                    break;
                }
            }
            result = listener.accept() => {
                match result {
                    Ok((stream, addr)) => {
                        info!("[ROUTER] New connection from {}", addr);
                        let tx_clone = tx.clone();
                        let db_clone = db.clone();
                        
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(stream, tx_clone, db_clone, addr).await {
                                error!("[ROUTER] Connection error from {}: {}", addr, e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("[ROUTER] Failed to accept connection: {}", e);
                    }
                }
            }
        }
    }

    info!("[ROUTER] Stopped");
    Ok(())
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    tx: broadcast::Sender<Trade>,
    db: Arc<Database>,
    addr: std::net::SocketAddr,
) -> Result<()> {
    info!("[ROUTER] Master MT5 connected from {}", addr);
    
    // Update provider_connected status to true
    if let Err(e) = update_provider_status(&db, true).await {
        error!("[ROUTER] Failed to update provider status: {}", e);
    }
    
    let reader = BufReader::new(stream);
    let mut lines = reader.lines();
    let mut trade_count = 0;

    while let Some(line) = lines.next_line().await? {
        if line.is_empty() {
            continue;
        }

        match serde_json::from_str::<Trade>(&line) {
            Ok(trade) => {
                trade_count += 1;
                info!("[ROUTER] Received trade from {}: {:?}", addr, trade);
                
                // Broadcast to all workers
                match tx.send(trade.clone()) {
                    Ok(receivers) => {
                        info!("[ROUTER] Broadcasted to {} workers", receivers);
                    }
                    Err(e) => {
                        error!("[ROUTER] Failed to broadcast: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("[ROUTER] Failed to parse trade: {}", e);
            }
        }
    }

    info!("[ROUTER] Master MT5 disconnected from {} (processed {} trades)", addr, trade_count);
    info!("[ROUTER] Waiting for master MT5 to reconnect...");
    
    // Update provider_connected status to false
    if let Err(e) = update_provider_status(&db, false).await {
        error!("[ROUTER] Failed to update provider status: {}", e);
    }
    
    Ok(())
}

async fn update_provider_status(db: &Arc<Database>, connected: bool) -> Result<()> {
    // Get current metrics to preserve other values
    let metrics = db.get_system_metrics().await?;
    
    if let Some(m) = metrics {
        db.upsert_system_metrics(
            &m.router_status,
            m.router_port as u16,
            m.copier_active,
            m.total_workers as i32,
            m.active_workers as i32,
            m.uptime_seconds as u64,
            connected,
        ).await?;
    }
    
    Ok(())
}
