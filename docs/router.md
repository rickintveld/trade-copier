# Router Documentation

## Overview

The Router is a TCP server that receives trade signals from the master MT5 terminal (Signal Provider EA) and broadcasts them to all registered workers. It acts as the central hub for trade distribution.

## Architecture

```
Signal Provider EA (Master MT5)
         |
         | TCP Connection
         v
    Router (0.0.0.0:5000)
         |
         | Broadcast Channel (tokio::sync::broadcast)
         v
    All Active Workers
```

## Configuration

**Port**: 5000 (hardcoded in `router.rs`)  
**Protocol**: TCP  
**Format**: Newline-delimited JSON

## Connection Handling

### Server Setup

The router binds to `0.0.0.0:5000` and accepts multiple concurrent connections:

```rust
let listener = TcpListener::bind("0.0.0.0:5000").await?;
```

### Connection Lifecycle

1. **Accept Connection**: Router accepts TCP connection from master MT5
2. **Spawn Handler**: Each connection is handled in a separate tokio task
3. **Read Messages**: Continuously reads newline-delimited JSON messages
4. **Broadcast**: Each valid trade is broadcast to all workers
5. **Disconnect**: Connection closes when master MT5 disconnects

### Concurrent Connections

Multiple master terminals can connect simultaneously. Each connection:
- Runs in its own async task
- Broadcasts to the same shared channel
- Operates independently

## Message Format

### Input Format

Router receives JSON messages terminated by newline (`\n`):

```json
{"id":1234567890,"symbol":"EURUSD","type":"buy","lots":1.0,"price":1.1234,"sl":1.1200,"tp":1.1300,"cmd":"open"}
```

### Trade Structure

```rust
struct Trade {
    id: u64,              // Unique trade identifier
    symbol: String,       // Trading symbol (e.g., "EURUSD")
    trade_type: Option<String>,  // "buy" or "sell" (optional for close/modify)
    lots: f64,            // Lot size
    price: Option<f64>,   // Execution price
    sl: Option<f64>,      // Stop loss level
    tp: Option<f64>,      // Take profit level
    cmd: String,          // Command: "open", "close", "partial_close", "modify"
}
```

### Supported Commands

- **open**: Open a new position
- **close**: Close entire position
- **partial_close**: Close partial position
- **modify**: Modify SL/TP levels

## Broadcasting Mechanism

### Broadcast Channel

Uses Tokio's broadcast channel with capacity of 1024 messages:

```rust
let (tx, _rx) = broadcast::channel::<Trade>(1024);
```

**Channel Properties:**
- Multiple senders (router + any other components)
- Multiple receivers (all workers subscribe)
- Lossy if consumer can't keep up (messages dropped)
- Non-blocking sends

### Broadcast Process

1. **Parse JSON**: Deserialize incoming message to `Trade` struct
2. **Validate**: Ensure all required fields are present
3. **Send**: Broadcast to channel
4. **Log**: Print confirmation with receiver count

```rust
match tx.send(trade.clone()) {
    Ok(receivers) => {
        println!("[ROUTER] Broadcasted to {} workers", receivers);
    }
    Err(e) => {
        eprintln!("[ROUTER] Failed to broadcast: {}", e);
    }
}
```

### Receiver Count

The `send()` method returns the number of active receivers. This indicates:
- Number of workers currently subscribed to trade channel
- Does NOT guarantee successful processing by workers

## Error Handling

### Parse Errors

If JSON is malformed or missing required fields:
```
[ROUTER] Failed to parse trade: <error details>
```

Trade is logged but not broadcast.

### Connection Errors

If connection fails during read:
```
[ROUTER] Connection error from <address>: <error>
```

Connection handler terminates, but router continues accepting new connections.

### Broadcast Errors

If broadcast fails (no receivers or channel closed):
```
[ROUTER] Failed to broadcast: <error>
```

This is logged but doesn't stop the router.

## Logging

Router logs all significant events:

### Connection Events
```
[ROUTER] Listening on port 5000 (TCP)
[ROUTER] New connection from 127.0.0.1:54321
[ROUTER] Connection closed from 127.0.0.1:54321
```

### Trade Events
```
[ROUTER] Received trade from 127.0.0.1:54321: Trade { ... }
[ROUTER] Broadcasted to 3 workers
```

### Error Events
```
[ROUTER] Failed to parse trade: missing field `symbol`
[ROUTER] Failed to accept connection: <error>
```

## Performance Characteristics

### Throughput

- **Non-blocking I/O**: Async Tokio runtime
- **Concurrent connections**: Multiple masters supported
- **Channel capacity**: 1024 messages buffer
- **Zero-copy broadcast**: Uses Arc internally

### Latency

Router adds minimal latency:
1. TCP receive: ~1-5ms (network dependent)
2. JSON parsing: ~10-50μs
3. Broadcast: ~1-5μs
4. Total router overhead: typically <100μs

### Scalability

- Can handle hundreds of concurrent master connections
- Broadcast channel supports up to 1024 workers efficiently
- Single-threaded per connection (Tokio async)

## Integration Points

### Signal Provider EA

Master MT5 terminal connects to router:
```mql5
input string RouterIP = "127.0.0.1";
input int RouterPort = 5000;
```

See [Signal Provider Setup](./mql5-signal-provider.md) for details.

### Worker Manager

Workers subscribe to broadcast channel:
```rust
let rx = trade_tx.subscribe(); // Each worker gets a receiver
```

See [Workers Documentation](./workers.md) for details.

## Failure Scenarios

### Router Crash

If router crashes:
- Master EA will fail to send trades
- Workers continue running but receive no new trades
- Restart router to restore functionality

### Master Disconnect

If master EA disconnects:
- Router continues listening
- Workers remain active
- Master can reconnect without restarting router

### No Workers

If no workers are running:
- Router continues accepting trades
- Broadcasts succeed but are not consumed
- No errors reported (by design)

## Monitoring

Router status is not directly exposed via API. Monitor via:

1. **System Metrics**: `GET /api/system/metrics`
   - Shows `router_status: "online"`
   - Shows `router_port: 5000`

2. **Logs**: Console output shows connection events

3. **Process Check**: Router runs in main process thread

## Best Practices

1. **Single Master**: Use one master terminal per router for consistency
2. **Firewall Rules**: Protect port 5000 if exposed to network
3. **Message Rate**: Limit trade frequency to avoid channel overflow
4. **Monitoring**: Watch logs for parse errors indicating EA issues
5. **Network Stability**: Use reliable network between master and router

## Troubleshooting

### Master Can't Connect

**Symptoms**: Signal Provider EA shows connection failed

**Solutions**:
- Verify router is running (`ps aux | grep trade-copier`)
- Check port 5000 is not in use (`lsof -i :5000`)
- Verify firewall allows TCP on port 5000
- Check master EA configuration (IP and port)

### Trades Not Broadcasting

**Symptoms**: Router receives trades but workers don't execute

**Solutions**:
- Check worker status via API (`GET /api/workers`)
- Verify workers are in `active` state
- Check worker logs for channel receive errors
- Verify workers are subscribed to correct channel

### High Latency

**Symptoms**: Trades take too long to reach workers

**Solutions**:
- Check network latency between master and router
- Monitor CPU usage (high load can delay processing)
- Verify JSON parsing isn't failing repeatedly
- Consider reducing message frequency from master

### Connection Drops

**Symptoms**: Master EA frequently disconnects

**Solutions**:
- Check network stability
- Verify master EA socket handling
- Review router logs for error patterns
- Consider implementing keepalive mechanism
