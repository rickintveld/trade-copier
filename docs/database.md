# Database Documentation

## Overview

Trade Copier uses SQLite for persistent storage of worker configurations, trade history, error logs, and system metrics. The database is stored in `trade_copier.db` at the application root.

## Database Schema

### Workers Table

Stores worker configurations and current state.

```sql
CREATE TABLE workers (
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
)
```

**Fields:**
- `id`: Auto-incrementing primary key
- `name`: Unique worker name/identifier
- `address`: TCP bind address (e.g., `127.0.0.1:5050`) - **UNIQUE**
- `multiplier`: Lot size multiplier (e.g., `1.0` = same size, `0.5` = half size)
- `state`: Current state - `active`, `inactive`, `error`, or `installing`
- `last_error`: Most recent error message (if any)
- `latency_us`: Last measured latency in microseconds
- `wine_prefix`: Path to Wine prefix for MT5 installation (macOS only)
- `mt5_connected`: Boolean indicating if MT5 terminal is connected
- `created_at`: Worker creation timestamp
- `updated_at`: Last state update timestamp

**Indexes:**
- `idx_workers_state` on `state` column for fast state-based queries

**States:**
- `active`: Worker is running and accepting trades
- `inactive`: Worker is stopped or not running
- `error`: Worker encountered an error
- `installing`: MT5 instance is being installed (temporary state)

### Trades Table

Stores complete audit log of all forwarded trades.

```sql
CREATE TABLE trades (
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
)
```

**Fields:**
- `id`: Auto-incrementing primary key
- `trade_id`: Original trade ID from master terminal
- `worker_id`: Foreign key to workers table
- `symbol`: Trading symbol (e.g., `EURUSD`)
- `trade_type`: Trade direction - `buy` or `sell` (null for close/modify)
- `lots`: Adjusted lot size (after applying multiplier)
- `price`: Execution price (optional)
- `sl`: Stop loss level (optional)
- `tp`: Take profit level (optional)
- `cmd`: Trade command - `open`, `close`, `partial_close`, or `modify`
- `created_at`: Trade execution timestamp

**Indexes:**
- `idx_trades_worker_id` on `worker_id`
- `idx_trades_trade_id` on `trade_id`
- `idx_trades_cmd` on `cmd`

**Foreign Keys:**
- `worker_id` references `workers(id)` with `ON DELETE CASCADE`

### Worker Errors Table

Tracks detailed error history for each worker.

```sql
CREATE TABLE worker_errors (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    worker_id INTEGER NOT NULL,
    severity TEXT NOT NULL,
    error_message TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (worker_id) REFERENCES workers(id) ON DELETE CASCADE
)
```

**Fields:**
- `id`: Auto-incrementing primary key
- `worker_id`: Foreign key to workers table
- `severity`: Error severity - `warning`, `error`, or `critical`
- `error_message`: Detailed error description
- `created_at`: Error occurrence timestamp

**Indexes:**
- `idx_worker_errors_worker_id` on `worker_id`
- `idx_worker_errors_severity` on `severity`
- `idx_worker_errors_created_at` on `created_at`

**Severity Levels:**
- `warning`: Non-critical issues (e.g., failed to save trade to DB, Wine process stopped)
- `error`: Operational errors (e.g., connection failures, send failures)
- `critical`: Critical failures requiring immediate attention (e.g., channel broken, bind failure)

### System Metrics Table

Stores global system status (single-row table).

```sql
CREATE TABLE system_metrics (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    router_status TEXT NOT NULL,
    router_port INTEGER NOT NULL,
    copier_active BOOLEAN NOT NULL,
    total_workers INTEGER NOT NULL,
    active_workers INTEGER NOT NULL,
    uptime_seconds INTEGER NOT NULL DEFAULT 0,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
)
```

**Fields:**
- `id`: Always `1` (single row enforced by CHECK constraint)
- `router_status`: Router status - `online` or `offline`
- `router_port`: Router TCP port (default: 5000)
- `copier_active`: Boolean indicating if service is running
- `total_workers`: Total number of configured workers
- `active_workers`: Number of workers in `active` state
- `uptime_seconds`: Service uptime in seconds
- `updated_at`: Last metrics update timestamp

## Database Operations

### Worker Management

**Create Worker:**
```rust
db.create_worker_with_state(name, address, multiplier, WorkerState::Inactive).await?;
```

**Update Worker State:**
```rust
db.update_worker_state(address, WorkerState::Active, None).await?;
db.update_worker_state(address, WorkerState::Error, Some("error message")).await?;
```

**Update Latency:**
```rust
db.update_worker_latency(address, latency_us).await?;
```

**Update MT5 Connection Status:**
```rust
db.update_mt5_connected(address, true).await?;
```

**Delete Worker:**
```rust
db.delete_worker(name).await?; // Cascades to trades and errors
```

### Trade Logging

**Insert Trade:**
```rust
db.insert_trade(address, &trade).await?;
```

### Error Logging

**Insert Error:**
```rust
db.insert_worker_error(address, ErrorSeverity::Warning, "message").await?;
```

### Query Operations

**Get All Workers:**
```rust
let workers = db.get_all_workers().await?;
```

**Get Worker by Name:**
```rust
let worker = db.get_worker_by_name(name).await?;
```

**Get Trade History:**
```rust
let trades = db.get_all_trades(Some(100)).await?; // Limit 100
```

**Get Error History:**
```rust
let errors = db.get_all_errors(Some(100)).await?; // Limit 100
```

**Get System Metrics:**
```rust
let metrics = db.get_system_metrics().await?;
```

## Database Lifecycle

### Initialization

Database is automatically initialized on first run:
1. Creates database file at `./trade_copier.db`
2. Creates all tables with indexes
3. Creates foreign key constraints

### Maintenance

**Metrics Updates:**
- System metrics are updated every 30 seconds by a background task
- Final metrics are written on shutdown

**Error Cleanup:**
- No automatic cleanup (errors are kept for audit)
- Consider manual cleanup for production deployments

**Cascade Deletes:**
- Deleting a worker automatically removes associated trades and errors

## Best Practices

1. **Worker Names**: Use descriptive names (e.g., `account-1`, `client-john`)
2. **Address Format**: Always use `IP:PORT` format (e.g., `127.0.0.1:5050`)
3. **Multiplier Range**: Typically 0.01 to 10.0, though no hard limit enforced
4. **Error Monitoring**: Regularly check `worker_errors` for critical issues
5. **Backup**: Periodically backup `trade_copier.db` for disaster recovery

## Performance

- **Connection Pooling**: Uses `tokio-rusqlite` for async SQLite access
- **Indexes**: Optimized for common query patterns
- **Non-blocking Operations**: Trade logging happens in background tasks
- **Single File**: All data in one SQLite file for easy backup/restore

## Migration Notes

Currently no migration system in place. For schema changes:
1. Backup existing database
2. Manually alter schema or recreate
3. Import worker configurations via API
