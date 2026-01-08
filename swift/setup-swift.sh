#!/bin/bash
#
# Swift Setup Script for pthkd
# Ensures Swift is installed and at the correct version
#

set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🍎 Swift Setup for pthkd"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Required Swift version (5.x)
REQUIRED_MAJOR=5

# Check if Swift is installed
echo "Checking for Swift installation..."
if ! command -v swift &> /dev/null; then
    echo "❌ Swift not found"
    echo ""
    echo "Swift is included with Xcode Command Line Tools."
    echo "Installing Xcode Command Line Tools..."
    echo ""

    # Install Xcode Command Line Tools
    xcode-select --install

    echo ""
    echo "⏳ Please complete the Xcode Command Line Tools installation in the popup window."
    echo "   Once installed, run this script again."
    exit 1
fi

# Get Swift version
SWIFT_VERSION=$(swift --version 2>&1 | head -1)
echo "✅ Swift found: $SWIFT_VERSION"

# Extract major version
SWIFT_MAJOR=$(swift --version 2>&1 | grep -oE 'Swift version [0-9]+' | grep -oE '[0-9]+' | head -1)

if [ -z "$SWIFT_MAJOR" ]; then
    echo "⚠️  Could not determine Swift version"
    echo "   Attempting to continue..."
else
    echo "   Swift major version: $SWIFT_MAJOR"

    if [ "$SWIFT_MAJOR" -lt "$REQUIRED_MAJOR" ]; then
        echo ""
        echo "❌ Swift version $SWIFT_MAJOR is too old (need $REQUIRED_MAJOR or higher)"
        echo ""
        echo "To update Swift, update Xcode Command Line Tools:"
        echo "  softwareupdate --list"
        echo "  softwareupdate --install --all"
        echo ""
        echo "Or install the latest Xcode from the App Store."
        exit 1
    fi
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Swift setup complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "You can now build the Swift library:"
echo "  cd swift && swift build -c release"
echo ""
echo "Or run the full build:"
echo "  cd .. && ./build.sh"
echo ""
