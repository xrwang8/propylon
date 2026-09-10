BINARY_NAME=propylon
CMD_DIR=./cmd/propylon
BUILD_DIR=./bin

.PHONY: all build run test clean lint help

all: build

build:
	@echo "==> Building $(BINARY_NAME)..."
	@mkdir -p $(BUILD_DIR)
	go build -ldflags="-s -w" -o $(BUILD_DIR)/$(BINARY_NAME) $(CMD_DIR)/main.go
	@echo "==> Build complete: $(BUILD_DIR)/$(BINARY_NAME)"

run: build
	@echo "==> Running $(BINARY_NAME)..."
	$(BUILD_DIR)/$(BINARY_NAME) --config ./configs/propylon.example.yaml

test:
	@echo "==> Running tests..."
	go test -v ./...

clean:
	@echo "==> Cleaning up..."
	rm -rf $(BUILD_DIR)

help:
	@echo "Usage: make [target]"
	@echo "  build   Build binary"
	@echo "  run     Build and run with example config"
	@echo "  test    Run unit tests"
	@echo "  clean   Remove build artifacts"
