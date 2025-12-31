# MQL5 Expert Advisors

## Overview

Trade Copier includes two MQL5 Expert Advisors (EAs) that enable communication between MetaTrader 5 terminals and the Trade Copier service:

1. **Signal Provider EA** - Runs on master terminal, sends trade signals
2. **Signal Receiver EA** - Runs on slave terminals, receives and executes trade signals

## Architecture

```
Master MT5 Terminal
    ↓
Signal Provider EA
    ↓ (TCP to port 5000)
Trade Copier Router
    ↓ (Broadcast)
Trade Copier Workers
    ↓ (TCP to port 5050+)
Signal Receiver EAs
    ↓
Slave MT5 Terminals
```

## Files

Both EAs are located in `mql5/Trading Rocket/`:

- `Signal Provider.mq5` - Master EA source code
- `Signal Provider.ex5` - Master EA compiled
- `Signal Receiver.mq5` - Slave EA source code
- `Signal Receiver.ex5` - Slave EA compiled

## Installation

### Automatic (Recommended for Slaves)

When creating instances via API, Expert Advisors are automatically installed:
```
<wine_prefix>/drive_c/Program Files/MetaTrader 5/MQL5/Experts/Trading Rocket/
```

### Manual Installation

1. Open MT5 terminal
2. Open Data Folder: `File` → `Open Data Folder`
3. Navigate to `MQL5/Experts/`
4. Create folder: `Trading Rocket`
5. Copy both `.mq5` and `.ex5` files to this folder
6. Restart MT5 or refresh Navigator

## Quick Start

### For Master Terminal (Signal Provider)

1. **Install Trade Copier Service**:
   ```bash
   cargo run --release
   ```

2. **Enable DLL Imports** in MT5:
   - `Tools` → `Options` → `Expert Advisors`
   - ☑ Allow DLL imports

3. **Add Signal Provider EA** to any chart:
   - Default settings: RouterIP=`127.0.0.1`, RouterPort=`5000`

4. **Enable AutoTrading** (Ctrl+E)

5. **Verify Connection**:
   - Check EA logs: `[SENDER] Connected to router successfully`

### For Slave Terminals (Signal Receiver)

1. **Create Instance** via API:
   ```bash
   curl -X POST http://localhost:3000/api/instances \
     -H "Content-Type: application/json" \
     -d '{"name":"worker-1","address":"127.0.0.1:5050","multiplier":1.0}'
   ```

2. **Start Instance**:
   ```bash
   curl -X POST http://localhost:3000/api/instances/worker-1/start
   ```

3. **Enable DLL Imports** in MT5 (same as master)

4. **Add Signal Receiver EA** to any chart:
   - Configure: WorkerIP=`127.0.0.1`, WorkerPort=`5050`

5. **Enable AutoTrading** (Ctrl+E)

6. **Verify Connection**:
   - Check EA logs: `[RECEIVER] Connected to worker successfully`

## Communication Protocol

### Message Format

All messages are JSON formatted and newline-terminated:
```json
{"id":1234567890,"symbol":"EURUSD","type":"buy","lots":1.0,"price":1.1234,"sl":1.1200,"tp":1.1300,"cmd":"open"}
```

### Commands

- **open**: Open new position
- **close**: Close entire position
- **partial_close**: Close partial position
- **modify**: Modify SL/TP

### Connection Handling

Both EAs feature:
- **Automatic Reconnection**: Retry every 5 seconds on connection loss
- **Connection Status Tracking**: Monitor and log connection state
- **Error Handling**: Graceful error recovery

## Configuration

### Signal Provider EA

| Parameter | Default | Description |
|-----------|---------|-------------|
| RouterIP | 127.0.0.1 | Trade Copier router IP address |
| RouterPort | 5000 | Trade Copier router TCP port |

### Signal Receiver EA

| Parameter | Default | Description |
|-----------|---------|-------------|
| WorkerIP | 127.0.0.1 | Trade Copier worker IP address |
| WorkerPort | 5050 | Trade Copier worker TCP port |
| MagicNumber | 999888 | Unique identifier for EA trades |
| Slippage | 10 | Maximum slippage in points |

## Trade Flow

### Opening a Trade

1. **Master Terminal**: Trader opens position (manually or via EA)
2. **Signal Provider**: Detects via `OnTradeTransaction()`, sends to router
3. **Router**: Broadcasts to all workers
4. **Workers**: Apply multiplier, send to Signal Receivers
5. **Signal Receivers**: Execute trade, send acknowledgment
6. **Workers**: Measure latency, log to database

### Closing a Trade

1. **Master Terminal**: Trader closes position
2. **Signal Provider**: Detects close event, sends close command
3. **Router**: Broadcasts to all workers
4. **Workers**: Forward close command
5. **Signal Receivers**: Close position, send acknowledgment

### Modifying a Trade

1. **Master Terminal**: Trader modifies SL/TP
2. **Signal Provider**: Detects change on tick, sends modify command
3. **Router**: Broadcasts to all workers
4. **Workers**: Forward modify command
5. **Signal Receivers**: Modify position, send acknowledgment

## Features

### Signal Provider

- ✅ Monitors all trade events (open, close, modify)
- ✅ Automatic position tracking
- ✅ Unique trade ID generation
- ✅ Partial close detection
- ✅ Automatic reconnection
- ✅ Works with any trade source (manual, EA, copy trading)

