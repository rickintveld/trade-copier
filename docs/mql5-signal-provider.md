# Signal Provider EA - Setup Guide

## Overview

The Signal Provider EA is an MQL5 Expert Advisor that runs on the master MT5 terminal. It monitors all trading activity and sends trade signals to the Trade Copier router for distribution to slave terminals.

**File**: `Signal Provider.mq5`  
**Location**: `mql5/Trading Rocket/Signal Provider.mq5`

## Prerequisites

- MetaTrader 5 terminal installed
- MT5 account (demo or live)
- Trade Copier service running
- DLL imports enabled in MT5
- Manual trading or another EA generating trades

## Installation

### Manual Installation

1. Locate the MT5 Data Folder:
   - Open MT5
   - Click `File` → `Open Data Folder`
   - Navigate to `MQL5/Experts/`

2. Create directory: `Trading Rocket`

3. Copy files:
   - Copy `Signal Provider.mq5` to `MQL5/Experts/Trading Rocket/`
   - Copy `Signal Provider.ex5` (compiled) if available

4. Compile (if needed):
   - Open MetaEditor (F4 in MT5)
   - Open `Signal Provider.mq5`
   - Click `Compile` (F7)
   - Verify no errors

## Configuration

### Step 1: Enable DLL Imports

**Critical**: Socket functions require DLL access.

1. Open MT5 Settings:
   - `Tools` → `Options` → `Expert Advisors` tab

2. Enable:
   - ☑ `Allow DLL imports`
   - ☑ `Allow WebRequest for listed URLs` (optional but recommended)

3. Click `OK`

### Step 2: Add EA to Chart

1. Open a chart (any symbol, any timeframe)
2. Open Navigator panel (Ctrl+N)
3. Expand `Expert Advisors` → `Trading Rocket`
4. Drag `Signal Provider` onto chart

### Step 3: Configure EA Parameters

EA configuration dialog will appear with these parameters:

#### Required Parameters

**RouterIP** (default: `127.0.0.1`)
- IP address of the Trade Copier router
- Use `127.0.0.1` if router is on same machine
- Use router's IP address if on different machine

**RouterPort** (default: `5000`)
- TCP port of the router
- Default router port is 5000
- Must match router configuration

### Step 4: Verify Configuration

Correct example configuration:
```
RouterIP = "127.0.0.1"
RouterPort = 5000
```

### Step 5: Enable Auto Trading

1. Click the `AutoTrading` button in MT5 toolbar (or press Ctrl+E)
2. Button should turn green
3. Verify EA is smiling (happy face icon on chart)

## Connection Process

### Initial Connection

When EA starts:
```
[SENDER] Trade Copier Master EA started
[SENDER] Sending signals to 127.0.0.1:5000
[SENDER] Socket created successfully, handle: 123
[SENDER] Connected to router successfully (TCP)
```

### Connection Established

After successful connection:
- EA monitors all trade events
- Ready to send signals to router
- Automatic reconnection on connection loss

## Trade Detection

### Monitored Events

The EA monitors these trade events via `OnTradeTransaction()`:

#### 1. Position Open
- Detects when new position is opened
- Captures: symbol, direction (buy/sell), lot size, price, SL, TP
- Generates unique trade ID
- Tracks position for subsequent events

#### 2. Position Close
- Detects when position is closed (full or partial)
- Sends close command with actual closed volume
- Removes position tracking on full close
- Maintains tracking on partial close

#### 3. Position Modify
- Detected via OnTick() polling (every tick)
- Monitors SL/TP changes
- Sends modify command when changes detected
- Updates internal position state

### Trade ID Generation

Each trade gets unique ID:
```cpp
ulong trade_id = (ulong)TimeLocal() * 1000000 + deal_ticket;
```

Format: timestamp + deal ticket number

### Position Tracking

EA maintains internal tracking:
- Maps position tickets to trade IDs
- Stores current SL/TP for modification detection
- Automatically cleans up closed positions

## Signal Format

### Open Signal
```json
{
  "id": 1735689600000123,
  "symbol": "EURUSD",
  "type": "buy",
  "lots": 1.0,
  "price": 1.1234,
  "sl": 1.1200,
  "tp": 1.1300,
  "cmd": "open"
}
```

### Close Signal
```json
{
  "id": 1735689600000123,
  "symbol": "EURUSD",
  "lots": 1.0,
  "cmd": "close"
}
```

### Partial Close Signal
```json
{
  "id": 1735689600000123,
  "symbol": "EURUSD",
  "lots": 0.3,
  "cmd": "partial_close"
}
```

### Modify Signal
```json
{
  "id": 1735689600000123,
  "symbol": "EURUSD",
  "type": "buy",
  "lots": 1.0,
  "sl": 1.1150,
  "tp": 1.1350,
  "cmd": "modify"
}
```

## Monitoring

### EA Status Indicators

**Happy Face (😊)**: EA running normally
- Connected to router
- Monitoring trades
- No errors

**Sad Face (☹️)**: EA has issues
- Connection failed
- Check Expert tab for errors

### Experts Tab

View logs in `Experts` tab (Ctrl+T):
```
2025.01.01 12:00:00   Signal Provider EURUSD,M1: [SENDER] Trade Copier Master EA started
2025.01.01 12:00:01   Signal Provider EURUSD,M1: [SENDER] Connected to router successfully (TCP)
2025.01.01 12:05:23   Signal Provider EURUSD,M1: [SENDER] Sending: {"id":123...}
```

### Journal Tab

