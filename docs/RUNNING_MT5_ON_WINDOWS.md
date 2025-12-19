# Running Multiple MetaTrader 5 Instances on Windows

This guide explains how to run multiple MetaTrader 5 instances on Windows.

## Prerequisites

- Windows 10/11
- MT5 installer (download from your broker)

## Installation

### Method 1: Portable Installation (Recommended)

This method allows you to run multiple independent MT5 instances without conflicts.

#### 1. Create Separate Installation Directories

```powershell
# Create directories for each instance
mkdir C:\MT5-Instance1
mkdir C:\MT5-Instance2
mkdir C:\MT5-Instance3
```

#### 2. Download MT5 Installer

Download the MT5 installer from your broker's website.

#### 3. Install MT5 in Portable Mode

For each instance:

1. Run the MT5 installer
2. During installation, select **Custom Installation**
3. Choose the installation directory (e.g., `C:\MT5-Instance1`)
4. Complete the installation
5. Repeat for each instance using different directories

#### 4. Launch Each Instance

Simply run the executable from each directory:

```powershell
# Launch first instance
Start-Process "C:\MT5-Instance1\terminal64.exe"

# Launch second instance
Start-Process "C:\MT5-Instance2\terminal64.exe"

# Launch third instance
Start-Process "C:\MT5-Instance3\terminal64.exe"
```

### Method 2: Using Sandboxie (Advanced)

Sandboxie allows you to run MT5 instances in isolated sandboxes.

#### 1. Install Sandboxie

Download from: https://sandboxie-plus.com/

#### 2. Create Sandboxes

1. Open Sandboxie Control
2. Create new sandbox: `Sandbox → Create New Box`
3. Name it `MT5-Instance1`
4. Repeat to create `MT5-Instance2`, `MT5-Instance3`, etc.

#### 3. Install MT5 in Each Sandbox

1. Right-click sandbox → Run → Run Any Program
2. Browse to MT5 installer
3. Install MT5 normally within the sandbox
4. Repeat for each sandbox

#### 4. Launch Instances

Right-click each sandbox → Run → Browse and select `terminal64.exe`

## Running MT5 Instances

### Manual Launch

```powershell
# Navigate to each directory and run
cd C:\MT5-Instance1
.\terminal64.exe

# Or use full paths
Start-Process "C:\MT5-Instance1\terminal64.exe"
Start-Process "C:\MT5-Instance2\terminal64.exe"
Start-Process "C:\MT5-Instance3\terminal64.exe"
```

### Create Launch Script

Create a PowerShell script to launch all instances at once:

**launch-mt5-instances.ps1:**
```powershell
# Launch all MT5 instances
Write-Host "Launching MT5 instances..."

Start-Process "C:\MT5-Instance1\terminal64.exe"
Start-Sleep -Seconds 2

Start-Process "C:\MT5-Instance2\terminal64.exe"
Start-Sleep -Seconds 2

Start-Process "C:\MT5-Instance3\terminal64.exe"
Start-Sleep -Seconds 2

Write-Host "All MT5 instances launched successfully!"
```

Run it:
```powershell
.\launch-mt5-instances.ps1
```

### Create Batch File Alternative

**launch-mt5-instances.bat:**
```batch
@echo off
echo Launching MT5 instances...

start "" "C:\MT5-Instance1\terminal64.exe"
timeout /t 2 /nobreak >nul

start "" "C:\MT5-Instance2\terminal64.exe"
timeout /t 2 /nobreak >nul

start "" "C:\MT5-Instance3\terminal64.exe"
timeout /t 2 /nobreak >nul

echo All MT5 instances launched!
pause
```

Double-click to run.

### Create Desktop Shortcuts

For each instance:

1. Right-click desktop → New → Shortcut
2. Browse to: `C:\MT5-Instance1\terminal64.exe`
3. Name it: `MT5 Instance 1`
4. Repeat for other instances

## Configuration

Each instance is completely independent:
- Separate login credentials
- Separate trading accounts
- Separate Expert Advisors (EAs)
- Separate data directories
- Independent updates

Configure each instance through its own MT5 interface.

## Troubleshooting

### Port Conflicts

If you encounter port conflicts, modify the port settings in each MT5 instance:
1. Tools → Options → Server
2. Change the port number for each instance

### Permission Issues

If MT5 fails to start:
1. Right-click `terminal64.exe`
2. Properties → Compatibility
3. ✅ Run this program as administrator
4. Apply

### Firewall Blocking

Add exceptions for each MT5 instance:
1. Windows Security → Firewall & network protection
2. Allow an app through firewall
3. Add each `terminal64.exe` path
4. Allow both Private and Public networks

### Instances Not Staying Separate

Make sure each installation is in a **completely separate directory**. Do not install multiple instances in subdirectories of the same parent folder.

## Startup on Boot

### Using Task Scheduler

1. Open Task Scheduler (`taskschd.msc`)
2. Create Basic Task → Name it "Launch MT5 Instances"
3. Trigger: When I log on
4. Action: Start a program
5. Browse to your `.bat` or `.ps1` script
6. Finish

### Using Startup Folder

1. Press `Win + R`
2. Type: `shell:startup`
3. Copy your `.bat` file to the Startup folder

## Performance Tips

- **RAM**: Each MT5 instance uses ~200-500 MB. Plan accordingly.
- **CPU**: Close unnecessary charts to reduce CPU usage
- **Disk**: Each instance needs ~500 MB - 1 GB disk space
- **Network**: Ensure stable internet connection for all instances

## Alternative Solutions

### Virtual Machines

- **Hyper-V** (Windows 10/11 Pro)
- **VirtualBox** (Free)
- **VMware Workstation** (Paid)

Each VM can run a full Windows installation with MT5.

**Pros:**
- Complete isolation
- Better for different broker requirements
- Can snapshot/backup entire environments

**Cons:**
- High resource usage
- Requires Windows licenses for each VM
- More complex setup

## Notes

- Windows natively supports multiple MT5 installations without Wine
- Each instance requires approximately 500 MB - 1 GB disk space
- Updates must be applied to each instance independently
- Consider using different brokers for each instance to avoid account conflicts
- Always test on demo accounts first

## Security Considerations

- Keep each instance updated separately
- Use different passwords for each trading account
- Enable two-factor authentication where available
- Run instances with least privileges (avoid admin unless necessary)
- Consider using a VPN for additional security
