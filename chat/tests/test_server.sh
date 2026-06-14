#!/bin/bash
# test_server.sh - Start the relay server for testing
set -e

PORT=${1:-5555}
KEYDIR="${2:-keys}"

echo "Starting ZeroMQ Relay Server..."
echo "  Port: $PORT"
echo "  Whitelist dir: $KEYDIR"

# Build if needed
mkdir -p build
cd build && cmake .. && make -j$(nproc) && cd ..

./chat-server "$PORT" "$KEYDIR"
