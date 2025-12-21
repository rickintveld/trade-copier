# 📘 Trade Copier – Complete Architecture & Technical Documentation

## 1. Overview
This project implements a **high-performance MetaTrader 5 → Rust → MetaTrader 5 trade copier**, designed for extremely low latency and unlimited scalability.

### Core workflow:
- **Master EA** sends trade signals via **TCP**.
- **Rust Router** receives signals and broadcasts them to workers using a **tokio broadcast channel**.
- **Rust Workers** act as TCP servers and send trades to connected slave EAs.
- **Slave EA** connects to worker, receives signals and executes trades.

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
 ├ main.rs
 ├ router.rs
 ├ worker.rs
 ├ types.rs
config/
 └ slaves.yaml
mql5/
 ├ signal_sender.mq5
 └ signal_receiver.mq5
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
```json
{"id":123456,"symbol":"EURUSD","type":"buy","lots":0.30,"price":1.08500,"sl":1.08000,"tp":1.09000,"cmd":"open"}
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

## 11. Testing
### Unit, integration, end-to-end tests.

## 12. MetaTrader 5 Setup
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

## 13. MT5 Instance Management

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

## 14. Deployment
Rust backend on Linux:
```bash
systemctl enable trade_copier
systemctl start trade_copier
```

## 15. Directory Structure
```
trade_copier/
 ├ src/
 ├ config/
 ├ mql5/
 ├ installers/
 │  ├ windows/    # MT5 manager for Windows
 │  └ mac/        # MT5 manager for macOS
 └ trade_copier.zip
```

## 15. Conclusion
A scalable, parallelized, low‑latency copier designed for professional environments.
