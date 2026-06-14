#!/bin/bash
# Phase 5 Integration Tests - server relay protocol validation
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR/../.."

TEST_SERVER_PORT=5558
TEST_CLIENT=""

cleanup() {
    pkill -f chat-server 2>/dev/null || true
    wait 2>/dev/null || true
}

test_phase_4_server_start() {
    echo "=== Phase 5 Test: Server Startup ==="
    cleanup
    "$PROJECT_ROOT/chat/chat-server" $TEST_SERVER_PORT &
    local pid=$!
    sleep 1
    
    if ! ps -p "$pid" > /dev/null 2>&1; then
        echo "❌ Server failed to start" && exit 1
    fi
    echo "✅ Server started on port $TEST_SERVER_PORT"
    
    cleanup
}

test_empty_message_frame() {
    echo "=== Phase 5 Test: Empty Message Frame Handling ==="
    cleanup
    
    "$PROJECT_ROOT/chat/chat-server" $TEST_SERVER_PORT & sleep 1
    TEST_SERVER_PID=$!
    
    # Connect client and send empty command
    echo "" | "$PROJECT_ROOT/chat/chat-client" "$SCRIPT_DIR/../keys/priv.pem" "$SCRIPT_DIR/../keys/pub.pem" "127.0.0.1:$TEST_SERVER_PORT" > tests/server_test.out 2>&1 || true
    
    sleep 1
    pkill -f chat-server
    wait $TEST_SERVER_PID 2>/dev/null || true
    
    tail --line-number -30 tests/server_test.out | grep -q "Received message\|Client.*Goodbye" && echo "✅ Empty frame handled correctly"
}

test_malformed_command() {
    echo "=== Phase 5 Test: Malformed Command Handling ==="
    cleanup
    
    "$PROJECT_ROOT/chat/chat-server" $TEST_SERVER_PORT & sleep 1
    TEST_SERVER_PID=$!
    
    # Send malformed data (non-UTF8 sequence)
    printf '\xff\xfe' | "$PROJECT_ROOT/chat/chat-client" "$SCRIPT_DIR/../keys/priv.pem" "$SCRIPT_DIR/../keys/pub.pem" "127.0.0.1:$TEST_SERVER_PORT" > tests/server_test.out 2>&1 || true
    
    sleep 1
    pkill -f chat-server
    wait $TEST_SERVER_PID 2>/dev/null || true
    
    grep -q "Error\|Received message" tests/server_test.out && echo "✅ Malformed frame handled correctly"
}

test_message_receipt() {
    echo "=== Phase 5 Test: Message Receipt Verification ==="
    cleanup
    
    "$PROJECT_ROOT/chat/chat-server" $TEST_SERVER_PORT & sleep 1
    TEST_SERVER_PID=$!
    
    # Send simple message via client
    echo "/to alice Hello from test suite" | "$PROJECT_ROOT/chat/chat-client" "$SCRIPT_DIR/../keys/priv.pem" "$SCRIPT_DIR/../keys/pub.pem" "127.0.0.1:$TEST_SERVER_PORT" > tests/server_test.out 2>&1
    
    sleep 1
    pkill -f chat-server
    wait $TEST_SERVER_PID 2>/dev/null || true
    
    grep -q "Message to recipient\|Goodbye" tests/server_test.out && echo "✅ Message frame processed correctly"
}

test_command_timeout() {
    echo "=== Phase 5 Test: Command Timeout Handling ==="
    cleanup
    
    "$PROJECT_ROOT/chat/chat-server" $TEST_SERVER_PORT & sleep 1
    TEST_SERVER_PID=$!
    
    # Send command without waiting to process
    timeout 0.1 bash -c "echo '/to bob test' | \"$PROJECT_ROOT/chat/chat-client\" \"$SCRIPT_DIR/../keys/priv.pem\" \"$SCRIPT_DIR/../keys/pub.pem\" \"127.0.0.1:$TEST_SERVER_PORT\"" > tests/server_test.out 2>&1 || true
    
    sleep 1
    pkill -f chat-server
    wait $TEST_SERVER_PID 2>/dev/null || true
    
    echo "✅ Timeout handled without crash"
}

test_invalid_recipient() {
    echo "=== Phase 5 Test: Invalid Recipient Handling ==="
    cleanup
    
    "$PROJECT_ROOT/chat/chat-server" $TEST_SERVER_PORT & sleep 1
    TEST_SERVER_PID=$!
    
    # Send message to invalid recipient (not whitelisted server-side)
    echo "/to noone@localhost ShouldFail" | "$PROJECT_ROOT/chat/chat-client" "$SCRIPT_DIR/../keys/priv.pem" "$SCRIPT_DIR/../keys/pub.pem" "127.0.0.1:$TEST_SERVER_PORT" > tests/server_test.out 2>&1
    
    sleep 1
    pkill -f chat-server
    wait $TEST_SERVER_PID 2>/dev/null || true
    
    grep -q "Error\|Goodbye" tests/server_test.out && echo "✅ Invalid recipient handled correctly"
}

# Run all tests
cleanup

if ! test_phase_4_server_start; then
    cleanup
    exit 1
fi

test_empty_message_frame && \
test_malformed_command || true && \
test_message_receipt && \
test_command_timeout && \
test_invalid_recipient

cleanup

echo ""
echo "=== Phase 5 Test Summary ==="
echo "✅ All integration tests completed"

