# Privacy Policy for Trade Copier

**Last Updated: January 10, 2026**

## Introduction

Trade Copier ("we", "our", or "the App") is committed to protecting your privacy. This Privacy Policy explains how we handle information when you use our desktop application.

## Information We Collect

### Information You Provide
- **MT5 Configuration**: TCP addresses, port numbers, and multiplier settings for your MetaTrader 5 instances
- **Wine Installation Paths**: File paths to MT5 installations on macOS (stored locally)
- **Trading Parameters**: Symbol prefixes and other trade copying configurations

### Automatically Collected Information
- **Trade Data**: Trade operations, timestamps, lot sizes, and symbols (stored locally in SQLite database)
- **Performance Metrics**: Latency measurements and worker status (stored locally)
- **Error Logs**: Application errors and diagnostic information (stored locally)

## How We Use Your Information

All data collected by Trade Copier is:
- **Stored locally** on your device in a SQLite database
- **Never transmitted** to external servers or third parties
- **Used solely** for the functionality of copying trades between MT5 instances

## Data Storage and Security

- All data is stored locally in: `~/Library/Application Support/trade-copier/` (macOS) or `%LOCALAPPDATA%\trade-copier\` (Windows)
- No cloud synchronization or remote data transmission occurs
- Data is only accessible by the Trade Copier application on your device
- You maintain full control over your data and can delete it at any time

## Network Communication

Trade Copier uses:
- **Local TCP connections** to communicate with MetaTrader 5 instances on your device or local network
- **No internet connectivity** for application functionality
- **Optional update checks** to GitHub (if updater is enabled) - only version information is transmitted

## Third-Party Services

Trade Copier does not integrate with or share data with any third-party services, analytics platforms, or advertising networks.

## Data Retention

- Trade history and logs are retained locally until you manually delete them
- Worker configurations are retained until you remove them through the application
- You can clear all data by deleting the application data directory

## Your Rights

You have the right to:
- Access all your data through the application interface
- Export your data (trade history can be exported to CSV)
- Delete your data by removing the application and its data directory
- Opt out of automatic updates by disabling the updater

## Children's Privacy

Trade Copier is not intended for use by children under 18 years of age. We do not knowingly collect information from children.

## Changes to This Privacy Policy

We may update this Privacy Policy from time to time. Any changes will be reflected in the "Last Updated" date above and distributed with application updates.

## Contact Information

For questions about this Privacy Policy, please contact:
- Discord: https://discord.com/invite/HV8ta8asQN
- Website: https://trading-rocket.nl

## Compliance

This application:
- Does not collect personally identifiable information (PII)
- Does not use cookies or tracking technologies
- Does not share data with third parties
- Operates entirely on your local device
- Complies with GDPR, CCPA, and other privacy regulations by design (local-first architecture)

## Disclaimer

Trade Copier is a tool for managing MetaTrader 5 trades. Users are solely responsible for:
- The security of their MT5 accounts and trading decisions
- Compliance with their broker's terms of service
- Financial risks associated with automated trading
- Securing their local device and data
