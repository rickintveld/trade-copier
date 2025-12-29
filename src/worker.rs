use anyhow::Result;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, watch};
use tokio::io::{AsyncWriteExt, AsyncBufReadExt, BufReader};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use crate::database::{Database, WorkerState, ErrorSeverity};
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
        WorkerState::Active,
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
            // Log critical error - worker cannot start
            if let Err(db_err) = db.insert_worker_error(
                &slave.address,
                ErrorSeverity::Critical,
                &error_msg,
            ).await {
                eprintln!("[WORKER:{}] Failed to log error to database: {}", slave.name, db_err);
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
                    // Log error - connection accept failed
                    if let Err(db_err) = db_clone.insert_worker_error(
                        &address_clone,
                        ErrorSeverity::Error,
                        &error_msg,
                    ).await {
                        eprintln!("[WORKER:{}] Failed to log error to database: {}", name_clone, db_err);
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

                    // Send trade to connected MT5 receiver and measure latency
                    match send_trade(&connection, &slave.name, &trade).await {
                        Ok(latency_us) => {
                            println!("[WORKER:{}] Trade sent successfully, latency: {}µs", slave.name, latency_us);
                            
                            // Spawn background task to update latency (non-blocking)
                            let db_clone = db.clone();
                            let address_clone = slave.address.clone();
                            let name_clone = slave.name.clone();
                            tokio::spawn(async move {
                                if let Err(e) = db_clone.update_worker_latency(&address_clone, latency_us).await {
                                    eprintln!("[WORKER:{}] Failed to update latency in database: {}", name_clone, e);
                                }
                            });
                            
                            // Spawn background task to save trade (non-blocking)
                            let db_clone = db.clone();
                            let address_clone = slave.address.clone();
                            let trade_clone = trade.clone();
                            let name_clone = slave.name.clone();
                            tokio::spawn(async move {
                                if let Err(e) = db_clone.insert_trade(&address_clone, &trade_clone).await {
                                    let error_msg = format!("Failed to save trade to database: {}", e);
                                    eprintln!("[WORKER:{}] {}", name_clone, error_msg);
                                    // Log warning - trade was sent but not saved
                                    if let Err(db_err) = db_clone.insert_worker_error(
                                        &address_clone,
                                        ErrorSeverity::Warning,
                                        &error_msg,
                                    ).await {
                                        eprintln!("[WORKER:{}] Failed to log error to database: {}", name_clone, db_err);
                                    }
                                }
                            });
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
                            // Log error - trade sending failed
                            if let Err(db_err) = db.insert_worker_error(
                                &slave.address,
                                ErrorSeverity::Error,
                                &error_msg,
                            ).await {
                                eprintln!("[WORKER:{}] Failed to log error to database: {}", slave.name, db_err);
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
                    // Log critical error - channel is broken, worker must stop
                    if let Err(db_err) = db.insert_worker_error(
                        &slave.address,
                        ErrorSeverity::Critical,
                        &error_msg,
                    ).await {
                        eprintln!("[WORKER:{}] Failed to log error to database: {}", slave.name, db_err);
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
        WorkerState::Inactive,
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
) -> Result<u64> {
    let mut conn_guard = connection.lock().await;
    
    if let Some(stream) = conn_guard.as_mut() {
        let trade_json = serde_json::to_string(trade)?;
        let message = format!("{}
", trade_json);
        
        // Start timing
        let start = Instant::now();
        
        // Send the trade
        match stream.write_all(message.as_bytes()).await {
            Ok(_) => {
                println!("[WORKER:{}] Sent trade: {}", worker_name, trade_json);
                
                // Wait for acknowledgment from MT5 receiver
                let mut reader = BufReader::new(stream);
                let mut ack_line = String::new();
                
                match reader.read_line(&mut ack_line).await {
                    Ok(0) => {
                        // Connection closed
                        eprintln!("[WORKER:{}] Connection closed while waiting for acknowledgment", worker_name);
                        *conn_guard = None;
                        Err(anyhow::anyhow!("Connection closed"))
                    }
                    Ok(_) => {
                        // Calculate latency in microseconds
                        let latency_us = start.elapsed().as_micros() as u64;
                        println!("[WORKER:{}] Received acknowledgment: {}", worker_name, ack_line.trim());
                        Ok(latency_us)
                    }
                    Err(e) => {
                        eprintln!("[WORKER:{}] Read error: {}", worker_name, e);
                        *conn_guard = None;
                        Err(anyhow::anyhow!("Failed to read acknowledgment: {}", e))
                    }
                }
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
