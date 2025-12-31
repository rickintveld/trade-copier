# Workers Documentation

## Overview

Workers are TCP servers that receive trade signals from the Router and forward them to individual slave MT5 terminals (Signal Receiver EAs). Each worker manages a single MT5 instance with its own risk multiplier and connection handling.

## Architecture

```
Router (Broadcast Channel)
         |
         v
    Worker (TCP Server)
         |
         | TCP Connection
         v
Signal Receiver EA (Slave MT5)
```

## Worker Lifecycle

### 1. Creation

Workers can be created via:
- **API**: `POST /api/instances` (recommended)
- **Database**: Direct insertion (not recommended)

Each worker requires:
- **Name**: Unique identifier
- **Address**: TCP bind address (e.g., `127.0.0.1:5050`)
- **Multiplier**: Lot size multiplier (e.g., `1.0`, `0.5`)
- **Wine Prefix**: Path to MT5 installation (auto-generated on macOS)

### 2. Initialization

On startup, workers are loaded from database:
```rust
let workers = db.get_worker_configs().await?;
worker_manager.load_workers().await?;
```

### 3. Activation

Worker becomes active when:
1. TCP server binds successfully to configured address
2. Worker subscribes to broadcast channel
3. Database state is updated to `active`

### 4. Running

Active workers:
- Listen for MT5 receiver connections
- Receive trades from broadcast channel
- Apply risk multiplier
- Forward trades to connected MT5 terminal
- Track latency and connection status
- Monitor Wine process (macOS only)

### 5. Shutdown

Workers can be stopped via:
- **API**: `POST /api/instances/{name}/stop`
- **System shutdown**: All workers stopped gracefully
- **Error**: Critical errors cause automatic shutdown

On shutdown:
- Database state updated to `inactive`
- TCP connections closed
- Resources cleaned up

## Worker Components

### TCP Server

Each worker runs a TCP server that:
- Accepts connections from Signal Receiver EA
- Stores single active connection (one MT5 per worker)
- Sends JSON trades over TCP
- Receives acknowledgments from MT5

**Connection Management:**
```rust
let listener = TcpListener::bind(&slave.address).await?;
```

Only one MT5 terminal can be connected at a time. New connections replace old ones.

### Broadcast Subscription

Workers subscribe to Router's broadcast channel:
```rust
let rx = trade_tx.subscribe();
```

Each worker has its own receiver, allowing independent processing.

### Risk Multiplier

Before forwarding trades, workers apply the multiplier:
```rust
trade.lots = trade.lots * slave.multiplier;
trade.lots = (trade.lots * 100.0).round() / 100.0; // Round to 2 decimals
```

**Examples:**
- Multiplier `1.0`: Original lot size (1.0 → 1.0)
- Multiplier `0.5`: Half size (1.0 → 0.50)
- Multiplier `2.0`: Double size (1.0 → 2.00)

### Latency Tracking

Workers measure round-trip latency:
1. Send trade to MT5
2. Wait for acknowledgment
3. Calculate elapsed time in microseconds
4. Update database asynchronously

```rust
let start = Instant::now();
// Send trade...
let latency_us = start.elapsed().as_micros() as u64;
db.update_worker_latency(&address, latency_us).await?;
```

### Connection Status

Workers track MT5 connection status:
- `mt5_connected = true`: MT5 receiver is connected
- `mt5_connected = false`: No MT5 connection

Updated on:
- Connection accept
- Connection close
- Read/write errors

### Wine Process Monitoring (macOS)

On macOS, workers monitor associated Wine process:

**Grace Period:**
- Waits for Wine process to start (no time limit)
- Silent during grace period

**Active Monitoring:**
- Checks every 5 seconds if Wine is running
- If Wine process stops:
  - Worker state set to `inactive`
  - `mt5_connected` set to `false`
  - Warning logged to database

**Process Detection:**
```bash
pgrep -f "<wine_prefix>/drive_c/Program Files/MetaTrader 5/terminal64.exe"
```

## Worker States

### Active
- Worker is running
- TCP server accepting connections
- Receiving and forwarding trades
- Normal operation state

### Inactive
- Worker is stopped or not running
- Not processing trades
- May be manually stopped or awaiting restart

### Error
- Worker encountered an error
- Last error message stored in database
- May require manual intervention

### Installing
- Temporary state during MT5 installation
- Worker not yet operational
- Automatically transitions to `inactive` when done

## Trade Processing

### 1. Receive from Channel

Worker receives trade from broadcast:
```rust
result = rx.recv() => match result {
    Ok(trade) => { /* process */ }
    Err(e) => { /* channel error */ }
}
```

### 2. Apply Multiplier

Adjust lot size and round:
```rust
trade.lots = trade.lots * multiplier;
trade.lots = (trade.lots * 100.0).round() / 100.0;
```

### 3. Send to MT5

Forward trade over TCP:
```rust
let trade_json = serde_json::to_string(&trade)?;
let message = format!("{}\n", trade_json);
stream.write_all(message.as_bytes()).await?;
```

### 4. Wait for Acknowledgment

Read response from MT5:
```rust
let mut reader = BufReader::new(stream);
let mut ack_line = String::new();
reader.read_line(&mut ack_line).await?;
```

