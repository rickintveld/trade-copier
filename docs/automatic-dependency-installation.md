# Automatic Dependency Installation

## Overview

Trade Copier automatically installs required dependencies when they are not present on your system. This feature ensures a smooth setup experience, especially for users who may not have Wine or Homebrew already installed.

## Supported Platforms

### macOS

On macOS, the application requires:
- **Homebrew** - Package manager for macOS
- **Wine** - Windows compatibility layer to run MetaTrader 5

Both will be automatically installed when you create or start an MT5 instance if they are not already present.

### Windows

On Windows, Wine is not required as MetaTrader 5 runs natively. However, the application includes support for **Chocolatey** package manager for potential future dependencies.

## How It Works

### macOS Installation Flow

When you create or start an MT5 instance, the application:

1. **Checks for Wine**: Looks for Wine in common installation paths:
   - `/opt/homebrew/bin/wine` (Apple Silicon)
   - `/usr/local/bin/wine` (Intel)
   - `/opt/local/bin/wine` (MacPorts)

2. **Checks for Homebrew** (if Wine not found): Verifies if Homebrew is installed

3. **Installs Homebrew** (if needed):
   - Downloads and runs the official Homebrew installation script
   - May require user interaction and sudo password
   - Automatically detects Apple Silicon vs Intel architecture

4. **Installs Wine** (if needed):
   - Uses Homebrew to install Wine: `brew install --cask wine-stable`
   - This may take several minutes depending on your system

### Windows Installation Flow

Windows users do not need Wine. The Chocolatey package manager installation is available but not required for current functionality.

## Installation Behavior

### Automatic Mode (GUI)

When running the GUI application:
- Wine installation happens **automatically** without prompts
- Progress is shown in the application logs
- Errors are logged to the database and displayed in the UI

### Interactive Mode (CLI)

For command-line usage, there's an interactive version that:
- Prompts the user before installing dependencies
- Allows cancellation if the user prefers manual installation

## Troubleshooting

### macOS

**Wine installation fails:**
- Ensure you have an active internet connection
- Check that you have sufficient disk space (Wine requires ~1GB+)
- Try manual installation: `brew install --cask wine-stable`

**Homebrew installation fails:**
- You may need to install Xcode Command Line Tools first: `xcode-select --install`
- Check Homebrew's official documentation at https://brew.sh

**Wine not found after installation:**
- Restart your terminal or application
- Add Homebrew to your PATH (Apple Silicon): `eval "$(/opt/homebrew/bin/brew shellenv)"`
- Add Homebrew to your PATH (Intel): `eval "$(/usr/local/bin/brew shellenv)"`

**Permission errors during installation:**
- Homebrew installation requires sudo privileges
- You may be prompted for your password during installation

### Windows

**Chocolatey installation fails:**
- Ensure you're running PowerShell as Administrator
- Check Windows execution policy settings
- Try manual installation from https://chocolatey.org

## Manual Installation

If you prefer to install dependencies manually:

### macOS

```bash
# Install Homebrew (if not already installed)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install Wine
brew install --cask wine-stable
```

### Windows

Wine is not needed on Windows. MetaTrader 5 runs natively.

## System Requirements

### macOS
- macOS 10.15 (Catalina) or later
- At least 2GB of free disk space for Wine and dependencies
- Internet connection for downloading packages
- Xcode Command Line Tools (usually installed automatically with Homebrew)

### Windows
- Windows 10 or later
- Administrator privileges (for package manager installation if needed)

## Security Considerations

### macOS
- Homebrew installation script is downloaded from the official Homebrew GitHub repository
- Wine is installed from the official Homebrew cask repository
- You may be prompted for your password for sudo operations

### Windows
- Chocolatey installation requires administrator privileges
- PowerShell execution policy must allow running scripts

## Performance Notes

- **First-time installation** may take 5-15 minutes depending on your internet speed and system performance
- **Subsequent operations** will be instant as dependencies are already installed
- Wine installation includes downloading and building several dependencies

## Logs and Debugging

Installation progress and errors are logged with the `[INSTALLER]` prefix:

```
[INSTALLER] Wine is required but not installed
[INSTALLER] Automatically installing Wine and Homebrew (if needed)...
[INSTALLER] Homebrew is not installed. Installing Homebrew...
[INSTALLER] Homebrew installed successfully
[INSTALLER] Installing wine-stable (this may take several minutes)...
[INSTALLER] Wine installed successfully
[INSTALLER] Wine installation complete!
```

Any errors during installation are also logged to the application's error log table and can be viewed via:
- The API endpoint: `GET /api/errors`
- The application UI error logs section

## Future Enhancements

Potential improvements for automatic dependency installation:
- Progress bars for long-running installations
- Ability to cancel installations
- Pre-flight checks before starting installation
- Cached installers for offline installation
- Alternative installation methods (MacPorts, Scoop, etc.)
