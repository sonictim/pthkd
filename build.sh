#!/bin/bash
set -e

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
