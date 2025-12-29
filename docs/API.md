# Trade Copier HTTP API

The Trade Copier exposes HTTP endpoints for monitoring workers, trades, errors, and managing MT5 instances.

## Base URLs
- Monitoring API: `http://localhost:8081`
- Instance Management API: `http://localhost:3000`

## Authentication
No authentication is required. This API is designed for local development use only.

## Endpoints

### Health Check
```
GET /api/health
```

Check if the API server is running.

**Response:**
```json
{
  "success": true,
  "data": {
    "status": "ok",
    "message": "Trade Copier API is running"
  }
}
```

---

### Get All Workers
```
GET /api/workers
```

Retrieve all configured workers with their current status.

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "id": 1,
      "name": "Slave 1",
      "address": "localhost:6000",
      "multiplier": 1.0,
      "state": "activated",
      "last_error": null,
      "latency_us": 45,
      "created_at": "2024-12-23 10:30:00",
      "updated_at": "2024-12-23 10:35:00"
    }
  ]
}
```

**Worker States:**
- `activated` - Worker is connected and processing trades
- `error` - Worker encountered an error
- `deactivated` - Worker is disconnected or shut down

---

### Get Recent Trades
```
GET /api/trades?limit=100
```

Retrieve recent trades executed by workers.

**Query Parameters:**
- `limit` (optional, default: 100) - Maximum number of trades to return

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "id": 1,
      "trade_id": 12345,
      "worker_id": 1,
      "worker_name": "Slave 1",
      "worker_address": "localhost:6000",
      "symbol": "EURUSD",
      "trade_type": "buy",
      "lots": 0.1,
      "price": 1.0850,
      "sl": 1.0800,
      "tp": 1.0900,
      "cmd": "open",
      "created_at": "2024-12-23 10:35:15"
    }
  ]
}
```

**Trade Commands:**
- `open` - Open a new position
- `close` - Close an existing position
- `partial_close` - Close a partial volume from an existing position
- `modify` - Modify an existing position

---

### Get Error Logs
```
GET /api/errors?limit=100
```

Retrieve recent error logs from workers.

**Query Parameters:**
- `limit` (optional, default: 100) - Maximum number of errors to return

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "id": 1,
      "worker_id": 1,
      "worker_name": "Slave 1",
      "worker_address": "localhost:6000",
      "severity": "error",
      "error_message": "Connection timeout",
      "created_at": "2024-12-23 10:30:00"
    }
  ]
}
```

**Error Severities:**
- `warning` - Minor issue, worker continues operating
- `error` - Significant issue, may affect operation
- `critical` - Severe issue, worker likely stopped

---

## Error Handling

All endpoints return errors in the following format:

```json
{
  "success": false,
  "error": "Error message here"
}
```

HTTP status codes:
- `200` - Success
- `500` - Internal server error (database or runtime error)

## CORS

CORS is enabled with permissive settings for local development.

## Example Usage

### Using curl
```bash
# Health check
curl http://localhost:8081/api/health

# Get all workers
curl http://localhost:8081/api/workers

# Get last 50 trades
curl http://localhost:8081/api/trades?limit=50

# Get last 20 errors
curl http://localhost:8081/api/errors?limit=20
```

### Using JavaScript (fetch)
```javascript
// Get all workers
fetch('http://localhost:8081/api/workers')
  .then(response => response.json())
  .then(data => console.log(data.data));

// Get recent trades
fetch('http://localhost:8081/api/trades?limit=50')
  .then(response => response.json())
  .then(data => console.log(data.data));
```

---

# MT5 Instance Management API

The Trade Copier includes an API for managing MT5 instances, including creating, starting, and deleting instances.

## Base URL
```
http://localhost:3000
```

## Prerequisites
- Trade Copier service must be running on port 3000
- On macOS: Wine must be installed (`brew install --cask wine-stable`)
- On Windows: Native MT5 installer will be used

## Endpoints

### List All Instances
```
GET /api/instances
```

Retrieve all configured MT5 instances.

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "id": 1,
      "name": "slave1",
      "address": "0.0.0.0:5051",
      "multiplier": 1.0,
      "path": "/Users/rickintveld/.wine-mt5-instance1",
      "created_at": "2025-12-29T15:00:00Z"
    }
  ]
}
```

---

### Create a New Instance
```
POST /api/instances
```

Create a new MT5 instance. This will:
- Add worker configuration to the database
- Download MT5 installer from official URL
- Create a Wine prefix (macOS) or directory (Windows)
- Install MT5 automatically
- Clean up the downloaded installer

