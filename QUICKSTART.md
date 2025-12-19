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
    address: "127.0.0.1:5050"      # Localhost for testing
    local_bind: "0.0.0.0:6001"
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
3. Set `ListenPort = 5050`

### 5. Test It
1. Open a trade on the **Master** terminal (e.g., Buy 0.1 EURUSD)
2. Watch Rust logs for trade processing
3. Check **Slave** terminal for copied trade (with multiplier applied)

---

## 📊 Architecture at a Glance

```
Master MT5 → [UDP:5000] → Rust Router → Broadcast Channel
                                             ↓
                                         Workers (apply multipliers)
                                             ↓
                                        [UDP:5050] → Slave MT5(s)
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
| No ACK received | Check firewall, verify slave EA is running |
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

- [README.md](README.md) - Architecture and design overview
- [INSTALLATION.md](INSTALLATION.md) - Detailed setup instructions
- `config/slaves.yaml` - Configuration examples

---

## ⚠️ Important Notes

- **Always test on demo accounts first**
- **UDP is unreliable** - Trades have ACK/retry logic (3 attempts)
- **Network security** - Use VPN/WireGuard in production
- **Lot size rounding** - Lots rounded to 2 decimals (0.01 minimum)
- **No position close handling** - Currently only copies new positions

---

## 💡 Tips

- Each slave needs a unique `local_bind` port (6001, 6002, 6003...)
- Multiplier of 1.0 = same lot size, 0.5 = half, 2.0 = double
- Check MT5 "Experts" tab for EA logs
- Watch Rust terminal for real-time trade flow
- Press Ctrl+C to gracefully stop the backend
