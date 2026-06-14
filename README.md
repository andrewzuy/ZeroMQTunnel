# ZeroMQTunnel - E2EE Relay Chat Protocol Implementation

## 📦 Project Status
**Phase 2-7: Standalone Makefile Build System (Stubs)**  

| Phase | Description | Status |
|-------|-------------|---------|
| P0-P1 | Project setup, basic I/O skeleton | ✅ Complete |
| P2 | OpenSSL crypto library (stub implementations) 🟡 In Progress |
| P3-4 | Auth handshake protocol (HELLO→CHALLENGE→AUTH→WELCOME) 🟡 Pending Phase 8 |
| P5-7 | Server relay routing, CLI interface ⚠️ Currently stub |
| P8 | CMake integration with ZeroMQ | 🔒 Requires ZeroMQ library installation |

## 🏗️ Architecture

```
┌───────────────── RELAY SERVER ─────────────────┐
│  [PORT] ←→ Listen socket                       │
│  Whitelisted public keys (keys/*.pem)          │
│                                                │
│  HELLO → CHALLENGE → AUTH → WELCOME            │
│  Secure message relay                            │
╰────────────────── ZeroMQ Forwarding ──────────╯
    ┌──────────────┐       Relay        ┌──────────────┐
    │ CLIENT A     │ ←──────────────→   │ SERVER       │ ←──────────────→ CLIENT B
    │ /to bob MSG  │                  │  AES-256      │              │
    │ /quit        │                  │ PKCS#1 RSA    │              │
    └──────────────┘                  │ Hybrid crypto │              │
                                      └──────────────┘              │
                                                                [CLIENT C]
```

## 📦 Quick Start

### Build from Source

```bash
cd ZeroMQTunnel/chat
make all            # Build chat-server, chat-client, keygen
make clean          # Remove binaries
make keys           # Generate RSA-2048 key pair
make test           # Build + generate keys + start server
```

### Run Server

```bash
./chat-server [port]   # e.g., ./chat-server 5558 &
```

### Run Client

```bash
./chat-client priv.pem pub.pem <<<'/quit'
```

#### Client Commands (stdin)

| Command | Description |
|---------|-------------|
| `/quit` | Exit client |
| `/to <recipient> MSG` | Send message to recipient channel |

## 🔐 Cryptography (Phase 2 - Stub Implementation)

```
Client                 Server                Recipient
─────────            ───────              ─────────
RSA-2048 keygen                  │
fingerprint generation ←          │
│                               │    whitelist:
HELLO<fingerprint>        →       │    {fp1: RSA, fp2: RSA}
│                              │   ──────────
CHALLENGE<nonce>           ←       │         ↓     Forward only (NO decryption)
│                              │
SIGN(CHALLENGE)            →       │    AES-256 session key encrypted per recipient
│                              │
WELCOME                    ←       │    RSA_encrypt(ES_AES_key) + IV + ciphertext
│                              │
Send: timestamp + enc_payload ←─       │
```

### Security Features

| Feature | Implementation Status |
|---------|----------------------|
| OpenSSL EVP_PKEY (stub) | ✅ Functional with deprecation warnings |
| AES-256-CBC session keys | 🔒 Placeholder in Phase 4 stubs |
| RSA PKCS#1 OAEP hybrid | 🟡 Phase 8 full implementation |
| PSS signature verification | ⏳ Phase 3 handshake protocol |
| Timestamp replay protection | ✅ Protocol header includes millisecond timestamp |
| Server security (no plaintext) | ✅ Server never decrypts payloads |

## 📝 Files Structure

```
ZeroMQTunnel/
├── CHAT.md                 # Chat client/server reference guide (TODO)
├── README.md               # This file
│
└── chat/
    ├── Makefile            # Phase 2-7 standalone build system
    │
    ├── include/            # Header files (.h)
    │   ├── client.h        # Client protocol declarations
    │   ├── crypto.h        # RSA/AES stub API
    │   ├── logging.h       # Logging macro definitions
    │   ├── protocol.h      # Message framing & states
    │   ├── server.h        # Server initialization
    │   └── utils.h         # UUID/string utilities
    │
    ├── src/                # Source files (.c)
    │   ├── client.c        # Client stdin/stdout interface
    │   ├── crypto.c        # RSA keygen stubs with EVP context
    │   ├── protocol.c      # Message framing utilities
    │   ├── server.c        # Server socket initialization
    │   ├── utils.c         # UUID/str ops / logging functions
    │   └── logging.c       # Logging level macros implementation
    │
    ├── tools/              # Utility programs
    │   └── keygen.c        # RSA 2048 generation utility
    │
    ├── keys/               # Public key whitelist
    │   ├── priv.pem       # Server private (localhost only)
    │   └── pub.pem        # Server public key
    │
    ├── doc/                # Generated documentation stubs (TODO)
    └── tests/              # Integration test suite shells (Phase 8+)
```

## 🧪 Testing

### Verify Build

```bash
make all && \
./keygen keys/priv.pem keys/pub.pem && \
ls -lh chat-server chat-client keygen
```

### Test Client Commands

```bash
export SERVER_PORT=5558
echo "/whoami
/to alice Hello! Testing client commands here.
/to bob Sending to Bob channel now.
/stop
/quit" | ./chat-client keys/priv.pem keys/pub.pem 127.0.0.1:$SERVER_PORT
```

## 📚 Security Architecture Documentation

| Document | Description | Location |
|----------|-------------|----------|
| `doc/security_model.pdf` | Hybrid crypto model, RSA-PSS signatures | Phase 4 doc stubs |
| `doc/completed_phases_2-10.md` | Technical overview of all phases | Project root |
| `/home/andrew/Development/ZeroMQTunnel/chat/doc/README.md` | Quick-start documentation | Client/server guide |

---

## 🚀 Next Steps (Phases 4+)

1. **Phase 3:** Integrate OpenSSL EVP properly (replace stub crypto functions)
2. **Phase 8:** Install ZeroMQ for full relay message forwarding  
3. **Phase 9:** Complete server whitelisting with INI config parser
4. **Phase 10:** Full handshake protocol and documentation completion

## ⚠️ Known Limitations (Current State)

- OpenSSL EVP calls use stub implementations (deprecated API warnings expected)
- ZeroMQ dependency skipped until Phase 8 (can build without for testing)
- Protocol handshakes are skeleton code awaiting full EVP integration

---

**License:** MIT (placeholders) - See individual files  
**Author:** Andrew - ZeroMQTunnel E2EE Protocol Implementation
