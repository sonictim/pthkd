#!/bin/bash
#
# Build script for ProTools Hotkey Daemon - Universal macOS Bundle with Swift UI
#
# Creates a signed, notarized universal binary .app bundle
#

set -e  # Exit on any error

# Get project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

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

# Build Swift library for ARM64
echo ""
echo "🍎 Building Swift library for ARM64 (Apple Silicon)..."
cd swift
swift build -c release --arch arm64
cd ..
echo "   ✓ Swift library built for ARM64"

# Build Swift library for x86_64
echo ""
echo "🖥️  Building Swift library for x86_64 (Intel)..."
cd swift
swift build -c release --arch x86_64
cd ..
echo "   ✓ Swift library built for x86_64"

# Create universal Swift dylib
echo ""
echo "🔗 Creating universal Swift library..."
mkdir -p "$BUILD_DIR"
lipo -create -output "$BUILD_DIR/libPTHKDui.dylib" \
    "swift/.build/arm64-apple-macosx/release/libPTHKDui.dylib" \
    "swift/.build/x86_64-apple-macosx/release/libPTHKDui.dylib"
echo "   ✓ Universal Swift library created"
lipo -info "$BUILD_DIR/libPTHKDui.dylib"

# Copy Swift dylib to Rust target dirs for linking
echo ""
echo "📋 Copying Swift library to Rust target directories..."
mkdir -p target/aarch64-apple-darwin/release
mkdir -p target/x86_64-apple-darwin/release
cp swift/.build/arm64-apple-macosx/release/libPTHKDui.dylib target/aarch64-apple-darwin/release/
cp swift/.build/x86_64-apple-macosx/release/libPTHKDui.dylib target/x86_64-apple-darwin/release/

# Build Rust for ARM64
echo ""
echo "🍎 Building Rust binary for ARM64 (Apple Silicon)..."
cargo build --release --target aarch64-apple-darwin

# Build Rust for x86_64
echo ""
echo "🖥️  Building Rust binary for x86_64 (Intel)..."
cargo build --release --target x86_64-apple-darwin

# Create universal Rust binary
echo ""
echo "🔗 Creating universal Rust binary..."
lipo -create -output "$BUILD_DIR/$BINARY_NAME" \
    "target/aarch64-apple-darwin/release/$BINARY_NAME" \
    "target/x86_64-apple-darwin/release/$BINARY_NAME"

# Verify the binary
echo "   ✓ Universal Rust binary created"
lipo -info "$BUILD_DIR/$BINARY_NAME"

# Create .app bundle structure
echo ""
echo "📦 Creating .app bundle..."
rm -rf "$APP_PATH"
mkdir -p "$APP_PATH/Contents/MacOS"
mkdir -p "$APP_PATH/Contents/Frameworks"
mkdir -p "$APP_PATH/Contents/Resources"

# Copy binary
cp "$BUILD_DIR/$BINARY_NAME" "$APP_PATH/Contents/MacOS/$BINARY_NAME"
chmod +x "$APP_PATH/Contents/MacOS/$BINARY_NAME"
echo "   ✓ Copied Rust binary"

# Copy Swift dylib to Frameworks
cp "$BUILD_DIR/libPTHKDui.dylib" "$APP_PATH/Contents/Frameworks/libPTHKDui.dylib"
echo "   ✓ Copied Swift library"

# Update the binary's rpath to look in @executable_path/../Frameworks
echo ""
echo "🔗 Updating library load paths..."
install_name_tool -change "@rpath/libPTHKDui.dylib" "@executable_path/../Frameworks/libPTHKDui.dylib" "$APP_PATH/Contents/MacOS/$BINARY_NAME"
echo "   ✓ Updated binary rpath"

# Update Swift dylib's id
install_name_tool -id "@executable_path/../Frameworks/libPTHKDui.dylib" "$APP_PATH/Contents/Frameworks/libPTHKDui.dylib"
echo "   ✓ Updated Swift library id"

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

# Sign the Swift library first
codesign --sign $CODESIGN_CERTIFICATE_ID \
    --force --options runtime \
    "$APP_PATH/Contents/Frameworks/libPTHKDui.dylib"
echo "   ✓ Signed Swift library"

# Sign the main binary
codesign --sign $CODESIGN_CERTIFICATE_ID \
    --force --options runtime \
    "$APP_PATH/Contents/MacOS/$BINARY_NAME"
echo "   ✓ Signed main binary"

# Sign the entire app bundle
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
fi

# Copy new version
echo "   Copying to /Applications..."
cp -r "$APP_PATH" /Applications/
# cp -r target/universal/release/pthkd.app /Applications/
echo "   ✓ Installed to $INSTALL_PATH"

# Create DMG for distribution
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "💿 Creating DMG for distribution..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

DMG_NAME="pthkd-v$VERSION.dmg"
DMG_OUTPUT_DIR="$HOME/dev/www/feralfreq.com/pthkd/download"
DMG_PATH="$DMG_OUTPUT_DIR/$DMG_NAME"

# Create output directory if it doesn't exist
mkdir -p "$DMG_OUTPUT_DIR"

# Create temporary DMG staging directory
DMG_TEMP_DIR=$(mktemp -d)
cp -r "$APP_PATH" "$DMG_TEMP_DIR/"

echo "   Creating disk image..."
# Create the DMG
hdiutil create -volname "$APP_NAME v$VERSION" \
    -srcfolder "$DMG_TEMP_DIR" \
    -ov -format UDZO \
    "$DMG_PATH"

# Clean up temporary directory
rm -rf "$DMG_TEMP_DIR"

echo "   ✓ DMG created: $DMG_NAME"
echo "   📁 Location: $DMG_PATH"
echo "   💾 Size: $(du -sh "$DMG_PATH" | cut -f1)"

# Increment version for next build
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📈 Incrementing version for next build..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Parse current version
IFS='.' read -r MAJOR MINOR PATCH <<< "$VERSION"

# Increment patch version
NEW_PATCH=$((PATCH + 1))
NEW_VERSION="$MAJOR.$MINOR.$NEW_PATCH"

# Update Cargo.toml
sed -i '' "s/^version = \"$VERSION\"/version = \"$NEW_VERSION\"/" Cargo.toml

echo "   Version updated: $VERSION → $NEW_VERSION"
echo "   Next build will be v$NEW_VERSION"
echo ""
# Git commit and push
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📝 Committing build to git..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Check if there are any changes to commit
if ! git diff-index --quiet HEAD --; then
    echo "   Changes detected, creating commit..."
    git add -A
    git commit -m "$(cat <<EOF
Release v$VERSION build

- Built and signed universal binary
- Notarized and stapled
- Created DMG for distribution
- Version updated: $VERSION → $NEW_VERSION

EOF
)"

    echo "   ✓ Committed changes"

    # Push to remote
    echo "   Pushing to remote..."
    if git push; then
        echo "   ✓ Pushed to remote"
    else
        echo "   ⚠ Failed to push to remote (continuing anyway)"
    fi
else
    echo "   No changes to commit"
fi

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
echo "💿 DMG: ✅"
echo "📈 Version Up ✅"
echo ""
echo "To launch:"
echo "   open /Applications/$BUNDLE_NAME"
echo ""

