# 🚀 ZMQTunnel — ZMQ Curve Encrypted Port Forwarding  

**ZeroMQ-tube-encrypted, auto-reconnecting, ssh-like local & remote port forwarding**.

A CLI tool providing **SSH**-like port forwarders (`ssh -L` and `ssh -R`) over **CurveZMQ**-encrypted tunnels with a central server mediator. All traffic between two clients is relayed through a secure, encrypted ZMQ link.

---

## 🌐 Architecture Overview

```
┌─────────────────────────┐    CurveZMQ         ┌─────────────────────────┐
│                         │  (Encrypted Core)   │                         │
│   CLIENT -L Forwarder   │  ───────────────►   │   SERVER (ROUTER)       │
│   Browser → Local:20232 │                    │                          │
│                        │◄────────────────────┤   Relay all traffic    │
│                        │ ZMQ_DEALER          │                          │
└─────────────┬───────────┘                    └──────────┬────────────────┘
              │                                          │
              │                                          │
              ▼                                          ▼
       ┌───────────────┐                         ┌─────────────────────────┐
       │   TARGET:80   │                         │   CLIENT -R Forwarder   │
       │  (Remote App) │                         │  Browser → :9090        │
       │               │                         │                          │
       └───────────────┘                         ▼    ┌─────────────────────┐
                                                   Client:22 (Local SSH)
```

### Topology (from plan.md Section 2.1)
- **Server**: Central broker with `ZMQ_ROUTER` socket handling connections from all clients
- **Clients**: `ZMQ_DEALER` sockets connecting to server, plus local `ZMQ_STREAM` for TCP termination
- **Encrypted core**: CurveZML authenticates + encrypts ALL traffic between client and server

---

