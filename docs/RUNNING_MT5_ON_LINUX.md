# Running Multiple MetaTrader 5 Instances on Linux

This guide explains how to run multiple MetaTrader 5 instances on Linux using Wine.

## Prerequisites

- Linux (Ubuntu/Debian/Fedora/Arch)
- Wine installed
- MT5 installer (download from your broker)

## Installation

### 1. Install Wine

#### Ubuntu/Debian:
```bash
# Enable 32-bit architecture
sudo dpkg --add-architecture i386

# Add WineHQ repository
sudo mkdir -pm755 /etc/apt/keyrings
sudo wget -O /etc/apt/keyrings/winehq-archive.key https://dl.winehq.org/wine-builds/winehq.key
sudo wget -NP /etc/apt/sources.list.d/ https://dl.winehq.org/wine-builds/ubuntu/dists/$(lsb_release -cs)/winehq-$(lsb_release -cs).sources

# Install Wine
sudo apt update
sudo apt install --install-recommends winehq-stable
```

#### Fedora:
```bash
sudo dnf config-manager --add-repo https://dl.winehq.org/wine-builds/fedora/$(rpm -E %fedora)/winehq.repo
sudo dnf install winehq-stable
```

#### Arch Linux:
```bash
sudo pacman -S wine wine-mono wine-gecko
```

### 2. Create Wine Prefixes

Each MT5 instance needs its own Wine prefix (isolated Windows environment):

```bash
# Create first instance
WINEPREFIX=~/.wine-mt5-instance1 winecfg

# Create second instance
WINEPREFIX=~/.wine-mt5-instance2 winecfg

# Create third instance
WINEPREFIX=~/.wine-mt5-instance3 winecfg
```

A configuration window will appear for each prefix. Set Windows version to **Windows 10** and click OK.

### 3. Install Dependencies

For each Wine prefix, install required Windows components:

```bash
# Install winetricks if not already installed
sudo apt install winetricks  # Ubuntu/Debian
# or
sudo dnf install winetricks  # Fedora
# or
yay -S winetricks  # Arch

# Install dependencies for each instance
WINEPREFIX=~/.wine-mt5-instance1 winetricks vcrun2019 corefonts
WINEPREFIX=~/.wine-mt5-instance2 winetricks vcrun2019 corefonts
WINEPREFIX=~/.wine-mt5-instance3 winetricks vcrun2019 corefonts
```

### 4. Install MT5 in Each Prefix

Download the MT5 installer from your broker, then install it in each prefix:

```bash
# Install in first instance
WINEPREFIX=~/.wine-mt5-instance1 wine ~/Downloads/mt5setup.exe

# Install in second instance
WINEPREFIX=~/.wine-mt5-instance2 wine ~/Downloads/mt5setup.exe

# Install in third instance
WINEPREFIX=~/.wine-mt5-instance3 wine ~/Downloads/mt5setup.exe
```

Follow the installation wizard for each instance.

## Running MT5 Instances

### Manual Launch

Run each instance with its corresponding Wine prefix:

```bash
# Run first instance
WINEPREFIX=~/.wine-mt5-instance1 wine ~/.wine-mt5-instance1/drive_c/Program\ Files/MetaTrader\ 5/terminal64.exe &

# Run second instance
WINEPREFIX=~/.wine-mt5-instance2 wine ~/.wine-mt5-instance2/drive_c/Program\ Files/MetaTrader\ 5/terminal64.exe &

# Run third instance
WINEPREFIX=~/.wine-mt5-instance3 wine ~/.wine-mt5-instance3/drive_c/Program\ Files/MetaTrader\ 5/terminal64.exe &
```

### Create Launch Script

Create a script to launch all instances at once:

**launch-mt5-instances.sh:**
```bash
#!/bin/bash

# Launch all MT5 instances in the background
echo "Launching MT5 instances..."

WINEPREFIX=~/.wine-mt5-instance1 wine ~/.wine-mt5-instance1/drive_c/Program\ Files/MetaTrader\ 5/terminal64.exe &
sleep 2

WINEPREFIX=~/.wine-mt5-instance2 wine ~/.wine-mt5-instance2/drive_c/Program\ Files/MetaTrader\ 5/terminal64.exe &
sleep 2

WINEPREFIX=~/.wine-mt5-instance3 wine ~/.wine-mt5-instance3/drive_c/Program\ Files/MetaTrader\ 5/terminal64.exe &

echo "All MT5 instances launched successfully!"
```

Make it executable:
```bash
chmod +x launch-mt5-instances.sh
```

Run it:
```bash
./launch-mt5-instances.sh
```

### Create Systemd Service (Advanced)

For running MT5 instances as system services:

**~/.config/systemd/user/mt5-instance1.service:**
```ini
[Unit]
Description=MetaTrader 5 Instance 1
After=network.target

[Service]
Type=simple
Environment="WINEPREFIX=/home/YOUR_USERNAME/.wine-mt5-instance1"
Environment="DISPLAY=:0"
ExecStart=/usr/bin/wine /home/YOUR_USERNAME/.wine-mt5-instance1/drive_c/Program Files/MetaTrader 5/terminal64.exe
Restart=on-failure
RestartSec=10

[Install]
WantedBy=default.target
```

