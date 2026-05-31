# ZeroMQTunnel - Resilient Encrypted TCP Tunnels

[![Phase 4](https://img.shields.io/badge/Phase-4-green)](https://raw.githubusercontent.com/zeroqmq-tunnel)

## 🚀 Features

- **Remote Forwarding** - Expose local services on other NATed hosts via a public port
- **Local Forwarding** - Access remote services through local port listeners
- **End-to-end encryption** with ZeroMQ CURVE authentication
- **Automatic reconnection** on network disruptions
- **Production hardening** with metrics, tracing, and resource limits

## 📥 Quick Start

### Prerequisites

1. Generate CURVE keypairs:

```bash
# Server key (public-facing)
cargo run --bin tunnel-server -- genkey --output /etc/tunnel/server.pem

# Agent keys for each host
cargo run --bin tunnel-agent -- genkey --output ~/.config/tunnel/agent-a.pem
cargo run --bin tunnel-agent -- genkey --output ~/.config/tunnel/agent-c.pem
```

### 2. Configure Tunnel Server

Create `/etc/tunnel/server.toml`:

```toml
[server]
control_port = 5555
listen_address = "0.0.0.0:1443"
key_file = "/etc/tunnel/server.pem"
global_max_connections = 1000
```

### 3. Start Server

```bash
cargo run --release --bin tunnel-server \
    --config /etc/tunnel/server.toml
```

### 4. Remote Forwarding (Expose local port)

On host C running your local service:

```bash
cargo run --release --bin tunnel-agent \
    --remote -s web-443 443 \
    --server-address http://server:5555 \
    --key-file ~/.config/tunnel/agent-c.pem
```

**Access:** Connect to `http://<server-ip>:1443` from anywhere!

### 5. Local Forwarding (Access remote service locally)

Server-side (host C):
```bash
cargo run --release --bin tunnel-agent \
    --remote -s web-443 443 \
    --server-address http://server:5555 \
    --key-file ~/.config/tunnel/agent-c.pem
```

Client-side (host A):
```bash
cargo run --release --bin tunnel-agent \
    --remote -s web-443 443 \
    --server-address http://server:5555 \
    --key-file ~/.config/tunnel/agent-a.pem
```

**Access:** Connect to `http://localhost:8080` on host A!

## 🔧 CLI Options

### Tunnel Server

```bash
cargo run --release --bin tunnel-server [OPTIONS]

Arguments:
  <CONFIG>              Path to configuration file (TOML format)

Options:
  -h, --help            Print help
```

### Tunnel Agent

```bash
cargo run --release --bin tunnel-agent [OPTIONS]

Arguments:
  <PORT>                Local service port to forward
  -s <service-id>       Human-readable service identifier (e.g. web-443)

Options:
  --remote              Enable remote forwarding mode (tunnel server mode)
  --local               Enable local forwarding mode (local proxy mode)
  --server-address      Control server address (required, e.g. zmq://server:5555)
  --key-file <PATH>     Path to agent's CURVE keypair
  -h, --help            Print help
```

## 🏗️ Architecture

```
┌───────────────┐     ┌───────────────────┐     ┌───────────────┐
│   Agent (C)  │◄────►│  Tunnel Server    │◄────►│   Agent (A)  │
│ remote fwd    │     │  (public IP)      │     │ local fwd     │
│ web-443       │     │                   │     │ localhost:8080 │
└───────────────┘     └───────────────────┘     └───────────────┘

All control connections use CURVE encrypted ZeroMQ
```

## 🔐 Security

- **CURVE Encryption**: All control channel connections use authenticated Curve25519
- **Agent Whitelist**: Only approved agents can connect to the tunnel server (ZAP-based)
- **Key Storage**: Keys stored in PEM format, age-encrypted at rest

**Note:** Public-facing ports for remote forwards are plain TCP. Applications can add TLS inside the tunnel if needed.

## 🧪 Testing Your Setup

```bash
# Test agent connection
./target/release/tunnel-agent --remote -s test 8000 \
    --server-address zmq://localhost:5555 \
    --key-file ~/.config/tunnel/agent-dev.pem
```

## 🔬 Development

```bash
# Build workspace
cargo build --workspace --release

# Run server (testing without config)
cd tunnel-server && cargo run --bin tunnel-server

# Run agent on same machine to test
cd ../tunnel-agent && cargo run --bin tunnel-agent \
    --remote -s local-test 8000 \
    --server-address zmq://localhost:5555 \
    --key-file ~/.config/tunnel/agent-dev.pem
```

---

**Built with Tokio & ZeroMQ for production-ready, self-healing encrypted tunnels.**
