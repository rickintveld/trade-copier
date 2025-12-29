# 📘 Trade Copier – Complete Architecture & Technical Documentation

## 1. Overview
This project implements a **high-performance MetaTrader 5 → Rust → MetaTrader 5 trade copier**, designed for extremely low latency and unlimited scalability.

### Core workflow:
- **Master EA** sends trade signals via **TCP** (open, close, modify)
- **Rust Router** receives signals and broadcasts them to workers using a **tokio broadcast channel**
- **Rust Workers** act as TCP servers and send trades to connected slave EAs
- **Slave EA** connects to worker, receives signals and executes trades with full position tracking

### Features:
- ✅ **Open positions** - Copy new trades with lot multipliers
- ✅ **Close positions** - Close specific positions across all slaves
- ✅ **Partial closes** - Close partial volumes from positions
- ✅ **Modify SL/TP** - Update stop loss and take profit in real-time
- ✅ **Multiple positions** - Handle multiple positions on the same symbol
- ✅ **Position tracking** - Maintain mapping between master and slave positions

### Architecture Diagram

```
    MASTER EA
        │ TCP
        ▼
 ┌──────────────┐
 │ RUST ROUTER  │   tx.send(trade) → triggers ALL workers
 └───────┬──────┘
         │ broadcast channel
 ┌───────┼─────────┬─────────────┐
 │       │         │             │
 ▼       ▼         ▼             ▼
W1      W2        W3            Wn
(TCP)   (TCP)     (TCP)         (TCP)
 ▲       ▲         ▲            ▲
 │       │         │            │ TCP clients
SL1    SL2        SL3          SLn
(EA)   (EA)       (EA)         (EA)
```

## 2. Components
| Component | Purpose |
|----------|---------|
| **Signal Sender EA** | Detects new trades and sends signals to Rust Router via TCP |
| **Rust Router** | Receives TCP connections from sender, broadcasts to workers |
| **Rust Workers** | TCP servers that apply risk management and send trades to receivers |
| **Signal Receiver EA** | Connects to worker, executes received trades |

## 3. Configuration (YAML)
```yaml
slaves:
  - name: "Account01"
    address: "0.0.0.0:5051"
    multiplier: 1.0

  - name: "Account02"
    address: "0.0.0.0:5052"
    multiplier: 0.5

  - name: "Account03"
    address: "0.0.0.0:5053"
    multiplier: 2.0
```

## 4. Rust Project Structure
```
src/
 ├ main.rs       # Entry point, spawns router, workers, and API
 ├ router.rs     # TCP server receiving trades from master EA
 ├ worker.rs     # TCP servers broadcasting to slave EAs
 ├ api.rs        # HTTP REST API for monitoring
 ├ database.rs   # SQLite persistence for trades and errors
 ├ types.rs      # Shared data structures
config/
 └ slaves.yaml   # Worker configuration
mql5/
 ├ signal_sender.mq5   # Master EA
 └ signal_receiver.mq5 # Slave EA
```

## 5. Rust Router (Broadcast Pattern)
```rust
let (tx, _) = broadcast::channel::<Trade>(1024);

for slave in config.slaves {
    let mut rx = tx.subscribe();
    tokio::spawn(async move {
        worker_loop(slave, &mut rx).await;
    });
}

tx.send(trade)?;
```

## 6. Worker Design (Parallel + Independent)
```rust
while let Ok(trade) = rx.recv().await {
    process_trade(trade);
}
```

## 7. TCP Communication & Message Framing
Worker → Slave (newline-delimited JSON):

**Open position:**
```json
{"id":123456,"symbol":"EURUSD","type":"buy","lots":0.30,"price":1.08500,"sl":1.08000,"tp":1.09000,"cmd":"open"}
```

**Close position:**
```json
{"id":123456,"symbol":"EURUSD","type":"buy","lots":0.30,"cmd":"close"}
```

**Partial close:**
```json
{"id":123456,"symbol":"EURUSD","lots":0.10,"cmd":"partial_close"}
```

