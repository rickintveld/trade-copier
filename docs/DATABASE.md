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

### View all workers
```bash
sqlite3 trade_copier.db "SELECT * FROM workers;"
```

### View active workers
```bash
sqlite3 trade_copier.db "SELECT name, address, multiplier FROM workers WHERE state = 'activated';"
```

### View workers with errors
```bash
sqlite3 trade_copier.db "SELECT name, address, state, last_error FROM workers WHERE state = 'error';"
```

### View worker history
```bash
sqlite3 trade_copier.db "SELECT name, address, state, updated_at FROM workers ORDER BY updated_at DESC;"
```

## Use Cases

1. **Monitoring**: Check which workers are currently active
2. **Debugging**: Review error messages for failed workers
3. **Analytics**: Track worker uptime and failure patterns
4. **Auditing**: Maintain a record of all worker configurations

## Notes

- The database uses SQLite's `ON CONFLICT` clause to handle address uniqueness
- If a worker with the same address is restarted, its entry is updated rather than creating a duplicate
- The database is automatically created on first run
- No manual initialization is required
