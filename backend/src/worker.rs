use anyhow::Result;
use log::{info, error};
use tokio::sync::{broadcast, watch, mpsc};
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use std::sync::Arc;
use std::time::{Instant, Duration};
use tokio::sync::Mutex;
use std::path::{Path, PathBuf};
use socket2::{Socket, TcpKeepalive};
use crate::database::{Database, WorkerState, ErrorSeverity};
use crate::types::{Trade, SlaveConfig, ProfitInfo};
use std::collections::HashMap;

pub async fn run_worker(
    slave: SlaveConfig,
    mut rx: broadcast::Receiver<Trade>,
    db: Arc<Database>,
    mut shutdown_rx: watch::Receiver<bool>,
    wine_prefix: Option<PathBuf>,
) -> Result<()> {
    info!("[WORKER:{}] Starting TCP server on {}", slave.name, slave.address);

    // Set worker state to activated in database
    let wine_prefix_str = wine_prefix.as_ref().map(|p| p.to_string_lossy().to_string());
    if let Err(e) = db.upsert_worker(crate::database::WorkerUpsertConfig {
        name: slave.name.clone(),
        address: slave.address.clone(),
        multiplier: slave.multiplier,
        state: WorkerState::Active,
        error: None,
        wine_prefix: wine_prefix_str,
        symbol_prefix: slave.symbol_prefix.clone(),
    }).await {
        error!("[WORKER:{}] Failed to update database on startup: {}", slave.name, e);
    }

    // Try to bind with automatic retry and port cleanup
    let listener = match crate::port_utils::bind_with_retry(&slave.address).await {
        Ok(l) => l,
        Err(e) => {
            // Update database with error state
            let error_msg = format!("Failed to bind to address: {}", e);
            if let Err(db_err) = db.update_worker_state(
                &slave.address,
                WorkerState::Error,
                Some(&error_msg),
            ).await {
                error!("[WORKER:{}] Failed to update database with error: {}", slave.name, db_err);
            }
            // Log critical error - worker cannot start
            if let Err(db_err) = db.insert_worker_error(
                &slave.address,
                ErrorSeverity::Critical,
                &error_msg,
            ).await {
                error!("[WORKER:{}] Failed to log error to database: {}", slave.name, db_err);
            }
            return Err(e.into());
        }
    };
    info!("[WORKER:{}] Listening on {} (TCP)", slave.name, slave.address);

    // Store writer half of connection (for send_trade and heartbeat)
    let writer: Arc<Mutex<Option<OwnedWriteHalf>>> = Arc::new(Mutex::new(None));
    let writer_accept = writer.clone();
    let name_clone = slave.name.clone();
    let address_clone = slave.address.clone();
    let db_clone = db.clone();
    
    // Channel for pending acknowledgments (trade_id -> sender)
    let pending_acks: Arc<Mutex<HashMap<u64, mpsc::Sender<String>>>> = Arc::new(Mutex::new(HashMap::new()));

    // Spawn Wine process monitoring task if prefix is provided
    if let Some(prefix) = wine_prefix {
        let shutdown_tx_clone = shutdown_rx.clone();
        let name_monitor = slave.name.clone();
        let db_monitor = db.clone();
        let address_monitor = slave.address.clone();
        
        tokio::spawn(async move {
            monitor_wine_process(prefix, name_monitor, db_monitor, address_monitor, shutdown_tx_clone).await;
        });
    }
    
    // Spawn connection health monitoring task (heartbeat)
    let writer_heartbeat = writer.clone();
    let name_heartbeat = slave.name.clone();
    let address_heartbeat = slave.address.clone();
    let db_heartbeat = db.clone();
    let shutdown_heartbeat = shutdown_rx.clone();
    
    tokio::spawn(async move {
        monitor_connection_health(
            writer_heartbeat,
            name_heartbeat,
            address_heartbeat,
            db_heartbeat,
            shutdown_heartbeat,
        ).await;
    });
    
    // Track active reader task for clean reconnection
    let reader_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>> = Arc::new(Mutex::new(None));
    let reader_task_accept = reader_task.clone();
    let pending_acks_accept = pending_acks.clone();
    let mut shutdown_accept = shutdown_rx.clone();

    // Spawn task to accept connections
    tokio::spawn(async move {
        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            info!("[WORKER:{}] MT5 receiver connected from {}", name_clone, addr);
                            
                            // Configure TCP keep-alive and socket options
                            if let Err(e) = configure_tcp_socket(&stream) {
                                error!("[WORKER:{}] Failed to configure socket options: {}", name_clone, e);
                            } else {
                                info!("[WORKER:{}] TCP keep-alive configured", name_clone);
                            }
                            
                            // Split stream into independent read/write halves (no shared lock)
                            let (read_half, write_half) = stream.into_split();
                            *writer_accept.lock().await = Some(write_half);
                            
                            // Abort old reader task and spawn a new one for this connection
                            {
                                let mut task = reader_task_accept.lock().await;
                                if let Some(old_task) = task.take() {
                                    old_task.abort();
                                }
                                
                                let pa = pending_acks_accept.clone();
                                let name = name_clone.clone();
                                let addr_str = address_clone.clone();
                                let db = db_clone.clone();
                                let shutdown = shutdown_accept.clone();
                                
                                *task = Some(tokio::spawn(async move {
                                    read_connection(read_half, pa, name, addr_str, db, shutdown).await;
                                }));
                            }
                            
                            // Update mt5_connected to true
                            if let Err(e) = db_clone.update_mt5_connected(&address_clone, true).await {
                                error!("[WORKER:{}] Failed to update mt5_connected status: {}", name_clone, e);
                            }
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to accept connection: {}", e);
                            error!("[WORKER:{}] {}", name_clone, error_msg);
                            if let Err(db_err) = db_clone.update_worker_state(
                                &address_clone,
                                WorkerState::Error,
                                Some(&error_msg),
                            ).await {
                                error!("[WORKER:{}] Failed to update database: {}", name_clone, db_err);
                            }
                            // Log error - connection accept failed
                            if let Err(db_err) = db_clone.insert_worker_error(
                                &address_clone,
                                ErrorSeverity::Error,
                                &error_msg,
                            ).await {
                                error!("[WORKER:{}] Failed to log error to database: {}", name_clone, db_err);
                            }
                        }
                    }
                }
                _ = shutdown_accept.changed() => {
                    if *shutdown_accept.borrow() {
                        info!("[WORKER:{}] Accept loop shutting down, releasing port", name_clone);
                        break;
                    }
                }
            }
        }
        // `listener` is dropped here, releasing the port
    });

    // Process trades from broadcast channel
    loop {
        tokio::select! {
            // Check for shutdown signal
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    info!("[WORKER:{}] Received shutdown signal", slave.name);
                    break;
                }
            }
            // Process incoming trades
            result = rx.recv() => match result {
                Ok(mut trade) => {
                info!("[WORKER:{}] Received trade: {:?}", slave.name, trade);

                // Apply risk multiplier
                trade.lots *= slave.multiplier;
                trade.lots = (trade.lots * 100.0).round() / 100.0; // Round to 2 decimals
                
                // Apply symbol prefix if configured
                if !slave.symbol_prefix.is_empty() {
                    trade.symbol = format!("{}{}", trade.symbol, slave.symbol_prefix);
                    info!("[WORKER:{}] Applied symbol prefix: {}", slave.name, trade.symbol);
                }

                info!(
                    "[WORKER:{}] Adjusted lots: {} (multiplier: {})",
                    slave.name, trade.lots, slave.multiplier
                );

                    // Send trade to connected MT5 receiver and measure latency
                    match send_trade(&writer, &pending_acks, &slave.name, &trade, &db, &slave.address).await {
                        Ok(latency_us) => {
                            info!("[WORKER:{}] Trade sent successfully, latency: {}µs", slave.name, latency_us);
                            
                            // Spawn background task to update latency (non-blocking)
                            let db_clone = db.clone();
                            let address_clone = slave.address.clone();
                            let name_clone = slave.name.clone();
                            tokio::spawn(async move {
                                if let Err(e) = db_clone.update_worker_latency(&address_clone, latency_us).await {
                                    error!("[WORKER:{}] Failed to update latency in database: {}", name_clone, e);
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
                                    error!("[WORKER:{}] {}", name_clone, error_msg);
                                    // Log warning - trade was sent but not saved
                                    if let Err(db_err) = db_clone.insert_worker_error(
                                        &address_clone,
                                        ErrorSeverity::Warning,
                                        &error_msg,
                                    ).await {
                                        error!("[WORKER:{}] Failed to log error to database: {}", name_clone, db_err);
                                    }
                                }
                            });
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to send trade: {}", e);
                            error!("[WORKER:{}] {}", slave.name, error_msg);
                            if let Err(db_err) = db.update_worker_state(
                                &slave.address,
                                WorkerState::Error,
                                Some(&error_msg),
                            ).await {
                                error!("[WORKER:{}] Failed to update database: {}", slave.name, db_err);
                            }
                            // Log error - trade sending failed
                            if let Err(db_err) = db.insert_worker_error(
                                &slave.address,
                                ErrorSeverity::Error,
                                &error_msg,
                            ).await {
                                error!("[WORKER:{}] Failed to log error to database: {}", slave.name, db_err);
                            }
                        }
                    }
                }
                Err(e) => {
                    let error_msg = format!("Channel error: {}", e);
                    error!("[WORKER:{}] {}", slave.name, error_msg);
                    if let Err(db_err) = db.update_worker_state(
                        &slave.address,
                        WorkerState::Error,
                        Some(&error_msg),
                    ).await {
                        error!("[WORKER:{}] Failed to update database: {}", slave.name, db_err);
                    }
                    // Log critical error - channel is broken, worker must stop
                    if let Err(db_err) = db.insert_worker_error(
                        &slave.address,
                        ErrorSeverity::Critical,
                        &error_msg,
                    ).await {
                        error!("[WORKER:{}] Failed to log error to database: {}", slave.name, db_err);
                    }
                    break;
                }
            }
        }
    }

    // Worker is shutting down - update state to deactivated
    info!("[WORKER:{}] Deactivating worker", slave.name);
    if let Err(e) = db.update_worker_state(
        &slave.address,
        WorkerState::Inactive,
        None,
    ).await {
        error!("[WORKER:{}] Failed to update database on shutdown: {}", slave.name, e);
    }
    
    // Set mt5_connected to false
    if let Err(e) = db.update_mt5_connected(&slave.address, false).await {
        error!("[WORKER:{}] Failed to update mt5_connected on shutdown: {}", slave.name, e);
    }

    Ok(())
}

