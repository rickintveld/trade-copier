# Database Documentation

## Overview

The trade copier now uses a SQLite database to persist slave worker state. This allows tracking of worker lifecycle, errors, and provides a historical record of worker activity.

## Database Location

The database file is created at: `trade_copier.db` in the project root directory.

## Schema

### Workers Table

```sql
CREATE TABLE workers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    address TEXT NOT NULL UNIQUE,
    multiplier REAL NOT NULL,
    state TEXT NOT NULL,
    last_error TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

**Constraints:**
- `address` is UNIQUE - prevents duplicate worker addresses
- Index on `state` for efficient querying

### Trades Table

```sql
CREATE TABLE trades (
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
);
```

**Constraints:**
- `worker_id` is a foreign key to `workers(id)` with CASCADE delete
- Indexes on `worker_id`, `trade_id`, and `cmd` for efficient querying

**Fields:**
- `id`: Auto-incrementing primary key
- `trade_id`: The original trade ID from MT5
- `worker_id`: Foreign key to the worker that processed this trade
- `symbol`: Trading symbol (e.g., "EURUSD")
- `trade_type`: Type of trade ("buy" or "sell")
- `lots`: Trade volume (after multiplier has been applied)
- `price`: Entry price (optional)
- `sl`: Stop loss level (optional)
- `tp`: Take profit level (optional)
- `cmd`: Command type ("open", "close", "modify")
- `created_at`: Timestamp when the trade was processed

## Worker States

Workers can be in one of three states:

1. **activated** - Worker has started successfully and is listening for connections
2. **error** - Worker encountered an error (stored in `last_error` field)
3. **deactivated** - Worker has been shut down gracefully

## State Transitions

### On Startup
When a worker starts:
- The worker is registered in the database (or updated if address already exists)
- State is set to `activated`
- `last_error` is cleared
- `updated_at` timestamp is set

### On Error
When a worker encounters an error:
- State is updated to `error`
- `last_error` field is populated with error message
- `updated_at` timestamp is set

Errors are tracked for:
- Failed to bind to address
- Failed to accept connection
- Failed to send trade
- Channel errors

### On Shutdown
When a worker shuts down gracefully:
- State is updated to `deactivated`
- `updated_at` timestamp is set
- `last_error` is preserved (if any existed)

## Querying the Database

### Workers Queries

#### View all workers
```bash
sqlite3 trade_copier.db "SELECT * FROM workers;"
```

#### View active workers
```bash
sqlite3 trade_copier.db "SELECT name, address, multiplier FROM workers WHERE state = 'activated';"
```

#### View workers with errors
```bash
sqlite3 trade_copier.db "SELECT name, address, state, last_error FROM workers WHERE state = 'error';"
```

#### View worker history
```bash
sqlite3 trade_copier.db "SELECT name, address, state, updated_at FROM workers ORDER BY updated_at DESC;"
```

### Trades Queries

#### View all trades
```bash
sqlite3 trade_copier.db "SELECT * FROM trades;"
```

#### View trades with worker information
```bash
sqlite3 trade_copier.db "SELECT t.*, w.name as worker_name, w.address FROM trades t JOIN workers w ON t.worker_id = w.id ORDER BY t.created_at DESC;"
```

#### View trades for a specific worker
```bash
sqlite3 trade_copier.db "SELECT t.* FROM trades t JOIN workers w ON t.worker_id = w.id WHERE w.name = 'Worker1' ORDER BY t.created_at DESC;"
```

#### View open positions (most recent 'open' command per trade_id)
```bash
sqlite3 trade_copier.db "SELECT t.*, w.name as worker_name FROM trades t JOIN workers w ON t.worker_id = w.id WHERE t.cmd = 'open' ORDER BY t.created_at DESC;"
```

#### View closed positions
```bash
sqlite3 trade_copier.db "SELECT t.*, w.name as worker_name FROM trades t JOIN workers w ON t.worker_id = w.id WHERE t.cmd = 'close' ORDER BY t.created_at DESC;"
```

#### View modified positions
```bash
sqlite3 trade_copier.db "SELECT t.*, w.name as worker_name FROM trades t JOIN workers w ON t.worker_id = w.id WHERE t.cmd = 'modify' ORDER BY t.created_at DESC;"
```

#### View trade statistics by worker
```bash
sqlite3 trade_copier.db "SELECT w.name, COUNT(t.id) as trade_count, SUM(t.lots) as total_lots FROM workers w LEFT JOIN trades t ON w.id = t.worker_id GROUP BY w.id, w.name;"
```

## Use Cases

### Worker Management
1. **Monitoring**: Check which workers are currently active
2. **Debugging**: Review error messages for failed workers
3. **Analytics**: Track worker uptime and failure patterns
4. **Auditing**: Maintain a record of all worker configurations

### Trade Tracking
1. **Position History**: View complete history of all positions processed by each worker
2. **Trade Analytics**: Analyze trade patterns, volumes, and frequencies
3. **Audit Trail**: Maintain a complete record of all buy, sell, close, and modify operations
4. **Performance Monitoring**: Track which workers are processing the most trades
5. **Debugging**: Investigate trade processing issues by reviewing trade history
6. **Compliance**: Maintain records for regulatory requirements

## Notes

- The database uses SQLite's `ON CONFLICT` clause to handle address uniqueness
- If a worker with the same address is restarted, its entry is updated rather than creating a duplicate
- The database is automatically created on first run
- No manual initialization is required
