use anyhow::Result;
use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use crate::types::Trade;

const ROUTER_PORT: u16 = 5000;
const BUFFER_SIZE: usize = 4096;

pub async fn run_router(tx: broadcast::Sender<Trade>) -> Result<()> {
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", ROUTER_PORT)).await?;
    println!("[ROUTER] Listening on port {}", ROUTER_PORT);

    let mut buf = vec![0u8; BUFFER_SIZE];

    loop {
        let (len, addr) = socket.recv_from(&mut buf).await?;
        let data = &buf[..len];

        match serde_json::from_slice::<Trade>(data) {
            Ok(trade) => {
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
}
