use anyhow::Result;
use tokio_rusqlite::Connection;

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
}
