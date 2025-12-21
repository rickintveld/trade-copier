use anyhow::Result;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::io::AsyncWriteExt;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::types::{Trade, SlaveConfig};

pub async fn run_worker(
    slave: SlaveConfig,
    mut rx: broadcast::Receiver<Trade>,
) -> Result<()> {
    println!("[WORKER:{}] Starting TCP server on {}", slave.name, slave.address);

    let listener = TcpListener::bind(&slave.address).await?;
    println!("[WORKER:{}] Listening on {} (TCP)", slave.name, slave.address);

    // Store active connection
    let connection: Arc<Mutex<Option<tokio::net::TcpStream>>> = Arc::new(Mutex::new(None));
    let connection_clone = connection.clone();
    let name_clone = slave.name.clone();

    // Spawn task to accept connections
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    println!("[WORKER:{}] MT5 receiver connected from {}", name_clone, addr);
                    *connection_clone.lock().await = Some(stream);
                }
                Err(e) => {
                    eprintln!("[WORKER:{}] Failed to accept connection: {}", name_clone, e);
                }
            }
        }
    });

    // Process trades from broadcast channel
    loop {
        match rx.recv().await {
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
                if let Err(e) = send_trade(&connection, &slave.name, &trade).await {
                    eprintln!("[WORKER:{}] Failed to send trade: {}", slave.name, e);
                }
            }
            Err(e) => {
                eprintln!("[WORKER:{}] Channel error: {}", slave.name, e);
                break;
            }
        }
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
