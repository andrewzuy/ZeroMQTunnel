# ZeroMQTunnel - End-to-End Encrypted Relay Chat

## Quick Start

```bash
mkdir -p keys && cd keys

# Generate test keys
./chat-client --gen-keys client_A client_B

./chat-server --gen-keys server_whitelist

# Run integration tests
cd ../tests
./setup.sh
```

## Build & Configuration

```bash
cmake -B build
cmake --build build --target all
```

### Server
```bash
./chat-server 5555 keys/server_whitelist
```

### Client
```bash
./chat-client client_B.pub client_B.pem tcp://localhost:5555
```

## Messages to stdout

```
<sender_fp> [12:34:56] Hello there!
<sender_fp> [12:34:57] This is binary data (size=1024 bytes)
```

### Commands

```bash
./chat-client client_B.pub client_B.pem tcp://localhost:5555
```

## Security Architecture

- **Client**: ZEROMQ_DEALER with identity-based async communication
- **Server**: ZEROMQ_ROUTER that forwards encrypted blobs only
- **Encryption**: Hybrid AES-256-CB + RSA-2048 per recipient
- **Authentication**: Challenge-response with RSA signatures
- **Replay protection**: Timestamps + sliding window on recipients
