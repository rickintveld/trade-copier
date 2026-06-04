# Trade Copier - Quick Start Guide

## Prerequisites Installed?
- ✅ Rust (check: `rustc --version`)
- ✅ Node.js (check: `node --version`)
- ✅ npm (check: `npm --version`)

## First Time Setup

1. **Install dependencies:**
```bash
npm install
```

2. **Build the Rust backend (optional, for verification):**
```bash
cd src-tauri
cargo build
cd ..
```

## Running the App

### Development Mode (Recommended for testing)
```bash
npm run tauri:dev
```

This will:
- Build the Rust backend
- Start the Vite dev server
- Launch the desktop application
- Enable hot-reload for frontend changes

**Expected output:**
```
🚀 Trade Copier Starting...
💾 Database initialized at ~/Library/Application Support/trade-copier/trade_copier.db
✅ Trade Copier backend is running
📡 TCP Router listening on port 5000
🌐 Tauri app initialized
```

### Production Build
```bash
npm run tauri:build
```

The distributable app will be in: `src-tauri/target/release/bundle/`

## Common Issues

### Port 5000 already in use
```bash
# Find what's using port 5000
lsof -i :5000

# Kill the process if needed
kill -9 <PID>
```

### Frontend build errors
```bash
# Clear node_modules and reinstall
rm -rf node_modules package-lock.json
npm install
```

### Rust build errors
```bash
# Clean and rebuild
cd src-tauri
cargo clean
cargo build
cd ..
```

### Database issues
The database is stored in your system's app data directory. To reset:
```bash
# macOS
rm -rf ~/Library/Application\ Support/trade-copier/

# Linux
rm -rf ~/.local/share/trade-copier/

# Windows
# Delete: %APPDATA%\trade-copier\
```

## Next Steps

1. **Create MT5 Workers:**
   - Open the app
   - Go to "Instances" or "Workers" section
   - Click "Add Worker" or "Create Instance"
   - Configure worker settings (name, address, multiplier)

2. **Connect Master MT5:**
   - Install Signal Provider EA in your master MT5 terminal
   - Configure it to connect to `localhost:5000`
   - Start the EA

3. **Connect Slave MT5:**
   - Install Signal Receiver EA in slave MT5 terminals
   - Configure each to connect to their respective worker ports
   - Start the EAs

4. **Monitor:**
   - View real-time trades in the dashboard
   - Check worker status and latency
   - Review error logs if needed

## Development Tips

- Frontend code is in `src/`
- Backend code is in `src-tauri/src/`
- API client is in `src/lib/api.ts`
- Tauri commands are in `src-tauri/src/tauri_commands.rs`
- Frontend changes hot-reload automatically in dev mode
- Backend changes require app restart

## Documentation

- `README.md` - Original project documentation
- `TAURI_SETUP.md` - Detailed Tauri setup and architecture
- `MIGRATION_SUMMARY.md` - Complete migration details
- `docs/` - Original API and component documentation

## Getting Help

If you encounter issues:
1. Check the console output in the terminal
2. Check the browser dev tools (in the Tauri window: right-click → Inspect)
3. Review error logs in the app's error section
4. Check the documentation files above