**Modify SL/TP:**
```json
{"id":123456,"symbol":"EURUSD","type":"buy","lots":0.30,"sl":1.07500,"tp":1.09500,"cmd":"modify"}
```

TCP provides built-in reliability, so no manual ACK/retry logic is needed.

## 8. Master Signal Sender EA (MT5)
```mql5
// Build JSON message
string json = "{";
json += "\"id\":" + IntegerToString(trade_id) + ",";
json += "\"symbol\":\"" + symbol + "\",";
json += "\"type\":\"" + trade_type + "\",";
json += "\"lots\":" + DoubleToString(lots, 2) + ",";
json += "\"price\":" + DoubleToString(price, 5);
if(sl > 0) json += ",\"sl\":" + DoubleToString(sl, 5);
if(tp > 0) json += ",\"tp\":" + DoubleToString(tp, 5);
json += ",\"cmd\":\"open\"}";
json += "\n";  // Add newline delimiter

// Send via TCP socket
uchar data[];
StringToCharArray(json, data, 0, StringLen(json));
int sent = SocketSend(socketHandle, data, ArraySize(data));
```

## 9. Slave Receiver EA
```mql5
void OnTick()
{
   CheckIncomingTrades();
}

void CheckIncomingTrades()
{
   uint len = SocketIsReadable(socketHandle);
   if(len == 0) return;
   
   uchar buffer[];
   ArrayResize(buffer, len);
   int received = SocketRead(socketHandle, buffer, len, 0);
   
   if(received > 0)
   {
      string data = CharArrayToString(buffer, 0, received);
      ParseAndExecuteTrade(data);
   }
}

bool ParseAndExecuteTrade(string json_data)
{
   // Extract fields: id, symbol, type, lots, price, sl, tp, cmd
   // Execute: trade.Buy() or trade.Sell()
   // TCP ensures delivery, no ACK needed
}
```

## 10. Optional Security Measures
- HMAC signatures  
- IP whitelisting  
- WireGuard  
- Nonce protection  

## 11. HTTP API
The Trade Copier includes a REST API for monitoring and querying trade data.

### API Server
- **Port:** 3000
- **Base URL:** `http://localhost:3000`
- **CORS:** Enabled (permissive)

### Endpoints

#### Health Check
```http
GET /api/health
```
Returns API status and health information.

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

#### Get Workers
```http
GET /api/workers
```
Returns all configured workers and their current state.

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "address": "0.0.0.0:5051",
      "state": "active",
      "connected_clients": 1,
      "last_updated": "2024-01-01T12:00:00Z"
    }
  ]
}
```

#### Get Trades
```http
GET /api/trades?limit=100
```
Returns recent trades with optional pagination.

**Query Parameters:**
- `limit` (optional, default: 100): Maximum number of trades to return

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "id": 123456,
      "symbol": "EURUSD",
      "type": "buy",
      "lots": 0.30,
      "price": 1.08500,
      "sl": 1.08000,
      "tp": 1.09000,
      "cmd": "open",
      "timestamp": "2024-01-01T12:00:00Z"
    }
  ]
}
```

#### Get Errors
```http
GET /api/errors?limit=100
```
Returns recent errors with optional pagination.

**Query Parameters:**
- `limit` (optional, default: 100): Maximum number of errors to return

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "worker_address": "0.0.0.0:5051",
      "error_message": "Connection timeout",
      "timestamp": "2024-01-01T12:00:00Z"
    }
  ]
}
```

#### Get System Metrics
```http
GET /api/system/metrics
```
Returns the latest system metrics snapshot.

**Response:**
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
    "created_at": "2024-01-01T12:00:00Z"
  }
}
```

#### Get System Metrics History
```http
GET /api/system/metrics/history?limit=100
```
Returns historical system metrics with optional pagination.

