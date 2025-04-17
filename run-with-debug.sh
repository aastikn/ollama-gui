#!/bin/bash

# Run Ollama Interface with full debug logging
export RUST_LOG=debug,tower_http=debug
echo "Running Ollama interface with debug logging enabled"
echo "Log level: $RUST_LOG"

# Build first if requested
if [ "$1" == "--build" ]; then
  echo "Building Ollama interface..."
  cargo build
fi

# Run the binary
cargo run
