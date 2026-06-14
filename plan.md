# End-to-End Encrypted Relay Chat – Development Plan

## 1. Overview
**Goal:** Build a command-line chat application in plain C where clients exchange text and binary messages via a central relay server.  
**Security:** The server **must not** be able to read message content. It only forwards encrypted blobs. Connection is allowed only if the server holds the client’s public key (whitelist).  
**Libraries:** ZeroMQ for network messaging, OpenSSL for all cryptographic operations.  
**Interface:** Clients interact via `stdin` / `stdout`.

---

## 2. System Architecture
+----------------+ ZeroMQ (ROUTER/DEALER) +----------------+
| Client A | <--------------------------------------> | |
| (DEALER socket)| | Relay Server |
+----------------+ | (ROUTER socket)|
| |
+----------------+ | |
| Client B | <--------------------------------------> | |
| (DEALER socket)| +----------------+
+----------------+

- **Client** uses a `ZMQ_DEALER` socket for asynchronous, identity‑based communication.
- **Server** uses a `ZMQ_ROUTER` socket that receives an identity frame automatically.
- Identities are managed by ZeroMQ; the server maps them to authenticated users.
- All message payloads are **end‑to‑end encrypted** – the server sees only ciphertext and metadata (sender/recipient fingerprints, timestamps).

---

## 3. Security Design & Protocol

### 3.1 Keys & Trust
- Every client generates an **RSA‑2048** key pair (or ECDSA/ECDH later, for efficiency).
- Public keys are exchanged out‑of‑band and stored in the server’s whitelist (directory of `.pem` files).
- Private keys never leave the client machine.

### 3.2 Authentication Handshake (Connection Establishment)
1. Client connects and sends `HELLO <fingerprint>` (SHA‑256 of its public key).
2. Server looks up fingerprint in its whitelist:
   - Unknown → disconnect.
   - Known → server replies `CHALLENGE <random‑nonce>`.
3. Client signs the nonce with its private key, sends `AUTH <signature>`.
4. Server verifies signature using the stored public key:
   - Success → connection marked as authenticated for that user.
   - Failure → disconnect.

### 3.3 Message Sending Protocol
For every message from A → B:
1. A builds a plaintext message (text or binary).
2. A generates an ephemeral AES‑256 key and a random IV.
3. Encrypts plaintext with AES‑256‑CBC (or GCM for authenticated encryption).
4. Encrypts the AES key with B’s RSA public key (`RSA_public_encrypt`).
5. Creates a payload:  
   `timestamp (8 bytes) || RSA_encrypted_AES_key || IV || AES_ciphertext`.
6. Signs `recipient_fingerprint || timestamp || RSA_encrypted_AES_key || IV || AES_ciphertext` with A’s private key.
7. Sends to server as a multipart ZeroMQ message:
   - Frame 0: `"MSG"`
   - Frame 1: recipient fingerprint (string)
   - Frame 2: `timestamp || encrypted_payload || signature`

**Server processing:**
- Extracts sender identity from the authenticated connection.
- Verifies signature using sender’s public key over `recipient_fingerprint || timestamp || encrypted_payload`.
- If valid, forwards unchanged payload to the recipient, prepending the sender’s fingerprint:
  - Frame 0: `"MSG"`
  - Frame 1: sender fingerprint
  - Frame 2: the same binary payload

**Recipient:**
- Verifies signature with sender’s public key.
- Decrypts AES key with its own private key.
- Decrypts the message with AES.
- Outputs the original plaintext.

### 3.4 Binary Data
- The same pipeline works for binary data; all payloads are treated as opaque byte arrays.
- The `stdin`/`stdout` interface will use a simple escape/prefix mechanism (e.g., `:bin <hex>` or a length‑prefixed mode) to distinguish text and binary input. Details in Phase 6.

### 3.5 Replay Protection
- Timestamps (UTC, millisecond precision) and a sliding window on the recipient side are used. Recipient keeps last N message hashes/timestamps per sender to detect replays.

---

## 4. Development Environment & Dependencies
- **Language:** C99 or C11 (plain C, no C++)
- **Build system:** CMake (portable, easy ZeroMQ/OpenSSL integration)
- **Libraries:**
  - `libzmq` (ZeroMQ ≥ 4.3)
  - `libssl` and `libcrypto` (OpenSSL ≥ 1.1.1)
