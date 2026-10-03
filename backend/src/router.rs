use crate::database::Database;
use crate::types::Trade;
use anyhow::{Context, Result};
use log::{error, info};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, watch, Mutex};

const ROUTER_PORT: u16 = 5000;

/// Tracks how many master (Signal Provider) connections are open and mirrors
/// "at least one is open" into `system_metrics.provider_connected`.
///
/// The in-memory counter is the source of truth. Every change to it is followed
/// by `sync()`, which takes `write_lock`, reads the counter *then* and writes it.
/// Because the lock serialises the writes and each one reads the latest count,
/// the last write always reflects the final state, even when a disconnect and a
/// reconnect race each other. Only connect/disconnect touch this, never the
/// per-trade hot path.
#[derive(Clone)]
pub struct ProviderStatus {
    connections: Arc<AtomicUsize>,
    write_lock: Arc<Mutex<()>>,
    db: Arc<Database>,
}

impl ProviderStatus {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            connections: Arc::new(AtomicUsize::new(0)),
            write_lock: Arc::new(Mutex::new(())),
            db,
        }
    }

    pub fn connection_count(&self) -> usize {
        self.connections.load(Ordering::SeqCst)
    }

    pub fn is_connected(&self) -> bool {
        self.connection_count() > 0
    }

    /// Write the current in-memory state to the DB.
    pub async fn sync(&self) -> Result<()> {
        // Held across the DB write on purpose: it orders the writes (see type docs).
        let _guard = self.write_lock.lock().await;
        let connected = self.is_connected();
        self.db
            .set_provider_connected(connected)
            .await
            .context("failed to write provider_connected")
    }

    /// Register an open master connection. The returned guard unregisters it on
    /// drop, so every exit path of the connection handler (clean EOF, read error,
    /// panic) is counted.
    ///
    /// The DB write is spawned rather than awaited so the first trade read after
    /// a (re)connect never waits behind it (it can queue behind worker inserts or
    /// a disconnect's sync). Ordering stays correct: the counter is incremented
    /// before the sync is spawned, and whichever sync takes `write_lock` last
    /// reads the final count.
    fn connect(&self) -> ProviderConnectionGuard {
        self.connections.fetch_add(1, Ordering::SeqCst);
        self.spawn_sync();
        ProviderConnectionGuard {
            status: self.clone(),
        }
    }

    fn spawn_sync(&self) {
        // Drop cannot await; if there is no runtime (process teardown) the next
        // startup resets the flag anyway.
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let status = self.clone();
            handle.spawn(async move {
                if let Err(e) = status.sync().await {
                    error!("[ROUTER] Failed to update provider status: {:#}", e);
                }
            });
        }
    }
}

struct ProviderConnectionGuard {
    status: ProviderStatus,
}

impl Drop for ProviderConnectionGuard {
    fn drop(&mut self) {
        self.status.connections.fetch_sub(1, Ordering::SeqCst);
        self.status.spawn_sync();
    }
}

pub async fn run_router(
    tx: broadcast::Sender<Trade>,
    db: Arc<Database>,
    shutdown_rx: watch::Receiver<bool>,
) -> Result<()> {
    let status = ProviderStatus::new(db);
    // Clear a stale `true` left behind by a crash or an unclean exit.
    if let Err(e) = status.sync().await {
        error!("[ROUTER] Failed to reset provider status: {:#}", e);
    }

    let address = format!("127.0.0.1:{}", ROUTER_PORT);
    let listener = crate::port_utils::bind_with_retry(&address).await?;
    info!("[ROUTER] Listening on port {} (TCP)", ROUTER_PORT);

    serve(listener, tx, status, shutdown_rx).await
}

