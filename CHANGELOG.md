# Changelog

All notable changes to Trade Copier will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-01-10

### Added
- Initial production release
- Real-time trade copying between multiple MT5 instances
- TCP router and worker architecture for trade broadcasting
- Risk multiplier support for individual workers
- Symbol prefix configuration for broker-specific symbol naming
- SQLite database for configuration and audit logging
- Performance metrics and latency tracking
- Automatic Wine installation for MT5 on macOS
- RESTful API integration via Tauri commands
- React-based dashboard with real-time updates
- Worker management (create, start, stop, delete instances)
- Trade history and error logging
- CSV export for trade data
- Performance analytics dashboard
- Legal documentation (LICENSE, Privacy Policy, Terms of Service)
- macOS entitlements.plist with network permissions for TCP operations
- Proper logging infrastructure using `log` and `env_logger` crates
- GitHub Actions workflow for automated builds and releases
- Unit tests for configuration validation and trade serialization
- `SlaveConfig::validate()` method for input validation
- Comprehensive RELEASE.md guide for app store submissions
- PRODUCTION_READY_SUMMARY.md documenting all improvements

### Features
- Support for open, close, partial close, and modify trade operations
- Connection resilience with automatic reconnection
- MT5 instance management with Wine-based installation
- System metrics monitoring (uptime, worker health)
- Configurable TCP ports for each worker
- Real-time status updates for all workers
- Limit and stop order management

### Changed
- Updated bundle identifier from `com.trade-copier.app` to `dev.rickintveld.trade-copier`
- Changed app category from "DeveloperTool" to "Finance"
- Replaced all `println!`/`eprintln!` with structured logging (info!, error!, warn!)
- Improved error handling throughout codebase (reduced unwrap() usage)
- Console output in frontend now wrapped in development-only conditionals
- Synced version numbers across package.json, Cargo.toml, and tauri.conf.json

### Fixed
- CSV export functionality for trade data
- Stop and limit order management
- Symbol prefix application timing
- Time formatting to use correct timezone
- Performance chart opacity (set to 50%)

### Security
- Scoped file system access (Downloads, Documents, Desktop only)
- Content Security Policy (CSP) enforcement
- Minimal permission allowlist (shell.open, dialog.save, fs.writeFile only)
- Local-only data storage (no cloud sync)
- TCP connections limited to localhost and local network
- macOS entitlements properly configured for network and file access
- Wine process memory execution allowed for MT5 compatibility

### Documentation
- Comprehensive README with architecture overview
- API documentation
- MQL5 Expert Advisor setup guides
- Installation and configuration instructions
- RELEASE.md - Complete guide for code signing and app store submission
- PRIVACY_POLICY.md - GDPR/CCPA compliant privacy policy
- TERMS_OF_SERVICE.md - Comprehensive terms with trading risk disclaimers
- PRODUCTION_READY_SUMMARY.md - Summary of all production improvements
- Added metadata to Cargo.toml (authors, license, description, repository)
- Added copyright and descriptions to tauri.conf.json

## [Unreleased]

### Planned
- Expanded automated testing suite (frontend and integration tests)
- Enhanced error recovery mechanisms
- Trade filtering and conditional copying
- Trade analytics and reporting dashboard

---

## Version History Legend

- `Added` - New features
- `Changed` - Changes in existing functionality
- `Deprecated` - Soon-to-be removed features
- `Removed` - Removed features
- `Fixed` - Bug fixes
- `Security` - Security improvements
