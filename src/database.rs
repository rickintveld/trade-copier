use anyhow::Result;
use tokio_rusqlite::Connection;
use crate::types::Trade;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WorkerState {
    Activated,
    Error,
    Deactivated,
}

impl WorkerState {
    fn as_str(&self) -> &'static str {
        match self {
            WorkerState::Activated => "activated",
            WorkerState::Error => "error",
            WorkerState::Deactivated => "deactivated",
        }
    }
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub async fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path).await?;
        
        // Create the workers table with unique address constraint
        conn.call(|conn| {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS workers (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    address TEXT NOT NULL UNIQUE,
                    multiplier REAL NOT NULL,
                    state TEXT NOT NULL,
                    last_error TEXT,
                    latency_ms INTEGER,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
                )",
                [],
            )?;
            
            // Create an index on state for faster queries
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_workers_state ON workers(state)",
                [],
            )?;
            
            // Create the trades table with foreign key to workers
            conn.execute(
                "CREATE TABLE IF NOT EXISTS trades (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    trade_id INTEGER NOT NULL,
                    worker_id INTEGER NOT NULL,
                    symbol TEXT NOT NULL,
                    trade_type TEXT NOT NULL,
                    lots REAL NOT NULL,
                    price REAL,
                    sl REAL,
                    tp REAL,
                    cmd TEXT NOT NULL,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY (worker_id) REFERENCES workers(id) ON DELETE CASCADE
                )",
                [],
            )?;
            
            // Create indexes for trades table
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_trades_worker_id ON trades(worker_id)",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_trades_trade_id ON trades(trade_id)",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_trades_cmd ON trades(cmd)",
                [],
            )?;
            
            Ok(())
        }).await?;
        
        Ok(Self { conn })
    }
    
    pub async fn upsert_worker(
        &self,
        name: &str,
        address: &str,
        multiplier: f64,
        state: WorkerState,
        error: Option<&str>,
    ) -> Result<()> {
        let state_str = state.as_str().to_string();
        let name = name.to_string();
        let address = address.to_string();
        let error = error.map(|s| s.to_string());
        
        self.conn.call(move |conn| {
            conn.execute(
                "INSERT INTO workers (name, address, multiplier, state, last_error, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, CURRENT_TIMESTAMP)
                 ON CONFLICT(address) DO UPDATE SET
                    name = excluded.name,
                    multiplier = excluded.multiplier,
                    state = excluded.state,
                    last_error = excluded.last_error,
                    updated_at = CURRENT_TIMESTAMP",
                rusqlite::params![&name, &address, multiplier, &state_str, &error],
            )?;
            Ok(())
        }).await?;
        
        Ok(())
    }
    
    pub async fn update_worker_state(
        &self,
        address: &str,
        state: WorkerState,
        error: Option<&str>,
    ) -> Result<()> {
        let state_str = state.as_str().to_string();
        let address = address.to_string();
        let error = error.map(|s| s.to_string());
        
        self.conn.call(move |conn| {
            conn.execute(
                "UPDATE workers 
                 SET state = ?1, last_error = ?2, updated_at = CURRENT_TIMESTAMP
                 WHERE address = ?3",
                rusqlite::params![&state_str, &error, &address],
            )?;
            Ok(())
        }).await?;
        
        Ok(())
    }
    
    pub async fn update_worker_latency(
        &self,
        address: &str,
        latency_ms: u64,
    ) -> Result<()> {
        let address = address.to_string();
        
        self.conn.call(move |conn| {
            conn.execute(
                "UPDATE workers 
                 SET latency_ms = ?1, updated_at = CURRENT_TIMESTAMP
                 WHERE address = ?2",
                rusqlite::params![latency_ms, &address],
            )?;
            Ok(())
        }).await?;
        
        Ok(())
    }
    
    pub async fn insert_trade(
        &self,
        address: &str,
        trade: &Trade,
    ) -> Result<()> {
        let address = address.to_string();
        let trade_id = trade.id;
        let symbol = trade.symbol.clone();
        let trade_type = trade.trade_type.clone();
        let lots = trade.lots;
        let price = trade.price;
        let sl = trade.sl;
        let tp = trade.tp;
        let cmd = trade.cmd.clone();
        
        self.conn.call(move |conn| {
            // First, get the worker_id from the address
            let worker_id: i64 = conn.query_row(
                "SELECT id FROM workers WHERE address = ?1",
                rusqlite::params![&address],
                |row| row.get(0),
            )?;
            
            // Insert the trade
            conn.execute(
                "INSERT INTO trades (trade_id, worker_id, symbol, trade_type, lots, price, sl, tp, cmd)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![trade_id, worker_id, &symbol, &trade_type, lots, price, sl, tp, &cmd],
            )?;
            Ok(())
        }).await?;
        
        Ok(())
    }
}