**Request Body:**
```json
{
  "name": "slave2",
  "address": "0.0.0.0:5052",
  "multiplier": 1.5
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "id": 2,
    "name": "slave2",
    "address": "0.0.0.0:5052",
    "multiplier": 1.5,
    "path": "/Users/rickintveld/.wine-mt5-instance2",
    "created_at": "2025-12-29T16:00:00Z"
  }
}
```

**Important:** After creating an instance, you must restart the trade copier for the new worker to be spawned.

---

### Delete an Instance
```
DELETE /api/instances/:name?force=true
```

Delete an MT5 instance. By default, this will return a warning. Use `force=true` to actually delete the instance.

**Query Parameters:**
- `force` (optional, default: false) - Set to `true` to confirm deletion

**Response:**
```json
{
  "success": true,
  "data": {
    "message": "Instance 'slave2' deleted successfully"
  }
}
```

**Important:** After deleting an instance, you must restart the trade copier to stop the worker.

---

### Start a Specific Instance
```
POST /api/instances/:name/start
```

Start a specific MT5 instance.

**Response:**
```json
{
  "success": true,
  "data": {
    "message": "Instance 'slave1' started successfully"
  }
}
```

---

### Start All Instances
```
POST /api/instances/start-all
```

Start all configured MT5 instances.

**Response:**
```json
{
  "success": true,
  "data": {
    "message": "All instances started successfully"
  }
}
```

---

## Example Usage

### Using curl
```bash
# List all instances
curl -X GET http://localhost:3000/api/instances

# Create a new instance
curl -X POST http://localhost:3000/api/instances \
  -H "Content-Type: application/json" \
  -d '{
    "name": "slave2",
    "address": "0.0.0.0:5052",
    "multiplier": 1.5
  }'

# Delete an instance (with force)
curl -X DELETE "http://localhost:3000/api/instances/slave2?force=true"

# Start a specific instance
curl -X POST http://localhost:3000/api/instances/slave1/start

# Start all instances
curl -X POST http://localhost:3000/api/instances/start-all
```

### Creating Multiple Instances
```bash
# Instance 1
curl -X POST http://localhost:3000/api/instances \
  -H "Content-Type: application/json" \
  -d '{"name": "acc1", "address": "0.0.0.0:5051", "multiplier": 1.0}'

# Instance 2
curl -X POST http://localhost:3000/api/instances \
  -H "Content-Type: application/json" \
  -d '{"name": "acc2", "address": "0.0.0.0:5052", "multiplier": 1.5}'

# Instance 3
curl -X POST http://localhost:3000/api/instances \
  -H "Content-Type: application/json" \
  -d '{"name": "acc3", "address": "0.0.0.0:5053", "multiplier": 2.0}'
```

---

## Verification

### Check Configuration Files
```bash
# Check worker configurations in database
curl -X GET http://localhost:3000/api/workers

# Check instance config
cat ~/.mt5-manager/instances.json

# Check Wine prefixes (macOS)
ls -la ~/.wine-mt5-instance*
```

### Expected Database Workers
Workers are stored in the database (`trade_copier.db`) and can be viewed via the API:
```json
{
  "success": true,
  "data": [
    {
      "id": 1,
      "name": "FN-200k",
      "address": "0.0.0.0:5050",
      "multiplier": 2.0,
      "state": "activated",
      "last_error": null,
      "latency_us": 45,
      "created_at": "2025-12-29 10:30:00",
      "updated_at": "2025-12-29 10:35:00"
    }
  ]
}
```

---

## Troubleshooting

### Wine Not Found (macOS)
Install Wine if you get a Wine-related error:
```bash
brew install --cask wine-stable
```

### Check Installation Logs
The trade copier outputs detailed logs during instance creation:
```
[INSTALLER] Downloading MT5 installer from https://...
[INSTALLER] Downloaded MT5 installer to /tmp/...
[INSTALLER] Creating Wine prefix at /Users/.../.wine-mt5-instance2
[INSTALLER] Installing MT5... This may take a few minutes.
[INSTALLER] MT5 installed successfully
[INSTALLER] Cleaned up installer file
[INSTALLER] Instance 'slave2' created successfully!
[INSTALLER] NOTE: Restart the trade copier to activate the new worker
```

### Important Notes
1. **Database Configuration:** Worker configurations are stored in the database (`trade_copier.db`) and managed via the API
2. **Initial Setup:** On first startup, the database will be empty. Add workers using the `POST /api/instances` endpoint
3. **Wine Requirement (macOS):** Make sure Wine is installed before creating instances
4. **Restart Required:** After creating or deleting instances, restart the trade copier to spawn/stop workers
5. **Download Time:** Instance creation may take 5-10 minutes due to MT5 installer download and installation
6. **Installer Cleanup:** The downloaded MT5 installer is automatically removed after installation