View connection events in `Journal` tab:
```
2025.01.01 12:00:00   Expert Signal Provider.ex5 loaded successfully
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
[SENDER] ERROR: Failed to connect to router at 127.0.0.1:5000, error code: 5200
```

**Solutions**:
1. Verify Trade Copier service is running
2. Check router is listening on port 5000
3. Verify IP and port configuration
4. Check firewall allows TCP connection
5. Restart EA after fixing configuration

### DLL Error

**Symptom**:
```
[SENDER] ERROR: Failed to create socket, error code: 5002
```

**Error Code 5002**: DLL imports not allowed

**Solution**:
1. Open MT5 Options
2. Expert Advisors tab
3. Enable "Allow DLL imports"
4. Restart EA

### Signals Not Sending

**Symptom**: EA connected but trades not being sent

**Check**:
1. EA is connected to router
2. Trades are actually being executed in MT5
3. No errors in Experts tab
4. Router is receiving connections
5. Check router logs for received trades

### Send Failures

**Symptom**:
```
[SENDER] ERROR: Send failed, error: 5273
```

**Error 5273**: Connection lost

**EA Behavior**:
- Automatically attempts reconnection every 5 seconds
- Reconnection message: `[SENDER] Attempting to reconnect...`

**Solutions**:
1. Wait for automatic reconnection
2. Verify router is still running
3. Check network stability
4. Restart EA if reconnection fails repeatedly

### Trades Not Detected

**Symptom**: Manual trades not being sent

**Possible Causes**:
1. EA not attached to chart
2. AutoTrading disabled
3. EA stopped or removed
4. Connection to router lost

**Solutions**:
1. Verify EA is running (check chart for EA icon)
2. Enable AutoTrading
3. Check EA logs for errors
4. Verify connection status

## Advanced Usage

### Multiple Charts

EA only needs to run on ONE chart:
- Monitors ALL positions across all symbols
- Symbol of attached chart doesn't matter
- Timeframe doesn't matter (uses event-driven detection)

**Recommendation**: Attach to any chart, leave running

### Remote Router

Connect to router on different machine:

1. Router configuration:
   - Ensure router binds to `0.0.0.0:5000` (not just localhost)

2. EA configuration:
   ```
   RouterIP = "192.168.1.100"
   RouterPort = 5000
   ```

3. Firewall:
   - Allow TCP connections to router port
   - Allow outbound connections from MT5 machine

### Compatibility

**Works with**:
- Manual trading
- Other Expert Advisors
- Copy trading services
- Any trade execution method

**Trade Source**: Doesn't matter - EA monitors all positions

## Reconnection Handling

### Automatic Reconnection

If connection lost:
1. EA detects connection failure
2. Sets `g_connection_lost` flag
3. Attempts reconnection every 5 seconds
4. Logs reconnection attempts
5. Resumes normal operation on success

### During Reconnection

- EA continues monitoring trades
- Trades are NOT sent during reconnection period
- After reconnection, new trades are sent
- Previous trades are NOT resent (by design)

**Important**: Avoid trading during reconnection if you need all trades copied.

## Performance

### CPU Usage

Minimal impact:
- Event-driven (no polling except for modify detection)
- Lightweight JSON generation
- Efficient socket communication

### Memory Usage

~1-2 MB per EA instance

### Latency

Signal sending is very fast:
- JSON generation: <10μs
- TCP send: ~100-500μs
- Total EA overhead: <1ms

## Best Practices

1. **Single Instance**: Run only ONE Signal Provider EA per terminal
2. **Chart Selection**: Attach to any chart (symbol doesn't matter)
3. **Always Running**: Keep EA active during trading hours
4. **Monitor Connection**: Watch for connection status in logs
5. **Network Stability**: Use reliable network for consistent signal delivery
6. **Testing**: Test on demo account first
7. **Backup**: Keep EA file backed up
8. **Updates**: Update EA when new version available

## Limitations

1. **Historical Trades**: Only new trades are sent (no historical sync)
2. **Reconnection Gap**: Trades during disconnection are not sent
3. **Position-Based**: Works with positions, not orders
4. **Hedge Accounts**: May need special handling (not tested)
5. **Single Terminal**: One provider per Trade Copier instance recommended

## Common Error Codes

| Code | Description | Solution |
|------|-------------|----------|
| 5002 | DLL not allowed | Enable DLL imports in settings |
| 4014 | Internal error | Restart MT5 terminal |
| 5200 | Socket error | Check router is running, check network/firewall |
| 5273 | I/O error | Connection lost, wait for reconnect |

## Integration with Trade Copier

### System Flow

```
Manual Trade / EA → Master MT5
                        ↓
                  Signal Provider EA
                        ↓
                  Router (port 5000)
                        ↓
                    Broadcast
                        ↓
                  All Workers
                        ↓
                  Signal Receivers
                        ↓
                  Slave MT5 Terminals
```

### Verification

After setup, verify:
1. Signal Provider connected: Check EA logs
2. Router receiving: Check router logs (`[ROUTER] Received trade...`)
3. Workers receiving: Check worker logs
4. Signal Receivers executing: Check slave terminals

## Support

For issues with:
- **EA not starting**: Check DLL permissions and AutoTrading
- **Connection problems**: Verify router is running and accessible
- **Trades not sending**: Check EA logs and router status
- **Router issues**: See [Router Documentation](./router.md)
- **System issues**: See [API Documentation](./api.md)

## Files

- **Source Code**: `mql5/Trading Rocket/Signal Provider.mq5`
- **Compiled**: `mql5/Trading Rocket/Signal Provider.ex5`
- **Location**: `MQL5/Experts/Trading Rocket/` in MT5 data folder
