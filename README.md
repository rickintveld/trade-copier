# Trade Copier

A desktop application for replicating MetaTrader 5 (MT5) trades across multiple terminals in real-time. A master MT5 terminal broadcasts trading signals to any number of slave terminals, each with an independently configured risk multiplier.

Built with Rust + Tauri 2 + React.

## Architecture

```
Master MT5 (Signal Provider EA)
         |
         | TCP (port 5000)
         v
    Router (Rust/Tokio)
         |
         | Broadcast channel
         v
   Worker Manager
         |
         +---> Worker 1 (TCP) ---> Slave MT5 #1 (Signal Receiver EA)
         +---> Worker 2 (TCP) ---> Slave MT5 #2 (Signal Receiver EA)
         +---> Worker N (TCP) ---> Slave MT5 #N (Signal Receiver EA)
```

The React dashboard communicates with the Rust backend via Tauri IPC — no HTTP API.

## Features

- **Real-time trade copying** — instant replication of open, close, partial close, and modify operations
- **Per-worker risk multiplier** — configure individual lot size scaling per slave terminal
- **Latency tracking** — microsecond-precision latency monitoring per worker
- **MT5 instance management** — automated Wine-based MT5 installation on macOS
- **Automatic dependency installation** — installs Wine and Homebrew on macOS when needed
- **Automatic EA sync** — Expert Advisors are copied to the master MT5 on startup
- **Audit logging** — full trade history, error log, and system metrics persisted in SQLite

## Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) (v18+)
- [Tauri CLI prerequisites](https://tauri.app/start/prerequisites/) for your platform
- MetaTrader 5

> **macOS users:** Wine and Homebrew are installed automatically when you create your first MT5 instance.

## Development

```bash
npm install
npm run tauri:dev
```

## Build

```bash
npm run tauri:build
```

Produces a native installer in `backend/target/release/bundle/` (`.dmg` on macOS, `.exe` on Windows, `.deb` on Linux).

## Documentation

- [Quick Start](./docs/quickstart.md)
- [Database schema](./docs/database.md)
- [Router](./docs/router.md)
- [Workers](./docs/workers.md)
- [API (Tauri IPC commands)](./docs/api.md)
- [EA synchronization](./docs/ea-sync.md)
- [Automatic dependency installation](./docs/automatic-dependency-installation.md)
- [MQL5 Expert Advisors](./docs/mql5-expert-advisors.md)
  - [Signal Provider](./docs/mql5-signal-provider.md)
  - [Signal Receiver](./docs/mql5-signal-receiver.md)
- [Release guide](./docs/release.md)
- [Privacy Policy](./docs/privacy-policy.md)
- [Terms of Service](./docs/terms-of-service.md)