## 📦 Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Quick Start](#quick-start)
  - [1. Generate Keys](#1-generate-keys)
  - [2. Server Mode](#2-run-server-mode)
  - [3. Client Local Forward (-L)](#3-client-local-forwarder-l-mode)
  - [4. Client Remote Forward (-R)](#4-client-remote-forwarder-r-mode)
- [Architecture Details](#architecture-details)
  - [Message Envelopes](#message-envelopes)
  - [Reconnection & Reliability](#reconnection--rel reliability)
- [Configuration](#configuration)
- [Troubleshooting](#troubleshooting)

---

## ✨ Features

| Feature | Description | Section |
|---------|-------------|--------|
| 🔒 **CurveZMQ Encryption** | All traffic encrypted, client authenticated | 2.1, 5 |
| 🔁 **Auto-Reconnect** | Exponential backoff with heartbeats | 6 |
| ❄️ **Zero-MT** | Message multiplexing over single ZMLink | 4.1-4.2 |
| 🔄 **Reliable Forwarding** | Session resume, ACL-based routing | 7.2 |

---

## 🔧 Installation

### Prerequisites

```bash
# Install Rust toolchain  
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh  

# Clone the project  
git clone https://github.com/youruser/zmqtunnel\.git  
cd zmqtunnel  

# Build release binary  
cargo build --release  
```

### Run from source (no cargo install needed)

```bash
./target/release/zmqtunnel server  # Server mode
./target/release/zmqtunnel local -l 8080:remotehost:22  # Client -L mode  
./target/release/zmqtunnel remote -r 9090:localapp:22   # Client -R mode
```

### Binary Release (Linux) + PyInstaller for Single Binary Distribution

See `CARGO-TOML` for dependencies and build configuration.

---

## 🚀 Quick Start

### Step 1 — Generate Keys

Generate CurveZML keypairs for secure connections:

#### Server Keypair
```bash
# Generate server curve public key (long-term credential)  
./target/release/zmqtunnel keygen --output ~/.zmqtunnel/server_public\.key  

# Copy server's public key to clients
cat ~/.zmqtunnel/server_public.key > ~/client_config.txt  # Share with peers
```

#### Client Keypair  
```bash
# Generate client curve private/public keypair  
./target/release/zmqtunnel keygen --server-only  
cp ~/.zmqtunnel/client_secret\.key ~/.zmqtunnel/client_private\.key  
cp ~/.zmqtunnel/client_public\.key ~/.zmqtunnel/client_public\.key  
```

### Environment Variables (After Key Gen)
```bash
# Server needs: server's curve public key for clients to authenticate with  
export SERVER_PUBLIC_KEY=$(cat ~/.zmqtunnel/server_public.key | awk '{print $1}')  

# Client needs: server_public.key to establish encrypted tunnel, target for forwarding  
export LOCAL_PORT=20232          # port where tcp listener lives (Section 2.4)
export TARGET="localhost:80"      # remote service via -L or -R mode  
export SERVER_PUBLIC_KEY_FILE=~/.zmqtunnel/server_public.key  # auth stub
```

### Step 2 — Run Server Mode (Central Broker)

The server acts as a central mediator with ROUTER socket handling CurveZMQ authentication.

```bash
# Start the server broker (central relay from plan.md Section 5)  
./target/release/zmqtunnel server \
  --port 9876 \               # ZMQ router port (listen for client curves connections)  
  --curve-auth-key ~/server_public\.key \   # long-term key shared with clients
  --acl-file ~/.zmqtunnel/allowed_connections\.csv  # Optional ACL per plan.md Section 5.4 auth

# Server binds to localhost:9876, accepts CurveZMQ authenticated client connections  
```

Wait for the server to start (Section 2.1 topology shows broker handles sessions/tunnels routing):
```
🚀 ZMQTunnel Server starting -- Phase 3+ message envelopes ready! 
✅ ROUTER + CurveZMQ Authenticator initialized
✅ ACL system loaded
Ready to accept client connections
```

### Step 3 — Client Local Forward (-L Mode) -SSH `ssh-L` Equivalent

Forward a local TCP listener to a remote service through the server.

#### Forward Web App (`httpbin`) From Browser to Server Mediated Target

Bind locally on port 2081, forward traffic via encrypted tunnel to a browser (Section 3.1 Table):
```bash

./target/release/zmqtunnel local -l 2081:localhost:554 \
  --server-port 9876 \               # connects to server from Section 2.1  
  --curve-key ~/.zmqtunnel/client_private\.key \
  --target "httpbin.org"              # remote forward destination (Section 3.1 plan.md)

# Result: Browser → Local TCP :2081 → ZMQ_STREAM Listener  
#          → OPEN_CONN to Server (Frame[0]=VERSION, Frame[1]=msg_type)
#          → Server routes OPEN_CONN to target peer session 
#          → Target dialer connects via ZMQ dealer  
#          ↺ Bidirectional DATA frames encrypted per CurveZML + Heartbeat loop  

```

#### Forward Browser Traffic to Browser (Self-Relay from Section 2.7)

Forward browser request (local TCP listener binds port 2081):
```bash

./target/release/zmqtunnel local -l 2032:localhost:80 \  
  --server-port 9876 \     
  --curve-key ~/.zmqtunnel/client_private\.key \  
  --target "localhost:80"     # redirect browser traffic from port 80  

# Result: Browser → Local TCP :2081 → Server → Target:80 (plan.md Table 4.2 OPEN_CONN frame relay)  
```

### Step 4 — Client Remote Forward (-R Mode) -SSH `ssh-R` Equivalent

Server-side client exposes a listener; traffic forwarded back to originating client's target.

Remote listener on server port 9090 redirects to client-local app (Section 3.2 SSH `-R` equivalent):
```bash

./target/release/zmqtunnel remote -r 9090:localhost:22 \  
  --server-port 9876 \      # connect to broker server  
  --curve-key ~/.zmqtunnel/client_private\.key \ 
  --origin-target "localhostssh"    # target for peer-to-peer relay  

# Result: External → :9090 (Server listener) → OPEN_CONN to Client 
#          → Client dialer connects to localhost ssh target

```

---

## 🏗 Architecture Details

### Message Envelopes (Frame[0], Frame[1] — Section 4.1 Table Protocol Version + Type Constants)
Each relayed message uses a compact binary header:

```
Frame[0]: protocol_version:uint8   // Always VERSION=1 (Section 2.1 Frame[0])
Frame[1]: msg_type:uint8            // Message type constants from protocol.md Table 4.2 (HELLO, OPEN_CONN, etc.)
Frame[2-3]...: payload              // Msgpack dict or raw bytes (optional)
```

#### Message Types (Table 2 - Plan md Section 4.2)
| Type | Direction | Payload Fields | Description |
|------|-----------|----------------|-------------|
| `HELLO` | client → server | `client_id`, `auth_token` | Register/authenticate per Table 2.1 |
| `HELLO_ACK` | server → client | `session_id` | Confirm registration + ZAP whitelist |
| `REGISTER_FORWARD` | client → server | `mode`, `bind_addr`, `target`, `peer_id` | Declare -L/-R forward rule (plan.md Section 3/4) |
| `FORWARD_ACK` | server → client | `tunnel_id` | Forward accepted/rejected |
| `OPEN_CONN` | bidirectional | `conn_id`, `target` | New TCP connection opened (Table 2.7 flow stub) |
| `OPEN_ACK` | bidirectional | `conn_id` | Target dial succeeded/failed |
| `DATA` | bidirectional | `conn_id`, `seq` | Raw stream bytes (optional payload frame) |
| `CLOSE_CONN` | bidirectional | `conn_id` | Connection torn down |
| `PING/PONG` | bidirectional | `timestamp` | Heartbeat liveness (Section 6.2) |

---

### Reconnection & Reliability (Phase 4, Plan MD Sections 2.1-6.3)

#### Auto-Reconnect Settings (Table 6.1)
Configure these ZMQ socket options for resilient tunnels:

```python
# DEALER/ZMQ socket options from plan.md Table 6.1  
DEALER.reconnect_ivl =          250  # ms, initial delay
DEALER.reconnect_ivl_max =      30000  # max backoff (1 min cap)  
DEALER.heartbeat_ivl=        1500  # send ping every 1.5 seconds  
DEALER.heartbeat_timeout=     6000  # if no pong in 6s → dead link
DEALER.tcp_keepalive=      True
```

**Session Resumption Strategy (Option A per plan.md Section 6.3):**
- On reconnect, client sends `HELLO` with previous `session_id`.  
- Server rebinds existing tunnel state to new ZMQ identity.  
- **Important caveat**: TCP `conn_id`s cannot survive link outage → drop on reconnect; fresh connections work immediately.

**Backpressure Handling:**
ZMLink buffers can grow unbounded. Two approaches:

1. **Credit-based flow control per conn_id**: Per-connection send window limiting bytes in flight.  
2. **Monitor HWM and pause reading when downstream congested**.  

---

## 📖 Configuration

### Server Configuration (broker.py)
```ini
# ~/.zmqtunnel/server.yaml
server:
  listen_port: 9876                    # ZMQ ROUTER port for client connections (Section 2.1 topology diagram)  
  curve_key: ~/.zmqtunnel/server_public.key  # Curve private key for signing (Table 5.2)  
  zap_whitelist: ~/.zmqtunnel/allowed\.keys\.csv   # Only listed client public keys connect
```

### Client Configuration
```ini
# ~/.zmqtunnel/client.yaml  
curve_key:     ~/.zmqtunnel/client_private.key    # Section 5.2 Table auth stub
remote_port:   9876                               # broker address (Table 2.1)
```

---

## 🛠 Troubleshooting

### Connection Refused by Server
- Verify server is running (`netstat -tuln | grep 9876`)  
- Check CurveZML key matches: copy `~/.zmqtunnel/server_public.key` to client config location  

### Auto-Reconnect Fails
```python  
# Increase retry interval for flaky network  
socket.reconnect_ivl=          500   
socket.reconnect_ivl_max =      30000
socket.heartbeat_timeout=     6000
```

**ZMQ STREAM Byte-Framing Issues:**
Wrap in `StreamBridge` module per plan.md Table 4.3 (encapsulate framing logic).

### High Latency / Timeout
- Check CPU/memory on server (single router may bottleneck many clients)  
- Consider multiple brokers for HA (plan.md Phase 12)  

### Connection Teardown Not Propagating - Section 6.5 Flow Table
Ensure `CLOSE_CONN` frame is emitted from ZMQ_STREAM connect listener before closing underlying socket (plan.md Table 4.2 flow stub).

---

## 📊 Monitoring & Metrics

```python
# Track active connections, bytes sent/received, reconnects  
with open(~/zmqtunnel/metrics\.log, "a") as f:  
    f.write(f"Active connections:{active_conns}; bytes:{total_bytes};\nreconnects:{reconnect_count}\n")  
```

---

## 🔐 Security Notes

- **CurveZML**: Provides transport-level encryption + authenticity (client must know server's public key; server validates client key).
- **ACL**: Whitelist client public keys in ZAP authenticator to prevent unauthorized relays.
- **No plaintext traffic visible** on the wire — all encrypted through CurveZML.

---

## 📜 License / Attribution

Built for secure zero-trust port forwarding over ZMQ. Use responsibly — this is a research prototype demonstrating:
- CurveZMQ tunneling (Section 5)  
- Auto-reconnecting sessions (Section 6)  
- End-to-end encrypted relay (Sections 2.1-7.2)  

---

## 📚 Additional Resources

- **Plan.md** — Full implementation specification with message type tables, architecture diagrams, reliability strategies (Sections 4.2-8 testing plan).  
- **Cargo.toml** — Dependencies + build instructions including PyInstaller packaging for Linux.  

```
---END OF README---"
