# Signal Receiver EA - Setup Guide

## Overview

The Signal Receiver EA is an MQL5 Expert Advisor that runs on slave MT5 terminals. It connects to a Trade Copier worker, receives trade signals, and executes them on the local terminal.

**File**: `Signal Receiver.mq5`  
**Location**: `mql5/Trading Rocket/Signal Receiver.mq5`

## Prerequisites

- MetaTrader 5 terminal installed
- MT5 account (demo or live)
- Trade Copier service running
- Worker configured for this terminal (via API)
- DLL imports enabled in MT5

## Installation

### Automatic Installation (Recommended)

When creating an instance via API, Expert Advisors are automatically copied to:
```
<wine_prefix>/drive_c/Program Files/MetaTrader 5/MQL5/Experts/Trading Rocket/
```

## Configuration

### Step 1: Enable DLL Imports and WebRequest

**Critical**: Socket functions require DLL access and WebRequest must be configured.

1. Open MT5 Settings:
   - `Tools` → `Options` → `Expert Advisors` tab

2. Enable:
   - ☑ `Allow DLL imports`
   - ☑ `Allow WebRequest for listed URLs`

3. Add allowed URLs:
   - In the WebRequest URL list, add: `http://127.0.0.1`
   - This must be configured **before** adding the EA to the chart

4. Click `OK`

### Step 2: Add EA to Chart

1. Open a chart (any symbol, any timeframe)
2. Open Navigator panel (Ctrl+N)
3. Expand `Expert Advisors` → `Trading Rocket`
4. Drag `Signal Receiver` onto chart

### Step 3: Configure EA Parameters

EA configuration dialog will appear with these parameters:

#### Required Parameters

**WorkerIP** (default: `127.0.0.1`)
- IP address of the worker TCP server
- Use `127.0.0.1` if worker is on same machine
- Use worker's IP address if on different machine

**WorkerPort** (default: `5050`)
- TCP port of the worker
- Must match the port in worker's address configuration
- Each worker has unique port (5050, 5051, 5052, etc.)

#### Optional Parameters

**MagicNumber** (default: `999888`)
- Unique identifier for trades opened by this EA
- Allows filtering trades in terminal
- Change if running multiple receiver EAs

**Slippage** (default: `10`)
- Maximum allowed slippage in points
- Adjust based on broker and market conditions
- Higher values = more tolerant of price movement

### Step 4: Verify Configuration

Correct example configuration:
```
WorkerIP = "127.0.0.1"
WorkerPort = 5050
MagicNumber = 999888
Slippage = 10
```

### Step 5: Enable Auto Trading

1. Click the `AutoTrading` button in MT5 toolbar (or press Ctrl+E)
2. Button should turn green
3. Verify EA is smiling (happy face icon on chart)

## Connection Process

### 1. Initial Connection

When EA starts:
```
[RECEIVER] Trade Copier Slave EA started
[RECEIVER] Connecting to worker at 127.0.0.1:5050
[RECEIVER] Socket created successfully, handle: 123
[RECEIVER] Connected to worker successfully (TCP)
```

### 2. Connection Established

After successful connection:
- EA polls for incoming trades every tick
- Worker shows `mt5_connected: true` in API
- Ready to receive and execute trades

### 3. Trade Reception

When trade is received:
```
[RECEIVER] Received trade: {"id":1234567890,"symbol":"EURUSD","type":"buy","lots":0.5,...}
[RECEIVER] Trade opened successfully
[RECEIVER] Sent acknowledgment: OK
```

## Trade Execution

### Trade Types

#### Open Position
```json
{
  "id": 1234567890,
  "symbol": "EURUSD",
  "type": "buy",
  "lots": 0.5,
  "price": 1.1234,
  "sl": 1.1200,
  "tp": 1.1300,
  "cmd": "open"
}
```

**Actions**:
- Opens new position (buy or sell)
- Sets stop loss and take profit
- Stores position mapping (trade_id → ticket)

