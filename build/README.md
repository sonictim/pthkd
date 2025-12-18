# Build Directory

This directory contains build scripts and assets for creating a signed, notarized macOS .app bundle.

## Files

- **bundle.sh** - Main build script (creates universal binary, signs, and notarizes)
- **entitlements.plist** - Code signing entitlements
- **icon.icns** - App icon (add your own icon here)
- **install_launchd.sh** - Install as LaunchAgent for auto-start
- **com.feralfrequencies.pthkd.plist** - LaunchAgent configuration

## Quick Start

### Build a signed, notarized app:

```bash
./build/bundle.sh
```

This will:
1. Build universal binary (ARM64 + x86_64)
2. Create .app bundle structure
3. Code sign with your Developer ID
4. Submit for notarization
5. Staple the notarization ticket

The final app will be at: `target/universal/release/pthkd.app`

## Creating an Icon

To add an app icon:

1. Create a 1024x1024 PNG icon
2. Convert to .icns format:
   ```bash
   # Create iconset directory
   mkdir icon.iconset

   # Generate all required sizes (use sips or Photoshop)
   sips -z 16 16     icon.png --out icon.iconset/icon_16x16.png
   sips -z 32 32     icon.png --out icon.iconset/icon_16x16@2x.png
   sips -z 32 32     icon.png --out icon.iconset/icon_32x32.png
   sips -z 64 64     icon.png --out icon.iconset/icon_32x32@2x.png
   sips -z 128 128   icon.png --out icon.iconset/icon_128x128.png
   sips -z 256 256   icon.png --out icon.iconset/icon_128x128@2x.png
   sips -z 256 256   icon.png --out icon.iconset/icon_256x256.png
   sips -z 512 512   icon.png --out icon.iconset/icon_256x256@2x.png
   sips -z 512 512   icon.png --out icon.iconset/icon_512x512.png
   sips -z 1024 1024 icon.png --out icon.iconset/icon_512x512@2x.png

   # Convert to .icns
   iconutil -c icns icon.iconset -o build/icon.icns

   # Cleanup
   rm -rf icon.iconset
   ```

3. The build script will automatically include it

## Installing the App

After building:

```bash
# Install to Applications
cp -r target/universal/release/pthkd.app /Applications/

# Or just open it to test
open target/universal/release/pthkd.app
```

## Auto-Start on Login

To make the app start automatically on login:

```bash
./build/install_launchd.sh
```

This installs a LaunchAgent that starts the app when you log in.

## Developer Credentials

The build script is pre-configured with your Apple Developer credentials:
- Certificate: CD96C81E43F0FFA026939DC37BF69875A96FEF81
- Apple ID: soundguru@gmail.com
- Team ID: 22D9VBGAWF

## Troubleshooting

### Build fails with "target not found"
```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
```

### Code signing fails
Check that your certificate is installed:
```bash
security find-identity -p codesigning -v
```

### Notarization fails
Check the log:
```bash
xcrun notarytool log <submission-id> \
  --apple-id soundguru@gmail.com \
  --password ndtq-xhsn-wxyl-lzji \
  --team-id 22D9VBGAWF
```
