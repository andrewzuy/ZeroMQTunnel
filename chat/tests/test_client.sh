#!/bin/bash
# test_client.sh - Start chat client with keys for testing
set -e

PUBLIC_KEY="${1:-keys/client_A.pub}"
PRIVATE_KEY="${2:-keys/client_A.pem}"
SERVER_ADDR="${3:-tcp://localhost:5555}"

echo "Starting Chat Client..."
echo "  Public key: $PUBLIC_KEY"
echo "  Private key: $PRIVATE_KEY"
echo "  Server: $SERVER_ADDR"

./chat-client "$PUBLIC_KEY" "$PRIVATE_KEY" "$SERVER_ADDR"