#### Close Position
```json
{
  "id": 1234567890,
  "symbol": "EURUSD",
  "lots": 0.5,
  "cmd": "close"
}
```

**Actions**:
- Closes entire position by trade_id
- Removes position mapping

#### Partial Close
```json
{
  "id": 1234567890,
  "symbol": "EURUSD",
  "lots": 0.2,
  "cmd": "partial_close"
}
```

**Actions**:
- Closes specified volume from position
- Keeps position mapping (position still exists)

#### Modify Position
```json
{
  "id": 1234567890,
  "symbol": "EURUSD",
  "type": "buy",
  "lots": 0.5,
  "sl": 1.1150,
  "tp": 1.1350,
  "cmd": "modify"
}
```

**Actions**:
- Updates stop loss and/or take profit
- Does not affect position size or direction

### Order Filling

EA uses `ORDER_FILLING_FOK` (Fill or Kill):
- Order must be filled completely
- Or rejected entirely
- No partial fills

**Alternative**: Edit code to use `ORDER_FILLING_IOC` if your broker requires it.

### Acknowledgments

EA sends acknowledgment for every trade:
```
OK: Trade opened successfully
OK: Trade closed successfully
ERROR: Failed to open trade
```

Worker measures latency based on acknowledgment timing.

## Monitoring

### EA Status Indicators

**Happy Face (😊)**: EA running normally
- Connected to worker
- Receiving trades
- No errors

**Sad Face (☹️)**: EA has issues
- Connection failed
- Not receiving trades
- Check Expert tab for errors

### Experts Tab

View logs in `Experts` tab (Ctrl+T):
```
2025.01.01 12:00:00   Signal Receiver EURUSD,M1: [RECEIVER] Trade Copier Slave EA started
2025.01.01 12:00:01   Signal Receiver EURUSD,M1: [RECEIVER] Connected to worker successfully (TCP)
2025.01.01 12:05:23   Signal Receiver EURUSD,M1: [RECEIVER] Trade opened successfully
```

### Journal Tab

View connection events in `Journal` tab:
```
2025.01.01 12:00:00   Expert Signal Receiver.ex5 loaded successfully
2025.01.01 12:00:01   Socket created successfully
```

## Troubleshooting

### EA Won't Start

**Symptom**: EA immediately disabled after attaching

**Solutions**:
1. Enable AutoTrading (Ctrl+E)
2. Check DLL imports are allowed
3. Verify EA is properly compiled
4. Check for errors in Experts tab

### Connection Failed

**Symptom**: 
```
[RECEIVER] ERROR: Failed to connect to worker at 127.0.0.1:5050, error code: 5200
```

**Solutions**:
1. Verify worker is running: `GET /api/workers`
2. Check worker state is `active`
3. Verify IP and port match worker address
4. Check firewall allows TCP connection
5. Restart EA after fixing configuration

### DLL Error

**Symptom**:
```
[RECEIVER] ERROR: Failed to create socket, error code: 5002
```

**Error Code 5002**: DLL imports not allowed

**Solution**:
1. Open MT5 Options
2. Expert Advisors tab
3. Enable "Allow DLL imports"
4. Restart EA

### No Trades Received

**Symptom**: EA connected but no trades executing

**Check**:
1. Signal Provider EA is running on master terminal
2. Master terminal is connected to router (port 5000)
3. Router is broadcasting trades
4. Worker is receiving broadcasts
5. Check worker logs for errors
6. Verify symbol is available in terminal
7. Check account has sufficient margin

### Trades Failing

**Symptom**:
```
[RECEIVER] ERROR: Failed to open trade
```

**Solutions**:
1. Check account has sufficient margin
2. Verify symbol exists and is tradeable
3. Check lot size meets broker minimum
4. Verify SL/TP are valid distances
5. Check market is open for symbol
6. Increase slippage parameter
7. Review broker trade restrictions

