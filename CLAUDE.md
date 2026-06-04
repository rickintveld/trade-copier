# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Trade Copier is a professional-grade Rust + React/Tauri desktop application that replicates MetaTrader 5 (MT5) trades across multiple MT5 instances in real-time. It enables a master MT5 terminal to broadcast trading signals to multiple slave terminals with risk multiplier support, latency tracking, and comprehensive audit logging.

## Architecture

### System Design

The application uses a **multi-layer architecture** with clear separation between frontend (React) and backend (Rust):

```
Master MT5 (Signal Provider EA)
         |
         | TCP (port 5000)
         v
    Router (Rust async)
         |
         | Tokio broadcast channel
         v
   Worker Manager (lifecycle management)
         |
         +---> Worker 1 (TCP) ---> Slave MT5 #1
         +---> Worker 2 (TCP) ---> Slave MT5 #2
         +---> Worker N (TCP) ---> Slave MT5 #N
```

### Technology Stack

**Frontend:**
- React 18.3 + TypeScript
- Tauri 2 (desktop framework, replaces Electron)
- Vite 5 (build tool on port 1420)
- React Query for state management
- Radix UI + shadcn/ui for components
- TailwindCSS for styling
- React Router v6 for navigation

**Backend (Rust):**
- Tokio (async runtime)
- Rusqlite (SQLite with tokio-rusqlite wrapper)
- Socket2 for TCP socket control
- Serde for JSON serialization
- Tauri IPC for frontend-backend communication

### Workspace Structure

The project is a **Cargo workspace** with two members:
- Root directory: Contains frontend code (React/TypeScript)
- `backend/`: Contains Rust backend code (Tauri backend)

Each has its own `Cargo.toml` and `package.json` respectively.

## Key Components

### Frontend (`/frontend`)

**Pages:**
- `pages/Index.tsx` - Main dashboard (single-page app)

**Core Hooks:**
- `hooks/useTradingData.ts` - Polls backend API every 2s for workers, trades, errors, metrics, profits
- `hooks/useDependencyStatus.ts` - Tracks Wine/Homebrew dependency status on macOS
- `hooks/useFeatureToggles.ts` - Feature flag management
- `hooks/useMobile.tsx` - Responsive design helper

**Main Components:**
- `Dashboard.tsx` - Root container with tabs for Workers, Positions, Errors, Performance, Profit, Economic Calendar
- `WorkersPanel.tsx` - Lists worker instances with status
- `WorkerCard.tsx` - Individual worker display with MT5 connection status
- `WorkerFormDialog.tsx` - Create/edit worker instance dialog
- `PositionTable.tsx` - View open positions replicated across workers
- `ErrorLogPanel.tsx` - View critical errors with severity levels
- `PerformanceChart.tsx` - Latency and trade frequency visualization
- `ProfitChart.tsx` - Per-worker profit history
- `EconomicCalendar.tsx` - Economic event calendar integration
- `DependencyCheckModal.tsx` - macOS Wine/Homebrew dependency checker
- `MetricsPanel.tsx` - System health (uptime, active workers, provider connection status)

**API Integration (`lib/api.ts`):**
- Thin wrapper around Tauri IPC (`invoke` command)
- Calls backend Tauri commands (e.g., `get_workers`, `start_instance`, `create_instance`)
- Response types align with backend Rust data structures

### Backend (`/backend/src`)

**Core Modules:**

1. **`main.rs`** - Application entry point
   - Initializes Tokio runtime
   - Sets up database
   - Spawns Router (TCP listener on port 5000)
   - Spawns Worker Manager (lifecycle control)
   - Spawns Metrics Collector (30s interval)
   - Builds Tauri app with command handlers
   - Graceful shutdown on window close

2. **`router.rs`** - TCP server for master MT5
   - Listens on port 5000
   - Accepts JSON trade messages
   - Broadcasts trades via Tokio broadcast channel to all active workers
   - Tracks provider connection status in database
   - Logs each trade to database

3. **`worker.rs`** - Individual worker task
   - TCP server for each worker (configured port per instance)
   - Receives broadcast trades from Router
   - Applies risk multiplier to lot sizes
   - Sends modified trades to slave MT5
   - Monitors Wine process on macOS
   - Health check heartbeat (detects dead connections)
   - Tracks latency in microseconds
   - Handles acknowledgments and error responses

4. **`worker_manager.rs`** - Lifecycle orchestration
   - Spawns/stops individual worker tasks
   - Maintains HashMap of running workers with shutdown channels
   - Processes `WorkerCommand` enum (Start/Stop)
   - Syncs database state on startup (marks stale workers inactive)
   - Graceful shutdown of all workers on app close

5. **`database.rs`** - SQLite persistence
   - **Tables:**
     - `workers`: Instance config (id, name, address, multiplier, state, latency_us, mt5_connected, symbol_prefix, wine_prefix)
     - `trades`: Trade history (trade_id, worker_id, symbol, type, lots, price, sl, tp, cmd)
     - `worker_errors`: Error log (worker_id, severity, message)
     - `system_metrics`: Health metrics (router_status, active_workers, total_trades, avg_latency, uptime_seconds, provider_connected)
     - `profits`: Worker profit tracking (worker_id, profit, created_at)
     - `feature_toggles`: Feature flags
     - `dependencies`: macOS Wine/Homebrew installation status
   - All queries use prepared statements (async with tokio-rusqlite)
   - Indexes on frequently queried columns (state, worker_id, trade_id, cmd)