/// Monitor Wine process associated with this worker's MT5 instance
async fn monitor_wine_process(
    wine_prefix: PathBuf,
    worker_name: String,
    db: Arc<Database>,
    address: String,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let check_interval = tokio::time::Duration::from_secs(5);
    let mut interval = tokio::time::interval(check_interval);
    
    info!("[WORKER:{}] Starting Wine process monitor for prefix: {:?}", worker_name, wine_prefix);
    info!("[WORKER:{}] Waiting for Wine process to start before monitoring...", worker_name);
    
    // Phase 1: Wait for Wine process to start (grace period)
    loop {
        tokio::select! {
            _ = interval.tick() => {
                match is_wine_running(&wine_prefix).await {
                    Ok(true) => {
                        info!("[WORKER:{}] Wine process detected, starting monitoring", worker_name);
                        break;
                    }
                    Ok(false) => {
                        // Wine not running yet, keep waiting silently
                    }
                    Err(e) => {
                        error!("[WORKER:{}] Error checking Wine process: {}", worker_name, e);
                    }
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    info!("[WORKER:{}] Wine monitor received shutdown signal before Wine started", worker_name);
                    return;
                }
            }
        }
    }
    
    // Phase 2: Monitor Wine process - only starts after Wine has been detected
    loop {
        tokio::select! {
            _ = interval.tick() => {
                // Check if Wine process is still running for this prefix
                match is_wine_running(&wine_prefix).await {
                    Ok(false) => {
                        let error_msg = format!(
                            "Wine process stopped for prefix {:?}. MT5 application closed.",
                            wine_prefix
                        );
                        error!("[WORKER:{}] {}", worker_name, error_msg);
                        
                        // Update worker state
                        if let Err(e) = db.update_worker_state(
                            &address,
                            WorkerState::Inactive,
                            Some(&error_msg),
                        ).await {
                            error!("[WORKER:{}] Failed to update state: {}", worker_name, e);
                        }
                        
                        // Update mt5_connected to false
                        if let Err(e) = db.update_mt5_connected(&address, false).await {
                            error!("[WORKER:{}] Failed to update mt5_connected status: {}", worker_name, e);
                        }
                        
                        // Log warning
                        if let Err(e) = db.insert_worker_error(
                            &address,
                            ErrorSeverity::Warning,
                            &error_msg,
                        ).await {
                            error!("[WORKER:{}] Failed to log error: {}", worker_name, e);
                        }
                        
                        info!("[WORKER:{}] Worker stopped due to Wine process termination", worker_name);
                        break;
                    }
                    Ok(true) => {
                        // Still running, continue monitoring
                    }
                    Err(e) => {
                        error!("[WORKER:{}] Error checking Wine process: {}", worker_name, e);
                    }
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    info!("[WORKER:{}] Wine monitor received shutdown signal", worker_name);
                    break;
                }
            }
        }
    }
}

