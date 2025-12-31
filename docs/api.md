# HTTP API Documentation

## Overview

The Trade Copier HTTP API provides RESTful endpoints for managing workers, monitoring system health, and accessing trade history. The API runs on port 3000 with CORS enabled for cross-origin requests.

**Base URL**: `http://localhost:3000`

## Common Response Format

All API responses follow this format:

**Success Response:**
```json
{
  "success": true,
  "data": { /* response data */ }
}
```

**Error Response:**
```json
{
  "success": false,
  "error": "Error message description"
}
```

## Endpoints

### Health Check

Check if the API service is running.

**Endpoint**: `GET /api/health`

**Response**: `200 OK`
```json
{
  "success": true,
  "data": {
    "status": "ok",
    "message": "Trade Copier API is running"
  }
}
```

**Example**:
```bash
curl http://localhost:3000/api/health
```

---

### Get All Workers

Retrieve list of all configured workers with their current status.

**Endpoint**: `GET /api/workers`

**Response**: `200 OK`
```json
{
  "success": true,
  "data": [
    {
      "id": 1,
      "name": "worker-1",
      "address": "127.0.0.1:5050",
      "multiplier": 1.0,
      "state": "active",
      "last_error": null,
      "latency_us": 1234,
      "wine_prefix": "/Users/user/.wine-mt5-instance1",
      "mt5_connected": true,
      "created_at": "2025-01-01 12:00:00",
      "updated_at": "2025-01-01 12:30:00"
    }
  ]
}
```

**Fields**:
- `id`: Database ID
- `name`: Worker name
- `address`: TCP bind address
- `multiplier`: Lot size multiplier
- `state`: Current state (`active`, `inactive`, `error`, `installing`)
- `last_error`: Last error message (null if no error)
- `latency_us`: Last measured latency in microseconds (null if not measured)
- `wine_prefix`: Path to Wine installation (null if not applicable)
- `mt5_connected`: Whether MT5 terminal is connected
- `created_at`: Worker creation timestamp
- `updated_at`: Last update timestamp

**Example**:
```bash
curl http://localhost:3000/api/workers
```

---

### Get Trade History

Retrieve recent trade execution history.

**Endpoint**: `GET /api/trades?limit=100`

**Query Parameters**:
- `limit` (optional): Maximum number of trades to return (default: 100)

**Response**: `200 OK`
```json
{
  "success": true,
  "data": [
    {
      "id": 1,
      "trade_id": 1735689600000123,
      "worker_id": 1,
      "worker_name": "worker-1",
      "worker_address": "127.0.0.1:5050",
      "symbol": "EURUSD",
      "trade_type": "buy",
      "lots": 0.5,
      "price": 1.1234,
      "sl": 1.1200,
      "tp": 1.1300,
      "cmd": "open",
      "created_at": "2025-01-01 12:00:00"
    }
  ]
}
```

**Fields**:
- `id`: Database record ID
- `trade_id`: Original trade ID from master terminal
- `worker_id`: Worker database ID
- `worker_name`: Worker name
- `worker_address`: Worker TCP address
- `symbol`: Trading symbol
- `trade_type`: Trade direction (`buy` or `sell`, null for close/modify)
- `lots`: Adjusted lot size (after multiplier)
- `price`: Execution price (optional)
- `sl`: Stop loss (optional)
- `tp`: Take profit (optional)
- `cmd`: Trade command (`open`, `close`, `partial_close`, `modify`)
- `created_at`: Trade execution timestamp

**Examples**:
```bash
# Get last 100 trades
curl http://localhost:3000/api/trades

# Get last 50 trades
curl http://localhost:3000/api/trades?limit=50
```

---

### Get Error History

Retrieve recent error logs from all workers.

**Endpoint**: `GET /api/errors?limit=100`

**Query Parameters**:
- `limit` (optional): Maximum number of errors to return (default: 100)

**Response**: `200 OK`
```json
{
  "success": true,
  "data": [
    {
      "id": 1,
      "worker_id": 1,
      "worker_name": "worker-1",
      "worker_address": "127.0.0.1:5050",
      "severity": "error",
      "error_message": "Failed to send trade: Connection lost",
      "created_at": "2025-01-01 12:00:00"
    }
  ]
}
```

**Fields**:
- `id`: Database record ID
- `worker_id`: Worker database ID
- `worker_name`: Worker name
- `worker_address`: Worker TCP address
- `severity`: Error severity (`warning`, `error`, `critical`)
- `error_message`: Detailed error description
- `created_at`: Error occurrence timestamp