6. **`tauri_commands.rs`** - Tauri IPC command handlers
   - Query commands: `get_workers`, `get_trades`, `get_errors`, `get_system_metrics`, `get_profit_history`, `get_feature_toggles`
   - Management commands: `create_instance`, `update_instance`, `delete_instance`, `start_instance`, `stop_instance`
   - Dependency commands: `get_dependency_status`, `check_dependencies`, `install_dependencies`
   - Economic calendar: `fetch_economic_calendar`
   - All return `ApiResponse<T>` with success flag and data

7. **`dependency_manager.rs`** - macOS automation
   - Detects missing Wine and Homebrew
   - Automatically installs Homebrew if missing
   - Automatically installs Wine via Homebrew
   - Shows dialog prompts on first run
   - Updates dependency status in database

8. **`installer.rs`** - MT5 instance management
   - Creates Wine prefixes for MT5 (macOS only)
   - Launches Wine/MT5 executables
   - Monitors running MT5 processes
   - `InstanceManager` handles platform-specific behavior

9. **`ea_sync.rs`** - Expert Advisor synchronization
   - On startup, copies MQL5 EAs from `backend/src/mql5/` to master MT5's EA folder
   - Detects MT5 installation on Windows/macOS
   - Ensures latest EA code is available in master terminal

10. **`port_utils.rs`** - TCP socket utilities
    - `bind_with_retry()`: Attempts to bind socket, kills stale processes if port in TIME_WAIT
    - Socket configuration (SO_REUSEADDR)
    - Uses socket2 crate for low-level control

11. **`types.rs`** - Rust data structures
    - `Trade`: Message format sent from master (id, symbol, type, lots, price, sl, tp, cmd, order_type)
    - `SlaveConfig`: Worker configuration with validation
    - `ProfitInfo`: Profit tracking struct

**MQL5 Expert Advisors (`/backend/src/mql5/`):**
- Signal Provider EA: Runs in master MT5, sends trades to Router on port 5000
- Signal Receiver EA: Runs in slave MT5 terminals, receives trades from Worker, applies risk multiplier

## Development Commands

### Frontend Development

```bash
# Install dependencies
npm install

# Run dev server (Vite on localhost:1420)
npm run dev

# Build frontend only (creates /dist)
npm run build

# Preview built frontend
npm run preview
```

### Tauri Development (Frontend + Backend Combined)

```bash
# Dev mode: runs Rust backend + Vite dev server
npm run tauri:dev

# Build release binary (builds frontend, embeds in Tauri app)
npm run tauri:build
```

### Rust Backend Only

```bash
# Build debug
cargo build

# Build release
cargo build --release

# Run release binary standalone (without Tauri window)
./target/release/trade-copier

# Run tests
cargo test

# Run with logging enabled (dev mode)
RUST_LOG=info ./target/release/trade-copier

# Check for issues without building
cargo check
```

### Code Quality

ESLint is configured via `eslint.config.js`. There is no `lint` or `format` npm script — run ESLint directly:

```bash
npx eslint frontend/
cargo fmt        # Rust formatting
cargo clippy     # Rust linting
```

## Key Concepts

### Real-time Data Flow

The frontend uses a polling approach via `useTradingData()` hook:
- Polls every 2 seconds (configurable)
- Fetches workers, trades, errors, metrics, profits via Tauri IPC
- Data transforms in `useTradingData.ts` convert API responses to frontend types
- UTC timestamps from SQLite are parsed with 'Z' suffix

### Worker Lifecycle

1. **Create**: User submits form → `create_instance` command → Database insert
2. **Start**: User clicks "Start" → `start_instance` command → Worker Manager spawns worker task
3. **Run**: Worker task:
   - Binds TCP socket to configured address
   - Subscribes to broadcast channel
   - Accepts trades, applies multiplier, sends to slave MT5
   - Monitors heartbeat, logs errors, tracks latency
4. **Stop**: User clicks "Stop" → `stop_instance` command → Worker Manager sends shutdown signal
5. **Delete**: User clicks "Delete" → `delete_instance` command → Graceful stop + database delete

### Worker State Machine

States in database:
- `inactive`: Stopped or never started
- `active`: Currently running
- `error`: Last attempt failed (check `last_error` field)
- `installing`: MT5 installation in progress (macOS only)

### Risk Multiplier

On trade receipt, worker adjusts lot size:
```
adjusted_lots = original_lots * multiplier
```
Example: 1.0 lot from master with 0.5 multiplier = 0.5 lot to slave

### Broadcast Channel

- Rust: `tokio::sync::broadcast::channel(8192)` sized for ~8K trades
- Each worker subscribes independently
- Dropped messages logged (high-frequency trading scenario)
- Resilient to slow workers

