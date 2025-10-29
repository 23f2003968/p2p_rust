#!/bin/bash

# P2P Chat Tauri - Quick Run Script
# This script makes it easy to run the P2P Chat application

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY="$SCRIPT_DIR/src-tauri/target/release/p2p_rust"

echo "============================================"
echo "🚀 P2P Chat - Tauri + Rust + Vue.js"
echo "============================================"
echo ""

# Check if binary exists
if [ ! -f "$BINARY" ]; then
    echo "❌ Binary not found!"
    echo "Please build the application first:"
    echo "  cd $SCRIPT_DIR"
    echo "  npm run tauri build"
    exit 1
fi

# Check if binary is executable
if [ ! -x "$BINARY" ]; then
    echo "Making binary executable..."
    chmod +x "$BINARY"
fi

echo "✅ Binary found: $BINARY"
echo "📦 Size: $(du -h "$BINARY" | cut -f1)"
echo ""
echo "Starting application..."
echo "============================================"
echo ""

# Run the application
exec "$BINARY"