### 5. Log Results

Update database with:
- Trade record
- Latency measurement
- Any errors encountered

## Error Handling

### Binding Errors

If TCP bind fails:
```
[WORKER:name] Failed to bind to address: <error>
```

- Worker state set to `error`
- Critical error logged to database
- Worker cannot start

### Connection Errors

If connection fails:
```
[WORKER:name] Failed to accept connection: <error>
```

- Worker state set to `error`
- Error logged to database
- Worker continues trying to accept connections

### Send Errors

If trade send fails:
```
[WORKER:name] Write error, connection lost: <error>
```

- Connection cleared
- `mt5_connected` set to `false`
- Worker state set to `error`
- Worker continues running (can reconnect)

### Channel Errors

If broadcast channel breaks:
```
[WORKER:name] Channel error: <error>
```

- Worker state set to `error`
- Critical error logged
- Worker stops (channel cannot be recovered)

### Database Errors

Database errors are non-fatal:
- Logged to console
- Worker continues operation
- Trade forwarding not affected

## Worker Manager

### Command System

Workers are controlled via commands:

```rust
enum WorkerCommand {
    Reload,        // Reload all workers from database
    Start(String), // Start specific worker by name
    Stop(String),  // Stop specific worker by name
}
```

### Reload Workers

Stops all workers and reloads from database:
```rust
worker_command_tx.send(WorkerCommand::Reload).await?;
```

Used after:
- Creating new instances
- Modifying worker configurations

### Start Worker

Starts a specific stopped worker:
```rust
worker_command_tx.send(WorkerCommand::Start("worker-1".to_string())).await?;
```

Requirements:
- Worker must exist in database
- Worker must not already be running
- Address must be available

### Stop Worker

Stops a specific running worker:
```rust
worker_command_tx.send(WorkerCommand::Stop("worker-1".to_string())).await?;
```

Process:
1. Send shutdown signal
2. Wait for graceful shutdown (5 second timeout)
3. Update database state
4. Clean up resources

## Performance Characteristics

### Latency

Typical latencies:
- **Channel receive**: <10μs
- **Multiplier calculation**: <1μs
- **TCP send**: 100-1000μs (network dependent)
- **MT5 acknowledgment**: 1-5ms (MT5 processing)
- **Total round-trip**: 2-10ms typical

### Throughput

Each worker can handle:
- 100+ trades per second (theoretical)
- Limited by MT5 processing speed
- Network latency is primary bottleneck

### Memory

Per worker memory usage:
- ~2-5 MB per worker (Rust)
- Additional Wine/MT5 overhead (macOS)

## Monitoring

### Via API

**Get all workers:**
```bash
curl http://localhost:3000/api/workers
```

Returns:
- Worker ID, name, address
- Current state
- Last error (if any)
- Latency (microseconds)
- MT5 connection status
- Wine prefix
- Timestamps

**Get specific worker:**
```bash
curl http://localhost:3000/api/workers | jq '.data[] | select(.name == "worker-1")'
```

### Via Logs

Workers log all significant events:
```
[WORKER:name] Starting TCP server on 127.0.0.1:5050
[WORKER:name] Listening on 127.0.0.1:5050 (TCP)
[WORKER:name] MT5 receiver connected from 127.0.0.1:54321
[WORKER:name] Received trade: Trade { ... }
[WORKER:name] Adjusted lots: 0.50 (multiplier: 0.5)
[WORKER:name] Trade sent successfully, latency: 1234µs
```

### Metrics

Monitor:
- **Latency trends**: Increasing latency indicates network issues
- **Error rate**: High error rate requires investigation
- **Connection status**: Frequent disconnects indicate instability
- **State changes**: Unexpected state changes indicate problems

## Best Practices

1. **Unique Addresses**: Each worker must have a unique TCP address
2. **Port Range**: Use ports 5050+ for workers (avoid system ports)
3. **Multiplier Testing**: Test multipliers thoroughly before live trading
4. **Connection Monitoring**: Watch for MT5 disconnections
5. **Error Review**: Regularly check error logs for patterns
6. **Resource Limits**: Monitor system resources with many workers
7. **Network Quality**: Use reliable network for consistent latency
8. **Graceful Shutdown**: Use API to stop workers cleanly

## Troubleshooting

### Worker Won't Start

**Check:**
- Address not already in use: `lsof -i :<port>`
- Valid IP address format
- Worker exists in database
- No conflicting worker names

### MT5 Won't Connect

**Check:**
- Signal Receiver EA is running
- Correct IP and port in EA settings
- Worker is in `active` state
- No firewall blocking connection
- Review worker logs for connection attempts

### High Latency

**Check:**
- Network latency between worker and MT5
- MT5 performance (CPU, memory)
- Number of concurrent trades
- Database performance

### Trades Not Forwarding

**Check:**
- Worker state is `active`
- `mt5_connected` is `true`
- Broadcast channel is working
- No errors in worker logs
- Signal Receiver EA is processing trades

### Wine Process Stops (macOS)

**Check:**
- MT5 didn't crash
- Wine installation is stable
- System resources available
- MT5 auto-restart not working
- Manual restart: `POST /api/instances/{name}/start`
