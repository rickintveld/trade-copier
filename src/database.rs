use anyhow::Result;
use tokio_rusqlite::Connection;
use serde::Serialize;
use crate::types::Trade;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WorkerState {
    Active,
    Error,
    Inactive,
    Installing,
}

impl WorkerState {
    fn as_str(&self) -> &'static str {
        match self {
            WorkerState::Active => "active",
            WorkerState::Error => "error",
            WorkerState::Inactive => "inactive",
            WorkerState::Installing => "installing",
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
                    latency_us INTEGER,
                    wine_prefix TEXT,
                    mt5_connected BOOLEAN NOT NULL DEFAULT 0,
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
                    trade_type TEXT,
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
            
            // Create the system_metrics table for tracking router and copier state
            // Single row table - always id=1
            conn.execute(
                "CREATE TABLE IF NOT EXISTS system_metrics (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    router_status TEXT NOT NULL,
                    router_port INTEGER NOT NULL,
                    copier_active BOOLEAN NOT NULL,
                    total_workers INTEGER NOT NULL,
                    active_workers INTEGER NOT NULL,
                    uptime_seconds INTEGER NOT NULL DEFAULT 0,
                    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
                )",
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
        wine_prefix: Option<&str>,
    ) -> Result<()> {
        let state_str = state.as_str().to_string();
        let name = name.to_string();
        let address = address.to_string();
        let error = error.map(|s| s.to_string());
        let wine_prefix = wine_prefix.map(|s| s.to_string());
        
        self.conn.call(move |conn| {
            conn.execute(
                "INSERT INTO workers (name, address, multiplier, state, last_error, wine_prefix, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, CURRENT_TIMESTAMP)
                 ON CONFLICT(address) DO UPDATE SET
                    name = excluded.name,
                    multiplier = excluded.multiplier,
                    state = excluded.state,
                    last_error = excluded.last_error,
                    wine_prefix = excluded.wine_prefix,
                    updated_at = CURRENT_TIMESTAMP",
                rusqlite::params![&name, &address, multiplier, &state_str, &error, &wine_prefix],
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
        latency_us: u64,
    ) -> Result<()> {
        let address = address.to_string();
        
        self.conn.call(move |conn| {
            conn.execute(
                "UPDATE workers 
                 SET latency_us = ?1, updated_at = CURRENT_TIMESTAMP
                 WHERE address = ?2",
                rusqlite::params![latency_us, &address],
            )?;
            Ok(())
        }).await?;
        
        Ok(())
    }
    
    pub async fn update_mt5_connected(
        &self,
        address: &str,
        connected: bool,
    ) -> Result<()> {
        let address = address.to_string();
        
        self.conn.call(move |conn| {
            conn.execute(
                "UPDATE workers 
                 SET mt5_connected = ?1, updated_at = CURRENT_TIMESTAMP
                 WHERE address = ?2",
                rusqlite::params![connected, &address],
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
    
    pub async fn upsert_system_metrics(
        &self,
        router_status: &str,
        router_port: u16,
        copier_active: bool,
        total_workers: i32,
        active_workers: i32,
        uptime_seconds: u64,
    ) -> Result<()> {
        let router_status = router_status.to_string();
        
        self.conn.call(move |conn| {
            conn.execute(
                "INSERT INTO system_metrics 
                 (id, router_status, router_port, copier_active, total_workers, active_workers, uptime_seconds, updated_at)
                 VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, CURRENT_TIMESTAMP)
                 ON CONFLICT(id) DO UPDATE SET
                    router_status = excluded.router_status,
                    router_port = excluded.router_port,
                    copier_active = excluded.copier_active,
                    total_workers = excluded.total_workers,
                    active_workers = excluded.active_workers,
                    uptime_seconds = excluded.uptime_seconds,
                    updated_at = CURRENT_TIMESTAMP",
                rusqlite::params![&router_status, router_port, copier_active, total_workers, active_workers, uptime_seconds as i64],
            )?;
            Ok(())
        }).await?;
        
        Ok(())
    }
    
    // Query methods for API endpoints
    pub async fn get_all_workers(&self) -> Result<Vec<WorkerRecord>> {
        let result = self.conn.call(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, address, multiplier, state, last_error, latency_us, wine_prefix, mt5_connected, created_at, updated_at 
                 FROM workers ORDER BY address, created_at DESC"
            )?;
            
            let workers = stmt.query_map([], |row| {
                Ok(WorkerRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    address: row.get(2)?,
                    multiplier: row.get(3)?,
                    state: row.get(4)?,
                    last_error: row.get(5)?,
                    latency_us: row.get(6)?,
                    wine_prefix: row.get(7)?,
                    mt5_connected: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
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
    
    pub async fn get_system_metrics(&self) -> Result<Option<SystemMetricsRecord>> {
        let result = self.conn.call(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, router_status, router_port, copier_active, 
                        total_workers, active_workers, uptime_seconds, updated_at
                 FROM system_metrics
                 WHERE id = 1"
            )?;
            
            let mut rows = stmt.query([])?;
            if let Some(row) = rows.next()? {
                Ok(Some(SystemMetricsRecord {
                    id: row.get(0)?,
                    router_status: row.get(1)?,
                    router_port: row.get(2)?,
                    copier_active: row.get(3)?,
                    total_workers: row.get(4)?,
                    active_workers: row.get(5)?,
                    uptime_seconds: row.get(6)?,
                    updated_at: row.get(7)?,
                }))
            } else {
                Ok(None)
            }
        }).await?;
        
        Ok(result)
    }
    
    // Create a new worker configuration entry with a specific state
    pub async fn create_worker_with_state(
        &self,
        name: &str,
        address: &str,
        multiplier: f64,
        state: WorkerState,
    ) -> Result<()> {
        let name = name.to_string();
        let address = address.to_string();
        let state_str = state.as_str().to_string();
        
        let result = self.conn.call(move |conn| {
            // Check if worker with this name or address already exists
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM workers WHERE name = ?1 OR address = ?2",
                rusqlite::params![&name, &address],
                |row| row.get(0),
            )?;
            
            if count > 0 {
                return Err(tokio_rusqlite::Error::Rusqlite(
                    rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT),
                        Some(format!("Worker with name '{}' or address '{}' already exists", name, address)),
                    )
                ));
            }
            
            // Insert new worker with specified state
            conn.execute(
                "INSERT INTO workers (name, address, multiplier, state, updated_at)
                 VALUES (?1, ?2, ?3, ?4, CURRENT_TIMESTAMP)",
                rusqlite::params![&name, &address, multiplier, &state_str],
            )?;
            
            Ok(())
        }).await;
        
        result.map_err(|e| anyhow::anyhow!(e))
    }
    
    /// Delete a worker by name
    pub async fn delete_worker(&self, name: &str) -> Result<()> {
        let name = name.to_string();
        let name_clone = name.clone();
        
        let result = self.conn.call(move |conn| {
            let rows_affected = conn.execute(
                "DELETE FROM workers WHERE name = ?1",
                rusqlite::params![&name],
            )?;
            
            if rows_affected == 0 {
                return Err(tokio_rusqlite::Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows));
            }
            
            Ok(())
        }).await;
        
        match result {
            Ok(_) => Ok(()),
            Err(tokio_rusqlite::Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows)) => {
                Err(anyhow::anyhow!("Worker '{}' not found", name_clone))
            }
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }
    
    /// Get all worker configurations for startup (simplified version without state)
    pub async fn get_worker_configs(&self) -> Result<Vec<WorkerConfig>> {
        let result = self.conn.call(|conn| {
            let mut stmt = conn.prepare(
                "SELECT name, address, multiplier, wine_prefix FROM workers ORDER BY created_at ASC"
            )?;
            
            let configs = stmt.query_map([], |row| {
                Ok(WorkerConfig {
                    name: row.get(0)?,
                    address: row.get(1)?,
                    multiplier: row.get(2)?,
                    wine_prefix: row.get(3)?,
                })
            })?.collect::<Result<Vec<_>, _>>()?;
            
            Ok(configs)
        }).await?;
        
        Ok(result)
    }

    /// Get a worker by name
    #[allow(dead_code)]
    pub async fn get_worker_by_name(&self, name: &str) -> Result<Option<WorkerRecord>> {
        let name = name.to_string();
        
        let result = self.conn.call(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, address, multiplier, state, last_error, latency_us, wine_prefix, mt5_connected, created_at, updated_at
                 FROM workers WHERE name = ?1"
            )?;
            
            let mut rows = stmt.query(rusqlite::params![&name])?;
            if let Some(row) = rows.next()? {
                Ok(Some(WorkerRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    address: row.get(2)?,
                    multiplier: row.get(3)?,
                    state: row.get(4)?,
                    last_error: row.get(5)?,
                    latency_us: row.get(6)?,
                    wine_prefix: row.get(7)?,
                    mt5_connected: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                }))
            } else {
                Ok(None)
            }
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
    pub latency_us: Option<i64>,
    pub wine_prefix: Option<String>,
    pub mt5_connected: bool,
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
    pub trade_type: Option<String>,
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

#[derive(Debug, Serialize)]
pub struct SystemMetricsRecord {
    pub id: i64,
    pub router_status: String,
    pub router_port: i64,
    pub copier_active: bool,
    pub total_workers: i64,
    pub active_workers: i64,
    pub uptime_seconds: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub name: String,
    pub address: String,
    pub multiplier: f64,
    pub wine_prefix: Option<String>,
}
