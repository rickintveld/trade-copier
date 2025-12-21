# Trade Copier - Quick Start Guide

## ⚡ 5-Minute Setup (Local Testing)

### 1. Build the Project
```bash
cargo build --release
```

### 2. Configure Slave(s)
Edit `config/slaves.yaml` with your slave account details:
```yaml
slaves:
  - name: "FN-200k"
    address: "0.0.0.0:5051"        # Worker TCP server address
    multiplier: 2.0
```

### 3. Start Rust Backend
```bash
cargo run --release
```

You should see:
```
🚀 Trade Copier Starting...
📋 Loaded 1 slave(s) from config
✅ Trade Copier is running
📡 Router listening on port 5000
```

### 4. Setup MetaTrader 5

**Enable Sockets (Required!):**
- Tools → Options → Expert Advisors
- ✅ Allow DLL imports
- ✅ Allow automated trading

**Master Terminal:**
1. Compile `mql5/signal_sender.mq5`
2. Attach to any chart
3. Set `RouterIP = "127.0.0.1"` and `RouterPort = 5000`

**Slave Terminal(s):**
1. Compile `mql5/signal_receiver.mq5`
2. Attach to any chart
3. Set `WorkerIP = "127.0.0.1"` and `WorkerPort = 5051`

### 5. Test It
1. Open a trade on the **Master** terminal (e.g., Buy 0.1 EURUSD)
2. Watch Rust logs for trade processing
3. Check **Slave** terminal for copied trade (with multiplier applied)

---

## 📊 Architecture at a Glance

```
Master MT5 → [TCP:5000] → Rust Router → Broadcast Channel
                                            ↓
                                        Workers (apply multipliers)
                                            ↓
                                   [TCP:5051/5052/...] ← Slave MT5(s)
```

---

## 🔧 Common Commands

### Development
```bash
# Check code (fast)
cargo check

# Run in debug mode
cargo run

# Run in release mode (optimized)
cargo run --release

# Build release binary
cargo build --release
```

### Production
```bash
# Run binary directly
./target/release/trade-copier

# Run as systemd service (Linux)
sudo systemctl start trade-copier
sudo systemctl status trade-copier
sudo journalctl -u trade-copier -f
```

---

## 🐛 Quick Troubleshooting

| Problem | Solution |
|---------|----------|
| Port 5000 already in use | Change `ROUTER_PORT` in `src/router.rs` |
| EA not sending trades | Enable "Allow DLL imports" in MT5 |
| Connection failed | Check firewall allows TCP, verify worker is running |
| Socket bind failed | Run MT5 as administrator |

---

## 📝 Key Files

| File | Purpose |
|------|---------|
| `src/main.rs` | Entry point, loads config and spawns tasks |
| `src/router.rs` | UDP listener, receives trades from Master EA |
| `src/worker.rs` | Applies multipliers, forwards to Slaves |
| `src/types.rs` | Data structures (Trade, Config, Ack) |
| `config/slaves.yaml` | Slave configuration |
| `mql5/signal_sender.mq5` | Master EA (sends trades) |
| `mql5/signal_receiver.mq5` | Slave EA (receives trades) |

---

## 🚀 Next Steps

1. ✅ **Test on demo accounts** - Never test on live accounts first!
2. ✅ **Monitor logs** - Watch for errors or timeouts
3. ✅ **Start small** - Use low multipliers initially (0.1, 0.5)
4. ✅ **Add more slaves** - Edit `config/slaves.yaml` and restart
5. ✅ **Deploy to VPS** - See INSTALLATION.md for systemd setup

---

## 📚 Full Documentation

- [README.md](../README.md) - Architecture and design overview
- [INSTALLATION.md](INSTALLATION.md) - Detailed setup instructions
- `config/slaves.yaml` - Configuration examples

---

## ⚠️ Important Notes

- **Always test on demo accounts first**
- **TCP connections** - Reliable, persistent connections between components
- **Network security** - Use VPN/WireGuard in production
- **Lot size rounding** - Lots rounded to 2 decimals (0.01 minimum)
- **Full position management** - Copies open, close, and SL/TP modifications

---

## 💡 Tips

- Each worker needs a unique port (5051, 5052, 5053...)
- Each slave EA must connect to its corresponding worker port
- Multiplier of 1.0 = same lot size, 0.5 = half, 2.0 = double
- Check MT5 "Experts" tab for EA logs
- Watch Rust terminal for real-time trade flow
- Press Ctrl+C to gracefully stop the backend

---

## 🎯 Position Management Features

The trade copier now supports **full position lifecycle management**:

### What Gets Copied

1. **Opening Positions** - When you open a trade on master, it opens on all slaves with their respective multipliers
2. **Closing Positions** - Close any specific position on master, and that exact position closes on all slaves
3. **Modifying SL/TP** - Adjust stop loss or take profit on master, changes replicate to slaves

### Multiple Positions Support

You can open multiple positions on the same symbol:
- Each position is tracked individually with a unique ID
- Close or modify specific positions without affecting others
- Master positions are mapped to corresponding slave positions

### How It Works

**Master EA** tracks every position with a unique trade ID and monitors:
- Position opens (sends `cmd: "open"`)
- Position closes (sends `cmd: "close"`)
- SL/TP changes (sends `cmd: "modify"`)

**Slave EA** maintains a mapping of trade IDs to position tickets:
- Executes opens and stores the mapping
- Uses mapping to close/modify the correct position
- Handles errors gracefully if position doesn't exist
