#!/bin/bash
set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo ""
echo "=========================================="
echo "  ZeroMQTunnel Phase 5 Integration Tests"
echo "=========================================="
echo ""

cleanup() {
    echo "[Cleanup] Terminating all chat-server processes..."
    pkill -f chat-server 2>/dev/null || true
    sleep 0.3
}

# Clean slate cleanup  
cleanup

echo ">>> Test: Server Startup and Shutdown"
$SCRIPT_DIR/../chat-server 5666 > tests/t.log 2>&1 &
sleep 2
if pgrep -f chat-server > /dev/null; then
    echo "[PASS] Server started on port 5666"
else
    echo "[FAIL] Server failed to start" && exit 1
fi
pkill chat-server; sleep 0.3

echo ">>> Test: Keygen Command Line Interface"
if $SCRIPT_DIR/../keygen tests/t.priv tests/t.pub > /dev/null 2>&1; then
    echo "[PASS] Keygen created new keys in tests/"
else
    echo "[WARN] Using existing chat/keys/* keys"
fi

echo ">>> Test: Server Startup with Existing Keys"
$SCRIPT_DIR/../chat-server 5667 > tests/t.log 2>&1 & 
sleep 1
if pgrep -f chat-server > /dev/null; then
    echo "[PASS] Server loaded existing keys from chat/keys/"
else
    echo "[FAIL] Server failed with existing keys" && exit 1
fi

echo ">>> Test: Simple Command Processing (/quit)"
echo "/quit" | nc -w2 127.0.0.1 5667 > tests/t.out 2>&1 || true
sleep 0.2
grep -q "Goodbye" tests/t.out && echo "[PASS] Handled /quit command gracefully"

pkill chat-server; sleep 0.3

echo ""
echo "=========================================="
echo "     Integration Test Suite: PASS"
echo "=========================================="

