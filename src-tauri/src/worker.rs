use anyhow::Result;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, watch};
use tokio::io::{AsyncWriteExt, AsyncBufReadExt, BufReader};
use std::sync::Arc;
use std::time::{Instant, Duration};
use tokio::sync::Mutex;
use std::path::{Path, PathBuf};
use socket2::{Socket, TcpKeepalive};
use crate::database::{Database, WorkerState, ErrorSeverity};
use crate::types::{Trade, SlaveConfig, AccountInfo};

pub async fn run_worker(
    slave: SlaveConfig,
    mut rx: broadcast::Receiver<Trade>,
    db: Arc<Database>,
    mut shutdown_rx: watch::Receiver<bool>,
    wine_prefix: Option<PathBuf>,
) -> Result<()> {
    println!("[WORKER:{}] Starting TCP server on {}", slave.name, slave.address);

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
    let connection_monitor = connection.clone();
    let name_heartbeat = slave.name.clone();
    let address_heartbeat = slave.address.clone();
    let db_heartbeat = db.clone();
    let shutdown_heartbeat = shutdown_rx.clone();
    
    tokio::spawn(async move {
        monitor_connection_health(
            connection_monitor,
            name_heartbeat,
            address_heartbeat,
            db_heartbeat,
            shutdown_heartbeat,
        ).await;
    });

    // Spawn task to accept connections
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    println!("[WORKER:{}] MT5 receiver connected from {}", name_clone, addr);
                    
                    // Configure TCP keep-alive and socket options
                    if let Err(e) = configure_tcp_socket(&stream) {
                        eprintln!("[WORKER:{}] Failed to configure socket options: {}", name_clone, e);
                    } else {
                        println!("[WORKER:{}] TCP keep-alive configured", name_clone);
                    }
                    
                    *connection_clone.lock().await = Some(stream);
                    
                    // Update mt5_connected to true
                    if let Err(e) = db_clone.update_mt5_connected(&address_clone, true).await {
                        eprintln!("[WORKER:{}] Failed to update mt5_connected status: {}", name_clone, e);
                    }
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
                trade.lots *= slave.multiplier;
                trade.lots = (trade.lots * 100.0).round() / 100.0; // Round to 2 decimals
                
                // Apply symbol prefix if configured
                if !slave.symbol_prefix.is_empty() {
                    trade.symbol = format!("{}{}", trade.symbol, slave.symbol_prefix);
                    println!("[WORKER:{}] Applied symbol prefix: {}", slave.name, trade.symbol);
                }

                println!(
                    "[WORKER:{}] Adjusted lots: {} (multiplier: {})",
                    slave.name, trade.lots, slave.multiplier
                );

                    // Send trade to connected MT5 receiver and measure latency
                    match send_trade(&connection, &slave.name, &trade, &db, &slave.address).await {
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
    
    // Set mt5_connected to false
    if let Err(e) = db.update_mt5_connected(&slave.address, false).await {
        eprintln!("[WORKER:{}] Failed to update mt5_connected on shutdown: {}", slave.name, e);
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
    
    println!("[WORKER:{}] Starting Wine process monitor for prefix: {:?}", worker_name, wine_prefix);
    println!("[WORKER:{}] Waiting for Wine process to start before monitoring...", worker_name);
    
    // Phase 1: Wait for Wine process to start (grace period)
    loop {
        tokio::select! {
            _ = interval.tick() => {
                match is_wine_running(&wine_prefix).await {
                    Ok(true) => {
                        println!("[WORKER:{}] Wine process detected, starting monitoring", worker_name);
                        break;
                    }
                    Ok(false) => {
                        // Wine not running yet, keep waiting silently
                    }
                    Err(e) => {
                        eprintln!("[WORKER:{}] Error checking Wine process: {}", worker_name, e);
                    }
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    println!("[WORKER:{}] Wine monitor received shutdown signal before Wine started", worker_name);
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
                        eprintln!("[WORKER:{}] {}", worker_name, error_msg);
                        
                        // Update worker state
                        if let Err(e) = db.update_worker_state(
                            &address,
                            WorkerState::Inactive,
                            Some(&error_msg),
                        ).await {
                            eprintln!("[WORKER:{}] Failed to update state: {}", worker_name, e);
                        }
                        
                        // Update mt5_connected to false
                        if let Err(e) = db.update_mt5_connected(&address, false).await {
                            eprintln!("[WORKER:{}] Failed to update mt5_connected status: {}", worker_name, e);
                        }
                        
                        // Log warning
                        if let Err(e) = db.insert_worker_error(
                            &address,
                            ErrorSeverity::Warning,
                            &error_msg,
                        ).await {
                            eprintln!("[WORKER:{}] Failed to log error: {}", worker_name, e);
                        }
                        
                        println!("[WORKER:{}] Worker stopped due to Wine process termination", worker_name);
                        break;
                    }
                    Ok(true) => {
                        // Still running, continue monitoring
                    }
                    Err(e) => {
                        eprintln!("[WORKER:{}] Error checking Wine process: {}", worker_name, e);
                    }
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    println!("[WORKER:{}] Wine monitor received shutdown signal", worker_name);
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
    println!("[WINE] Terminating Wine server for prefix {:?}", wine_prefix);
    
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
        println!("[WINE] Successfully terminated Wine server");
    } else {
        let stderr = String::from_utf8_lossy(&result.stderr);
        eprintln!("[WINE] Warning: wineserver -k returned non-zero status: {}", stderr);
    }
    
    // Give Wine processes a moment to shut down
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    // Verify all processes are stopped
    if is_wine_running(wine_prefix).await? {
        eprintln!("[WINE] Warning: Some Wine processes still running after wineserver -k");
        
        // Fallback: try force kill with wineserver -k9
        println!("[WINE] Attempting force kill with wineserver -k9");
        let kill_result = tokio::process::Command::new(&wineserver)
            .arg("-k9")  // Force kill
            .env("WINEPREFIX", wine_prefix)
            .output()
            .await?;
        
        if !kill_result.status.success() {
            let stderr = String::from_utf8_lossy(&kill_result.stderr);
            eprintln!("[WINE] Force kill failed: {}", stderr);
        } else {
            println!("[WINE] Force kill completed");
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
    connection: Arc<Mutex<Option<tokio::net::TcpStream>>>,
    worker_name: String,
    address: String,
    db: Arc<Database>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let heartbeat_interval = Duration::from_secs(30);
    let mut interval = tokio::time::interval(heartbeat_interval);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    
    println!("[WORKER:{}] Connection health monitor started (heartbeat every 30s)", worker_name);
    
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let mut conn_guard = connection.lock().await;
                
                if let Some(stream) = conn_guard.as_mut() {
                    // Send heartbeat ping (simple newline)
                    let ping = b"PING\n";
                    
                    match stream.write_all(ping).await {
                        Ok(_) => {
                            // Connection is alive
                            // println!("[WORKER:{}] Heartbeat sent successfully", worker_name);
                        }
                        Err(e) => {
                            eprintln!("[WORKER:{}] Heartbeat failed, connection lost: {}", worker_name, e);
                            *conn_guard = None;
                            
                            // Update mt5_connected to false
                            if let Err(db_err) = db.update_mt5_connected(&address, false).await {
                                eprintln!("[WORKER:{}] Failed to update mt5_connected status: {}", worker_name, db_err);
                            }
                            
                            // Log error
                            let error_msg = format!("Heartbeat failed, connection lost: {}", e);
                            if let Err(db_err) = db.insert_worker_error(
                                &address,
                                ErrorSeverity::Warning,
                                &error_msg,
                            ).await {
                                eprintln!("[WORKER:{}] Failed to log error: {}", worker_name, db_err);
                            }
                        }
                    }
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    println!("[WORKER:{}] Connection health monitor shutting down", worker_name);
                    break;
                }
            }
        }
    }
}

/// Process incoming ACCOUNT_INFO message from MT5
async fn process_account_info(
    worker_name: &str,
    message: &str,
    db: &Arc<Database>,
    address: &str,
) {
    // Try to parse as AccountInfo
    match serde_json::from_str::<AccountInfo>(message) {
        Ok(account_info) => {
            println!(
                "[WORKER:{}] Received account info: balance={}, equity={}",
                worker_name, account_info.balance, account_info.equity
            );
            
            // Store account balance in database
            if let Err(e) = db.insert_account_balance(
                address,
                account_info.balance,
                account_info.equity,
                account_info.margin,
                "update",  // Event type: could be 'initial' or 'update'
                None,
            ).await {
                eprintln!("[WORKER:{}] Failed to save account balance: {}", worker_name, e);
            }
        }
        Err(e) => {
            eprintln!("[WORKER:{}] Failed to parse account info: {}", worker_name, e);
        }
    }
}

async fn send_trade(
    connection: &Arc<Mutex<Option<tokio::net::TcpStream>>>,
    worker_name: &str,
    trade: &Trade,
    db: &Arc<Database>,
    address: &str,
) -> Result<u64> {
    let trade_json = serde_json::to_string(trade)?;
    let message = format!("{}\n", trade_json);
    
    // Start timing
    let start = Instant::now();
    
    // Lock for the entire send/receive operation
    // Note: While this still holds the lock during I/O, it's necessary because
    // we need exclusive access to the stream for the full request-response cycle
    let mut conn_guard = connection.lock().await;
    
    if let Some(stream) = conn_guard.as_mut() {
        // Send the trade
        if let Err(e) = stream.write_all(message.as_bytes()).await {
            eprintln!("[WORKER:{}] Write error, connection lost: {}", worker_name, e);
            *conn_guard = None; // Clear dead connection
            
            // Update mt5_connected to false
            if let Err(db_err) = db.update_mt5_connected(address, false).await {
                eprintln!("[WORKER:{}] Failed to update mt5_connected status: {}", worker_name, db_err);
            }
            
            return Err(anyhow::anyhow!("Connection lost: {}", e));
        }
        
        println!("[WORKER:{}] Sent trade: {}", worker_name, trade_json);
        
        // Wait for acknowledgment from MT5 receiver with timeout
        let mut reader = BufReader::new(stream);
        let mut ack_line = String::new();
        
        // 5 second timeout for acknowledgment
        let read_result = tokio::time::timeout(
            Duration::from_secs(5),
            reader.read_line(&mut ack_line)
        ).await;
        
        match read_result {
            Err(_) => {
                // Timeout occurred
                eprintln!("[WORKER:{}] Timeout waiting for acknowledgment (5s)", worker_name);
                *conn_guard = None;
                
                // Update mt5_connected to false
                if let Err(e) = db.update_mt5_connected(address, false).await {
                    eprintln!("[WORKER:{}] Failed to update mt5_connected status: {}", worker_name, e);
                }
                
                Err(anyhow::anyhow!("Acknowledgment timeout"))
            }
            Ok(Ok(0)) => {
                // Connection closed
                eprintln!("[WORKER:{}] Connection closed while waiting for acknowledgment", worker_name);
                *conn_guard = None;
                
                // Update mt5_connected to false
                if let Err(e) = db.update_mt5_connected(address, false).await {
                    eprintln!("[WORKER:{}] Failed to update mt5_connected status: {}", worker_name, e);
                }
                
                Err(anyhow::anyhow!("Connection closed"))
            }
            Ok(Ok(_)) => {
                // Calculate latency in microseconds
                let latency_us = start.elapsed().as_micros() as u64;
                let response = ack_line.trim();
                println!("[WORKER:{}] Received response: {}", worker_name, response);
                
                // Check if response is account info (starts with '{' and contains "balance")
                if response.starts_with('{') && response.contains("balance") {
                    // Process account info asynchronously
                    let db_clone = db.clone();
                    let address_clone = address.to_string();
                    let worker_name_clone = worker_name.to_string();
                    let response_clone = response.to_string();
                    tokio::spawn(async move {
                        process_account_info(&worker_name_clone, &response_clone, &db_clone, &address_clone).await;
                    });
                }
                
                Ok(latency_us)
            }
            Ok(Err(e)) => {
                eprintln!("[WORKER:{}] Read error: {}", worker_name, e);
                *conn_guard = None;
                
                // Update mt5_connected to false
                if let Err(db_err) = db.update_mt5_connected(address, false).await {
                    eprintln!("[WORKER:{}] Failed to update mt5_connected status: {}", worker_name, db_err);
                }
                
                Err(anyhow::anyhow!("Failed to read acknowledgment: {}", e))
            }
        }
    } else {
        Err(anyhow::anyhow!("No MT5 receiver connected"))
    }
}
