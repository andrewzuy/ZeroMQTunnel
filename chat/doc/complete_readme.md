# ZeroMQTunnel - Complete Documentation (Phases 1-10)

End-to-end encrypted relay chat application in plain C99/C11 using OpenSSL for 
cryptography and ZeroMQ for network messaging.

**Architecture**: Server relays encrypted message blobs only—never decrypts content.  
Clients authenticate via RSA signature challenge-response; session keys use AES-256-CBC.

---

## Quick Start

```bash
mkdir -p keys/client_A && cd keys/client_A
# Generate test key pair or use provided scripts
gcc ../tools/keygen.c -o keygen $@
./keygen generate_rsa_keypair client_A.pub client_A.pem

cd ..
cd chat/src
make clean all && make chat-server chat-client
```

### Server Mode
```bash
./chat-server 5555 keys/whitelist   # Starts relay on port 5555
```

### Client Mode  
```bash
./chat-client client_A.pub tcp://*:server:5555
```

---

## Complete Reference

### Phase 0-1: Project Setup & Basic I/O Skeleton [COMPLETE]
- Directory structure: `src/`, `include/`, `keys/`, `tests/`
- Build system: Makefile with C99, stddef.h, openssl headers
- Chat-server & chat-client stub binaries (Phase 1)

### Phase 2: Complete Key Generation Library [COMPLETE]
`crypto.c`:
- `generate_rsa_keypair()`: RSA-2048 key generation via `EVP_PKEY_keymgen()`
- `load_private_key()`: PEM/DER format loading with fallback
- `RSA_encrypt()` / `RSA_decrypt()`: Hybrid crypto session key transport  
- `RSA_sign()` / `RSA_verify()`: PSS padding signature schemes (P-256)

### Phase 3: Authentication Handshake [COMPLETE]
Protocol flow:
1. Client sends `HELLO <fingerprint>` (SHA256 of public key DER)
2. Server looks up in whitelist; rejects unknown fingerprints immediately  
3. Server sends `CHALLENGE <nonce>` (32-byte random buffer)
4. Client signs nonce → sends `AUTH <signature>` to server
5. Server verifies with stored public key; replies `WELCOME` or `ERROR`

### Phase 4: Secure Message Exchange [COMPLETE]
Payload format (from client-to-server):
```
[timestamp_ms(8)] || [RSA_encrypted_AES_key] || [IV(16 bytes)] \
               || [AES_ciphertext(variable)] || [Signature(variable)]
```

Server:
- Validates sender signature using stored public key
- Prepends frame with `"MSG"` type identifier and sender fingerprint  
- Forwards unchanged binary blob to recipient connection

### Phase 5-6: Server Relay Routing & CLI Interface [COMPLETE]


Server maintains: `{fingerprint → identity}` mapping for routing.

Client CLI commands (`/` prefix):
- `/to <fprint>`: Set current recipient fingerprint (required before sending)
- `/text "Hello"`: Plain text message to recipient  
- `/bin <file>`: Reads file binary and sends as encrypted blob
- `/quit`: Gracefully closes connection

Stdin/stdout loop uses `select()` with non-blocking socket I/O.

### Phase 7: Binary Data Support [COMPLETE]
Content type flags embedded in payload header:
- Bit 0 = text (default output format)
- Bit 1 = binary (hex dump or redirection mode, configurable)  
Size prefix for large transfers to handle terminal buffering correctly.

### Phase 8: Error Handling, Logging, Configuration [COMPLETE]
- Logging levels: `LEVEL_DEBUG=0`, `LEVEL_INFO=1`, `LEVEL_WARN=2`, `LEVEL_ERROR=3`
- Every `zmq_*`/`RSA_*` call returns `-99` on failure; OpenSSL errors via `ERR_get_error()`  
- Config INI parser (section/key/value format, optional):
  ```ini
  [server]
  port = 5555
  whitelist_dir = keys
  log_level = INFO
  
  [client]
  recipients_path = client_A.pub
  private_key_path = client_A.pem
  server_addr = tcp://relay.example:5555
  ```

- Graceful SIGTERM handling (Phase 8): closes ZEROMQ sockets, frees allocated memory buffers.

### Phase 9: Testing & Security Audit [COMPLETE]

#### Unit Tests (`tests/crypto/`)
```bash
gcc -Iinclude tests/crypto/test_crypto.c src/crypto.c ../src/utils.c \
    -L/usr/local/lib -lssl -lcrypto -o check_crypto
./check_crypto
# Verifies fingerprint consistency, RSA encrypt/decrypt cycles sign/verify.
```

#### Integration Tests (`tests/integration/run_tests.sh`)
- ✅ Message encryption cycle (encrypt→forward→decrypt)  
- ✅ Server cannot read plaintext (verifiable via server logs)  
- ✅ Replay attack detection via timestamp sliding windows (per-sender cache)  
- ✅ Binary file transfer (images, audio data)  
- ✅ Memory safety (valgrind `--leak-check=full` no leaks detected)

### Phase 10: Documentation & Packaging [COMPLETE]
- `README.md`: Build instructions, key generation, example usage  
- Quick-start script generating test keys and launching local setup  
- Man pages (`--help`) for client/server binaries  
- Security considerations document (RSA encryption limits, TLS vs E2EE comparison)

---

## Message Protocol Specification

### Authenticated Connection Messages (framed multipart)
Frame 1: MSG type string identifier ("MSG" or "HELLO")
Frame 2: Recipient fingerprint (string, 64 hex chars + null)  
Frame 3+: Payload (binary buffer or text with newline terminator, depending on content_type byte)

### Signed Message Signatures
Signature covers all bytes of payload starting from timestamp header onward. RSA signature appended to ciphertext as suffix (variable length).

---

## Security Design Summary

**ZeroMQTunnel security model ensures server cannot read message content:**  
1. Client generates RSA-2048 key pair; stores private key locally only  
2. Public keys exchanged out-of-band; server holds whitelist of `<fprint,public_key>` mappings  
3. Session encryption uses AES-256-CBC (ephemeral per-message)  
4. AES session key encrypted with recipient's RSA public key (hybrid crypto)  
5. Sender signs entire payload including timestamp, encrypted key, IV, ciphertext  
6. Server verifies sender signature; forwards unchanged blob to recipient  
7. Recipient decrypts session key; AES decrypts message

**Attack Resilience**:
- Replay: Timestamp window prevents reused messages (per-sender cache N=100, default)  
- MITM: Forged signatures rejected immediately at signature verification stage  
- Whitespace-only input: Command parsing rejects empty/null lines with exit code 4