Replace `YOUR_USERNAME` with your actual username.

Create similar files for `mt5-instance2.service` and `mt5-instance3.service`.

Enable and start services:
```bash
systemctl --user daemon-reload
systemctl --user enable mt5-instance1.service
systemctl --user enable mt5-instance2.service
systemctl --user enable mt5-instance3.service

systemctl --user start mt5-instance1.service
systemctl --user start mt5-instance2.service
systemctl --user start mt5-instance3.service
```

Check status:
```bash
systemctl --user status mt5-instance1.service
```

## Configuration

Each instance is completely independent:
- Separate login credentials
- Separate trading accounts
- Separate Expert Advisors (EAs)
- Separate settings and data directories

Configure each instance through its own MT5 interface.

## Troubleshooting

### Wine Not Found
```bash
# Check if Wine is installed
which wine

# Check version
wine --version

# If not found, reinstall Wine
```

### Display Issues

If you encounter graphics problems:

```bash
# Install required graphics libraries
sudo apt install libgl1-mesa-glx libgl1-mesa-dri  # Ubuntu/Debian
```

Set graphics mode in winecfg:
```bash
WINEPREFIX=~/.wine-mt5-instance1 winecfg
```
Go to Graphics tab → Set to "Emulate a virtual desktop"

### Missing DLL Errors

Install missing Visual C++ runtimes:
```bash
WINEPREFIX=~/.wine-mt5-instance1 winetricks vcrun2019
```

### Font Rendering Issues

```bash
WINEPREFIX=~/.wine-mt5-instance1 winetricks corefonts
```

### Audio Errors (Can be ignored)

If you see ALSA/audio errors but MT5 works, you can safely ignore them or disable audio:
```bash
WINEPREFIX=~/.wine-mt5-instance1 winecfg
```
Go to Audio tab → Disable audio output

### Network/Firewall Issues

Allow Wine through firewall:
```bash
# UFW (Ubuntu)
sudo ufw allow from any to any port 443 proto tcp

# Firewalld (Fedora)
sudo firewall-cmd --permanent --add-port=443/tcp
sudo firewall-cmd --reload
```

### Performance Issues

- Close unnecessary applications
- Reduce the number of charts in each MT5 instance
- Use a lightweight desktop environment (XFCE, LXDE)
- Increase Wine's memory limits in winecfg

## Running Headless (VPS)

For running on a headless server, use Xvfb (virtual framebuffer):

### Install Xvfb
```bash
sudo apt install xvfb  # Ubuntu/Debian
sudo dnf install xorg-x11-server-Xvfb  # Fedora
```

### Launch with Xvfb
```bash
#!/bin/bash
# launch-mt5-headless.sh

# Start virtual display
Xvfb :99 -screen 0 1024x768x24 &
export DISPLAY=:99

# Launch MT5 instances
WINEPREFIX=~/.wine-mt5-instance1 wine ~/.wine-mt5-instance1/drive_c/Program\ Files/MetaTrader\ 5/terminal64.exe &
sleep 2

WINEPREFIX=~/.wine-mt5-instance2 wine ~/.wine-mt5-instance2/drive_c/Program\ Files/MetaTrader\ 5/terminal64.exe &
sleep 2

WINEPREFIX=~/.wine-mt5-instance3 wine ~/.wine-mt5-instance3/drive_c/Program\ Files/MetaTrader\ 5/terminal64.exe &

echo "MT5 instances running headless on display :99"
```

## Alternative Solutions

### PlayOnLinux

User-friendly Wine wrapper with GUI:
```bash
sudo apt install playonlinux
```

Allows easy management of multiple Wine prefixes through a graphical interface.

### Virtual Machines

- **KVM/QEMU** (Native Linux virtualization)
- **VirtualBox** (Free)
- **VMware Workstation** (Paid)

Each VM can run Windows with native MT5 support.

## Performance Optimization

### Wine Staging

Wine Staging includes experimental patches for better performance:
```bash
sudo apt install --install-recommends winehq-staging
```

### DXVK (DirectX to Vulkan)

Improves graphics performance:
```bash
WINEPREFIX=~/.wine-mt5-instance1 winetricks dxvk
```

### Disable Wine Debug Output

Reduce console spam:
```bash
export WINEDEBUG=-all
```

Add this to your launch script or `.bashrc`.

## Notes

- Each Wine prefix takes approximately 1-2 GB of disk space
- MT5 updates will need to be applied to each instance independently
- Keep Wine updated: `sudo apt update && sudo apt upgrade wine`
- Wine performance is generally good for MT5, but native Windows is faster
- Test thoroughly on demo accounts before using on live accounts

## Security Considerations

- Run Wine as a regular user, never as root
- Use firewall rules to restrict network access
- Consider using AppArmor or SELinux for additional isolation
- Keep Wine and system packages updated
- Use VPN/WireGuard for network security in production
