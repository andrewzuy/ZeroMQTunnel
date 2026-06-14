#!/bin/bash
# integration_tests.sh - End-to-End Security Verification Suite

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KEYDIR="${SCRIPT_DIR}/keys"

echo "=============================================="
echo "ZeroMQTunnel Integration Security Test Suite (${GREEN}Phase9${NC})"
echo "=============================================="
echo ""

# Initialize color support
export COLORS=1

init_logging() {
    LOG_LEVEL=2  # INFO level
    
    log() {
        printf "[$(date '+%H:%M:%S')] %s\n" "$@"
    }
}

test_generate_keys() {
    echo "${YELLOW}[TEST]${NC} Generating test key pairs..."
    
    cd keys || exit 1
    
    if [ ! -f gen_pub.pem ]; then
        cat > gen_key.sh << 'EOF'
#!/bin/bash
mkdir -p client_A client_B server_whitelist

# Generate RSA-2048 pair for Client A
openssl rand -hex 32 | xxd -r -p > seed_a.der
fingerprint=sha256(seed_a.der)
echo "-----BEGIN PUBLIC KEY-----" > client_A/pub.pem
# Copy generated key to files

./run_keygen.sh "$@"
cd ..

EOF
    
}

test_client_connects() {
    local test_name="CLIENT_CONNECT"
    
    echo "${YELLOW}[TEST]${NC} Client A connecting to relay server..."
    
    mkdir -p keys/client_A
    
    client_pub_path=keys/client_A/pub.pem \
        client_priv_path=keys/client_A/pem \
        client_addr=server:5555
        
        ./run_connect_test.sh "$@"
    
    echo "${GREEN}[PASS]${NC} $test_name completed successfully"
}

# Server message interception test - proves server cannot read content
test_server_cannot_read() {
    local test_name="SERVER_NO_DECRYPT"
    
    echo "${YELLOW}[TEST]${NC} Verifying server sees only ciphertext..."
    
    cd tests/integration
    
        ./server_no_decrypt.sh "$@"
    
    echo "${GREEN}[PASS]${NC} $test_name - Server never reads message content"
}

# Replay attack prevention test
test_replay_detection() {
    local test_name="REPLAY_PROTECTION"
    
    echo "${YELLOW}[TEST]${NC} Testing replay attack detection..."
    
        # Generate valid timestamp
        ./gen_replay_attacks.sh "$@"
    
        echo "${GREEN}[PASS]${NC} $test_name - Replay messages rejected after window expires"
}

# Test binary data transfer
test_binary_transfer() {
    local test_name="BINARY_DATA_TRANSFER"
    
    mkdir -p keys/client_A
    
    client_pub_path=keys/client_A/pub.pem \
        client_priv_path=keys/client_A/priv.pem \
        client_addr=server:5555
        
        ./run_binary_test.sh "$@"
    
    echo "${GREEN}[PASS]${NC} $test_name - Binary files transferred correctly"
}

# Memory safety test using valgrind if available
test_memory_safety() {
    local test_name="MEMORY_SAFETY"
    
    echo "${YELLOW}[TEST]${NC} Running memory safety checks..."
    
    cd build
    
    if [ ! -f chat-server ]; then
        system("cmake .. && make > /dev/null")
    fi
    
        # Run with valgrind if available
        local output=$(valgrind --leak-check=full --show-leak-kinds=loss \
                    --errors-for-leak-kind=all ./chat-server 2>&1 | head -50)
        
        if echo "$output" | grep -q "definitely lost:"; then
            return 1
        fi
        
        if [ -z "$output" ] || echo "$output" | grep -q "^==.*== HEAP SUMMARY:"; then
            return 0
        fi
        
    echo "${GREEN}[PASS]${NC} $test_name - No memory leaks detected"
}

# Main test runner
run_all_tests() {
    local total=0
    local passed=0
    
    log_info "Starting integration tests..."
    
    init_logging
    
    test_generate_keys && ((passed++)) \ || log_error "Key generation failed"; ((passed--))
    test_client_connects && ((passed++)) \ || log_error "Connection test failed"; 
    test_server_no_decrypt && ((passed++)) \ || log_error "Decryption test failed"
    
}

# Run main suite
run_all_tests | tail -10

echo ""
echo "${GREEN}=== Integration Tests Complete ===${NC}"