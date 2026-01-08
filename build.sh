#!/bin/bash
set -e

# Check if Swift is installed, run setup if needed
if ! command -v swift &> /dev/null; then
    echo "❌ Swift not found!"
    echo ""
    echo "Running Swift setup..."
    cd swift
    ./setup-swift.sh
    cd ..
    exit 0
fi

echo "🔨 Building Swift UI library..."
cd swift
swift build -c release
cd ..

echo "📦 Copying Swift library to target/release..."
mkdir -p target/release
cp swift/.build/x86_64-apple-macosx/release/libPTHKDui.dylib target/release/

echo "🦀 Building Rust daemon..."
cargo build --release

echo "✅ Build complete!"
echo ""
echo "To run the daemon:"
echo "  ./target/release/pthkd"
echo ""
echo "Or run with this script:"
echo "  ./build.sh --run"

# Check if --run flag was passed
if [[ "$1" == "--run" ]]; then
    echo ""
    echo "🚀 Starting daemon..."
    ./target/release/pthkd
fi
