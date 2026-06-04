# Expert Advisor Synchronization

## Overview

The Trade Copier application automatically synchronizes Expert Advisors (EAs) from the application bundle to the master MetaTrader 5 installation on every startup. This ensures that the latest version of the EAs is always available in the master trading account.

## How It Works

### Startup Process

When the application starts, it performs the following steps:

1. **Locate Source Directory**: Finds the EA files bundled with the application
   - Development mode: `./src-tauri/src/mql5/Trading Rocket/`
   - Production mode: `<app-bundle>/Resources/src/mql5/Trading Rocket/`

2. **Locate Master MT5 Installation**: Detects the master MT5 installation
   - **macOS**: `~/Library/Application Support/net.metaquotes.wine.metatrader5/drive_c/Program Files/MetaTrader 5/MQL5/Experts/`
   - **Windows**: 
     - `C:\Program Files\MetaTrader 5\MQL5\Experts\`
     - `C:\Program Files (x86)\MetaTrader 5\MQL5\Experts\`
     - `%LOCALAPPDATA%\Programs\MetaTrader 5\MQL5\Experts\`

3. **Copy Files**: Copies all EA files to `Trading Rocket/` subdirectory in the master MT5 Experts folder
   - Overwrites existing files to ensure latest version
   - Skips hidden files (like `.DS_Store`)
   - Creates target directory if it doesn't exist

4. **Error Handling**: If sync fails (e.g., MT5 not installed), logs a warning but continues startup
   - User can install MT5 later and restart the application
   - User can also manually install EAs if needed

### Files Synchronized

The following files are copied on each startup:

- `Signal Provider.mq5` - Source code for the signal provider EA
- `Signal Provider.ex5` - Compiled signal provider EA
- `Signal Receiver.mq5` - Source code for the signal receiver EA
- `Signal Receiver.ex5` - Compiled signal receiver EA

## Benefits

1. **Always Up-to-Date**: Users automatically get the latest EA version with each app update
2. **Zero Manual Installation**: No need to manually copy files to MT5 directory
3. **Cross-Platform**: Works seamlessly on both macOS and Windows
4. **Reliable Updates**: Every restart ensures EAs are synchronized

## Master vs Slave Instances

- **Master Instance**: The local MT5 installation where the "Signal Provider" EA runs
  - This is where you manually execute trades
  - The EA sends trade signals to the Trade Copier application
  - EAs are automatically synced to this installation on app startup

- **Slave Instances**: Managed MT5 instances created through the application
  - These receive and replicate trades from the master
  - EAs are installed automatically during instance creation
  - Located in isolated Wine prefixes (macOS) or separate directories (Windows)

## Troubleshooting

### EA Sync Failed Warning on Startup

If you see a warning about EA sync failure:

1. **Check MT5 Installation**: Ensure MetaTrader 5 is installed on your system
2. **Verify Path**: Check that MT5 is installed in one of the standard locations listed above
3. **Manual Installation**: If needed, manually copy EAs from:
   - `<app-install-dir>/Resources/src/mql5/Trading Rocket/`
   - To: `<MT5-install-dir>/MQL5/Experts/Trading Rocket/`
4. **Restart Application**: After installing MT5, restart the Trade Copier application

### Custom MT5 Installation Path

If your MT5 is installed in a non-standard location:

1. The sync will fail with a warning
2. Manually copy the EA files to your MT5 `Experts` directory
3. Consider moving MT5 to a standard location if possible

## Technical Details

### Implementation

The EA sync functionality is implemented in `src-tauri/src/ea_sync.rs` and called during application startup in `main.rs`.

Key functions:
- `sync_expert_advisors_to_master()`: Main entry point, called on app startup
- `get_master_mt5_experts_path()`: Platform-specific MT5 path detection
- `get_ea_source_path()`: Locates EA files in app bundle

### Logging

All sync operations are logged with the `[EA_SYNC]` prefix:
- `[EA_SYNC] Starting Expert Advisor synchronization...`
- `[EA_SYNC] Source: <path>`
- `[EA_SYNC] Destination: <path>`
- `[EA_SYNC]   ✓ Copied: <filename>`
- `[EA_SYNC] ✓ Successfully synchronized N Expert Advisor file(s)`

### Error Cases

The sync operation is non-blocking:
- Errors are logged as warnings
- Application continues startup even if sync fails
- User can manually install EAs or fix the issue and restart
