use anyhow::Result;
use tokio_rusqlite::Connection;
use serde::Serialize;
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ErrorSeverity {
    Warning,
    Error,
    Critical,
}

impl ErrorSeverity {
    fn as_str(&self) -> &'static str {
        match self {
            ErrorSeverity::Warning => "warning",
            ErrorSeverity::Error => "error",
            ErrorSeverity::Critical => "critical",
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
            
            // Create the worker_errors table for detailed error tracking
            conn.execute(
                "CREATE TABLE IF NOT EXISTS worker_errors (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    worker_id INTEGER NOT NULL,
                    severity TEXT NOT NULL,
                    error_message TEXT NOT NULL,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY (worker_id) REFERENCES workers(id) ON DELETE CASCADE
                )",
                [],
            )?;
            
            // Create indexes for worker_errors table
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_worker_errors_worker_id ON worker_errors(worker_id)",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_worker_errors_severity ON worker_errors(severity)",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_worker_errors_created_at ON worker_errors(created_at)",
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
    
    pub async fn insert_worker_error(
        &self,
        address: &str,
        severity: ErrorSeverity,
        error_message: &str,
    ) -> Result<()> {
        let address = address.to_string();
        let severity_str = severity.as_str().to_string();
        let error_message = error_message.to_string();
        
        self.conn.call(move |conn| {
            // First, get the worker_id from the address
            let worker_id: i64 = conn.query_row(
                "SELECT id FROM workers WHERE address = ?1",
                rusqlite::params![&address],
                |row| row.get(0),
            )?;
            
            // Insert the error
            conn.execute(
                "INSERT INTO worker_errors (worker_id, severity, error_message)
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![worker_id, &severity_str, &error_message],
            )?;
            Ok(())
        }).await?;
        
        Ok(())
    }
    
    // Query methods for API endpoints
    pub async fn get_all_workers(&self) -> Result<Vec<WorkerRecord>> {
        let result = self.conn.call(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, address, multiplier, state, last_error, latency_ms, created_at, updated_at 
                 FROM workers ORDER BY created_at DESC"
            )?;
            
            let workers = stmt.query_map([], |row| {
                Ok(WorkerRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    address: row.get(2)?,
                    multiplier: row.get(3)?,
                    state: row.get(4)?,
                    last_error: row.get(5)?,
                    latency_ms: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })?.collect::<Result<Vec<_>, _>>()?;
            
            Ok(workers)
        }).await?;
        
        Ok(result)
    }
    
    pub async fn get_all_trades(&self, limit: Option<i64>) -> Result<Vec<TradeRecord>> {
        let limit = limit.unwrap_or(100);
        
        let result = self.conn.call(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT t.id, t.trade_id, t.worker_id, w.name as worker_name, w.address as worker_address,
                        t.symbol, t.trade_type, t.lots, t.price, t.sl, t.tp, t.cmd, t.created_at
                 FROM trades t
                 JOIN workers w ON t.worker_id = w.id
                 ORDER BY t.created_at DESC
                 LIMIT ?1"
            )?;
            
            let trades = stmt.query_map([limit], |row| {
                Ok(TradeRecord {
                    id: row.get(0)?,
                    trade_id: row.get(1)?,
                    worker_id: row.get(2)?,
                    worker_name: row.get(3)?,
                    worker_address: row.get(4)?,
                    symbol: row.get(5)?,
                    trade_type: row.get(6)?,
                    lots: row.get(7)?,
                    price: row.get(8)?,
                    sl: row.get(9)?,
                    tp: row.get(10)?,
                    cmd: row.get(11)?,
                    created_at: row.get(12)?,
                })
            })?.collect::<Result<Vec<_>, _>>()?;
            
            Ok(trades)
        }).await?;
        
        Ok(result)
    }
    
    pub async fn get_all_errors(&self, limit: Option<i64>) -> Result<Vec<ErrorRecord>> {
        let limit = limit.unwrap_or(100);
        
        let result = self.conn.call(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT e.id, e.worker_id, w.name as worker_name, w.address as worker_address,
                        e.severity, e.error_message, e.created_at
                 FROM worker_errors e
                 JOIN workers w ON e.worker_id = w.id
                 ORDER BY e.created_at DESC
                 LIMIT ?1"
            )?;
            
            let errors = stmt.query_map([limit], |row| {
                Ok(ErrorRecord {
                    id: row.get(0)?,
                    worker_id: row.get(1)?,
                    worker_name: row.get(2)?,
                    worker_address: row.get(3)?,
                    severity: row.get(4)?,
                    error_message: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?.collect::<Result<Vec<_>, _>>()?;
            
            Ok(errors)
        }).await?;
        
        Ok(result)
    }
}

// API response types
#[derive(Debug, Serialize)]
pub struct WorkerRecord {
    pub id: i64,
    pub name: String,
    pub address: String,
    pub multiplier: f64,
    pub state: String,
    pub last_error: Option<String>,
    pub latency_ms: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct TradeRecord {
    pub id: i64,
    pub trade_id: i64,
    pub worker_id: i64,
    pub worker_name: String,
    pub worker_address: String,
    pub symbol: String,
    pub trade_type: String,
    pub lots: f64,
    pub price: Option<f64>,
    pub sl: Option<f64>,
    pub tp: Option<f64>,
    pub cmd: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorRecord {
    pub id: i64,
    pub worker_id: i64,
    pub worker_name: String,
    pub worker_address: String,
    pub severity: String,
    pub error_message: String,
    pub created_at: String,
}