/// Check if a Wine process is running for the given prefix
async fn is_wine_running(wine_prefix: &Path) -> Result<bool> {
    // Construct the path to the MT5 executable
    let mt5_path = wine_prefix.join("drive_c/Program Files/MetaTrader 5/terminal64.exe");
    let mt5_path_str = mt5_path.display().to_string();
    
    // Use pgrep to check if the executable is running
    let output = tokio::process::Command::new("pgrep")
        .arg("-f")
        .arg(&mt5_path_str)
        .output()
        .await?;

    Ok(output.status.success() && !output.stdout.is_empty())
}

/// Kill the Wine process for the given prefix
pub async fn kill_wine_process(wine_prefix: &PathBuf) -> Result<()> {
    info!("[WINE] Terminating Wine server for prefix {:?}", wine_prefix);
    
    // Get wineserver path
    #[cfg(target_os = "macos")]
    let wineserver = crate::installer::mac::get_wineserver_path()?
        .to_string_lossy()
        .to_string();
    
    #[cfg(not(target_os = "macos"))]
    let wineserver = "wineserver".to_string();
    
    // Use wineserver -k to kill all Wine processes for this prefix
    let result = tokio::process::Command::new(&wineserver)
        .arg("-k")  // Kill all processes in this prefix
        .env("WINEPREFIX", wine_prefix)
        .output()
        .await?;
    
    if result.status.success() {
        info!("[WINE] Successfully terminated Wine server");
    } else {
        let stderr = String::from_utf8_lossy(&result.stderr);
        error!("[WINE] Warning: wineserver -k returned non-zero status: {}", stderr);
    }
    
    // Give Wine processes a moment to shut down
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    // Verify all processes are stopped
    if is_wine_running(wine_prefix).await? {
        error!("[WINE] Warning: Some Wine processes still running after wineserver -k");
        
        // Fallback: try force kill with wineserver -k9
        info!("[WINE] Attempting force kill with wineserver -k9");
        let kill_result = tokio::process::Command::new(&wineserver)
            .arg("-k9")  // Force kill
            .env("WINEPREFIX", wine_prefix)
            .output()
            .await?;
        
        if !kill_result.status.success() {
            let stderr = String::from_utf8_lossy(&kill_result.stderr);
            error!("[WINE] Force kill failed: {}", stderr);
        } else {
            info!("[WINE] Force kill completed");
        }
    }
    
    Ok(())
}

