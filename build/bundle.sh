#!/bin/bash
#
# Build script for ProTools Hotkey Daemon - Universal macOS Bundle
#
# Creates a signed, notarized universal binary .app bundle
#

set -e  # Exit on any error

# Change to project root (one level up from build directory)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

# Configuration
BINARY_NAME="pthkd"
APP_NAME="ProTools Hotkey Daemon"
BUNDLE_NAME="pthkd.app"
VERSION=$(awk '/\[package\]/ {flag=1} flag && /^version =/ {print $3; exit}' Cargo.toml | tr -d '"')
BUILD_DIR="target/universal/release"
APP_PATH="$BUILD_DIR/$BUNDLE_NAME"

# Your Apple Developer credentials
CODESIGN_CERTIFICATE_ID="CD96C81E43F0FFA026939DC37BF69875A96FEF81"
NOTARIZE_USERNAME="soundguru@gmail.com"
NOTARIZE_PASSWORD="ndtq-xhsn-wxyl-lzji"
NOTARIZE_TEAM_ID="22D9VBGAWF"
BUNDLE_IDENTIFIER="com.feralfrequencies.pthkd"

# Cleanup on exit
clean_up() {
    echo "Cleaning up temporary files..."
    [ -n "$TEMP_DIR" ] && {
        rm -rf "$TEMP_DIR"
    }
}
trap clean_up EXIT

export MACOSX_DEPLOYMENT_TARGET=12.0

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔨 Building $APP_NAME v$VERSION"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Ensure targets are installed
echo "📦 Ensuring Rust targets are installed..."
rustup target add aarch64-apple-darwin x86_64-apple-darwin

# Build for ARM64
echo ""
echo "🍎 Building for ARM64 (Apple Silicon)..."
cargo build --release --target aarch64-apple-darwin

# Build for x86_64
echo ""
echo "🖥️  Building for x86_64 (Intel)..."
cargo build --release --target x86_64-apple-darwin

# Create universal build directory
echo ""
echo "🔗 Creating universal binary..."
mkdir -p "$BUILD_DIR"

# Create universal binary with lipo
lipo -create -output "$BUILD_DIR/$BINARY_NAME" \
    "target/aarch64-apple-darwin/release/$BINARY_NAME" \
    "target/x86_64-apple-darwin/release/$BINARY_NAME"

# Verify the binary
echo "   ✓ Universal binary created"
lipo -info "$BUILD_DIR/$BINARY_NAME"

# Create .app bundle structure
echo ""
echo "📦 Creating .app bundle..."
rm -rf "$APP_PATH"
mkdir -p "$APP_PATH/Contents/MacOS"
mkdir -p "$APP_PATH/Contents/Resources"

# Copy binary
cp "$BUILD_DIR/$BINARY_NAME" "$APP_PATH/Contents/MacOS/$BINARY_NAME"
chmod +x "$APP_PATH/Contents/MacOS/$BINARY_NAME"
echo "   ✓ Copied binary"

# Copy icon if it exists
if [ -f "build/icon.icns" ]; then
    cp "build/icon.icns" "$APP_PATH/Contents/Resources/AppIcon.icns"
    echo "   ✓ Copied icon"
else
    echo "   ⚠ No icon found at build/icon.icns (optional)"
fi


# Generate Info.plist
cat > "$APP_PATH/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>$BINARY_NAME</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleIdentifier</key>
    <string>$BUNDLE_IDENTIFIER</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>$APP_NAME</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>CFBundleVersion</key>
    <string>$VERSION</string>
    <key>LSMinimumSystemVersion</key>
    <string>12.0</string>
    <key>LSUIElement</key>
    <true/>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

echo "   ✓ Generated Info.plist"

# Code signing
echo ""
echo "✍️  Code signing application..."
codesign --sign $CODESIGN_CERTIFICATE_ID \
    --deep --force --options runtime \
    --entitlements "build/entitlements.plist" \
    "$APP_PATH"

# Verify code signing
echo "   ✓ Code signed"
echo ""
echo "🔍 Verifying code signature..."
codesign --verify --deep --strict --verbose=2 "$APP_PATH"
echo "   ✓ Signature verified"

# Show signature details
echo ""
echo "📋 Signature details:"
codesign -dv "$APP_PATH" 2>&1 | grep -E "Authority|TeamIdentifier|Signed Time"

# Notarization
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📮 Starting notarization process..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Create temporary directory for notarization
ORIGINAL_DIR=$(pwd)
TEMP_DIR=$(mktemp -d)
ZIP_NAME="$BINARY_NAME.v$VERSION.zip"

echo ""
echo "Creating zip file for notarization..."
ditto -c -k --keepParent "$APP_PATH" "$TEMP_DIR/$ZIP_NAME"
cd "$TEMP_DIR"

# Submit for notarization
echo ""
echo "Submitting to Apple for notarization..."
echo "(This may take a few minutes...)"
echo ""

NOTARIZE_RESPONSE=$(xcrun notarytool submit "$ZIP_NAME" \
    --wait \
    --apple-id "$NOTARIZE_USERNAME" \
    --password "$NOTARIZE_PASSWORD" \
    --team-id "$NOTARIZE_TEAM_ID" | tee /dev/tty)

# Check notarization status
if ! echo "$NOTARIZE_RESPONSE" | grep -q "status: Accepted"; then
    echo ""
    echo "❌ Notarization failed:"
    echo "$NOTARIZE_RESPONSE"
    exit 1
fi

echo ""
echo "✅ Notarization successful!"

cd "$ORIGINAL_DIR"

# Staple the notarization ticket
echo ""
echo "📌 Stapling notarization ticket..."
xcrun stapler staple "$APP_PATH"

# Verify stapling
if ! xcrun stapler validate "$APP_PATH"; then
    echo "❌ Stapling verification failed"
    exit 1
fi

echo "   ✓ Stapling verified"

# Install to Applications
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📥 Installing to /Applications..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

INSTALL_PATH="/Applications/$BUNDLE_NAME"

# Kill running process if it exists
if pgrep -x "$BINARY_NAME" > /dev/null; then
    echo "   Stopping running instance..."
    killall "$BINARY_NAME" 2>/dev/null || true
    sleep 1
fi

# Remove old version if it exists
if [ -d "$INSTALL_PATH" ]; then
    echo "   Removing old version..."
    rm -rf "$INSTALL_PATH"
    rm -rf "~/Library/Application Support/pthkd"
fi

# Copy new version
echo "   Copying to /Applications..."
cp -r "$APP_PATH" /Applications/

echo "   ✓ Installed to $INSTALL_PATH"

# Final summary
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Build complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "📍 Location: $INSTALL_PATH"
echo "📊 Size: $(du -sh "$INSTALL_PATH" | cut -f1)"
echo "🔐 Signed: ✅"
echo "📮 Notarized: ✅"
echo "📌 Stapled: ✅"
echo "💾 Installed: ✅"
echo ""
echo "To launch:"
echo "   open /Applications/$BUNDLE_NAME"
echo ""
