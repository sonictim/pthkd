#!/bin/bash
#
# Creates a macOS .app bundle for the ProTools Hotkey Daemon
#
# Usage: ./scripts/create_bundle.sh
#
# The bundle will be created at: target/release/pthkd.app
#

set -e  # Exit on error

BUNDLE_NAME="pthkd.app"
BUNDLE_DIR="target/release/$BUNDLE_NAME"
BINARY="target/release/pthkd"

echo "🔨 Building ProTools Hotkey Daemon..."
cargo build --release

echo ""
echo "📦 Creating .app bundle at $BUNDLE_DIR..."

# Remove old bundle if it exists
rm -rf "$BUNDLE_DIR"

# Create bundle structure
mkdir -p "$BUNDLE_DIR/Contents/MacOS"
mkdir -p "$BUNDLE_DIR/Contents/Resources"

echo "   ✓ Created bundle directories"

# Copy binary
cp "$BINARY" "$BUNDLE_DIR/Contents/MacOS/pthkd"
chmod +x "$BUNDLE_DIR/Contents/MacOS/pthkd"

echo "   ✓ Copied binary"

# Generate Info.plist
cat > "$BUNDLE_DIR/Contents/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>pthkd</string>
    <key>CFBundleIdentifier</key>
    <string>com.feralfrequencies.pthkd</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>ProTools Hotkey Daemon</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.15</string>
    <key>LSUIElement</key>
    <true/>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

echo "   ✓ Generated Info.plist"

# Copy config.toml to bundle (so it can be found when run from bundle)
if [ -f "config.toml" ]; then
    cp "config.toml" "$BUNDLE_DIR/Contents/MacOS/config.toml"
    echo "   ✓ Copied config.toml"
fi

echo ""
echo "✅ Bundle created successfully!"
echo ""
echo "📍 Location: $BUNDLE_DIR"
echo ""
echo "To test:"
echo "   open $BUNDLE_DIR"
echo ""
echo "To install:"
echo "   cp -r $BUNDLE_DIR /Applications/"
echo ""
