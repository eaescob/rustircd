#!/bin/bash

# RustIRCd Development Setup Script
# Run this on a new machine to set up the development environment

set -e

echo "🚀 Setting up RustIRCd development environment..."

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust not found. Please install Rust first:"
    echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo "✅ Rust found: $(rustc --version)"

# Check Rust version
RUST_VERSION=$(rustc --version | cut -d' ' -f2)
echo "📦 Rust version: $RUST_VERSION"

# Install required components
echo "🔧 Installing Rust components..."
rustup component add clippy rustfmt

# Check dependencies
echo "📋 Checking project dependencies..."
cargo check

# Build the project
echo "🏗️  Building project..."
cargo build

# Run tests
echo "🧪 Running tests..."
cargo test

# Create example config if it doesn't exist
if [ ! -f "config.toml" ]; then
    echo "📝 Creating example configuration..."
    cp config_example.toml config.toml
    echo "✅ Created config.toml from example"
fi

echo ""
echo "🎉 Setup complete! You can now:"
echo "   • Run the daemon: cargo run"
echo "   • Run with debug: RUST_LOG=debug cargo run"
echo "   • Run tests: cargo test"
echo "   • Format code: cargo fmt"
echo "   • Check code: cargo clippy"
echo ""
echo "📖 See DEVELOPMENT.md for more details"