**Query Parameters:**
- `limit` (optional, default: 100): Maximum number of metrics snapshots to return

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "id": 2,
      "router_status": "online",
      "router_port": 5000,
      "copier_active": true,
      "total_workers": 3,
      "active_workers": 3,
      "uptime_seconds": 3630,
      "created_at": "2024-01-01T12:30:00Z"
    },
    {
      "id": 1,
      "router_status": "online",
      "router_port": 5000,
      "copier_active": true,
      "total_workers": 3,
      "active_workers": 2,
      "uptime_seconds": 3600,
      "created_at": "2024-01-01T12:00:00Z"
    }
  ]
}
```

### System Metrics Collection
The trade copier automatically collects system metrics every 30 seconds, including:
- **Router status** - Whether the router is online/offline
- **Router port** - The port the router listens on (5000)
- **Copier active** - Whether the trade copier is active
- **Total workers** - Number of configured workers
- **Active workers** - Number of workers currently in "activated" state
- **Uptime** - System uptime in seconds

### Error Responses
All endpoints return consistent error responses:
```json
{
  "success": false,
  "error": "Error message description"
}
```

## 12. Testing
### Unit, integration, end-to-end tests.

## 13. MetaTrader 5 Setup
Before running the EAs, configure MT5 permissions:

1. Open **Tools → Options → Expert Advisors**
2. Enable the following:
   - ☑ **Allow automated trading**
   - ☑ **Allow DLL imports**
3. In **"Allow WebRequest for listed URL"**, add:
   ```
   127.0.0.1:5000
   127.0.0.1:5050
   ```
4. Click **OK** and restart MT5

Without these permissions, the EAs will fail with "Failed to connect" errors.

## 14. MT5 Instance Management

### Installers for Windows and macOS

This project includes CLI tools to manage multiple MT5 slave instances:

#### Windows (`./installers/windows/`)

Build the MT5 manager:
```bash
cd installers/windows
cargo build --release
```

Create and manage multiple MT5 instances:
```bash
# Create instances
mt5-manager create slave1
mt5-manager create slave2

# List all instances
mt5-manager list

# Start all instances (with 2s delay between each)
mt5-manager start

# Start specific instance
mt5-manager start slave1

# Delete instance
mt5-manager delete slave1 --force
```

After creating instances, install MT5 from your broker to the created directories (e.g., `C:\MT5-slave1`).

#### macOS (`./installers/mac/`)

**Prerequisites:** Wine must be installed (`brew install --cask wine-stable`)

Build the MT5 manager:
```bash
cd installers/mac
cargo build --release
```

Create and manage MT5 instances with Wine:
```bash
# Create instance (installs MT5 automatically)
mt5-manager create --name slave1 --installer ~/Downloads/mt5setup.exe

# List all instances
mt5-manager list

# Start all instances
mt5-manager start --all

# Start specific instance
mt5-manager start slave1

# Delete instance
mt5-manager delete slave1
```

Each instance runs in an isolated Wine prefix at `~/.wine-mt5-instance{N}`.

See the respective README files in `./installers/windows/` and `./installers/mac/` for detailed documentation.

## 15. Deployment
Rust backend on Linux:
```bash
systemctl enable trade_copier
systemctl start trade_copier
```

## 16. Directory Structure
```
trade_copier/
 ├ src/
 │  ├ main.rs       # Entry point, spawns router, workers, and API
 │  ├ router.rs     # TCP server receiving trades from master EA
 │  ├ worker.rs     # TCP servers broadcasting to slave EAs
 │  ├ api.rs        # HTTP REST API for monitoring
 │  ├ database.rs   # SQLite persistence for trades and errors
 │  └ types.rs      # Shared data structures
 ├ config/
 │  └ slaves.yaml   # Worker configuration
 ├ mql5/
 │  ├ signal_sender.mq5   # Master EA
 │  └ signal_receiver.mq5 # Slave EA
 ├ installers/
 │  ├ windows/    # MT5 manager for Windows
 │  └ mac/        # MT5 manager for macOS
 ├ docs/
 ├ Cargo.toml
 ├ Cargo.lock
 ├ trade_copier.db  # SQLite database (created at runtime)
 └ README.md
```

## 17. Conclusion
A scalable, parallelized, low‑latency copier designed for professional environments.
