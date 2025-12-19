# 📘 Trade Copier – Complete Architecture & Technical Documentation

## 1. Overview
This project implements a **high-performance MetaTrader 5 → Rust → MetaTrader 5 trade copier**, designed for extremely low latency and unlimited scalability.

### Core workflow:
- **Master EA** sends trade signals via **UDP**.
- **Rust Router** receives signals and broadcasts them to workers using a **tokio broadcast channel**.
- **Rust Workers** apply individualized risk settings and forward signals via UDP.
- **Slave EA** receives signals, executes trades, and returns an ACK.

### Architecture Diagram

```
    MASTER EA
        │ UDP
        ▼
 ┌──────────────┐
 │ RUST ROUTER  │   tx.send(trade) → triggers ALL workers
 └───────┬──────┘
         │ broadcast channel
 ┌───────┼─────────┬─────────────┐
 │       │         │             │
 ▼       ▼         ▼             ▼
W1      W2        W3            Wn
UDP     UDP       UDP           UDP
 ▼       ▼         ▼            ▼
SL1    SL2        SL3          SLn
(EA)   (EA)       (EA)         (EA)
```

## 2. Components
| Component | Purpose |
|----------|---------|
| **Signal Sender EA** | Detects new trades and sends signals to Rust |
| **Rust Router** | Receives signals and broadcasts them |
| **Rust Workers** | Apply risk management and forward trades to slave terminals |
| **Signal Receiver EA** | Executes trades and sends ACK replies |

## 3. Configuration (YAML)
```yaml
slaves:
  - name: "Account01"
    address: "192.168.1.101:5050"
    local_bind: "0.0.0.0:6001"
    multiplier: 1.0

  - name: "Account02"
    address: "192.168.1.102:5050"
    local_bind: "0.0.0.0:6002"
    multiplier: 0.5

  - name: "Account03"
    address: "192.168.1.103:5050"
    local_bind: "0.0.0.0:6003"
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

## 7. UDP ACK + Retry Logic
Worker → Slave:
```json
{"id":123456,"symbol":"EURUSD","type":"buy","lots":0.30,"price":1.08500,"sl":1.08000,"tp":1.09000,"cmd":"open"}
```

Slave → Worker ACK:
```json
{"ack":123456}
```

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

// Send via UDP socket
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
   // Send ACK: SendAck(trade_id)
}
```

## 10. Optional Security Measures
- HMAC signatures  
- IP whitelisting  
- WireGuard  
- Nonce protection  

## 11. Testing
### Unit, integration, end-to-end tests.

## 12. Deployment
Rust backend on Linux:
```bash
systemctl enable trade_copier
systemctl start trade_copier
```

## 13. Directory Structure
```
trade_copier/
 ├ src/
 ├ config/
 ├ mql5/
 └ trade_copier.zip
```

## 14. Conclusion
A scalable, parallelized, low‑latency copier designed for professional environments.