**Severity Levels**:
- `warning`: Non-critical issues
- `error`: Operational errors
- `critical`: Critical failures requiring immediate attention

**Examples**:
```bash
# Get last 100 errors
curl http://localhost:3000/api/errors

# Get last 25 errors
curl http://localhost:3000/api/errors?limit=25
```

---

### Get System Metrics

Retrieve system-wide metrics and status.

**Endpoint**: `GET /api/system/metrics`

**Response**: `200 OK`
```json
{
  "success": true,
  "data": {
    "id": 1,
    "router_status": "online",
    "router_port": 5000,
    "copier_active": true,
    "total_workers": 3,
    "active_workers": 2,
    "uptime_seconds": 3600,
    "updated_at": "2025-01-01 12:00:00"
  }
}
```

**Fields**:
- `id`: Always 1 (single-row table)
- `router_status`: Router status (`online` or `offline`)
- `router_port`: Router TCP port
- `copier_active`: Whether service is running
- `total_workers`: Total number of configured workers
- `active_workers`: Number of workers in `active` state
- `uptime_seconds`: Service uptime in seconds
- `updated_at`: Last metrics update timestamp

**Example**:
```bash
curl http://localhost:3000/api/system/metrics
```

---

### List MT5 Instances

List all MT5 instances (same as Get All Workers).

**Endpoint**: `GET /api/instances`

**Response**: `200 OK`

Returns same format as `GET /api/workers`.

**Example**:
```bash
curl http://localhost:3000/api/instances
```

---

### Create MT5 Instance

Create a new MT5 instance with automated installation (macOS only).

**Endpoint**: `POST /api/instances`

**Request Body**:
```json
{
  "name": "worker-2",
  "address": "127.0.0.1:5051",
  "multiplier": 0.5
}
```

**Request Fields**:
- `name` (required): Unique worker name
- `address` (required): TCP bind address (format: `IP:PORT`)
- `multiplier` (required): Lot size multiplier (typically 0.01 to 10.0)

**Response**: `201 Created`
```json
{
  "success": true,
  "data": {
    "id": 2,
    "name": "worker-2",
    "address": "127.0.0.1:5051",
    "multiplier": 0.5,
    "state": "installing",
    "last_error": null,
    "latency_us": null,
    "wine_prefix": "/Users/user/.wine-mt5-instance2",
    "mt5_connected": false,
    "created_at": "2025-01-01 12:00:00",
    "updated_at": "2025-01-01 12:00:00"
  }
}
```

**Process**:
1. Creates worker in database with state `installing`
2. Returns immediately with worker record
3. Background task:
   - Downloads MT5 installer
   - Creates Wine prefix
   - Installs MT5
   - Copies Expert Advisors
   - Updates state to `inactive`
   - Triggers worker reload

**Notes**:
- Installation happens asynchronously in background
- Poll `GET /api/workers` to check installation progress
- State transitions: `installing` → `inactive` → `active` (after manual start)
- Installation can take 5-10 minutes

**Error Response**: `500 Internal Server Error`
```json
{
  "success": false,
  "error": "Worker with name 'worker-2' or address '127.0.0.1:5051' already exists"
}
```

**Example**:
```bash
curl -X POST http://localhost:3000/api/instances \
  -H "Content-Type: application/json" \
  -d '{"name":"worker-2","address":"127.0.0.1:5051","multiplier":0.5}'
```

---

### Delete MT5 Instance

Delete an MT5 instance and all associated data.

**Endpoint**: `DELETE /api/instances/{name}?force=true`

**Path Parameters**:
- `name`: Worker name to delete

**Query Parameters**:
- `force` (required): Must be `true` to confirm deletion

**Response**: `200 OK`
```json
{
  "success": true,
  "data": {
    "message": "Instance 'worker-2' deleted successfully. Worker stopped."
  }
}
```

**Process**:
1. Stops worker (if running)
2. Deletes Wine prefix directory (if exists)
3. Removes worker from database
4. Cascades delete to trades and errors

**Notes**:
- Worker must not be in `active` state (stop it first)
- Deletion is permanent and cannot be undone
- All trade history and errors are deleted

**Error Response**: `500 Internal Server Error`
```json
{
  "success": false,
  "error": "Deletion cancelled - use force=true to confirm"
}
```

