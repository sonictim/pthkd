#!/bin/bash
#
# Installs the ProTools Hotkey Daemon as a launchd agent
# This will make it auto-start on login and auto-restart on crash
#
# Usage: ./scripts/install_launchd.sh
#
# Prerequisites:
# - The app bundle must be installed in /Applications/pthkd.app
#   Run: cp -r target/release/pthkd.app /Applications/
#

set -e

PLIST_NAME="com.feralfrequencies.pthkd.plist"
PLIST_SRC="scripts/$PLIST_NAME"
PLIST_DEST="$HOME/Library/LaunchAgents/$PLIST_NAME"

# Check if app bundle exists
if [ ! -d "/Applications/pthkd.app" ]; then
    echo "❌ Error: /Applications/pthkd.app not found"
    echo ""
    echo "Please install the app first:"
    echo "   ./scripts/create_bundle.sh"
    echo "   cp -r target/release/pthkd.app /Applications/"
    echo ""
    exit 1
fi

echo "📦 Installing launchd agent..."

# Create LaunchAgents directory if it doesn't exist
mkdir -p "$HOME/Library/LaunchAgents"

# Copy plist
cp "$PLIST_SRC" "$PLIST_DEST"
echo "   ✓ Copied plist to $PLIST_DEST"

# Unload if already loaded (ignore errors if not loaded)
launchctl unload "$PLIST_DEST" 2>/dev/null || true

# Load the agent
launchctl load "$PLIST_DEST"
echo "   ✓ Loaded launchd agent"

echo ""
echo "✅ Installation complete!"
echo ""
echo "The hotkey daemon will now:"
echo "   • Start automatically on login"
echo "   • Restart automatically if it crashes"
echo ""
echo "To check status:"
echo "   launchctl list | grep pthkd"
echo ""
echo "To view logs:"
echo "   tail -f /tmp/pthkd.stdout.log"
echo "   tail -f /tmp/pthkd.stderr.log"
echo ""
echo "To uninstall:"
echo "   launchctl unload $PLIST_DEST"
echo "   rm $PLIST_DEST"
echo ""
