# Rust IRC Daemon Makefile

.PHONY: help build test clean run config install

# Default target
help:
	@echo "Rust IRC Daemon - Available targets:"
	@echo "  build     - Build the project"
	@echo "  test      - Run tests"
	@echo "  clean     - Clean build artifacts"
	@echo "  run       - Run the daemon"
	@echo "  config    - Generate default configuration"
	@echo "  install   - Install the daemon"
	@echo "  check     - Check code without building"
	@echo "  fmt       - Format code"
	@echo "  clippy    - Run clippy linter"

# Build the project
build:
	cargo build --release

# Run tests
test:
	cargo test

# Clean build artifacts
clean:
	cargo clean

# Run the daemon
run:
	cargo run --release

# Generate default configuration
config:
	cargo run --release -- config

# Install the daemon
install: build
	cargo install --path .

# Check code without building
check:
	cargo check

# Format code
fmt:
	cargo fmt

# Run clippy linter
clippy:
	cargo clippy

# Run with specific configuration
run-config:
	cargo run --release -- --config config.toml

# Run in daemon mode
run-daemon:
	cargo run --release -- --daemon

# Test configuration
test-config:
	cargo run --release -- --test-config

# Show server info
info:
	cargo run --release -- info

# Show version
version:
	cargo run --release -- version
