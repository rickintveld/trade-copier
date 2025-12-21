# Trade Copier - Installation & Setup Guide

## Prerequisites

### Rust Backend
- Rust toolchain (1.70+): Install from [rustup.rs](https://rustup.rs)
- Linux/macOS/Windows with network access

### MetaTrader 5
- MetaTrader 5 terminal (build 2280 or higher for native socket support)
- Administrator privileges (for network sockets)

## Installation Steps

### 1. Build Rust Backend

```bash
# Clone or navigate to the project
cd trade-copier

# Build release version (optimized)
cargo build --release

# The binary will be at: target/release/trade-copier
```

### 2. Configure Slaves

Edit `config/slaves.yaml`:

```yaml
slaves:
  - name: "Account01"
    address: "0.0.0.0:5051"         # Worker TCP server address
    multiplier: 1.0                 # Lot size multiplier

  - name: "Account02"
    address: "0.0.0.0:5052"
    multiplier: 0.5                 # Trade half the master lot size

  - name: "Account03"
    address: "0.0.0.0:5053"
    multiplier: 2.0                 # Trade double the master lot size
```

**Important Notes:**
- `address`: The TCP server address where this worker will listen for slave MT5 EA connections
- Each worker needs a unique port
- `multiplier`: Risk adjustment per slave (1.0 = same size, 0.5 = half, 2.0 = double)

### 3. Install MetaTrader 5 Expert Advisors

#### 3.1 Compile Expert Advisors

**Master Terminal:**
1. Open `mql5/signal_sender.mq5` in MetaEditor
2. Click "Compile" (F7)
3. Check for errors in the Toolbox

**Slave Terminals:**
1. Open `mql5/signal_receiver.mq5` in MetaEditor
2. Click "Compile" (F7)
3. Check for errors in the Toolbox

#### 3.2 Configure EAs

**Master EA Settings:**
- `RouterIP`: IP address where Rust router is running (default: "127.0.0.1")
- `RouterPort`: Router listening port (default: 5000)

**Slave EA Settings:**
- `WorkerIP`: IP address of the worker to connect to (default: "127.0.0.1")
- `WorkerPort`: Port of the worker to connect to (e.g., 5051, 5052, 5053)
- `MagicNumber`: Unique identifier for trades (default: 999888)
- `Slippage`: Maximum slippage in points (default: 10)

### 4. Enable Socket Connections in MT5

> **Note:** Native socket functions are available in MT5 build 2280+ (May 2019). Check your MT5 version under Help → About.

**CRITICAL:** MetaTrader 5 requires explicit permission for socket operations.

1. Open MetaTrader 5
2. Go to: **Tools → Options → Expert Advisors**
3. Enable the following checkboxes:
   - ✅ **Allow WebRequest for listed URL**
   - ✅ **Allow DLL imports**
   - ✅ **Allow automated trading**
4. Click **OK**

## Running the Trade Copier

### 1. Start Rust Backend

```bash
# From project root
./target/release/trade-copier

# Or with cargo
cargo run --release
```

Expected output:
```
🚀 Trade Copier Starting...
📋 Loaded 1 slave(s) from config
  - FN-200k @ 0.0.0.0:5051 (multiplier: 2x)
[WORKER:FN-200k] Starting TCP server on 0.0.0.0:5051
[WORKER:FN-200k] Listening on 0.0.0.0:5051 (TCP)
[ROUTER] Listening on port 5000 (TCP)
✅ Trade Copier is running
📡 Router listening on port 5000
⏳ Press Ctrl+C to stop
```

### 2. Attach EAs to Charts

**Master Terminal:**
1. Open any chart (symbol doesn't matter)
2. Drag `signal_sender.mq5` onto the chart
3. Configure Router IP/Port if needed (default: 127.0.0.1:5000)
4. Click **OK**
5. Verify in Experts tab: `[SENDER] Trade Copier Master EA started`

**Slave Terminals:**
1. Open any chart on each slave terminal
2. Drag `signal_receiver.mq5` onto the chart
3. Configure Worker IP/Port to match your worker address (e.g., 127.0.0.1:5051)
4. Click **OK**
5. Verify in Experts tab: `[RECEIVER] Connected to worker at 127.0.0.1:5051`

### 3. Test the System

**Manual Test:**
1. On master terminal, open a trade manually (e.g., Buy 0.1 lots EURUSD)
2. Check Rust backend logs for trade reception and broadcast
3. Check slave terminal(s) for copied trades with adjusted lot sizes

Expected log flow:
```
[ROUTER] New connection from 127.0.0.1:XXXXX
[ROUTER] Received trade from 127.0.0.1:XXXXX: Trade { id: 123456, symbol: "EURUSD", ... }
[ROUTER] Broadcasted to 1 workers
[WORKER:FN-200k] Received trade: Trade { id: 123456, ... }
[WORKER:FN-200k] Adjusted lots: 0.20 (multiplier: 2)
[WORKER:FN-200k] Sent trade: {...}
```

## Network Configuration

### Local Setup (Same Machine)
- Master → Router: `127.0.0.1:5000` (TCP)
- Slaves → Workers: `127.0.0.1:5051`, `127.0.0.1:5052`, etc. (TCP)

### Remote Setup (Different Machines)
- Master → Router: `<ROUTER_IP>:5000` (TCP)
- Slaves → Workers: `<WORKER_IP>:5051`, `<WORKER_IP>:5052`, etc. (TCP)

**Firewall Rules:**
- Allow TCP port 5000 (Router incoming - Master EA connections)
- Allow TCP ports 5051-505X (Workers incoming - Slave EA connections)

## Production Deployment

### Linux Systemd Service

Create `/etc/systemd/system/trade-copier.service`:

```ini
[Unit]
Description=Trade Copier Service
After=network.target

[Service]
Type=simple
User=trader
WorkingDirectory=/opt/trade-copier
ExecStart=/opt/trade-copier/target/release/trade-copier
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable trade-copier
sudo systemctl start trade-copier
sudo systemctl status trade-copier
```

View logs:
```bash
sudo journalctl -u trade-copier -f
```

## Troubleshooting

### Rust Backend Issues

**Problem:** "Failed to bind to port 5000"
- **Solution:** Port already in use. Stop other services or change ROUTER_PORT in `router.rs`

**Problem:** "Failed to send trade signal"
- **Solution:** Check network connectivity, firewall rules, and verify slave EA is connected to worker

### MetaTrader Issues

**Problem:** EA not sending/receiving
- **Solution:** Verify "Allow DLL imports" is enabled in MT5 settings

**Problem:** "Socket creation failed"
- **Solution:** Restart MT5 with administrator privileges

**Problem:** "Socket bind failed"
- **Solution:** Port already in use. Change worker address in config/slaves.yaml

**Problem:** No trades copied
- **Solution:** Check Master EA logs for "Connected to router" and "Trade signal sent"
- **Solution:** Check Rust logs for "Received trade from..." and "Broadcasted to X workers"
- **Solution:** Check Slave EA logs for "Connected to worker" and "Received trade"

### Network Issues

**Problem:** Connection timeout
- **Solution:** Check firewall allows TCP traffic
- **Solution:** Verify worker is running and listening on the correct port
- **Solution:** Verify slave EA is configured with correct Worker IP/Port
- **Solution:** Test connectivity with `telnet <IP> <PORT>` or `nc <IP> <PORT>`

## Performance Tuning

- **Broadcast Channel Size:** Increase broadcast channel size in `main.rs` for high-frequency trading (default: 1024)
- **TCP Buffer Size:** Adjust socket buffer sizes for high-throughput scenarios
- **Connection Pooling:** TCP maintains persistent connections, eliminating connection overhead

## Security Considerations

- **TCP provides reliable delivery** - No lost trades due to packet loss
- Use VPN (WireGuard) for production deployments
- Implement TLS for encrypted TCP communication
- Add IP whitelisting in router
- Implement HMAC signatures for message authentication
- Never expose router port to public internet

## Next Steps

- Test with demo accounts first
- Monitor logs for errors
- Start with small lot sizes
- Gradually increase multipliers
- Set up monitoring/alerting