/// Configure TCP socket with keep-alive and other options
fn configure_tcp_socket(stream: &tokio::net::TcpStream) -> Result<()> {
    use std::os::unix::io::{AsRawFd, FromRawFd};
    
    let fd = stream.as_raw_fd();
    // Borrow the socket without taking ownership
    let socket = unsafe { Socket::from_raw_fd(fd) };
    
    // Enable TCP keep-alive with aggressive settings
    let keepalive = TcpKeepalive::new()
        .with_time(Duration::from_secs(30))      // Start probes after 30s idle
        .with_interval(Duration::from_secs(10)); // Probe every 10s
    
    let result = socket.set_tcp_keepalive(&keepalive)
        .and_then(|_| socket.set_nodelay(true))
        .and_then(|_| socket.set_read_timeout(Some(Duration::from_secs(30))))
        .and_then(|_| socket.set_write_timeout(Some(Duration::from_secs(10))));
    
    // Prevent socket from being dropped and closing the fd
    // We borrowed it from tokio's TcpStream which owns it
    std::mem::forget(socket);
    
    result.map_err(|e: std::io::Error| anyhow::anyhow!(e))
}

/// Monitor connection health with periodic heartbeat
async fn monitor_connection_health(
    writer: Arc<Mutex<Option<OwnedWriteHalf>>>,
    worker_name: String,
    address: String,
    db: Arc<Database>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let heartbeat_interval = Duration::from_secs(30);
    let mut interval = tokio::time::interval(heartbeat_interval);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    
    info!("[WORKER:{}] Connection health monitor started (heartbeat every 30s)", worker_name);
    
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let mut conn_guard = writer.lock().await;
                
                if let Some(stream) = conn_guard.as_mut() {
                    // Send heartbeat ping (simple newline)
                    let ping = b"PING\n";
                    
                    match stream.write_all(ping).await {
                        Ok(_) => {
                            // Connection is alive
                            // info!("[WORKER:{}] Heartbeat sent successfully", worker_name);
                        }
                        Err(e) => {
                            error!("[WORKER:{}] Heartbeat failed, connection lost: {}", worker_name, e);
                            *conn_guard = None;
                            
                            // Update mt5_connected to false
                            if let Err(db_err) = db.update_mt5_connected(&address, false).await {
                                error!("[WORKER:{}] Failed to update mt5_connected status: {}", worker_name, db_err);
                            }
                            
                            // Log error
                            let error_msg = format!("Heartbeat failed, connection lost: {}", e);
                            if let Err(db_err) = db.insert_worker_error(
                                &address,
                                ErrorSeverity::Warning,
                                &error_msg,
                            ).await {
                                error!("[WORKER:{}] Failed to log error: {}", worker_name, db_err);
                            }
                        }
                    }
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    info!("[WORKER:{}] Connection health monitor shutting down", worker_name);
                    break;
                }
            }
        }
    }
}

