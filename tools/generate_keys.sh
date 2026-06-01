#!/bin/bash
# ZeroMQ Tunnel Key Generation Script
# Generates REAL Ed25519 CURVE keypairs using OpenSSL

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=== ZeroMQ Tunnel Key Generator ==="
echo "Using OpenSSL to generate real Ed25519 CURVE keypairs"
echo ""

# Default directories (keys are stored next to this script)
SERVER_KEY_DIR="${SERVER_KEY_DIR:-${SCRIPT_DIR}/../tunnel-server/config}"
AGENT_KEY_DIR="${AGENT_KEY_DIR:-${SCRIPT_DIR}/../tunnel-agent/config}"

mkdir -p "$SERVER_KEY_DIR" 2>/dev/null || true
mkdir -p "$AGENT_KEY_DIR" 2>/dev/null || true

# Check if already generated (real Ed25519 keys)
if [ -f "${SERVER_KEY_DIR}/server.pem" ] && grep -q "BEGIN PRIVATE KEY" "${SERVER_KEY_DIR}/server.pem" 2>/dev/null; then
    echo "Server key already exists at ${SERVER_KEY_DIR}/server.pem"
fi

if [ -f "${AGENT_KEY_DIR}/agent.pem" ] && grep -q "BEGIN PRIVATE KEY" "${AGENT_KEY_DIR}/agent.pem" 2>/dev/null; then
    echo "Agent key already exists at ${AGENT_KEY_DIR}/agent.pem"
fi

# Generate server key if not exists (or placeholder)
if [ ! -f "${SERVER_KEY_DIR}/server.pem" ]; then
    echo ""
    echo "Generating server CURVE Ed25519 keypair..."
    openssl genpkey -algorithm Ed25519 -out "${SERVER_KEY_DIR}/server.pem" 2>/dev/null
    echo "Server key generated at ${SERVER_KEY_DIR}/server.pem"
else
    # Check if it's a placeholder (random hex) and regenerate
    if ! grep -q "BEGIN PRIVATE KEY\|PUBLIC KEY" "${SERVER_KEY_DIR}/server.pem" 2>/dev/null; then
        echo "Warning: Server key is a placeholder. Regenerating with real Ed25519 key..."
        openssl genpkey -algorithm Ed25519 -out "${SERVER_KEY_DIR}/server.pem" 2>/dev/null
        echo "Server key regenerated at ${SERVER_KEY_DIR}/server.pem"
    fi
fi

# Generate agent key if not exists (or placeholder)
if [ ! -f "${AGENT_KEY_DIR}/agent.pem" ]; then
    echo ""
    echo "Generating agent CURVE Ed25519 keypair..."
    openssl genpkey -algorithm Ed25519 -out "${AGENT_KEY_DIR}/agent.pem" 2>/dev/null
    echo "Agent key generated at ${AGENT_KEY_DIR}/agent.pem"
else
    # Check if it's a placeholder (random hex) and regenerate
    if ! grep -q "BEGIN PRIVATE KEY\|PUBLIC KEY" "${AGENT_KEY_DIR}/agent.pem" 2>/dev/null; then
        echo "Warning: Agent key is a placeholder. Regenerating with real Ed25519 key..."
        openssl genpkey -algorithm Ed25519 -out "${AGENT_KEY_DIR}/agent.pem" 2>/dev/null
        echo "Agent key regenerated at ${AGENT_KEY_DIR}/agent.pem"
    fi
fi

echo ""
echo "=== Keys generated successfully ==="
echo ""
echo "Server key:  ${SERVER_KEY_DIR}/server.pem"
echo "Agent key:   ${AGENT_KEY_DIR}/agent.pem"
echo ""
echo "Both keys are REAL Ed25519 CURVE keypairs compatible with ed25519-dalek."
echo ""
echo "Key format:"
echo "  PRIVATE KEY (PEM) - used for signing authentication messages"
echo ""
echo "To use with server:"
echo "  cargo run --bin tunnel-server config.toml \\"
echo "    --key-file ${SERVER_KEY_DIR}/server.pem"
echo ""
echo "To use with agent:"
echo "  cargo run --bin tunnel-agent --remote -s myservice 8080 \\"
echo "    --key-file ${AGENT_KEY_DIR}/agent.pem"
echo ""
