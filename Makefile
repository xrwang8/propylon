BINARY_NAME=propylon
TARGET_DIR=./target/release

.PHONY: all build run test clean check help

all: build

build:
	@echo "==> Building $(BINARY_NAME) (Rust Release)..."
	cargo build --release
	@echo "==> Binary ready: $(TARGET_DIR)/$(BINARY_NAME)"

run:
	@echo "==> Running $(BINARY_NAME) with example configuration..."
	cargo run -- --config configs/propylon.example.yaml

test:
	@echo "==> Running tests..."
	cargo test

check:
	@echo "==> Checking compilation..."
	cargo check

clean:
	@echo "==> Cleaning build artifacts..."
	cargo clean

help:
	@echo "Usage: make [target]"
	@echo "  build   Compile optimized release binary"
	@echo "  run     Run gateway with example configuration"
	@echo "  test    Execute unit & integration tests"
	@echo "  check   Fast type and borrow check"
	@echo "  clean   Remove cargo target directory"