**Examples**:
```bash
# Without force (will fail)
curl -X DELETE http://localhost:3000/api/instances/worker-2

# With force (will succeed)
curl -X DELETE "http://localhost:3000/api/instances/worker-2?force=true"
```

---

### Start MT5 Instance

Start an MT5 instance and its associated worker.

**Endpoint**: `POST /api/instances/{name}/start`

**Path Parameters**:
- `name`: Worker name to start

**Response**: `200 OK`
```json
{
  "success": true,
  "data": {
    "message": "Instance 'worker-2' and worker started successfully"
  }
}
```

**Process**:
1. Copies Expert Advisors to MT5 directory
2. Launches MT5 application via Wine
3. Starts worker (subscribes to trade channel)
4. Worker waits for MT5 connection

**Notes**:
- Worker must exist in database
- Worker must not already be running
- MT5 application starts in background
- Manual MT5 login required (first time)

**Error Response**: `500 Internal Server Error`
```json
{
  "success": false,
  "error": "Instance 'worker-2' not found"
}
```

**Example**:
```bash
curl -X POST http://localhost:3000/api/instances/worker-2/start
```

---

### Stop MT5 Instance

Stop a running worker (does not close MT5 application).

**Endpoint**: `POST /api/instances/{name}/stop`

**Path Parameters**:
- `name`: Worker name to stop

**Response**: `200 OK`
```json
{
  "success": true,
  "data": {
    "message": "Worker 'worker-2' stop command sent successfully"
  }
}
```

**Process**:
1. Sends stop command to worker manager
2. Worker receives shutdown signal
3. Worker closes TCP connections
4. Worker state updated to `inactive`
5. Resources cleaned up

**Notes**:
- Graceful shutdown with 5-second timeout
- MT5 application continues running (not closed)
- Worker can be restarted without relaunching MT5

**Error Response**: `500 Internal Server Error`
```json
{
  "success": false,
  "error": "Failed to send stop command: channel closed"
}
```

**Example**:
```bash
curl -X POST http://localhost:3000/api/instances/worker-2/stop
```

---

## CORS Support

The API includes CORS support with permissive settings:
- All origins allowed (`Access-Control-Allow-Origin: *`)
- All methods allowed
- All headers allowed

Suitable for development. Consider restricting in production.

## Error Handling

All endpoints return appropriate HTTP status codes:

- `200 OK`: Successful request
- `201 Created`: Resource created successfully
- `500 Internal Server Error`: Server-side error

Error responses include descriptive messages in the `error` field.

## Rate Limiting

Currently no rate limiting implemented. Consider adding for production deployments.

## Authentication

Currently no authentication required. Consider adding for production deployments exposed to network.

## Best Practices

1. **Error Monitoring**: Regularly check `/api/errors` for issues
2. **Health Checks**: Monitor `/api/health` in automation scripts
3. **Polling**: Poll `/api/workers` during installations to track progress
4. **Force Deletes**: Always use `force=true` to avoid accidental deletions
5. **Graceful Operations**: Stop workers before deleting instances
6. **JSON Validation**: Validate request bodies before sending

## Example Workflows

### Creating and Starting a New Instance

```bash
# 1. Create instance
curl -X POST http://localhost:3000/api/instances \
  -H "Content-Type: application/json" \
  -d '{"name":"worker-3","address":"127.0.0.1:5052","multiplier":1.0}'

# 2. Wait for installation (poll status)
watch curl http://localhost:3000/api/workers

# 3. Start instance when state is "inactive"
curl -X POST http://localhost:3000/api/instances/worker-3/start

# 4. Configure and start Signal Receiver EA in MT5

# 5. Verify worker is active and MT5 is connected
curl http://localhost:3000/api/workers
```

### Monitoring System Health

```bash
# Check API health
curl http://localhost:3000/api/health

# Check system metrics
curl http://localhost:3000/api/system/metrics

# Check all workers
curl http://localhost:3000/api/workers

# Check recent errors
curl http://localhost:3000/api/errors?limit=10

# Check recent trades
curl http://localhost:3000/api/trades?limit=10
```

### Removing an Instance

```bash
# 1. Stop worker
curl -X POST http://localhost:3000/api/instances/worker-3/stop

# 2. Close MT5 application manually

# 3. Delete instance
curl -X DELETE "http://localhost:3000/api/instances/worker-3?force=true"
```
