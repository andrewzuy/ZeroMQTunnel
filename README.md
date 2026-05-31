
# ZeroMQTunnel - Resilient Encrypted TCP Tunnels
[![Phase 4](https://img.shields.io/badge/Phase-4-green)](https://raw.githubusercontent.com/zeroqmq-tunnel)


## 🚀 Features

- **Remote Forwarding** - Expose local services on other NATed hosts via a public port
- **Local Forwarding** - Access remote services through local port listeners  
- **End-to-end encryption** with ZeroMQ CURVE authentication
- **Automatic reconnection** on network disruptions
- **Production hardening** with metrics, tracing, and resource limits

## 📥 Quick Start

### 1. Generate Keys

```bash
cargo run --bin tunnel-server -- genkey --output /etc/tunnel/server.pem
cargo run --bin tunnel-agent -- genkey --output ~/.config/tunnel/agent.pem
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
---
### 4. Remote Forwarding (Expose local port 443)

On host C running your local service:
```bash
cargo run --release --bin tunnel-agent \
    --remote -R 443 web-443 --server-address http://server:5555 \
    --key-file ~/.config/tunnel/agent.pem
```

**Access:** Connect to `http://<server-ip>:1443` from anywhere!

### 5. Local Forwarding (Access remote service locally)

Server-side (host C):
```bash
cargo run --release --bin tunnel-agent \
    --remote -R 443 web-443 --server-address http://server:5555 \
    --key-file ~/.config/tunnel/agent-c.pem
```

Client-side (host A):
```bash
cargo run --release --bin tunnel-agent \
    --local -L 8080 web-443 \
    --server-address http://server:5555 \
    --key-file ~/.config/tunnel/agent-a.pem
```

**Access:** Connect to `http://localhost:8080` on host A!
---

## 🔧 CLI Options

### Tunnel Server

```bash
cargo run --release --bin tunnel-server [OPTIONS]
  --config <FILE>      Read configuration from default to file
  --control-port NUM   Port for control messages (default: 5555)
  --listen-address     Public port to listen on (default: 1443)
```

### Tunnel Agent  

```bash
cargo run --release --bin tunnel-agent [OPTIONS]

Options:
  --config <FILE>       Read configuration from file  
  --key-file <PATH>     Path to agents keypair
  
Remote Forwarding (-R):
  --remote             Enable remote forwarding mode
  -R <PORT>            Local service port to forward  
  --service-id         Human-readable service identifier

Local Forwarding (-L):
  --local              Enable local forwarding mode  
  -L <PORT>            Port to listen on locally

Other Options:
  --server-address     Control server address (required)
```
---

## 🏗️ Architecture

```
┌───────────────┐     ┌───────────────────┐     ┌───────────────┐
│   Agent (C)  │◄────►│  Tunnel Server    │◄────►│   Agent (A)  │
│ remote fwd    │     │  (public IP)      │     │ local fwd     │
│ web-443       │     │                   │     │ localhost:8080 │
└───────────────┘     └───────────────────┘     └───────────────┘

All control connections use CURVE encrypted ZeroMQ
```
---

## 🔐 Security

- **CURVE Encryption**: All control channel connections use authenticated Curve25519
- **Agent Whitelist**: Only approved agents can connect to the tunnel server  
- **Key Storage**: Keys stored in PEM format, age-encrypted at rest

**Note:** Public-facing ports for remote forwards are plain TCP. Applications can add TLS inside the tunnel if needed.

## 🏠 Docker Usage

### Server Container

```bash
docker run -d \
    --name tunnel-server \
    -p 5555:5555 -p 1443:1443 \
    -v /etc/tunnel:/etc/tunnel:ro \
    registry.example.com/zeroqmq-tunnel/server:prod
```

### Agent Container

```bash  
docker run -d \
    --name remote-agent \
    -e SERVICE_HOST=C --service-id=web-443 -p 9443:1443 \
    -v /etc/tunnel/agent:ro \
    registry.example.com/zeroqmq-tunnel/agent:prod
```

## 🚨 Troubleshooting

### "Cannot establish encrypted control connection"
**Solution:** Verify agent public key is whitelisted on server, or regenerate keys.

### Repeated Reconnects
**Fix:** Network instability - automatic recovery takes max 30 seconds.

### "ZAP failed - unauthorized"  
**Solution:** Add agents CURVE public key to `/etc/tunnel/agent_keys.txt` and restart server.

## 🧪 Testing Your Setup

```bash
# Test remote forwarding  
curl http://<server-ip>:1443/health  # Should see local service response!

# Test local forwarding  
curl http://localhost:8080/health    # From host A accessing remote web-443
```


## 🧪 Development

```bash
# Build workspace
cargo build --workspace --release

# Run server (testing)
cd tunnel-server && cargo run --bin tunnel-server

# Run agent on same machine to test
cargo run --bin tunnel-agent --local -L 8000 remote-service-443 \
    --key-file ~/.config/tunnel/agent-dev.pem
```

---

**Built with Tokio & ZeroMQ for production-ready, self-healing encrypted tunnels.**
