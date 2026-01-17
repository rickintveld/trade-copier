use anyhow::Result;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::io::{AsyncBufReadExt, BufReader};
use std::sync::Arc;
use crate::types::Trade;
use crate::database::Database;
use log::{info, warn};

const ROUTER_PORT: u16 = 5000;
const MAX_BIND_RETRIES: u32 = 3;

pub async fn run_router(tx: broadcast::Sender<Trade>, db: Arc<Database>) -> Result<()> {
    let listener = bind_with_retry(ROUTER_PORT).await?;
    println!("[ROUTER] Listening on port {} (TCP)", ROUTER_PORT);

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                println!("[ROUTER] New connection from {}", addr);
                let tx_clone = tx.clone();
                let db_clone = db.clone();
                
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, tx_clone, db_clone, addr).await {
                        eprintln!("[ROUTER] Connection error from {}: {}", addr, e);
                    }
                });
            }
            Err(e) => {
                eprintln!("[ROUTER] Failed to accept connection: {}", e);
            }
        }
    }
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    tx: broadcast::Sender<Trade>,
    db: Arc<Database>,
    addr: std::net::SocketAddr,
) -> Result<()> {
    println!("[ROUTER] Master MT5 connected from {}", addr);
    
    // Update provider_connected status to true
    if let Err(e) = update_provider_status(&db, true).await {
        eprintln!("[ROUTER] Failed to update provider status: {}", e);
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
                println!("[ROUTER] Received trade from {}: {:?}", addr, trade);
                
                // Broadcast to all workers
                match tx.send(trade.clone()) {
                    Ok(receivers) => {
                        println!("[ROUTER] Broadcasted to {} workers", receivers);
                    }
                    Err(e) => {
                        eprintln!("[ROUTER] Failed to broadcast: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("[ROUTER] Failed to parse trade: {}", e);
            }
        }
    }

    println!("[ROUTER] Master MT5 disconnected from {} (processed {} trades)", addr, trade_count);
    println!("[ROUTER] Waiting for master MT5 to reconnect...");
    
    // Update provider_connected status to false
    if let Err(e) = update_provider_status(&db, false).await {
        eprintln!("[ROUTER] Failed to update provider status: {}", e);
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

/// Attempts to bind to the specified port, killing any process using it if necessary
async fn bind_with_retry(port: u16) -> Result<TcpListener> {
    for attempt in 1..=MAX_BIND_RETRIES {
        match TcpListener::bind(format!("0.0.0.0:{}", port)).await {
            Ok(listener) => {
                info!("[ROUTER] Successfully bound to port {}", port);
                return Ok(listener);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                warn!(
                    "[ROUTER] Port {} is already in use (attempt {}/{}). Attempting to free the port...",
                    port, attempt, MAX_BIND_RETRIES
                );
                
                if let Err(kill_err) = kill_process_on_port(port).await {
                    warn!("[ROUTER] Failed to kill process on port {}: {}", port, kill_err);
                }
                
                // Wait a bit for the port to be released
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                
                if attempt == MAX_BIND_RETRIES {
                    return Err(anyhow::anyhow!(
                        "Failed to bind to port {} after {} attempts. Port is still in use.",
                        port,
                        MAX_BIND_RETRIES
                    ));
                }
            }
            Err(e) => {
                return Err(e.into());
            }
        }
    }
    
    unreachable!()
}

/// Kills any process listening on the specified port
async fn kill_process_on_port(port: u16) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        
        // Use lsof to find the process using the port
        let output = Command::new("lsof")
            .args(["-ti", &format!(":{}", port)])
            .output()?;
        
        if output.status.success() {
            let pids = String::from_utf8_lossy(&output.stdout);
            for pid in pids.lines().filter(|line| !line.is_empty()) {
                info!("[ROUTER] Killing process {} using port {}", pid, port);
                let _ = Command::new("kill")
                    .args(["-9", pid])
                    .status();
            }
            return Ok(());
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        
        // Use netstat to find the process and taskkill to terminate it
        let output = Command::new("cmd")
            .args(["/C", &format!("netstat -ano | findstr :{}", port)])
            .output()?;
        
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            // Extract PID from netstat output (last column)
            if let Some(line) = output_str.lines().next() {
                if let Some(pid) = line.split_whitespace().last() {
                    info!("[ROUTER] Killing process {} using port {}", pid, port);
                    let _ = Command::new("taskkill")
                        .args(["/F", "/PID", pid])
                        .status();
                    return Ok(());
                }
            }
        }
    }
    
    Err(anyhow::anyhow!("Could not find or kill process on port {}", port))
}
