# ZeroMQTunnel - E2EE Relay Chat (Phase 2-10 Complete)

## Overview
End-to-end encrypted relay chat in plain C using OpenSSL for encryption and 
signatures. Server relays **encrypted blobs only** - never reads message content.

### Security Architecture
- **Client**: Generates RSA-2048 key pairs, authenticates via signature challenge
- **Server**: Whitelist-based public keys only; forwards encrypted messages between clients
- **Encryption**: AES-256-CBC session keys encrypted with recipient's RSA public key
- **Authentication**: Challenge-response using RSA/ECDSA signatures
- **Replay Protection**: Timestamp windows (default 30 seconds, configurable)

## Quick Start

### Build from Source
```bash
# Clone repository
git clone https://github.com/user/ZeroMQTunnel.git
cd ZeroMQTunnel/chat

# Configure (Phase 1 setup)
make clean all
cd build/clean || make -C ../src > /dev/null 2>&1

# Build binaries
make -C src chat-server chat-client
```

### Server Command
```bash
./src/chat-server [port] [keys_dir]
# Example: ./chat-server 5555 keys
```

### Client Commands
```bash
# Connect with your key pair
./src/chat-client <public_key.pem> [<server_address>]

# Set recipient and send message
/to <recipient_fp> "Hello World"

# Send binary file
/bin /path/to/file.dat
/bin ./keys/server_whitelist/pub
/bin <filename>
```

## Command Reference (`/` prefix)

| Command | Description |
|---------|-------------|
| `/to <fp>` | Set current recipient fingerprint (required for sending) |
| `/text "msg"` | Send plaintext message to recipient |
| `/bin <file>` | Read and send binary file as payload |
| `/exit` or `/quit` | Gracefully disconnect from server |

## Messages to stdout

```
<sender_fp> [HH:MM:SS] This is a text message
<sender_fp> [HH:MM:#S] size=1024 (binary data, see size=XYZ bytes)
<ERROR> Unknown recipient or connection failed
```

## Key Generation & Setup

### Generate New Keys
```bash
# Place in keys/client_<name>/ subdirectory for each user
openssl genrsa -out client_A.pem 2048
openssl rsa -in client_A.pem -pubout -out client_A.pub

# Or use included keygen utility
gcc tools/keygen.c -o keygen $@ && ./keygen client_A client_B keys
```

### Configure Server Whitelist
Place public keys in `keys/` directory with naming:
- `keys/client_A/pub.pem` → fingerprints file generated at startup  
- Server scans directory and builds `<fingerprint>→public_key` mappings

## Message Protocol

### Secure Payload Format
```
timestamp (8 bytes) || RSA_encrypted_AES_key || IV (16) || Cipher_text || Signature
```

Server forwards payload unchanged after verifying sender signature.

## Testing & Security Audit

### Run Integration Tests
```bash
cd tests/integration
chmod +x *.sh && run_tests.sh  # or cd build && ./run_tests.sh
```

Test suite verifies:
- ✓ Message encryption/decryption cycle
- ✓ Server cannot read plaintext (logs show only ciphertext)  
- ✓ Replay attack detection via timestamp windows
- ✓ Authentication rejects forged signatures
- ✓ Binary file transfers (test with /bin <file>)

## Configuration Files

Optional `config.ini` (INI format):
```
[server]
port = 5555
whitelist_dir = keys
log_level = INFO  # DEBUG, INFO, WARN, ERROR

[client]
recipients_dir = clients
private_key_path = client_A.pem
```

## Platform Support
- **Linux**: Primary (gcc, cmake, valgrind recommended)
- **macOS**: Works with Homebrew OpenSSL + pkg-config ZeroMQ  
- **Windows**: MinGW-w64 GCC required; adjust OpenSSL includes

## Memory Safety & Error Handling
- All ZEROMQ calls checked with `zmq_strerror()` before proceeding
- OpenSSL errors captured via `ERR_get_error()` on critical operations
- Valgrind passes on all test runs (no leaks, no invalid reads/writes)
- Graceful SIGTERM termination: closes sockets, releases allocated resources

## Security Considerations
- RSA-2048 sufficient for encrypting small data; hybrid crypto used for files
- Server never stores decrypted messages or private keys
- Timestamps prevent replay attacks (30-second sliding window per sender)
- Private keys protected via filesystem permissions (chmod 600 on .pem files)

## See Also
- Phase 1: Project setup & CMake build system  
- Phase 2: Full OpenSSL crypto library implementation  
- Phase 3-7: Authentication, relay routing, stdin/stdout interface  
- Phase 8: Error handling, logging, configuration parsing  
- Phase 9: Comprehensive testing suite with security audits  
- Phase 10: Man pages (`--help`), packaging scripts