/// Continuously read messages from a single MT5 connection.
/// Spawned per-connection; aborted on reconnection.
/// Reads directly from OwnedReadHalf — no mutex, no polling delay.
async fn read_connection(
    mut reader: OwnedReadHalf,
    pending_acks: Arc<Mutex<HashMap<u64, mpsc::Sender<String>>>>,
    worker_name: String,
    address: String,
    db: Arc<Database>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let mut recv_buffer = String::new();
    let mut buf = [0u8; 4096];
    
    loop {
        tokio::select! {
            biased;
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    info!("[WORKER:{}] MT5 reader shutting down", worker_name);
                    break;
                }
            }
            result = reader.read(&mut buf) => {
                match result {
                    Ok(0) => {
                        info!("[WORKER:{}] MT5 connection closed", worker_name);
                        if let Err(e) = db.update_mt5_connected(&address, false).await {
                            error!("[WORKER:{}] Failed to update mt5_connected: {}", worker_name, e);
                        }
                        break;
                    }
                    Ok(n) => {
                        recv_buffer.push_str(&String::from_utf8_lossy(&buf[..n]));
                        
                        // Extract all complete newline-delimited messages
                        while let Some(pos) = recv_buffer.find('\n') {
                            let line = recv_buffer[..pos].trim().to_string();
                            recv_buffer.drain(..pos + 1);
                            if !line.is_empty() {
                                process_mt5_message(
                                    &worker_name,
                                    &line,
                                    &pending_acks,
                                    &db,
                                    &address,
                                ).await;
                            }
                        }
                    }
                    Err(e) => {
                        error!("[WORKER:{}] Read error from MT5: {}", worker_name, e);
                        if let Err(e) = db.update_mt5_connected(&address, false).await {
                            error!("[WORKER:{}] Failed to update mt5_connected: {}", worker_name, e);
                        }
                        break;
                    }
                }
            }
        }
    }
}

