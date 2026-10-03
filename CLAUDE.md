# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Trade Copier is a Tauri 2 desktop app (Rust backend + React frontend) that copies MetaTrader 5 trades from one master terminal to many slave terminals in real time, with a per-slave lot multiplier, latency tracking and SQLite audit logging. Targets macOS (MT5 runs under Wine) and Windows.

Detailed design docs live in `docs/` (router, workers, database schema, IPC API, EA sync, MQL5 EAs, release process) — read the relevant one before changing a subsystem.

## Commands

```bash
npm install
npm run tauri:dev      # Rust backend + Vite dev server (localhost:1420)
npm run tauri:build    # Release bundle (.dmg / .exe / .msi)
npm run build          # Frontend only -> dist/

cargo check            # Run from repo root (Cargo workspace, single member: backend/)
cargo test
cargo test test_slave_config_validation_valid   # Single test
cargo fmt && cargo clippy
npx eslint frontend/   # No npm lint script exists
```

Rust unit tests exist only in `backend/src/types.rs` and `backend/src/ea_sync.rs`. There is no frontend test setup.

Logging: `env_logger` is only initialized in debug builds (`#[cfg(debug_assertions)]` in `main.rs`), so release binaries produce no log output regardless of `RUST_LOG`.

## Layout

- `frontend/` — React 19 + TypeScript, Vite 8, react-router v7 (`HashRouter`), shadcn/ui (`frontend/components/ui/`, generated — avoid hand-editing), Tailwind 3. Import alias `@` → `frontend/`.
- `backend/` — the Tauri crate (`tauri.conf.json`, capabilities, icons) and Rust sources in `backend/src/`.
- `backend/src/mql5/Trading Rocket/` — MQL5 EAs (`Signal Provider` for master, `Signal Receiver` for slaves). Both `.mq5` source and compiled `.ex5` are committed; after editing `.mq5`, the `.ex5` must be recompiled in MetaEditor. This folder is bundled as a Tauri resource.

## Architecture

```
Master MT5 (Signal Provider EA) --TCP :5000 (JSON)--> router.rs
   router --tokio::broadcast<Trade> (cap 8192)--> one task per worker (worker.rs)
   worker binds its own host:port listener; Slave MT5 (Signal Receiver EA) connects to it
```

- **`main.rs`** wires everything: opens the DB at `dirs::data_local_dir()/trade-copier/trade_copier.db`, syncs EAs to the master MT5, creates the broadcast channel, starts `WorkerManager`, the router, a 30s metrics collector, then the Tauri app. Background tasks share a `watch<bool>` shutdown signal; on window close it calls `prevent_close`, runs `WorkerManager::shutdown_all()` (stops workers, kills Wine servers, marks DB inactive) and then `process::exit(0)`.
- **`worker_manager.rs`** owns running workers (`HashMap` of join handle + shutdown `watch` sender) and is driven by an `mpsc` of `WorkerCommand::{Start, Stop}`. Tauri commands never spawn workers directly — they send commands through `AppState.worker_command_tx`. On startup `sync_database_state()` resets workers left `active` by a crash.
- **`worker.rs`** per trade: `lots *= multiplier`, then appends `symbol_prefix` to the symbol (it is effectively a suffix, e.g. `EURUSD` → `EURUSD.m`), sends to the slave and waits up to 5s for an ack, recording latency (µs) and errors to the DB. On macOS it also monitors the worker's Wine prefix process.
- **`port_utils::bind_with_retry`** is used for all listeners; it sets SO_REUSEADDR and may kill a stale process holding the port.
- **`installer.rs`** (`InstanceManager`) dispatches to `installer/mac.rs` (Wine prefixes under the app data dir, `wine-mt5-instance{id}`) or `installer/windows.rs` via `#[cfg(target_os)]`; shared code in `installer/common.rs` and `installer/package_manager.rs`. `dependency_manager.rs` handles Homebrew/Wine detection and install on macOS.
- **`types.rs`** `Trade` is the wire format shared with both EAs (`type` field renamed via serde, `cmd` defaults to `"open"`, optional `order_type` for pending orders). Changing it requires matching changes in both `.mq5` files.

### Database (`database.rs`)

All access is async via `tokio-rusqlite`. The schema is created in `Database::new()` with `CREATE TABLE IF NOT EXISTS`; migrations are ad hoc `ALTER TABLE ... ADD COLUMN` statements whose errors are deliberately ignored (`let _ = conn.execute(...)`) — follow that pattern for new columns. Default feature toggles are seeded there with `INSERT OR IGNORE`. Worker states: `inactive`, `active`, `error`, `installing`.

### Frontend ↔ backend

There is no HTTP API; everything goes through Tauri IPC. Adding a command means: implement `#[tauri::command]` in `tauri_commands.rs` returning `ApiResponse<T>`, register it in the `generate_handler!` list in `main.rs`, and add a typed wrapper to `tradeCopierApi` in `frontend/lib/api.ts` (whose `Api*` interfaces mirror the Rust structs in snake_case).

`frontend/hooks/useTradingData.ts` polls the backend every 2s with `setInterval` (not React Query) and transforms `Api*` types into the camelCase types in `frontend/types/trading.ts`. SQLite `CURRENT_TIMESTAMP` values are UTC without a zone, so they are parsed by appending `Z` — use `parseUTCTimestamp` for any new timestamp field.

## Release

Pushing a `v*` tag triggers `.github/workflows/release.yml`, which builds macOS and Windows bundles with `tauri-action` and publishes a GitHub release (also the updater endpoint configured in `backend/tauri.conf.json`). Signing details are in `docs/release.md`.
