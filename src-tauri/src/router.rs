use anyhow::Result;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::io::{AsyncBufReadExt, BufReader};
use crate::types::Trade;

const ROUTER_PORT: u16 = 5000;

pub async fn run_router(tx: broadcast::Sender<Trade>) -> Result<()> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", ROUTER_PORT)).await?;
    println!("[ROUTER] Listening on port {} (TCP)", ROUTER_PORT);

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                println!("[ROUTER] New connection from {}", addr);
                let tx_clone = tx.clone();
                
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, tx_clone, addr).await {
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
    addr: std::net::SocketAddr,
) -> Result<()> {
    println!("[ROUTER] Master MT5 connected from {}", addr);
    
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
    Ok(())
}
