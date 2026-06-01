#!/bin/bash
# ZeroMQ Tunnel Key Generation Script
# Generates CURVE keypairs for server and agent
# Keys are stored in the tools/ directory by default

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=== ZeroMQ Tunnel Key Generator ==="
echo ""

# Use tools/ as default directory (relative to script location)
# User can override with SERVER_KEY_DIR and AGENT_KEY_DIR environment variables
SERVER_KEY_DIR="${SERVER_KEY_DIR:-${SCRIPT_DIR}/../tunnel-server/config}"
AGENT_KEY_DIR="${AGENT_KEY_DIR:-${SCRIPT_DIR}/../tunnel-agent/config}"

mkdir -p "$SERVER_KEY_DIR" 2>/dev/null || true
mkdir -p "$AGENT_KEY_DIR" 2>/dev/null || true

# Check if already generated
if [ -f "${SERVER_KEY_DIR}/server.pem" ]; then
    echo "Server key already exists at ${SERVER_KEY_DIR}/server.pem"
fi

if [ -f "${AGENT_KEY_DIR}/agent.pem" ]; then
    echo "Agent key already exists at ${AGENT_KEY_DIR}/agent.pem"
fi

# Generate server key if not exists
if [ ! -f "${SERVER_KEY_DIR}/server.pem" ]; then
    echo ""
    echo "Generating server CURVE keypair..."

    # Generate 64 hex characters (32 bytes for x25519 private key)
    local_hex_key=$(head -c 32 /dev/urandom | od -An -t x1 | tr -d ' \n')

    cat > "${SERVER_KEY_DIR}/server.pem" << EOF
-----BEGIN CURVE KEYPAIR-----
${local_hex_key}
-----END CURVE KEYPAIR-----
EOF
    echo "Server key generated at ${SERVER_KEY_DIR}/server.pem"
fi

# Generate agent key if not exists
if [ ! -f "${AGENT_KEY_DIR}/agent.pem" ]; then
    echo ""
    echo "Generating agent CURVE keypair..."

    # Generate 64 hex characters (32 bytes for x25519 private key)
    local_hex_key=$(head -c 32 /dev/urandom | od -An -t x1 | tr -d ' \n')

    cat > "${AGENT_KEY_DIR}/agent.pem" << EOF
-----BEGIN CURVE KEYPAIR-----
${local_hex_key}
-----END CURVE KEYPAIR-----
EOF
    echo "Agent key generated at ${AGENT_KEY_DIR}/agent.pem"
fi

echo ""
echo "=== Keys generated successfully ==="
echo ""
echo "Server key:  ${SERVER_KEY_DIR}/server.pem"
echo "Agent key:   ${AGENT_KEY_DIR}/agent.pem"
echo ""
echo "Location: Both keys are in tools/ subdirectory of their respective apps"
echo ""
echo "To use with server:"
echo "  cargo run --bin tunnel-server /path/to/config/server.toml \\"
echo "    --key-file ${SERVER_KEY_DIR}/server.pem"
echo ""
echo "To use with agent:"
echo "  cargo run --bin tunnel-agent --remote -s myservice 8080 \\"
echo "    --key-file ${AGENT_KEY_DIR}/agent.pem"
echo ""
