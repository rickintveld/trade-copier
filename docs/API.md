# Trade Copier HTTP API

The Trade Copier exposes read-only HTTP endpoints for monitoring workers, trades, and errors.

## Base URL
```
http://localhost:8081
```

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
      "latency_ms": 45,
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
