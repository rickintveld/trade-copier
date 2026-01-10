# Release Guide for Trade Copier

This document provides comprehensive instructions for building, signing, and releasing Trade Copier to app stores and GitHub.

## Table of Contents
- [Prerequisites](#prerequisites)
- [Code Signing Setup](#code-signing-setup)
- [Local Build](#local-build)
- [GitHub Actions Release](#github-actions-release)
- [App Store Submission](#app-store-submission)
- [Troubleshooting](#troubleshooting)

---

## Prerequisites

### Development Environment
- **Node.js**: v20 or later
- **Rust**: Latest stable version (install via [rustup](https://rustup.rs/))
- **Tauri CLI**: Installed via `npm install` (included in dev dependencies)

### Platform-Specific Requirements

#### macOS
- **Xcode**: Install from Mac App Store
- **Xcode Command Line Tools**: `xcode-select --install`
- **Apple Developer Account**: Required for code signing and notarization

#### Windows
- **Visual Studio**: Visual Studio 2019 or later with C++ build tools
- **Code Signing Certificate**: From a trusted Certificate Authority

---

## Code Signing Setup

### macOS Code Signing

#### 1. Join Apple Developer Program
- Sign up at [developer.apple.com](https://developer.apple.com)
- Cost: $99/year for individuals, $299/year for organizations

#### 2. Create Certificates
1. Open **Xcode** → **Preferences** → **Accounts**
2. Sign in with your Apple ID
3. Select your team → **Manage Certificates**
4. Click **+** and create:
   - **Developer ID Application** (for distribution outside Mac App Store)
   - **Mac App Distribution** (for Mac App Store submission)

#### 3. Export Certificate for CI/CD
```bash
# Export Developer ID certificate
security find-identity -v -p codesigning

# Export to .p12 file (you'll be prompted for a password)
# Replace IDENTITY_HASH with the hash from the previous command
security export -k ~/Library/Keychains/login.keychain -t identities \
  -f pkcs12 -o certificate.p12 IDENTITY_HASH

# Convert to base64 for GitHub Secrets
base64 -i certificate.p12 -o certificate.base64.txt
```

#### 4. Create App-Specific Password
1. Go to [appleid.apple.com](https://appleid.apple.com)
2. Sign in → **Security** → **App-Specific Passwords**
3. Click **Generate Password** → Save it securely

#### 5. Get Team ID
```bash
# Find your Team ID
xcrun altool --list-providers -u "your-apple-id@example.com" -p "app-specific-password"
```

#### 6. Update tauri.conf.json
```json
{
  "tauri": {
    "bundle": {
      "macOS": {
        "signingIdentity": "Developer ID Application: Your Name (TEAM_ID)"
      }
    }
  }
}
```

### Windows Code Signing

#### 1. Obtain Code Signing Certificate
- Purchase from: DigiCert, Sectigo, or GlobalSign
- Typical cost: $200-$500/year
- Choose: **Code Signing Certificate** (EV not required for public distribution)

#### 2. Export Certificate
1. Install certificate in Windows Certificate Manager
2. Export to `.pfx` file with private key
3. Note the certificate thumbprint:
```powershell
Get-ChildItem -Path Cert:\CurrentUser\My | Where-Object {$_.Subject -like "*Your Name*"}
```

#### 3. Update tauri.conf.json
```json
{
  "tauri": {
    "bundle": {
      "windows": {
        "certificateThumbprint": "YOUR_CERTIFICATE_THUMBPRINT",
        "digestAlgorithm": "sha256",
        "timestampUrl": "http://timestamp.digicert.com"
      }
    }
  }
}
```

---

## Local Build

### Development Build
```bash
# Install dependencies
npm install

# Run in development mode
npm run tauri:dev
```

### Production Build

#### macOS
```bash
# Build for macOS
npm run tauri build

# Output locations:
# - DMG: src-tauri/target/release/bundle/dmg/
# - App: src-tauri/target/release/bundle/macos/Trade Copier.app
```

#### Windows
```bash
# Build for Windows
npm run tauri build

# Output locations:
# - MSI: src-tauri/target/release/bundle/msi/
# - EXE: src-tauri/target/release/bundle/nsis/
```

### Manual Notarization (macOS)

After building on macOS, notarize the app:

```bash
# Store credentials (one-time setup)
xcrun notarytool store-credentials "notarytool-profile" \
  --apple-id "your-apple-id@example.com" \
  --team-id "YOUR_TEAM_ID" \
  --password "app-specific-password"

# Submit for notarization
xcrun notarytool submit "src-tauri/target/release/bundle/dmg/Trade Copier.dmg" \
  --keychain-profile "notarytool-profile" \
  --wait

# Staple the notarization ticket
xcrun stapler staple "src-tauri/target/release/bundle/dmg/Trade Copier.dmg"
```

---

## GitHub Actions Release

### Setting Up GitHub Secrets

Navigate to your repository → **Settings** → **Secrets and variables** → **Actions** → **New repository secret**

#### Required Secrets

##### macOS Signing
| Secret Name | Description | How to Get |
|-------------|-------------|------------|
| `APPLE_CERTIFICATE` | Base64-encoded .p12 certificate | See step 3 in macOS setup |
| `APPLE_CERTIFICATE_PASSWORD` | Password for .p12 file | Password you set during export |
| `APPLE_SIGNING_IDENTITY` | Full signing identity name | `security find-identity -v -p codesigning` |
| `APPLE_ID` | Your Apple ID email | Your developer account email |
| `APPLE_PASSWORD` | App-specific password | Created in Apple ID settings |
| `APPLE_TEAM_ID` | Your team ID | From `xcrun altool --list-providers` |

##### Windows Signing (Optional)
| Secret Name | Description |
|-------------|-------------|
| `WINDOWS_CERTIFICATE` | Base64-encoded .pfx certificate |
| `WINDOWS_CERTIFICATE_PASSWORD` | Certificate password |

### Triggering a Release

#### 1. Update Version
Update version in all three files:
- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

#### 2. Update CHANGELOG
Add release notes to `CHANGELOG.md`

#### 3. Commit and Tag
```bash
# Commit version changes
git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json CHANGELOG.md
git commit -m "Release v1.0.0"

# Create and push tag
git tag -a v1.0.0 -m "Release version 1.0.0"
git push origin develop
git push origin v1.0.0
```

#### 4. Monitor GitHub Actions
- Go to **Actions** tab in your repository
- Watch the **Trade Copier Release** workflow
- Release draft will be created automatically

#### 5. Publish Release
- Go to **Releases** tab
- Edit the draft release
- Add release notes
- Click **Publish release**

---

## App Store Submission

### Mac App Store

#### 1. Prepare App Store Connect
1. Go to [appstoreconnect.apple.com](https://appstoreconnect.apple.com)
2. Create new app: **Apps** → **+** → **New App**
3. Fill in:
   - Platform: macOS
   - Name: Trade Copier
   - Primary Language: English
   - Bundle ID: `dev.rickintveld.trade-copier`
   - SKU: `trade-copier-macos`

#### 2. Build with Mac App Store Certificate
Update `tauri.conf.json`:
```json
{
  "tauri": {
    "bundle": {
      "macOS": {
        "signingIdentity": "Mac App Distribution: Your Name (TEAM_ID)",
        "providerShortName": "YOUR_TEAM_ID"
      }
    }
  }
}
```

#### 3. Upload to App Store
```bash
# Build for Mac App Store
npm run tauri build -- --target universal-apple-darwin

# Upload with Transporter app or altool
xcrun altool --upload-app \
  --type macos \
  --file "src-tauri/target/release/bundle/macos/Trade Copier.app" \
  --apiKey YOUR_API_KEY \
  --apiIssuer YOUR_ISSUER_ID
```

#### 4. Submit for Review
1. Add app information (description, screenshots, keywords)
2. Upload Privacy Policy URL
3. Add screenshots (required sizes: 1280x800, 1440x900, 2560x1600, 2880x1800)
4. Submit for review

### Microsoft Store (Windows)

#### 1. Create Microsoft Partner Account
- Sign up at [partner.microsoft.com](https://partner.microsoft.com)
- Cost: $19 one-time fee for individuals

#### 2. Reserve App Name
1. Go to **Partner Center** → **Apps and games**
2. Click **New product** → **App**
3. Reserve name: "Trade Copier"

#### 3. Create App Package
```powershell
# Build MSIX package for Microsoft Store
npm run tauri build
```

#### 4. Upload Package
1. In Partner Center → **Packages**
2. Upload the .msix file from `src-tauri/target/release/bundle/msi/`
3. Fill in store listing:
   - Description
   - Screenshots
   - Privacy policy URL
4. Submit for certification

---

## Troubleshooting

### macOS Issues

#### "App is damaged and can't be opened"
- App is not properly signed or notarized
- Solution: Verify signing with `codesign -dv --verbose=4 "Trade Copier.app"`

#### "Developer cannot be verified"
- App is not notarized
- Solution: Run notarization process again

#### Notarization fails
- Check entitlements are correct
- Verify signing identity is valid
- Check Apple ID credentials

### Windows Issues

#### "Windows protected your PC"
- App is not signed with a valid certificate
- Users can bypass by clicking "More info" → "Run anyway"
- Solution: Sign with valid EV or standard code signing certificate

#### Certificate not found
- Ensure certificate is installed in the correct store
- Verify thumbprint matches

### General Build Issues

#### Rust compilation errors
```bash
# Update Rust
rustup update stable

# Clean build
cargo clean
npm run tauri build
```

#### Node dependencies issues
```bash
# Clear cache and reinstall
rm -rf node_modules package-lock.json
npm install
```

---

## Additional Resources

- [Tauri Documentation](https://tauri.app/v1/guides/)
- [Apple Code Signing Guide](https://developer.apple.com/support/code-signing/)
- [Windows Code Signing](https://docs.microsoft.com/en-us/windows/win32/appxpkg/how-to-sign-a-package-using-signtool)
- [App Store Review Guidelines](https://developer.apple.com/app-store/review/guidelines/)
- [Microsoft Store Policies](https://docs.microsoft.com/en-us/windows/uwp/publish/store-policies)

---

## Support

For issues specific to Trade Copier releases:
- GitHub Issues: https://github.com/rickintveld/trade-copier/issues
- Email: [your-email@example.com]

**Note**: Replace placeholder values (email addresses, team IDs, etc.) with your actual information before releasing.
