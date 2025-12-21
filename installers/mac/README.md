# MT5 Manager

A CLI tool to manage multiple MetaTrader 5 slave instances on macOS for the trade copier system.

## Prerequisites

- macOS
- Homebrew
- Wine (installed via `brew install --cask wine-stable`)
- MT5 installer from your broker

## Installation

Build the project:

```bash
cargo build --release
```

The binary will be available at `target/release/mt5-manager`.

Optionally, install it globally:

```bash
cargo install --path .
```

## Usage

### Create a new MT5 instance

```bash
mt5-manager create --name slave1 --installer ~/Downloads/mt5setup.exe
```

This will:
1. Check if Wine is installed
2. Create a Wine prefix at `~/.wine-mt5-instance{N}`
3. Install MT5 from the provided installer
4. Save instance metadata

### List all instances

```bash
mt5-manager list
```

Shows all created instances with their IDs, paths, and creation dates.

### Start instances

Start a specific instance:

```bash
mt5-manager start slave1
```

Start all instances:

```bash
mt5-manager start --all
```

Instances are launched in the background with a 2-second delay between each.

### Delete an instance

```bash
mt5-manager delete slave1
```

This will prompt for confirmation before deleting the Wine prefix and removing the instance metadata.

## How it works

- Each instance has an isolated Wine prefix at `~/.wine-mt5-instance{N}`
- Instance metadata is stored in `~/.mt5-manager/instances.json`
- MT5 is installed at `{prefix}/drive_c/Program Files/MetaTrader 5/terminal64.exe`
- Instances are completely independent with separate:
  - Login credentials
  - Trading accounts
  - Expert Advisors (EAs)
  - Settings and data

## Troubleshooting

### Wine not found

```bash
brew install --cask wine-stable
```

### Display issues

Install XQuartz:

```bash
brew install --cask xquartz
```

### Instance won't start

Check that MT5 is properly installed by verifying the executable exists:

```bash
ls ~/.wine-mt5-instance{N}/drive_c/Program\ Files/MetaTrader\ 5/terminal64.exe
```

## Examples

```bash
# Create three slave instances
mt5-manager create --name slave1 --installer ~/Downloads/mt5setup.exe
mt5-manager create --name slave2 --installer ~/Downloads/mt5setup.exe
mt5-manager create --name slave3 --installer ~/Downloads/mt5setup.exe

# List all instances
mt5-manager list

# Start all instances
mt5-manager start --all

# Start a specific instance
mt5-manager start slave2

# Delete an instance
mt5-manager delete slave3
```

## Related

See [../../trade-copier/docs/RUNNING_MT5_ON_MACOS.md](../../trade-copier/docs/RUNNING_MT5_ON_MACOS.md) for more details on running MT5 on macOS.