/// Process a message received from MT5
async fn process_mt5_message(
    worker_name: &str,
    message: &str,
    pending_acks: &Arc<Mutex<HashMap<u64, mpsc::Sender<String>>>>,
    db: &Arc<Database>,
    address: &str,
) {
    // Check if it's profit info (JSON with profit field)
    if message.starts_with('{') && message.contains("profit") {
        // Process as profit info
        process_profit_info(worker_name, message, db, address).await;
    } else if message.starts_with("OK:") || message.starts_with("ERROR:") {
        // This is an acknowledgment - try to match it to a pending trade
        // For now, just forward to any waiting receiver
        // (We could parse trade_id from the ack message if needed)
        let acks = pending_acks.lock().await;
        
        // Send to first pending ack (FIFO)
        if let Some((_, sender)) = acks.iter().next() {
            let _ = sender.send(message.to_string()).await;
        } else {
            // No pending ack, just log
            info!("[WORKER:{}] Received unexpected ack: {}", worker_name, message);
        }
    } else {
        // Unknown message type
        info!("[WORKER:{}] Received unknown message: {}", worker_name, message);
    }
}

/// Process incoming profit message from MT5
async fn process_profit_info(
    worker_name: &str,
    message: &str,
    db: &Arc<Database>,
    address: &str,
) {
    // Try to parse as ProfitInfo
    match serde_json::from_str::<ProfitInfo>(message) {
        Ok(profit_info) => {
            info!(
                "[WORKER:{}] Received profit: {}",
                worker_name, profit_info.profit
            );
            
            // Store profit in database
            if let Err(e) = db.insert_profit(
                address,
                profit_info.profit,
            ).await {
                error!("[WORKER:{}] Failed to save profit: {}", worker_name, e);
            }
        }
        Err(e) => {
            error!("[WORKER:{}] Failed to parse profit info: {}", worker_name, e);
        }
    }
}

async fn send_trade(
    writer: &Arc<Mutex<Option<OwnedWriteHalf>>>,
    pending_acks: &Arc<Mutex<HashMap<u64, mpsc::Sender<String>>>>,
    worker_name: &str,
    trade: &Trade,
    db: &Arc<Database>,
    address: &str,
) -> Result<u64> {
    let trade_json = serde_json::to_string(trade)?;
    let message = format!("{}\n", trade_json);
    
    // Start timing
    let start = Instant::now();
    
    // Create a channel to receive the acknowledgment
    let (ack_tx, mut ack_rx) = mpsc::channel::<String>(1);
    
    // Register this trade as pending acknowledgment
    {
        let mut acks = pending_acks.lock().await;
        acks.insert(trade.id, ack_tx);
    }
    
    // Send the trade (writer lock is independent of reader — no contention)
    {
        let mut conn_guard = writer.lock().await;
        
        if let Some(stream) = conn_guard.as_mut() {
            if let Err(e) = stream.write_all(message.as_bytes()).await {
                error!("[WORKER:{}] Write error, connection lost: {}", worker_name, e);
                *conn_guard = None;
                
                // Clean up pending ack
                pending_acks.lock().await.remove(&trade.id);
                
                // Update mt5_connected to false
                if let Err(db_err) = db.update_mt5_connected(address, false).await {
                    error!("[WORKER:{}] Failed to update mt5_connected status: {}", worker_name, db_err);
                }
                
                return Err(anyhow::anyhow!("Connection lost: {}", e));
            }
            
            info!("[WORKER:{}] Sent trade: {}", worker_name, trade_json);
        } else {
            // Clean up pending ack
            pending_acks.lock().await.remove(&trade.id);
            return Err(anyhow::anyhow!("No MT5 receiver connected"));
        }
    }
    
    // Wait for acknowledgment with timeout
    match tokio::time::timeout(Duration::from_secs(5), ack_rx.recv()).await {
        Ok(Some(response)) => {
            // Clean up pending ack
            pending_acks.lock().await.remove(&trade.id);
            
            let latency_us = start.elapsed().as_micros() as u64;
            info!("[WORKER:{}] Received ack: {}", worker_name, response);
            Ok(latency_us)
        }
        Ok(None) => {
            // Channel closed
            pending_acks.lock().await.remove(&trade.id);
            Err(anyhow::anyhow!("Acknowledgment channel closed"))
        }
        Err(_) => {
            // Timeout
            pending_acks.lock().await.remove(&trade.id);
            error!("[WORKER:{}] Timeout waiting for acknowledgment (5s)", worker_name);
            Err(anyhow::anyhow!("Acknowledgment timeout"))
        }
    }
}