### macOS-Specific Features

- Automatic Wine installation (dependency_manager.rs)
- MT5 installation via Wine (installer.rs)
- Expert Advisor sync to master MT5 on startup (ea_sync.rs)
- Wine process monitoring per worker (monitor_wine_process in worker.rs)
- Wine prefix path stored in database for each worker

### Database Persistence

- All trades, errors, metrics, profits logged to SQLite
- Database file: `~/.local/share/trade-copier/trade_copier.db` (Linux/Windows) or equivalent on macOS
- No in-memory state recovery (clean slate on app restart, but database maintains history)

### Graceful Shutdown

On window close:
1. Signal all background tasks (router, metrics collector)
2. Stop all workers (broadcast stop to wine monitors, close TCP sockets)
3. Update database state (mark workers inactive)
4. Exit process

Prevents orphaned Wine processes and ensures clean database state.

## Important Files

| File | Purpose |
|------|---------|
| `frontend/App.tsx` | Root React component, Tauri IPC setup |
| `frontend/pages/Index.tsx` | Entry point (renders Dashboard) |
| `frontend/components/Dashboard.tsx` | Main UI layout with tabs |
| `frontend/lib/api.ts` | Tauri IPC wrapper |
| `frontend/hooks/useTradingData.ts` | Core data polling and transformation |
| `backend/src/main.rs` | App initialization, background tasks |
| `backend/src/router.rs` | Master MT5 TCP listener |
| `backend/src/worker.rs` | Individual worker TCP server |
| `backend/src/worker_manager.rs` | Worker lifecycle management |
| `backend/src/database.rs` | SQLite schema and queries |
| `backend/src/tauri_commands.rs` | IPC command implementations |
| `vite.config.ts` | Frontend build config (Tauri Chromium/WebKit targets) |
| `tsconfig.json` | TypeScript config (relaxed strict mode) |
| `tailwind.config.ts` | Custom colors for trading UI (buy/sell/open/close) |

## Common Modifications

### Adding a New Worker Command

1. Define command in `tauri_commands.rs`:
```rust
#[tauri::command]
pub async fn my_command(state: State<'_, AppState>) -> Result<ApiResponse<T>, String> {
    // implementation
}
```

2. Register in `main.rs`:
```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    tauri_commands::my_command,
])
```

3. Call from frontend:
```typescript
const response = await invoke<ApiResponse<T>>('my_command', { /* args */ });
```

### Adding a New Dashboard Component

1. Create component in `frontend/components/`
2. Import in `Dashboard.tsx`
3. Add Tab in TabsContent (or integrate into existing component)
4. Hook up data from `useTradingData()` or create new custom hook

### Modifying Database Schema

1. Update `database.rs` in `new()` method (CREATE TABLE or ALTER)
2. Add migration logic if handling upgrades
3. Update query methods to access new columns
4. Update Tauri command handlers to expose new data

### Adding Feature Toggle

1. Insert row in `feature_toggles` table
2. Call `isEnabled('feature_key')` from `useFeatureToggles()` hook
3. Conditionally render UI based on toggle

## Testing

The Rust backend includes unit tests in modules:
```bash
# Run all tests
cargo test

# Run specific test
cargo test test_slave_config_validation

# Run with output
cargo test -- --nocapture
```

Frontend testing setup (Vitest/Jest) not yet configured; currently manual testing via Tauri dev mode.

## Performance Considerations

- **Broadcast channel size (8192)**: Tuned for high-frequency trading; increase if messages are dropped
- **Polling interval (2s)**: Frontend refresh rate; lower = more responsive but higher CPU
- **Database indexes**: Present on state, worker_id, trade_id, cmd for fast lookups
- **TCP keep-alive**: Enabled on worker sockets to detect dead connections
- **Metrics collection (30s)**: Background task that aggregates stats
- **Latency tracking**: Microsecond precision (latency_us in database)

## Debugging

### Enable Logging (Rust)

```bash
RUST_LOG=info cargo run --release
RUST_LOG=debug cargo run  # More verbose
```

Logs print to console in dev mode, not in release Tauri app (can be captured via file).

### Check Database State

```bash
sqlite3 ~/.local/share/trade-copier/trade_copier.db
.schema        # View all tables
SELECT * FROM workers;
SELECT * FROM trades LIMIT 20;
```

### Monitor TCP Connections

```bash
# macOS
lsof -i :5000  # Master MT5 listener
lsof -i :5001  # Worker port example

# Linux
ss -tlnp | grep :5000
```

### Tauri Dev Tools

- Open DevTools in dev mode (Ctrl+Shift+I on Windows, Cmd+Shift+I on macOS)
- Inspect frontend via Chrome DevTools
- View Tauri console for IPC logs

## Build for Production

```bash
npm run tauri:build
```

Creates native installers (.dmg on macOS, .exe on Windows, .deb on Linux) in `backend/target/release/bundle/`.

Configured in `backend/tauri.conf.json` with:
- Auto-update via GitHub releases
- Code signing setup (empty by default)
- App icon and metadata