async fn serve(
    listener: TcpListener,
    tx: broadcast::Sender<Trade>,
    status: ProviderStatus,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Result<()> {
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
                        let status_clone = status.clone();

                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(stream, tx_clone, status_clone, addr).await {
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
    status: ProviderStatus,
    addr: std::net::SocketAddr,
) -> Result<()> {
    info!("[ROUTER] Master MT5 connected from {}", addr);

    // Dropped on every return path below, including `?` on a read error.
    let _connection = status.connect();

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

    info!(
        "[ROUTER] Master MT5 disconnected from {} (processed {} trades)",
        addr, trade_count
    );
    info!("[ROUTER] Waiting for master MT5 to reconnect...");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpStream;
    use tokio::time::{sleep, timeout, Duration};

    struct Harness {
        addr: std::net::SocketAddr,
        db: Arc<Database>,
        status: ProviderStatus,
        trades: broadcast::Receiver<Trade>,
        shutdown_tx: watch::Sender<bool>,
    }

    async fn start() -> Harness {
        let db = Arc::new(Database::new(":memory:").await.unwrap());
        let status = ProviderStatus::new(db.clone());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, trades) = broadcast::channel(16);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        tokio::spawn(serve(listener, tx, status.clone(), shutdown_rx));
        Harness {
            addr,
            db,
            status,
            trades,
            shutdown_tx,
        }
    }

    async fn db_connected(db: &Database) -> bool {
        db.get_system_metrics()
            .await
            .unwrap()
            .map(|m| m.provider_connected)
            .unwrap_or(false)
    }

    /// Wait until both the in-memory count and the DB flag match.
    async fn wait_for(h: &Harness, count: usize, connected: bool) {
        let result = timeout(Duration::from_secs(5), async {
            loop {
                if h.status.connection_count() == count && db_connected(&h.db).await == connected {
                    return;
                }
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        assert!(
            result.is_ok(),
            "timed out waiting for count={} connected={} (count={}, db={})",
            count,
            connected,
            h.status.connection_count(),
            db_connected(&h.db).await
        );
    }

    #[tokio::test]
    async fn connect_and_clean_disconnect_toggle_provider_connected() {
        let mut h = start().await;

        let mut master = TcpStream::connect(h.addr).await.unwrap();
        wait_for(&h, 1, true).await;

        master
            .write_all(b"{\"id\":1,\"symbol\":\"EURUSD\",\"type\":\"buy\",\"lots\":0.1}\n")
            .await
            .unwrap();
        let trade = timeout(Duration::from_secs(5), h.trades.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(trade.id, 1);

        drop(master);
        wait_for(&h, 0, false).await;
        let _ = h.shutdown_tx.send(true);
    }

    // Regression: a read error used to return early via `?` and skip the
    // "set false" on disconnect, leaving the indicator stuck at true.
    #[tokio::test]
    async fn read_error_still_clears_provider_connected() {
        let h = start().await;

        let mut master = TcpStream::connect(h.addr).await.unwrap();
        wait_for(&h, 1, true).await;

        // Invalid UTF-8 makes `next_line()` return Err(InvalidData).
        master.write_all(&[0xff, 0xfe, b'\n']).await.unwrap();
        wait_for(&h, 0, false).await;

        drop(master);
        let _ = h.shutdown_tx.send(true);
    }

    // Regression: with two master connections open, closing one used to set
    // false while the other was still connected.
    #[tokio::test]
    async fn stays_connected_while_any_master_connection_is_open() {
        let h = start().await;

        let first = TcpStream::connect(h.addr).await.unwrap();
        let second = TcpStream::connect(h.addr).await.unwrap();
        wait_for(&h, 2, true).await;

        drop(first);
        wait_for(&h, 1, true).await;
        // `wait_for` can pass before the first disconnect's spawned sync has run.
        // `#[tokio::test]` is single-threaded and the guard's Drop has already
        // spawned that sync, so yielding lets it take `write_lock` first; then queue our own
        // sync behind it (the lock is FIFO): once ours returns, every earlier
        // write has landed, so the flag below is the settled value.
        tokio::task::yield_now().await;
        h.status.sync().await.unwrap();
        assert!(db_connected(&h.db).await);

        drop(second);
        wait_for(&h, 0, false).await;
        let _ = h.shutdown_tx.send(true);
    }

    #[tokio::test]
    async fn sync_resets_stale_true_when_nothing_is_connected() {
        let db = Arc::new(Database::new(":memory:").await.unwrap());
        db.set_provider_connected(true).await.unwrap();

        ProviderStatus::new(db.clone()).sync().await.unwrap();

        assert!(!db_connected(&db).await);
    }
}
