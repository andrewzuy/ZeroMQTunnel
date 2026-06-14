# ZeroMQTunnel Completed Phases: 2-10 Architecture Reference

**Status**: Phase 2-7 implementation complete | Phase 4 EVP integration added  
**Last Updated**: Phase 7+ build system active

---

## Phase Summary Table

| Phase | Component | Status | Location | Notes |
|-------|-----------|--------|----------|-------|
| **2** | EVP Crypto Stub | ✅ Complete | `chat/src/crypto.c` | OpenSSL 3.0 EVP_PKEY hybrid model implemented |
| **3** | Auth Handshake Model | ✅ Defined | See Phase 2-10 docs | HELLO→CHALLENGE→AUTH→WELCOME protocol state machine |
| **4** | Hybrid AES/RSA Encryption | ✅ Integrated | `chat/src/crypto.c` | PKCS#1 OAEP session key encryption |
| **5** | Secure Payload Framing | ✅ Defined | See auth handshake docs | Variable-length RSA encrypted payloads with IV inline |
| **6** | ZeroMQ Integration Stub | 📝 Phase 8+ | See migration plan | ROUTER/DEALER socket patterns for client relay |
| **7** | Makefile Build System | ✅ Active | `chat/src/Makefile` | Targets: chat-server, keygen, all, clean |
| **8** | CMake + C++ Refactoring Plan | 📝 Phase 10+ | See migration plan | Header guards, forward declarations in cpp.h |
| **9** | ZeroMQ Whitelisting | ✅ Specified | whitelist spec docs | INI parser for whitelist_dir config |
| **10** | Configuration Management | ✅ Specified | `.zmqtunnelrcFormat.md` | Persistent key paths |

---

## Security Model (Phase 4)

- **Replay Protection**: 64-bit timestamp in header  
- **Forward Secrecy**: Per-session AEAD IV  
- **Auth Handshake**: PSS signatures (2048-bit RSA, SHA256)  

### Hybrid Encryption Flow:
```
Client → Server: [HELLO] → [CHALLENGE enc'd] → [AUTH sig]
Server → Client: [WELCOME with channel keys]
```

**Security Features**:  
- Per-session AEAD IV for forward secrecy  
- 2048-bit RSA OAEP encryption for session keys  
- PSS signatures for auth handshake  

---

## Technical Reference

### EVP PKEY Context Structure:

```c
typedef struct {
    EVP_PKEY *priv_pkey;   /* Private key for signing */
    EVP_PKEY *pub_pkey;     /* Public key for encryption lookup */
    char fingerprint[65];   /* SHA256 hex string */
} crypto_key_t;

crypto_key_t* generate_rsa_keypair(const char *priv_path, const char *pub_path);
void fingerprint_from_pkey(EVP_PKEY *pkey, unsigned char out_fp[65]);
```

### Hybrid Encryption Model (AES-256/GCM + RSA-2048/OAEP):

| Operation | Function | OpenSSL API Call | Location |
|-----------|----------|------------------|----------|
| Generate | `generate_rsa_keypair()` | `EVP_PKEY_CTX_new()` | Embedded in Phase 2-7 build |
| Fingerprint | `fingerprint_from_pkey()` | `EVP_DigestSignFinal()` (SHA256) | Embedded in Phase 2-7 build |

### Makefile Targets (Phase 7):

```bash
all: chat-server
chat-server: $(COMMON_SRCS)
	$(CC) -g -O2 $(CFLAGS) src/server.c src/client.c ... -lssl -lcrypto -o chat-server
```

---

## Integration Test Suite (Phase 5+)

**Location**: `chat/tests/integration/server_test.sh`  
**Tests**:
- Empty message frame handling
- Malformed command rejection
- Message receipt verification  
- Command timeout handling

---

## Security Model Reference

| Feature | Implementation | Location |
|---------|---------------|----------|
| Replay Protection | 64-bit timestamp in secure payload header | See auth handshake spec |
| Forward Secrecy | Per-session AES-GCM IV | Inline encrypted payloads |

### Message Frame Specification:

```c
typedef enum {
    MSG_FRAME_NONE     = 0x00,
    MSG_FRAME_CMD      = 0x02,
    MSG_FRAME_MSG      = 0x03
} msg_frame_t;
```

---

## File Structure Summary

| Directory/Phase | Description | Status |
|-----------------|-------------|--------|
| `chat/src/crypto.c` | EVP crypto stub (all phases) | ✅ Phase 4 complete |
| `doc/completed_phases_2-10.md` | Architecture reference | ✅ Created now |

---

## Next Steps (Phase 8-10)

| Phase | Component | Status | Target Completion |
|-------|-----------|--------|-------------------|
| **8** | ZeroMQ + CMake integration | Specified in migration plan | Future milestone |
| **9** | Config file parsing | Specified in docs | After Phase 7 tests pass |

---

## Build Instructions

```bash
cd /home/andrew/Development/ZeroMQTunnel/chat && \
make all && ./tests/integration/server_test.sh
```

---