### Signal Receiver

- ✅ Executes all trade commands
- ✅ Position mapping (trade_id → ticket)
- ✅ Acknowledgment system
- ✅ Automatic reconnection
- ✅ Magic number filtering
- ✅ Configurable slippage

## Requirements

### MT5 Settings

**Critical**: Both EAs require DLL imports enabled:
1. Open `Tools` → `Options` → `Expert Advisors`
2. Enable ☑ `Allow DLL imports`
3. Click `OK`

Without this, EAs will fail with error 5002.

### Socket Functions

EAs use these MQL5 socket functions:
- `SocketCreate()` - Create socket
- `SocketConnect()` - Connect to server
- `SocketSend()` - Send data
- `SocketRead()` - Read data (Receiver only)
- `SocketIsReadable()` - Check for data (Receiver only)
- `SocketClose()` - Close socket

All require DLL imports enabled.

## Monitoring

### EA Status Indicators

- **😊 Happy Face**: EA running normally, connected
- **☹️ Sad Face**: EA has errors or connection issues

### Log Messages

#### Signal Provider
```
[SENDER] Trade Copier Master EA started
[SENDER] Connected to router successfully (TCP)
[SENDER] Sending: {"id":...}
```

#### Signal Receiver
```
[RECEIVER] Trade Copier Slave EA started
[RECEIVER] Connected to worker successfully (TCP)
[RECEIVER] Trade opened successfully
```

### Viewing Logs

- **Experts Tab** (Ctrl+T): EA-specific logs
- **Journal Tab**: System events and errors

## Troubleshooting

### Common Issues

| Issue | Solution |
|-------|----------|
| EA won't start | Enable AutoTrading (Ctrl+E) |
| Error 5002 | Enable DLL imports in settings |
| Connection failed | Verify service is running, check IP/port |
| No trades received | Check entire signal chain (provider → router → worker → receiver) |
| High latency | Check network connection, system performance |

### Verification Checklist

**Master Terminal:**
- [ ] Trade Copier service running
- [ ] Router listening on port 5000
- [ ] Signal Provider EA attached to chart
- [ ] AutoTrading enabled
- [ ] DLL imports allowed
- [ ] EA shows connected in logs

**Slave Terminal:**
- [ ] Worker created and started
- [ ] Worker state is `active`
- [ ] Signal Receiver EA attached to chart
- [ ] AutoTrading enabled
- [ ] DLL imports allowed
- [ ] EA shows connected in logs
- [ ] Worker shows `mt5_connected: true`

## Best Practices

1. **Testing**: Always test on demo accounts first
2. **Single Provider**: Use only one Signal Provider per Trade Copier instance
3. **Chart Independence**: EAs work on any chart/symbol/timeframe
4. **Connection Monitoring**: Regularly check EA logs for connection status
5. **Network Stability**: Use reliable network connections
6. **Margin Management**: Ensure sufficient margin for multiplied lot sizes
7. **Symbol Availability**: Verify all symbols exist on slave terminals
8. **Backup**: Keep copies of EA files

## Performance

### Latency

Typical end-to-end latency (master trade → slave execution):
- **Signal Provider**: <1ms
- **Router**: <1ms
- **Worker**: <1ms
- **Signal Receiver**: 2-5ms (MT5 execution)
- **Total**: 5-15ms typical

### Resource Usage

Per EA:
- **CPU**: Minimal (<1% on modern systems)
- **Memory**: ~1-2 MB
- **Network**: <1 KB per trade

## Advanced Topics

### Multiple Signal Receivers

Run multiple receivers on same terminal:
```
EA1: WorkerPort=5050, MagicNumber=999888
EA2: WorkerPort=5051, MagicNumber=999889
EA3: WorkerPort=5052, MagicNumber=999890
```

Each connects to different worker for different masters or multipliers.

### Remote Connections

Connect EAs to remote Trade Copier service:

**Signal Provider:**
```
RouterIP = "192.168.1.100"
RouterPort = 5000
```

**Signal Receiver:**
```
WorkerIP = "192.168.1.100"
WorkerPort = 5050
```

Ensure firewall allows connections.

### Broker Compatibility

Most brokers are compatible, but some require adjustments:

**Order Filling:**
- Most brokers: `ORDER_FILLING_FOK` (default)
- Some brokers: `ORDER_FILLING_IOC`
- Edit source code to change if needed

**Slippage:**
- ECN brokers: 5-10 points
- Market makers: 20-50 points

## Documentation Links

- [Signal Provider Setup Guide](./mql5-signal-provider.md) - Master EA detailed documentation
- [Signal Receiver Setup Guide](./mql5-signal-receiver.md) - Slave EA detailed documentation and README
- [Router Documentation](./router.md) - Router configuration and troubleshooting
- [Workers Documentation](./workers.md) - Worker configuration and management
- [API Documentation](./api.md) - HTTP API for system management

## Support

For detailed setup and troubleshooting:
- Signal Provider issues → See [Signal Provider Guide](./mql5-signal-provider.md)
- Signal Receiver issues → See [Signal Receiver Guide](./mql5-signal-receiver.md)
- Connection issues → Check router and worker status
- System issues → See [API Documentation](./api.md)