- **Platform:** Linux (primary), macOS and Windows (MinGW/Cygwin) as secondary targets.
- **Tools:** `gcc` or `clang`, `cmake`, `make`, `pkg-config`, `valgrind` (memory checks), `cppcheck` (static analysis).

---

## 5. Development Phases

### Phase 0 – Project Setup & Build System
- Create directory structure:
chat/
├── CMakeLists.txt
├── src/
│ ├── server.c, server.h
│ ├── client.c, client.h
│ ├── protocol.h
│ ├── crypto.c, crypto.h
│ ├── transport.c, transport.h
│ └── utils.c, utils.h
├── include/
├── keys/ (sample keys, server whitelist)
└── tests/

- Write `CMakeLists.txt`: find packages `ZeroMQ` and `OpenSSL`, define `chat-server` and `chat-client` targets.
- Implement a simple logging utility (`utils.c`) with levels (DEBUG, INFO, WARN, ERROR).

### Phase 1 – Basic ZeroMQ Communication Skeleton
- **Server:** create a `ROUTER` socket bound to a configurable endpoint (e.g., `tcp://*:5555`). Infinite loop receiving multipart messages, printing identity and payload (raw).
- **Client:** create a `DEALER` socket, set identity (random UUID or `pid`), connect to server. Send a test `"PING"` and wait for `"PONG"`.
- Goal: verify messaging works, identities are preserved.

### Phase 2 – Key Generation & Management
- Implement `crypto.c`:
- `generate_keypair()`: generate RSA 2048, save private to file, public to file (PEM).
- `load_public_key(filename)`, `load_private_key(filename)`.
- `fingerprint(key)`: SHA‑256 of DER‑encoded public key, output as hex string.
- `sign(private_key, data, len)`, `verify(public_key, data, len, signature)`.
- `rsa_encrypt(public_key, data, len)`, `rsa_decrypt(private_key, data, len)`.
- `aes_encrypt(key, iv, plaintext)`, `aes_decrypt(key, iv, ciphertext)`.
- Add a small utility to generate keys for testing.
- Server whitelist: simple directory scan at startup; store fingerprints → public key mapping.

### Phase 3 – Authentication Handshake
- **Client:**
- After connection, load its own key pair.
- Compute fingerprint, send `HELLO <fp>`.
- Await `CHALLENGE <nonce>`.
- Sign nonce, send `AUTH <signature>`.
- Wait for `WELCOME` or `ERROR`.
- **Server:**
- On `HELLO`, look up fingerprint in whitelist. If not found, send `ERROR` and disconnect.
- Generate random nonce (32 bytes), send `CHALLENGE`.
- On `AUTH`, verify signature. If ok, mark connection as authenticated (store identity → fingerprint mapping), send `WELCOME`.
- Store per‑connection state (authenticated flag, peer fingerprint).

### Phase 4 – Secure Message Exchange (Client ↔ Server)
- Implement full message composition (AES + RSA + sign) as described in §3.3.
- Implement decryption + verification on receiving side.
- At this stage, test with a dummy “loopback” server that only echoes back, but verifies signatures. (Later the real relay will forward.)
- Key point: ensure server code **never** holds private keys or performs decryption.

### Phase 5 – Server Relay & Routing
- Server maintains a table: `{ fingerprint → connection_identity }` (the ZeroMQ identity).
- When a `MSG` arrives from an authenticated connection:
- Verify signature (must match connection’s fingerprint).
- Extract recipient fingerprint from frame 1.
- Look up recipient’s connection identity (must be online).
- Forward message prepending sender fingerprint.
- If recipient offline, optionally queue (for later implementation) or reply `ERROR` to sender.
- Handle disconnections: clean up table entries.

### Phase 6 – Client stdin/stdout Interface
- Use `select()` or a simple multi‑threaded approach (one thread for ZeroMQ, one for stdin) – or non‑blocking loop with `zmq_poll`.
- Incoming messages from network: output to `stdout` in format:
- Text: `<sender_fp> [HH:MM:SS] message`
- Binary: `<sender_fp> [HH:MM:SS] <binary size=XYZ bytes>` then hex dump or redirection hint.
- Outgoing user input:
- Normal text lines: send as plaintext to a default recipient (configurable).
- Commands prefixed with `/`:
  - `/to <fingerprint>` – set current recipient.
  - `/bin <filename>` – read file and send as binary.
  - `/quit`
- `/raw` to enter raw mode for piping binary data.
- Handle binary safety: messages passed internally as `uint8_t*` + length.

