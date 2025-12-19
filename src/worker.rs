use anyhow::Result;
use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use tokio::time::{timeout, Duration};
use crate::types::{Trade, SlaveConfig, Ack};

const ACK_TIMEOUT_MS: u64 = 1000;
const MAX_RETRIES: u32 = 3;
const BUFFER_SIZE: usize = 4096;

pub async fn run_worker(
    slave: SlaveConfig,
    mut rx: broadcast::Receiver<Trade>,
) -> Result<()> {
    println!("[WORKER:{}] Starting worker for {}", slave.name, slave.address);

    let socket = UdpSocket::bind(&slave.local_bind).await?;
    println!("[WORKER:{}] Bound to {}", slave.name, slave.local_bind);

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

                // Send trade with retry logic
                if let Err(e) = send_trade_with_retry(&socket, &slave, &trade).await {
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

async fn send_trade_with_retry(
    socket: &UdpSocket,
    slave: &SlaveConfig,
    trade: &Trade,
) -> Result<()> {
    let trade_json = serde_json::to_string(trade)?;

    for attempt in 1..=MAX_RETRIES {
        println!(
            "[WORKER:{}] Sending trade (attempt {}): {}",
            slave.name, attempt, trade_json
        );

        // Send trade to slave
        socket.send_to(trade_json.as_bytes(), &slave.address).await?;

        // Wait for ACK
        match wait_for_ack(socket, trade.id).await {
            Ok(_) => {
                println!("[WORKER:{}] ACK received for trade {}", slave.name, trade.id);
                return Ok(());
            }
            Err(e) => {
                eprintln!(
                    "[WORKER:{}] ACK timeout for trade {} (attempt {}): {}",
                    slave.name, trade.id, attempt, e
                );

                if attempt == MAX_RETRIES {
                    return Err(anyhow::anyhow!(
                        "Max retries reached for trade {}",
                        trade.id
                    ));
                }
            }
        }
    }

    Ok(())
}

async fn wait_for_ack(socket: &UdpSocket, expected_id: u64) -> Result<()> {
    let mut buf = vec![0u8; BUFFER_SIZE];

    match timeout(Duration::from_millis(ACK_TIMEOUT_MS), socket.recv_from(&mut buf)).await {
        Ok(Ok((len, _addr))) => {
            let data = &buf[..len];
            match serde_json::from_slice::<Ack>(data) {
                Ok(ack) => {
                    if ack.ack == expected_id {
                        Ok(())
                    } else {
                        Err(anyhow::anyhow!(
                            "ACK ID mismatch: expected {}, got {}",
                            expected_id,
                            ack.ack
                        ))
                    }
                }
                Err(e) => Err(anyhow::anyhow!("Failed to parse ACK: {}", e)),
            }
        }
        Ok(Err(e)) => Err(anyhow::anyhow!("Socket error: {}", e)),
        Err(_) => Err(anyhow::anyhow!("ACK timeout")),
    }
}
