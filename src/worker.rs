use anyhow::Result;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, watch};
use tokio::io::AsyncWriteExt;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::database::{Database, WorkerState};
use crate::types::{Trade, SlaveConfig};

pub async fn run_worker(
    slave: SlaveConfig,
    mut rx: broadcast::Receiver<Trade>,
    db: Arc<Database>,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Result<()> {
    println!("[WORKER:{}] Starting TCP server on {}", slave.name, slave.address);

    // Set worker state to activated in database
    if let Err(e) = db.upsert_worker(
        &slave.name,
        &slave.address,
        slave.multiplier,
        WorkerState::Activated,
        None,
    ).await {
        eprintln!("[WORKER:{}] Failed to update database on startup: {}", slave.name, e);
    }

    let listener = match TcpListener::bind(&slave.address).await {
        Ok(l) => l,
        Err(e) => {
            // Update database with error state
            let error_msg = format!("Failed to bind to address: {}", e);
            if let Err(db_err) = db.update_worker_state(
                &slave.address,
                WorkerState::Error,
                Some(&error_msg),
            ).await {
                eprintln!("[WORKER:{}] Failed to update database with error: {}", slave.name, db_err);
            }
            return Err(e.into());
        }
    };
    println!("[WORKER:{}] Listening on {} (TCP)", slave.name, slave.address);

    // Store active connection
    let connection: Arc<Mutex<Option<tokio::net::TcpStream>>> = Arc::new(Mutex::new(None));
    let connection_clone = connection.clone();
    let name_clone = slave.name.clone();
    let address_clone = slave.address.clone();
    let db_clone = db.clone();

    // Spawn task to accept connections
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    println!("[WORKER:{}] MT5 receiver connected from {}", name_clone, addr);
                    *connection_clone.lock().await = Some(stream);
                }
                Err(e) => {
                    let error_msg = format!("Failed to accept connection: {}", e);
                    eprintln!("[WORKER:{}] {}", name_clone, error_msg);
                    if let Err(db_err) = db_clone.update_worker_state(
                        &address_clone,
                        WorkerState::Error,
                        Some(&error_msg),
                    ).await {
                        eprintln!("[WORKER:{}] Failed to update database: {}", name_clone, db_err);
                    }
                }
            }
        }
    });

    // Process trades from broadcast channel
    loop {
        tokio::select! {
            // Check for shutdown signal
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    println!("[WORKER:{}] Received shutdown signal", slave.name);
                    break;
                }
            }
            // Process incoming trades
            result = rx.recv() => match result {
                Ok(mut trade) => {
                println!("[WORKER:{}] Received trade: {:?}", slave.name, trade);

                // Apply risk multiplier
                trade.lots = trade.lots * slave.multiplier;
                trade.lots = (trade.lots * 100.0).round() / 100.0; // Round to 2 decimals

                println!(
                    "[WORKER:{}] Adjusted lots: {} (multiplier: {})",
                    slave.name, trade.lots, slave.multiplier
                );

                    // Send trade to connected MT5 receiver
                    match send_trade(&connection, &slave.name, &trade).await {
                        Ok(_) => {
                            // Save trade to database after successful processing
                            if let Err(e) = db.insert_trade(&slave.address, &trade).await {
                                eprintln!("[WORKER:{}] Failed to save trade to database: {}", slave.name, e);
                            }
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to send trade: {}", e);
                            eprintln!("[WORKER:{}] {}", slave.name, error_msg);
                            if let Err(db_err) = db.update_worker_state(
                                &slave.address,
                                WorkerState::Error,
                                Some(&error_msg),
                            ).await {
                                eprintln!("[WORKER:{}] Failed to update database: {}", slave.name, db_err);
                            }
                        }
                    }
                }
                Err(e) => {
                    let error_msg = format!("Channel error: {}", e);
                    eprintln!("[WORKER:{}] {}", slave.name, error_msg);
                    if let Err(db_err) = db.update_worker_state(
                        &slave.address,
                        WorkerState::Error,
                        Some(&error_msg),
                    ).await {
                        eprintln!("[WORKER:{}] Failed to update database: {}", slave.name, db_err);
                    }
                    break;
                }
            }
        }
    }

    // Worker is shutting down - update state to deactivated
    println!("[WORKER:{}] Deactivating worker", slave.name);
    if let Err(e) = db.update_worker_state(
        &slave.address,
        WorkerState::Deactivated,
        None,
    ).await {
        eprintln!("[WORKER:{}] Failed to update database on shutdown: {}", slave.name, e);
    }

    Ok(())
}

async fn send_trade(
    connection: &Arc<Mutex<Option<tokio::net::TcpStream>>>,
    worker_name: &str,
    trade: &Trade,
) -> Result<()> {
    let mut conn_guard = connection.lock().await;
    
    if let Some(stream) = conn_guard.as_mut() {
        let trade_json = serde_json::to_string(trade)?;
        let message = format!("{}
", trade_json);
        
        match stream.write_all(message.as_bytes()).await {
            Ok(_) => {
                println!("[WORKER:{}] Sent trade: {}", worker_name, trade_json);
                Ok(())
            }
            Err(e) => {
                eprintln!("[WORKER:{}] Write error, connection lost: {}", worker_name, e);
                *conn_guard = None; // Clear dead connection
                Err(anyhow::anyhow!("Connection lost: {}", e))
            }
        }
    } else {
        Err(anyhow::anyhow!("No MT5 receiver connected"))
    }
}