### Phase 7 – Binary Data Support
- Ensure all cryptographic functions operate on binary buffers, no string assumptions.
- Extend the message payload to contain a `content_type` byte (e.g., `0x00` = text, `0x01` = binary) so receiver can display appropriately.
- Test with files (images, etc.).

### Phase 8 – Error Handling, Logging, Configuration
- Every ZeroMQ call checked, all OpenSSL errors retrieved with `ERR_get_error()`.
- Server and client read a configuration file (ini style or JSON via jsmn) for:
- Server port, whitelist directory, log level.
- Client private key path, server address, default recipient.
- Graceful shutdown (SIGINT handler) – close sockets, free memory.
- Memory management: use clean allocation/deallocation, run Valgrind regularly.

### Phase 9 – Testing & Security Audit
- **Unit tests** (using CMake/CTest) for:
- Key generation, fingerprint computation.
- Sign/verify, encrypt/decrypt cycles.
- Protocol frame building/parsing.
- **Integration tests** (bash scripts or Python test harness):
- Two clients + one server, exchange text and binary.
- Test replay detection.
- Test server cannot read content (inspect server logs).
- Authentication failure scenarios (wrong key, fake signature).
- **Fuzz testing** on protocol parsers.
- Manual code review focusing on buffer overflows, use of `memcpy`, input validation.

### Phase 10 – Documentation & Packaging
- `README.md`: build instructions, key generation, server setup, example usage.
- Man pages or `--help` for client and server.
- Provide a quick‑start script that generates test keys and launches a local setup.

---

## 6. Implementation Details

### Key Data Structures

```c
// server connection state
typedef struct {
  char identity[256];    // ZeroMQ identity
  char fingerprint[65];  // hex SHA256
  RSA *pubkey;           // loaded from whitelist
  bool authenticated;
} conn_t;

// message frame layout (inside payload)
typedef struct {
  uint64_t timestamp;
  // followed by:
  //   uint16_t rsa_encrypted_key_len;
  //   uint8_t  rsa_encrypted_key[];
  //   uint8_t  iv[16];
  //   uint32_t ciphertext_len;
  //   uint8_t  ciphertext[];
  //   uint16_t signature_len;
  //   uint8_t  signature[];
} secure_payload_t;

Core Crypto Functions (crypto.c)

    crypto_init() – seed PRNG, load OpenSSL error strings.

    crypto_cleanup().

    fingerprint_from_rsa(RSA *key, char hex_out[65]).

    generate_nonce(unsigned char *buf, int len).

    hybrid_encrypt(RSA *recip_pub, const unsigned char *plain, size_t plain_len, unsigned char **out, size_t *out_len).

    hybrid_decrypt(RSA *my_priv, const unsigned char *in, size_t in_len, unsigned char **out, size_t *out_len).

    sign_data(RSA *priv, const unsigned char *data, size_t len, unsigned char **sig, size_t *sig_len).

    verify_data(RSA *pub, const unsigned char *data, size_t len, const unsigned char *sig, size_t sig_len).

ZeroMQ Message Frames

Multipart messages are built and parsed with zmq_msg_send/recv. Example for MSG send:
zmq_send(socket, "MSG", 3, ZMQ_SNDMORE);
zmq_send(socket, recipient_fp, 64, ZMQ_SNDMORE);
zmq_send(socket, payload, payload_len, 0);

7. Testing Strategy

    Continuous Integration: GitHub Actions building on Ubuntu (gcc+clang) with ZeroMQ and OpenSSL from repos.

    Memory: valgrind --leak-check=full on every test run.

    Security: Test server internals – ensure no private keys, no decrypted content in logs.

    Performance: Measure latency with ping‑like test, throughput for large binary transfers.

8. Potential Challenges & Mitigations
Challenge	Mitigation
RSA encryption size limit	Use hybrid encryption (AES session key).
Connection identity persistence across reconnects	Use a client‑generated UUID as identity, stored in config.
Replay attacks	Timestamp + sequence number, sliding window, duplicate cache.
Offline message delivery	Phase‑2 feature: store encrypted messages on server, deliver on connect. Server still cannot decrypt.
ZeroMQ multi‑thread safety	Keep socket operations in one thread per socket; use zmq_poll for multiplexing stdin and socket.
Binary data on terminal	Use base64 encoding for display, raw mode for piping, /bin command to send files.

