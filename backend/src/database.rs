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

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum DependencyStatus {
    Pending,
    Installing,
    Installed,
    Error,
}

impl DependencyStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DependencyStatus::Pending => "pending",
            DependencyStatus::Installing => "installing",
            DependencyStatus::Installed => "installed",
            DependencyStatus::Error => "error",
        }
    }
    
    pub fn from_str(s: &str) -> Self {
        match s {
            "pending" => DependencyStatus::Pending,
            "installing" => DependencyStatus::Installing,
            "installed" => DependencyStatus::Installed,
            "error" => DependencyStatus::Error,
            _ => DependencyStatus::Pending,
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
                    symbol_prefix TEXT NOT NULL DEFAULT '',
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
            
            // Add total_trades column if it doesn't exist (migration)
            let _ = conn.execute(
                "ALTER TABLE system_metrics ADD COLUMN total_trades INTEGER NOT NULL DEFAULT 0",
                [],
            );
            
            // Add avg_latency_ms column if it doesn't exist (migration)
            let _ = conn.execute(
                "ALTER TABLE system_metrics ADD COLUMN avg_latency_ms REAL NOT NULL DEFAULT 0.0",
                [],
            );
            
            // Add provider_connected column if it doesn't exist (migration)
            let _ = conn.execute(
                "ALTER TABLE system_metrics ADD COLUMN provider_connected BOOLEAN NOT NULL DEFAULT 0",
                [],
            );
            
            // Create system_dependencies table for tracking dependency installation
            // Single row table - always id=1
            conn.execute(
                "CREATE TABLE IF NOT EXISTS system_dependencies (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    os_name TEXT NOT NULL,
                    os_version TEXT,
                    package_manager_name TEXT NOT NULL,
                    package_manager_status TEXT NOT NULL,
                    wine_status TEXT NOT NULL,
                    last_checked_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    error_message TEXT
                )",
                [],
            )?;
            
            // Create profits table for tracking trade profits
            conn.execute(
                "CREATE TABLE IF NOT EXISTS profits (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    worker_id INTEGER NOT NULL,
                    profit REAL NOT NULL,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY (worker_id) REFERENCES workers(id) ON DELETE CASCADE
                )",
                [],
            )?;
            
            // Create indexes for profits table
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_profits_worker_id ON profits(worker_id)",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_profits_created_at ON profits(created_at)",
                [],
            )?;
            
            // Create feature_toggles table
            conn.execute(
                "CREATE TABLE IF NOT EXISTS feature_toggles (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    key TEXT NOT NULL UNIQUE,
                    enabled BOOLEAN NOT NULL DEFAULT 1,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
                )",
                [],
            )?;
            
            // Seed default feature toggles (ignore if already exist)
            conn.execute(
                "INSERT OR IGNORE INTO feature_toggles (key, enabled) VALUES ('news_notifications', 1)",
                [],
            )?;
            conn.execute(
                "INSERT OR IGNORE INTO feature_toggles (key, enabled) VALUES ('error_notifications', 1)",
                [],
            )?;
            
            Ok(())
        }).await?;
        
        Ok(Self { conn })
    }
    
    pub async fn upsert_worker(&self, config: WorkerUpsertConfig) -> Result<()> {
        let state_str = config.state.as_str().to_string();
        let name = config.name;
        let address = config.address;
        let multiplier = config.multiplier;
        let error = config.error;
        let wine_prefix = config.wine_prefix;
        let symbol_prefix = config.symbol_prefix;
        
        self.conn.call(move |conn| {
            conn.execute(
                "INSERT INTO workers (name, address, multiplier, state, last_error, wine_prefix, symbol_prefix, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, CURRENT_TIMESTAMP)
                 ON CONFLICT(address) DO UPDATE SET
                    name = excluded.name,
                    multiplier = excluded.multiplier,
                    state = excluded.state,
                    last_error = excluded.last_error,
                    wine_prefix = excluded.wine_prefix,
                    symbol_prefix = excluded.symbol_prefix,
                    updated_at = CURRENT_TIMESTAMP",
                rusqlite::params![&name, &address, multiplier, &state_str, &error, &wine_prefix, &symbol_prefix],
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
        provider_connected: bool,
    ) -> Result<()> {
        let router_status = router_status.to_string();
        
        // Calculate total trades and avg latency from database
        let (total_trades, avg_latency_ms) = self.conn.call(|conn| {
            let total_trades: i64 = conn.query_row(
                "SELECT COALESCE(MAX(id), 0) FROM trades",
                [],
                |row| row.get(0),
            ).unwrap_or(0);
            
            let avg_latency: Option<f64> = conn.query_row(
                "SELECT AVG(latency_us) FROM workers WHERE latency_us IS NOT NULL AND latency_us > 0",
                [],
                |row| row.get(0),
            ).unwrap_or(None);
            
            let avg_latency_ms = avg_latency.unwrap_or(0.0) / 1000.0;
            
            Ok::<(i64, f64), tokio_rusqlite::Error>((total_trades, avg_latency_ms))
        }).await?;
        
        self.conn.call(move |conn| {
            conn.execute(
                "INSERT INTO system_metrics 
                 (id, router_status, router_port, copier_active, total_workers, active_workers, uptime_seconds, total_trades, avg_latency_ms, provider_connected, updated_at)
                 VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, CURRENT_TIMESTAMP)
                 ON CONFLICT(id) DO UPDATE SET
                    router_status = excluded.router_status,
                    router_port = excluded.router_port,
                    copier_active = excluded.copier_active,
                    total_workers = excluded.total_workers,
                    active_workers = excluded.active_workers,
                    uptime_seconds = excluded.uptime_seconds,
                    total_trades = excluded.total_trades,
                    avg_latency_ms = excluded.avg_latency_ms,
                    provider_connected = excluded.provider_connected,
                    updated_at = CURRENT_TIMESTAMP",
                rusqlite::params![&router_status, router_port, copier_active, total_workers, active_workers, uptime_seconds as i64, total_trades, avg_latency_ms, provider_connected],
            )?;
            Ok(())
        }).await?;
        
        Ok(())
    }
    
    // Query methods for API endpoints
    pub async fn get_all_workers(&self) -> Result<Vec<WorkerRecord>> {
        let result = self.conn.call(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, address, multiplier, state, last_error, latency_us, wine_prefix, mt5_connected, symbol_prefix, created_at, updated_at 
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
                    symbol_prefix: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
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
                        total_workers, active_workers, uptime_seconds, 
                        COALESCE(total_trades, 0) as total_trades,
                        COALESCE(avg_latency_ms, 0.0) as avg_latency_ms,
                        COALESCE(provider_connected, 0) as provider_connected,
                        updated_at
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
                    total_trades: row.get(7)?,
                    avg_latency_ms: row.get(8)?,
                    provider_connected: row.get(9)?,
                    updated_at: row.get(10)?,
                }))
            } else {
                Ok(None)
            }
        }).await?;
        
        Ok(result)
    }
    
    // Create a new worker configuration entry with a specific state and return the worker ID
    pub async fn create_worker_with_state(
        &self,
        name: &str,
        address: &str,
        multiplier: f64,
        state: WorkerState,
    ) -> Result<i64> {
        let name = name.to_string();
        let address = address.to_string();
        let state_str = state.as_str().to_string();
        
        let result = self.conn.call(move |conn| {
            // Check if worker with this name or address already exists
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM workers WHERE address = ?2",
                rusqlite::params![&name, &address],
                |row| row.get(0),
            )?;
            
            if count > 0 {
                return Err(tokio_rusqlite::Error::Rusqlite(
                    rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT),
                        Some(format!("Worker with address '{}' already exists", address)),
                    )
                ));
            }
            
            // Insert new worker with specified state
            conn.execute(
                "INSERT INTO workers (name, address, multiplier, state, updated_at)
                 VALUES (?1, ?2, ?3, ?4, CURRENT_TIMESTAMP)",
                rusqlite::params![&name, &address, multiplier, &state_str],
            )?;
            
            // Get the last inserted row ID
            let worker_id = conn.last_insert_rowid();
            
            Ok(worker_id)
        }).await;
        
        result.map_err(|e| anyhow::anyhow!(e))
    }
    
    /// Update editable worker settings by ID (name, address, multiplier, symbol_prefix)
    pub async fn update_worker_settings(
        &self,
        id: i64,
        name: &str,
        address: &str,
        multiplier: f64,
        symbol_prefix: &str,
    ) -> Result<()> {
        let name = name.to_string();
        let address = address.to_string();
        let symbol_prefix = symbol_prefix.to_string();
        
        self.conn.call(move |conn| {
            // Check if another worker already uses this address
            let conflict: Option<i64> = conn.query_row(
                "SELECT id FROM workers WHERE address = ?1 AND id != ?2",
                rusqlite::params![&address, id],
                |row| row.get(0),
            ).ok();
            
            if conflict.is_some() {
                return Err(tokio_rusqlite::Error::Rusqlite(
                    rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT),
                        Some(format!("Another worker already uses address '{}'", address)),
                    )
                ));
            }
            
            let rows_affected = conn.execute(
                "UPDATE workers SET name = ?1, address = ?2, multiplier = ?3, symbol_prefix = ?4, updated_at = CURRENT_TIMESTAMP WHERE id = ?5",
                rusqlite::params![&name, &address, multiplier, &symbol_prefix, id],
            )?;
            
            if rows_affected == 0 {
                return Err(tokio_rusqlite::Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows));
            }
            
            Ok(())
        }).await.map_err(|e| anyhow::anyhow!(e))?;
        
        Ok(())
    }
    
    /// Get all worker configurations for startup (simplified version without state)
    pub async fn get_worker_configs(&self) -> Result<Vec<WorkerConfig>> {
        let result = self.conn.call(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, address, multiplier, wine_prefix, COALESCE(symbol_prefix, '') as symbol_prefix FROM workers ORDER BY created_at ASC"
            )?;
            
            let configs = stmt.query_map([], |row| {
                Ok(WorkerConfig {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    address: row.get(2)?,
                    multiplier: row.get(3)?,
                    wine_prefix: row.get(4)?,
                    symbol_prefix: row.get(5)?,
                })
            })?.collect::<Result<Vec<_>, _>>()?;
            
            Ok(configs)
        }).await?;
        
        Ok(result)
    }

    /// Get a worker by name
    pub async fn get_worker_by_name(&self, name: &str) -> Result<Option<WorkerRecord>> {
        let name = name.to_string();
        
        let result = self.conn.call(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, address, multiplier, state, last_error, latency_us, wine_prefix, mt5_connected, COALESCE(symbol_prefix, '') as symbol_prefix, created_at, updated_at
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
                    symbol_prefix: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                }))
            } else {
                Ok(None)
            }
        }).await?;
        
        Ok(result)
    }
    
    /// Get a worker by ID
    pub async fn get_worker_by_id(&self, id: i64) -> Result<Option<WorkerRecord>> {
        let result = self.conn.call(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, address, multiplier, state, last_error, latency_us, wine_prefix, mt5_connected, COALESCE(symbol_prefix, '') as symbol_prefix, created_at, updated_at
                 FROM workers WHERE id = ?1"
            )?;
            
            let mut rows = stmt.query(rusqlite::params![id])?;
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
                    symbol_prefix: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                }))
            } else {
                Ok(None)
            }
        }).await?;
        
        Ok(result)
    }
    
    /// Delete a worker by ID
    pub async fn delete_worker_by_id(&self, id: i64) -> Result<()> {
        let result = self.conn.call(move |conn| {
            let rows_affected = conn.execute(
                "DELETE FROM workers WHERE id = ?1",
                rusqlite::params![id],
            )?;
            
            if rows_affected == 0 {
                return Err(tokio_rusqlite::Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows));
            }
            
            Ok(())
        }).await;
        
        match result {
            Ok(_) => Ok(()),
            Err(tokio_rusqlite::Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows)) => {
                Err(anyhow::anyhow!("Worker with ID {} not found", id))
            }
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }
    
    /// Upsert system dependencies status
    pub async fn upsert_system_dependencies(
        &self,
        os_name: &str,
        os_version: Option<&str>,
        package_manager_name: &str,
        package_manager_status: DependencyStatus,
        wine_status: DependencyStatus,
        error_message: Option<&str>,
    ) -> Result<()> {
        let os_name = os_name.to_string();
        let os_version = os_version.map(|s| s.to_string());
        let package_manager_name = package_manager_name.to_string();
        let pm_status_str = package_manager_status.as_str().to_string();
        let wine_status_str = wine_status.as_str().to_string();
        let error_message = error_message.map(|s| s.to_string());
        
        self.conn.call(move |conn| {
            conn.execute(
                "INSERT INTO system_dependencies 
                 (id, os_name, os_version, package_manager_name, package_manager_status, wine_status, last_checked_at, error_message)
                 VALUES (1, ?1, ?2, ?3, ?4, ?5, CURRENT_TIMESTAMP, ?6)
                 ON CONFLICT(id) DO UPDATE SET
                    os_name = excluded.os_name,
                    os_version = excluded.os_version,
                    package_manager_name = excluded.package_manager_name,
                    package_manager_status = excluded.package_manager_status,
                    wine_status = excluded.wine_status,
                    last_checked_at = CURRENT_TIMESTAMP,
                    error_message = excluded.error_message",
                rusqlite::params![&os_name, &os_version, &package_manager_name, &pm_status_str, &wine_status_str, &error_message],
            )?;
            Ok(())
        }).await?;
        
        Ok(())
    }
    
    /// Get system dependencies status
    pub async fn get_system_dependencies(&self) -> Result<Option<SystemDependenciesRecord>> {
        let result = self.conn.call(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, os_name, os_version, package_manager_name, package_manager_status, wine_status, last_checked_at, error_message
                 FROM system_dependencies
                 WHERE id = 1"
            )?;
            
            let mut rows = stmt.query([])?;
            if let Some(row) = rows.next()? {
                Ok(Some(SystemDependenciesRecord {
                    id: row.get(0)?,
                    os_name: row.get(1)?,
                    os_version: row.get(2)?,
                    package_manager_name: row.get(3)?,
                    package_manager_status: row.get(4)?,
                    wine_status: row.get(5)?,
                    last_checked_at: row.get(6)?,
                    error_message: row.get(7)?,
                }))
            } else {
                Ok(None)
            }
        }).await?;
        
        Ok(result)
    }
    
    /// Insert a profit record
    pub async fn insert_profit(
        &self,
        address: &str,
        profit: f64,
    ) -> Result<()> {
        let address = address.to_string();
        
        self.conn.call(move |conn| {
            // First, get the worker_id from the address
            let worker_id: i64 = conn.query_row(
                "SELECT id FROM workers WHERE address = ?1",
                rusqlite::params![&address],
                |row| row.get(0),
            )?;
            
            // Insert the profit record
            conn.execute(
                "INSERT INTO profits (worker_id, profit)
                 VALUES (?1, ?2)",
                rusqlite::params![worker_id, profit],
            )?;
            Ok(())
        }).await?;
        
        Ok(())
    }
    
    /// Get profit history for all workers or a specific worker
    pub async fn get_profit_history(
        &self,
        worker_id: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<ProfitRecord>> {
        let limit = limit.unwrap_or(1000);
        
        let result = self.conn.call(move |conn| {
            let map_row = |row: &rusqlite::Row| -> rusqlite::Result<ProfitRecord> {
                Ok(ProfitRecord {
                    id: row.get(0)?,
                    worker_id: row.get(1)?,
                    worker_name: row.get(2)?,
                    worker_address: row.get(3)?,
                    profit: row.get(4)?,
                    created_at: row.get(5)?,
                })
            };
            
            let profits = if let Some(wid) = worker_id {
                let mut stmt = conn.prepare(
                    "SELECT p.id, p.worker_id, w.name as worker_name, w.address as worker_address,
                            p.profit, p.created_at
                     FROM profits p
                     JOIN workers w ON p.worker_id = w.id
                     WHERE p.worker_id = ?1
                     ORDER BY p.created_at ASC
                     LIMIT ?2"
                )?;
                let rows = stmt.query_map(rusqlite::params![wid, limit], map_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                rows
            } else {
                let mut stmt = conn.prepare(
                    "SELECT p.id, p.worker_id, w.name as worker_name, w.address as worker_address,
                            p.profit, p.created_at
                     FROM profits p
                     JOIN workers w ON p.worker_id = w.id
                     ORDER BY p.created_at ASC
                     LIMIT ?1"
                )?;
                let rows = stmt.query_map(rusqlite::params![limit], map_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                rows
            };
            
            Ok(profits)
        }).await?;
        
        Ok(result)
    }
    
    /// Get all feature toggles
    pub async fn get_feature_toggles(&self) -> Result<Vec<FeatureToggleRecord>> {
        let result = self.conn.call(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, key, enabled, created_at, updated_at FROM feature_toggles ORDER BY id ASC"
            )?;
            
            let toggles = stmt.query_map([], |row| {
                Ok(FeatureToggleRecord {
                    id: row.get(0)?,
                    key: row.get(1)?,
                    enabled: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })?.collect::<Result<Vec<_>, _>>()?;
            
            Ok(toggles)
        }).await?;
        
        Ok(result)
    }
    
    /// Update a feature toggle by key
    pub async fn update_feature_toggle(&self, key: &str, enabled: bool) -> Result<()> {
        let key = key.to_string();
        
        self.conn.call(move |conn| {
            let rows_affected = conn.execute(
                "UPDATE feature_toggles SET enabled = ?1, updated_at = CURRENT_TIMESTAMP WHERE key = ?2",
                rusqlite::params![enabled, &key],
            )?;
            
            if rows_affected == 0 {
                return Err(tokio_rusqlite::Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows));
            }
            
            Ok(())
        }).await?;
        
        Ok(())
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
    pub symbol_prefix: String,
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
    pub total_trades: i64,
    pub avg_latency_ms: f64,
    pub provider_connected: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub id: i64,
    pub name: String,
    pub address: String,
    pub multiplier: f64,
    pub wine_prefix: Option<String>,
    pub symbol_prefix: String,
}

#[derive(Debug, Clone)]
pub struct WorkerUpsertConfig {
    pub name: String,
    pub address: String,
    pub multiplier: f64,
    pub state: WorkerState,
    pub error: Option<String>,
    pub wine_prefix: Option<String>,
    pub symbol_prefix: String,
}

#[derive(Debug, Serialize)]
pub struct SystemDependenciesRecord {
    pub id: i64,
    pub os_name: String,
    pub os_version: Option<String>,
    pub package_manager_name: String,
    pub package_manager_status: String,
    pub wine_status: String,
    pub last_checked_at: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProfitRecord {
    pub id: i64,
    pub worker_id: i64,
    pub worker_name: String,
    pub worker_address: String,
    pub profit: f64,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct FeatureToggleRecord {
    pub id: i64,
    pub key: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}
