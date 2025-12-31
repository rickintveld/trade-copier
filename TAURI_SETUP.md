# Trade Copier - Tauri Desktop Application

This project has been converted from a standalone API service to a Tauri desktop application with a React frontend.

## Architecture

- **Backend**: Rust (Tauri) - Located in `src-tauri/`
  - TCP Router on port 5000 for MT5 master terminal
  - Worker Manager for managing slave terminals
  - SQLite database for configuration and metrics
  - Tauri commands replace the previous HTTP API

- **Frontend**: React + TypeScript + Vite
  - shadcn/ui components with Radix UI
  - TanStack Query for data fetching
  - Tailwind CSS for styling
  - Communicates with backend via Tauri's invoke API

## Prerequisites

- Rust (latest stable)
- Node.js (v16 or later)
- npm or yarn
- Wine (for macOS MT5 instances)

## Installation

1. Install dependencies:
```bash
npm install
```

2. Build Rust backend:
```bash
cd src-tauri
cargo build
cd ..
```

## Development

Run the application in development mode:

```bash
npm run tauri:dev
```

This will:
1. Start the Vite dev server for the React frontend (port 1420)
2. Build and run the Tauri/Rust backend
3. Launch the desktop application window
4. Start the TCP router on port 5000

## Building for Production

Build the application for distribution:

```bash
npm run tauri:build
```

This creates distributable binaries in `src-tauri/target/release/bundle/`.

## Key Changes from API Service

### Backend Changes
- Removed axum HTTP API server
- Added Tauri commands to replace REST endpoints
- Database now uses Tauri's app data directory
- Core functionality (TCP router, workers) remains unchanged

### Frontend Changes
- All API calls now use `@tauri-apps/api` invoke instead of fetch
- API client in `src/lib/api.ts` updated to use Tauri commands
- No changes to UI components or pages

### Available Tauri Commands
- `health_check` - Check application health
- `get_workers` - Get all worker instances
- `get_trades` - Get trade history
- `get_errors` - Get error logs
- `get_system_metrics` - Get system metrics
- `get_instances` - Get MT5 instances
- `create_instance` - Create new MT5 instance
- `delete_instance` - Delete MT5 instance
- `start_instance` - Start MT5 instance
- `stop_instance` - Stop MT5 instance

## Directory Structure

```
trade-copier/
├── src/                    # React frontend
├── src-tauri/              # Tauri/Rust backend
│   ├── src/
│   │   ├── main.rs        # Tauri app entry point
│   │   ├── tauri_commands.rs  # Tauri command handlers
│   │   ├── router.rs      # TCP router
│   │   ├── worker.rs      # Worker implementation
│   │   └── ...
│   ├── Cargo.toml
│   └── tauri.conf.json
├── public/                 # Static assets
├── dist/                   # Built frontend (generated)
├── package.json
├── vite.config.ts
└── tsconfig.json
```

## Troubleshooting

### Port 5000 already in use
The TCP router requires port 5000. Ensure no other service is using this port.

### Database location
The database is now stored in the OS-specific app data directory:
- macOS: `~/Library/Application Support/trade-copier/trade_copier.db`
- Linux: `~/.local/share/trade-copier/trade_copier.db`
- Windows: `%APPDATA%\trade-copier\trade_copier.db`

### Frontend not connecting to backend
The frontend uses Tauri's IPC (Inter-Process Communication) and doesn't require network calls. If you see connection errors, check the browser console for Tauri-specific errors.

## MQL5 Expert Advisors

The MQL5 Expert Advisors in the `mql5/` directory remain unchanged:
- Signal Provider EA connects to port 5000 (TCP Router)
- Signal Receiver EAs connect to worker ports as configured

See the original documentation in `docs/` for MQL5 setup instructions.