### Reconnection Issues

**Symptom**: Connection lost and won't reconnect

**EA Behavior**:
- Attempts reconnect every 5 seconds
- Shows in logs: `[RECEIVER] Attempting to reconnect...`

**Solutions**:
1. Wait for automatic reconnection
2. Check worker is still running
3. Restart EA if reconnection fails repeatedly
4. Verify network stability

## Advanced Configuration

### Multiple EAs

Run multiple Signal Receiver EAs on same terminal:
1. Each EA connects to different worker
2. Use different MagicNumber for each EA
3. Attach to different charts (symbols don't matter)

Example:
- EA1: WorkerPort=5050, MagicNumber=999888
- EA2: WorkerPort=5051, MagicNumber=999889
- EA3: WorkerPort=5052, MagicNumber=999890

### Remote Worker

Connect to worker on different machine:

1. Worker configuration:
   - Use `127.0.0.1:<port>` for worker address (binds to all interfaces)
   - Or specific network IP

2. EA configuration:
   - Set WorkerIP to worker machine's IP address
   - Keep WorkerPort same as worker

3. Firewall:
   - Allow TCP connections on worker port
   - Allow inbound connections to worker machine

Example:
```
WorkerIP = "192.168.1.100"
WorkerPort = 5050
```

### Custom Magic Number

Change magic number to avoid conflicts:

1. Edit EA parameters:
   ```
   MagicNumber = 123456
   ```

2. Only affects new trades
3. Existing trades keep old magic number
4. Use for trade filtering in terminal

### Broker-Specific Settings

Some brokers require specific settings:

**Fill Policy**:
- Most brokers: `ORDER_FILLING_FOK` (default)
- Some brokers: `ORDER_FILLING_IOC`
- Edit source code if needed (line 58)

**Slippage**:
- ECN brokers: Lower slippage (5-10 points)
- Market makers: Higher slippage (20-50 points)

**Minimum Distance**:
- Some brokers enforce minimum SL/TP distance
- EA may need modification to handle broker requirements

## Best Practices

1. **Testing**: Test on demo account first
2. **Monitoring**: Watch first few trades closely
3. **Margin**: Ensure sufficient margin for multiplied lot sizes
4. **Symbol Availability**: Verify all symbols exist on slave terminal
5. **Market Hours**: Ensure broker allows trading during signal hours
6. **Connection**: Use stable network connection
7. **Updates**: Keep EA files synchronized across terminals
8. **Backup**: Keep backup copies of EA files
9. **Documentation**: Document EA configurations for each worker
10. **Restart**: Restart EA after configuration changes

## Performance Tips

1. **Chart Selection**: Attach to any chart (symbol doesn't matter)
2. **Timeframe**: Use any timeframe (EA runs on tick events)
3. **CPU Usage**: Minimal - EA is event-driven
4. **Memory**: ~1-2 MB per EA instance
5. **Latency**: Typical 2-10ms round-trip to worker

## Common Error Codes

| Code | Description | Solution |
|------|-------------|----------|
| 5002 | DLL not allowed | Enable DLL imports in settings |
| 4014 | Internal error | Restart MT5 terminal |
| 5200 | Socket error | Check network/firewall |
| 5273 | I/O error | Connection lost, wait for reconnect |

## Support

For issues with:
- **EA not starting**: Check DLL permissions and AutoTrading
- **Connection problems**: Verify worker is running and accessible
- **Trade execution**: Check broker requirements and symbol availability
- **Worker issues**: See [Workers Documentation](./workers.md)
- **System issues**: See [API Documentation](./api.md)

## Files

- **Source Code**: `mql5/Trading Rocket/Signal Receiver.mq5`
- **Compiled**: `mql5/Trading Rocket/Signal Receiver.ex5`
- **Installation**: Automatically installed with instance creation
- **Location**: `MQL5/Experts/Trading Rocket/` in MT5 data folder
