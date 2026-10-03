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

npm run typecheck      # tsc (strict) for app + node configs
npm run lint           # eslint frontend/
npm test               # Vitest (jsdom, TZ=UTC); npm run test:watch for watch mode

cargo check            # Run from repo root (Cargo workspace, single member: backend/)
cargo test
cargo test adjust_applies_multiplier   # Single test (substring match)
cargo fmt --check && cargo clippy --all-targets -- -D warnings
```

CI (`.github/workflows/ci.yml`) runs all of the above on every PR: frontend on Ubuntu, Rust on macOS and Windows. Keep both at zero warnings.

Tests: Rust tests are `#[cfg(test)] mod tests` inside each module (binary crate, so no `tests/` dir); DB tests use `Database::new(":memory:")`, network tests bind `127.0.0.1:0` (never fixed ports — `bind_with_retry` may kill the holder). Frontend tests are `*.test.ts(x)` next to the code and mock the Tauri boundary (`@tauri-apps/api/core` or `@/lib/api`).

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

There is no HTTP API; everything goes through Tauri IPC. Adding a command means: implement `#[tauri::command]` in `tauri_commands.rs` returning `ApiResponse<T>`, register it in the `generate_handler!` list in `main.rs`, and add a typed wrapper to `tradeCopierApi` in `frontend/lib/api.ts` (whose `Api*` interfaces mirror the Rust structs in snake_case). See the `add-tauri-command` skill.

**Invoke argument keys must be camelCase** (`{ workerId, symbolPrefix }`): Tauri 2 maps them to snake_case Rust params and silently drops snake_case keys. `frontend/lib/api.test.ts` guards this.

`frontend/hooks/useTradingData.ts` polls the backend every 2s with `setInterval` (not React Query); the pure transforms from `Api*` types to the camelCase types in `frontend/types/trading.ts` live in `frontend/lib/transforms.ts`. SQLite `CURRENT_TIMESTAMP` values are UTC without a zone — parse every timestamp with `parseUTCTimestamp` from `frontend/lib/time.ts`.

## Claude Code agents and skills

Project agents live in `.claude/agents/`:

- `rust-senior` — implements backend work; writes only `backend/`, `Cargo.*`, `docs/`.
- `react-senior` — implements frontend work; writes only `frontend/`, frontend configs, `docs/`.
- `rust-reviewer` / `react-reviewer` — read-only reviewers that report ranked findings.

The builders can read across the whole repo, but a `PreToolUse` hook (`.claude/hooks/guard-scope.sh`) blocks writes outside their domain. For a cross-cutting feature, run `rust-senior` first; it ends with a hand-off describing the IPC contract, which goes to `react-senior`. Then run both reviewers. Skills in `.claude/skills/` (`add-tauri-command`, `add-db-column`, `change-trade-wire-format`) hold the checklists for the risky cross-layer changes. `.claude/settings.json` pre-allows the verification commands and runs rustfmt on edited `.rs` files.

## Release

Pushing a `v*` tag triggers `.github/workflows/release.yml`, which builds macOS and Windows bundles with `tauri-action` and publishes a GitHub release (also the updater endpoint configured in `backend/tauri.conf.json`). Signing details are in `docs/release.md`.
