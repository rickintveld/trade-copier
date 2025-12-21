# MT5 Manager

A CLI tool to manage multiple MetaTrader 5 slave instances for the trade copier system.

## Overview

This tool simplifies the creation, deletion, and startup of multiple MT5 installations on Windows, enabling you to run multiple independent MT5 instances as slaves for trade copying.

## Features

- **Create** MT5 instance directories with automatic registration
- **Delete** MT5 instances with confirmation protection
- **Start** individual or all MT5 instances with proper timing
- **List** all managed instances with their status

## Installation

### Build from Source

```bash
cargo build --release
```

The binary will be available at `target/release/mt5-manager.exe`

### Install Globally (Optional)

```bash
cargo install --path .
```

## Usage

### Create a New Instance

Create a new MT5 instance with default path:

```bash
mt5-manager create slave1
```

Create with custom installation path:

```bash
mt5-manager create slave1 --path C:\CustomPath\MT5-Slave1
```

**What happens:**
- Creates the directory structure
- Registers the instance in the config
- Provides instructions for MT5 installation

### List All Instances

View all configured MT5 instances:

```bash
mt5-manager list
```

Output shows:
- Instance name
- Installation path
- Status (Installed / Not installed)

### Start MT5 Instances

Start a specific instance:

```bash
mt5-manager start slave1
```

Start all instances:

```bash
mt5-manager start
```

**Note:** There's a 2-second delay between starting each instance to prevent conflicts.

### Delete an Instance

Delete with confirmation prompt:

```bash
mt5-manager delete slave1
```

Force delete without confirmation:

```bash
mt5-manager delete slave1 --force
```

**Warning:** This permanently removes the instance directory and all files.

## Workflow

### 1. Create Instance Directories

```bash
mt5-manager create slave1
mt5-manager create slave2
mt5-manager create slave3
```

### 2. Install MT5

For each instance:
1. Download the MT5 installer from your broker
2. Run the installer
3. Select **Custom Installation**
4. Choose the directory created by mt5-manager (e.g., `C:\MT5-slave1`)
5. Complete the installation

### 3. Verify Installation

```bash
mt5-manager list
```

All instances should show "Installed" status.

### 4. Start Instances

Launch all slaves:

```bash
mt5-manager start
```

Or launch specific slaves:

```bash
mt5-manager start slave1
```

## Configuration

Instance metadata is stored in:
- **Windows**: `%APPDATA%\mt5-manager\config.json`
- **Other platforms** (for testing): `~/.mt5-manager/config.json`

The config file tracks:
- Instance name
- Installation path
- Creation timestamp
- Port number (reserved for future use)

## Platform Support

This tool is designed specifically for **Windows** where MT5 runs natively. The start functionality uses Windows CMD commands to launch terminal64.exe.

For testing purposes, the tool can compile on other platforms but will only perform dry-run operations.

## Requirements

- Windows 10/11
- MT5 installer from your broker
- Sufficient disk space (~1GB per instance)
- Sufficient RAM (~500MB per instance)

## Troubleshooting

### "MT5 executable not found"

The instance directory exists but MT5 is not installed. Install MT5 to the directory shown by `mt5-manager list`.

### "Instance already exists"

An instance with that name is already registered. Use `mt5-manager list` to view existing instances.

### "Directory already exists"

The target directory already exists. Either:
- Delete the directory manually
- Use a different instance name
- Specify a different path with `--path`

## Examples

```bash
# Create 3 slave instances
mt5-manager create slave1
mt5-manager create slave2
mt5-manager create slave3

# List all instances
mt5-manager list

# Start all instances
mt5-manager start

# Start specific instance
mt5-manager start slave2

# Delete an instance
mt5-manager delete slave3 --force
```

## Related Documentation

See `../../trade-copier/docs/RUNNING_MT5_ON_WINDOWS.md` for detailed information about running multiple MT5 instances on Windows.

## License

This tool is part of the trade copier project.
