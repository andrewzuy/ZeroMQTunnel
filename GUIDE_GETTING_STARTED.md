# 🔐 ZeroMQ TCP Tunnel - User Guide & Tutorial

A resilient, encrypted TCP tunnel system for NAT traversal with automatic re_connection. Perfect for exposing local services (like web servers) through intermediary machines or cloud infrastructure.

---

## 📝 Table of Contents

1. [Quick Start in 3 Steps](#quick-start-in-3-steps)
2. [Configuration Guide](#configuration-guide)
3. [How It Works](#how-it-works)
4. [Security & Authentication Explained](#security--authentication-explained)
5. [Deployment Example](#deployment-example)

---

## 🚀 Quick Start in 3 Steps

### Step 1: Install Dependencies

```bash
# Option A: System packages (recommended)
apt install python3-pyzmq libsodium-dev libzmq3-dev

# Option B: pip install (bundled libraries)
pip install pyzmq aiofiles numpy
```

### Step 2: Generate Cryptographic Certificates

```bash
cd /path/to/tunnel
python server/run_server.py --generate-certs --key-dir=/tmp/.server_curves
```

This creates:
- `server.key` (private key - keep secret!)
- `server.pub` (public key - share with clients)

### Step 3: Start the Tunnel Server

```bash
python server/run_server.py
# Output: Server listening on: control=5555, stream=443, data=5556
```

🎉 **Done!** Your server is now running. Clients can connect via TCP to port 443 to access local services.

---

## ⚙️ Configuration Guide

### Changing the Exposed Port (e.g., 8080 instead of 443)

Edit `server/run_server.py`:

```python
class ServiceAgent:
    def __init__(self):
        # ... existing code ...
        
        # Change line ~47:
        self.stream_socket.bind(f"0.0.0.0:8080")  # <-- Your preferred port
        
        # Also update firewall rules and client configs!
```

**Recommended Ports:**
- Port **443** - HTTP Secure (production web traffic)
- Port **8080** - Development/testing
- Port **1443** - If port 443 is already occupied by another service

**Alternative: Use command-line argument** (if you add `--port` support):
```bash
python run_server.py --port 8080 --control-port 5555
```

---

### Agent Configuration (Client Side)

Each client needs its own CURVE certificate pair:

```bash
# Generate agent certificate for client A
python server/run_server.py --generate-agent-certs --client=clientA

# Output files:
#   - /tmp/.server_curves/agent_clientA.key
#   - /tmp/.server_curves/agent_clientA.pub
```

**Store securely:** Keep private keys (.key) secret and never share them publicly.

---

## 🔍 How It Works (Architecture)

### Component Overview

```
┌───────────────┐    ┌───────────────────┐    ┌─────────────┐
│ Client A      │◄──control──►│ Tunnel Server Z │◄──control──►│ Agent C │
│ (port 80)     │◄──data───►│ (exposes 443)│◄──data───►│ (local:443)│
└───────────────┘    └───────────────────┘    └─────────────┘
```

### Data Flow Example

1. **Client connects:** Someone from anywhere dials your server's public IP port 443 via HTTP or custom TCP app.

2. **Server forwards:** Server routes that connection to `AgentC`, which is running a local web service on `localhost:443`.

3. **Data relayed:** All bytes between client and `A`'s service pass through server unchanged (stream proxy).

4. **Heartbeat keeps alive:** Every 5 seconds, Client sends `HEARTBEAT`; Server replies with `OK`. If no heartbeats in 30 seconds, server disconnects and closes all related connections.

### Ports Used

| Port | Purpose | Accessible From |
|------|---------|-----------------|
| **5555** | Control plane - Agent registration & heartbeats | Agents only (CURVE auth required) |
| **5556** | Internal agent data relay | Server ↔ Agent (encrypted) |
| **443** or **8080** | Public-facing TCP stream relay | Any external client |

**Port 5555 and 5556 are private - never expose them publicly!**

---

## 🔒 Security & Authentication Explained

### What is CURVE?

ZeroMQ's CURVE (Curve25519) mechanism provides:
- **Authentication:** Only approved clients can connect to server
- **Encryption:** All control traffic between server and client is encrypted
- **Key exchange:** Securely establishes shared secrets without sending keys over wire

### Certificate Lifecycle

#### Server-Side Keys (Generated Once)

```bash
python server/run_server.py --generate-certs
# Creates: /tmp/.server_curves/

/tmp/.server_curves/server.key      ← Private key (KEEP SECRET!)
/tmp/.server_curves/server.pub      ← Public key (SAFE to share)
```

#### Client/Agent Keys (Generated Per Client)

```bash
python server/run_server.py --generate-client-cert --client=new_client
# Creates: /tmp/.server_curves/client_new.key+pub

# The client MUST use these files for ZAP authentication!

# Example client startup:
python client/connect.py \
    --server-host 192.168.1.100 \
    --client-key-file .keys/client1.key \
    --client-pub-key-file .keys/client1.pub
```

**Important:** Never expose `*.key` files publicly! Upload them only via secure channels (SFTP, encrypted transfer) to the server.

### Authorization Flow

1. **Client connects** → ZeroMQ's ZAP protocol intercepts request
2. **Server checks client certificate** → Does client public key match any authorized key in registry?
3. **If yes → authentication successful**; control channel established
4. **If no → connection rejected with ZAP error**

### End-to-End Encryption Guarantee

| Connection | Encrypted | Why? |
|------------|-----------|------|
| Client ↔ Server (control) | ✅ Yes | CURVE on top of TCP, authenticated and encrypted payloads |
| Server ↔ Agent (data plane) | ✅ Yes | Same ZAP + CURVE mechanism on ROUTER/DEALER sockets |
| Client → Internet → Server | ✅ Yes | Data between client and server is encrypted in transit |

**Note:** Traffic from the server's public-facing `ZMQ_STREAM` socket (port 443) receives raw TCP streams. The *server-side* data plane encrypts these with CURVE before routing to agents. Clients need not know about this—they send raw data, server forwards it encrypted.

---

### Threat Model

**What we protect:**
- Client identity and integrity via ZAP authentication
- Encrypted channel between client and agent (prevents sniffing)
- Protection against man-in-the-middle attacks (curve25519+AEAC)

**Not protected:**
- Traffic from public-facing stream socket (port 443) is plaintext before decryption by server
- Server itself must be trusted (it handles all relay traffic)

---

### Best Practices for Key Management

#### When deploying to multiple clients:

```bash
# 1. Create a master private key once (or use system package defaults)
python server/run_server.py --generate-certs --key-dir /opt/tunnel/keys

# 2. For each client, generate and distribute keys securely:
for i in A B C D E; do
    python server/run_server.py \
        --generate-agent-certs \
        --agent-name client_$i \
        --agent-key-file "/tmp/.server_curves/client_$i.key"
done

# 3. Manually upload generated keys to each client via secure channel:
#    - Use SFTP
#    - Use scp with SSH
#    - Never email!
```

#### When rotating keys (for security incidents):

1. Generate new server certificate pair
2. Stop server, update `server.key` and `server.pub`
3. Notify all clients to download new certificate via secure channel
4. Restart server

---

## 📦 Deployment Example

### Single Machine Setup

```bash
#!/bin/bash
# deploy.sh - Install tunnel service on Debian/Ubuntu

set -e

SERVICE_NAME="zeroqmtunnel"

install_apt() {
    apt-get update && apt-get install --no-install-recommends \
        python3-pyzmq libsodium-dev libzmq3-dev build-essential > /dev/null 2>&1
}

pip_install() {
    pip install pyzmq aiofiles numpy > /dev/null 2>&1
}

create_dirs() {
    mkdir -p /opt/$SERVICE_NAME/{log,keys,config}
}

generate_certs() {
    python /opt/$SERVICE_NAME/tunnel/server/run_server.py \
        --generate-certs --key-dir=/opt/$SERVICE_NAME/keys
}

install_service() {
    install_apt || pip_install
    pip_install
    create_dirs
    generate_certs
    
    # Copy your tunnel source files into /opt/$SERVICE_NAME/tunnel (git clone, scp etc)
    
    # Create systemd unit file
    cat > /etc/systemd/system/${SERVICE_NAME}.service << 'UNITFILE'
[Unit]
Description=ZeroMQ TCP Tunnel Server
After=network.target

[Service]
Type=simple
User=nobody
WorkingDirectory=/opt/$SERVICE_NAME/tunnel
ExecStart=/usr/bin/python3 /opt/$SERVICE_NAME/tunnel/server/run_server.py --daemon
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
UNITFILE
    
    systemctl enable $SERVICE_NAME.service
    systemctl start $SERVICE_NAME.service
    
    echo "✓ ZeroMQ TCP Tunnel deployed at port 443"
    echo "Control socket: tcp://localhost:5555 (internal)"
}

install_service
```

### Access from External Clients

From a client machine:
```bash
# Connect via telnet to the service exposed port
telnet <server-ip-or-domain> 8080

# Or using curl if you mounted your local site
curl http://<server-ip>:443/

# Note: Port 443 is the external-facing one for clients!
```

### Firewall Configuration (ufw recommended)

Allow inbound connections on the exposed port:
```bash
sudo ufw allow 8080/tcp  # Or use 443 if that's your chosen port
# Allow SSH as always needed
sudo ufw allow ssh
sudo ufw enable
```

---

## 🧪 Testing End-to-End Locally (Before Public Deployment)

```bash
# Start tunnel server locally
python server/run_server.py

# Start a simple HTTP server on localhost:443
python -m http.server 443 --bind 127.0.0.1 &

# Connect via telnet from another terminal
telnet localhost 8080  
# (Will show "Connected to localhost" if everything works)

# Or use curl through tunnel:
curl http://localhost/ > /dev/null && echo "✓ Tunnel working!"
```

---

## ❓ Troubleshooting

### Issue: `Invalid argument` when generating certificates

**Solution:** Install system libsodium (ZeroMQ CURVE requires native libraries):

```bash
apt install python3-sodium libzmq3-dev
pip uninstall pyzmq -y && pip install pyzmq==25.1.0  # Use bundled version that links to libzmq3
```

### Issue: `ZAP authentication denied`

**Cause:** Client certificate not recognized by server.

**Solution:** Generate client keys matching one of your authorized registrations, or upload correct cert pair:

```bash
# On client side, using the exact files generated during setup:
python client/connect.py \
    --server-host <public-ip> \
    --key-file /root/.ssh/myagent.key \
    --pub-key /root/.ssh/myagent.pub
```

Ensure these keys were distributed via secure channel (like scp) to clients.

### Issue: `Connection reset by peer` after 30 seconds

**Cause:** Server heartbeat timeout - consider increasing `CLEANUP_TIMEOUT` in config.

Edit config, restart server with larger value.

---

## 📊 Architecture Reference

```
┌────────────────────────────────────────────────────────────┐
│                        ZeroMQ Tunnel                         │
│                                                                │
│  ┌──────────────────┐      ┌─────────────────────────────┐  │
│  │ Control Socket   │◄─────┤     ZAP Auth Handler        │  │
│  │ (ROUTER, port 5) │      │                           │  │
│  │ - Agent Reg      │◄────│ - Certificate Validation     │  │
│  │ - Heartbeat ACK  │     │ - Dead Agent Cleanup         │  │
│  └──────────────────┘      └─────────────────────────────┘  │
│                                                              │
│  ┌──────────────────────────────────────────────────────────┐│
│  │              Stream Socket (ZMQ_STREAM)                   ││
│  │                (External: port 443 or custom)             ││
│  │              - Accepts raw TCP connections from public    ││  
│  │              - Session mapping tracks active streams      ││
│  └──────────────────────────────────────────────────────────┘│
│                                                              │
│  ┌──────────────────┐      ┌─────────────────────────────┐  │
│  │ Agent Proxy     │◄─────┤ Routing & Session Table      │  │
│  │ (Dealers, port5) │      │                           │  │
│  │ - Connect local TCP sockets |                         │  │
│  │ - Relay data frames    │◄─── ────────────────────────►│  │
│  └──────────────────┘                                    │  │
└────────────────────────────────────────────────────────────┘
```

---

## 🎓 Advanced Usage: Expose Different Services

The same server can handle multiple backend services by changing registration commands:

```bash
# Example: Register two different agents (C and D) behind one tunnel
python client/register.py --host myserver.com \
    --service-id=web --backend-host=localhost --backend-port=80 \
    --agent-cert /etc/tunnel/client_web_key.key
    
python client/register.py --host myserver.com \
    --service-id=api  --backend-host=localhost --backend-port=8081 \
    --agent-cert /etc/tunnel/client_api_key.key
```

---

## 📜 License & Credits

© ZeroMQ Tunnel Project - Released with Apache License 2.0

**Based on:**
- [ZeroMQ TCP proxying docs](https://zeromq.org/faq)
- CURVE security model from libzmq

---

## 💬 Questions? Need Help?

If you encounter issues:

1. Check system logs: `journalctl -u zeroqmtunnel --no-pager`
2. Verify firewall is allowing needed ports
3. Ensure client certificates were uploaded via secure channel

For production deployments, consult your security team about CURVE key rotation procedures and ZAP authentication policies.
