# Trade Copier Documentation

## Overview

Trade Copier is a high-performance Rust-based service that replicates MetaTrader 5 (MT5) trades across multiple MT5 instances. It enables a master MT5 terminal to broadcast trading signals to multiple slave terminals in real-time with risk multiplier support.

## Architecture

The system consists of several key components:

1. **Router** - TCP server that receives trades from the master MT5 terminal
2. **Workers** - TCP servers that forward trades to individual slave MT5 terminals
3. **Worker Manager** - Manages the lifecycle of worker instances
4. **Database** - SQLite database for configuration, metrics, and audit logging
5. **HTTP API** - RESTful API for management and monitoring
6. **MQL5 Expert Advisors** - MT5 plugins for signal provider and receivers

## System Flow

```
Master MT5 (Signal Provider EA)
         |
         | TCP (port 5000)
         v
    Router (Rust)
         |
         | Broadcast Channel
         v
   Worker Manager
         |
         +---> Worker 1 (TCP) ---> Slave MT5 #1 (Signal Receiver EA)
         +---> Worker 2 (TCP) ---> Slave MT5 #2 (Signal Receiver EA)
         +---> Worker N (TCP) ---> Slave MT5 #N (Signal Receiver EA)
```

## Features

- **Real-time Trade Copying**: Instant replication of trades across multiple terminals
- **Risk Multiplier**: Configure individual lot size multipliers per worker
- **Trade Operations**: Support for open, close, partial close, and modify operations
- **Latency Tracking**: Monitor performance with microsecond precision
- **Connection Resilience**: Automatic reconnection handling
- **MT5 Instance Management**: Automated Wine-based MT5 installation on macOS
- **Automatic Dependency Installation**: Automatically installs Wine and Homebrew on macOS when needed
- **RESTful API**: Full HTTP API for configuration and monitoring
- **Audit Logging**: Complete trade history and error tracking
- **System Metrics**: Real-time status, uptime, and worker health monitoring

## Quick Start

### Prerequisites

- Rust (latest stable)
- MetaTrader 5

**Note for macOS users**: Wine and Homebrew will be automatically installed when you create your first MT5 instance if they are not already present on your system.

### Running the Service

```bash
cargo build --release
./target/release/trade-copier
```

The service will start:
- TCP Router on port **5000** (for master MT5 connection)
- HTTP API on port **3000** (for management)
- Individual worker TCP servers on configured ports

## Documentation Structure

- [Database](./docs/database.md) - Database schema and operations
- [Router](./docs/router.md) - Trade routing and broadcasting
- [Workers](./docs/workers.md) - Worker lifecycle and trade forwarding
- [API](./docs/api.md) - HTTP API endpoints and usage
- [Automatic Dependency Installation](./docs/automatic-dependency-installation.md) - Wine and Homebrew auto-installation
- [MQL5 Expert Advisors](./docs/mql5-expert-advisors.md) - MT5 plugins documentation
  - [Signal Provider Setup](./docs/mql5-signal-provider.md) - Master EA configuration
  - [Signal Receiver Setup](./docs/mql5-signal-receiver.md) - Slave EA configuration and README

## Configuration

Worker configurations are stored in the SQLite database (`trade_copier.db`) and can be managed through:
- HTTP API endpoints (recommended)
- Direct database access (not recommended)

Each worker configuration includes:
- **Name**: Unique identifier
- **Address**: TCP bind address (e.g., `127.0.0.1:5050`)
- **Multiplier**: Risk multiplier for lot sizes (e.g., `0.5` for half size)
- **Wine Prefix**: Path to MT5 Wine installation (macOS only)

## Monitoring

Access the HTTP API at `http://localhost:3000`:

- `GET /api/health` - Service health check
- `GET /api/workers` - List all workers and their status
- `GET /api/trades` - View trade history
- `GET /api/errors` - View error logs
- `GET /api/system/metrics` - System metrics and uptime

## Support

For issues and questions, please refer to the individual documentation files for detailed information on each component.
