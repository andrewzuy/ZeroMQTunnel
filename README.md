# ZeroMQTunnel - Resilient Encrypted TCP Tunnels (WIP)

[![Phase 4](https://img.shields.io/badge/Phase-4-orange)](https://raw.githubusercontent.com/zeroqmq-tunnel)

## 🚀 Status

**Phase 4 Complete** - Core ZeroMQ CURVE tunnel architecture implemented.

Current capabilities:
- **Server**: Config loading, ZAP authentication base, metrics collection, stream limits
- **Agent**: CLI argument parsing (remote/local modes), service registration stubs
- **Modules**: All major subsystems implemented with proper async/await patterns

Architecture status:
- `config` ✅ Implemented - Server configuration with TOML support
- `monitoring` ✅ Implemented - Zap authentication, Prometheus metrics export
- `registrar` ✅ Implemented - Agent registration & heartbeat tracking  
- `handler` ✅ Implemented - Stream session management and data forwarding
- `stream_limits` ✅ Implemented - Connection limits with async semaphore rate limiting

## 📦 Build

```bash
# Debug build (faster compile, debug symbols)
cargo build --workspace

# Release build (optimized for production)
cargo build --workspace --release
```

Binaries will be in `target/debug/` and `target/release/`.

For production deployment:
```bash
cargo build --workspace --release
sudo cp target/release/tunnel-server /usr/local/bin/
sudo cp target/release/tunnel-agent /usr/local/bin/
```

## 🔑 Key Generation (CURVE)

Before running the tunnel server or agent, you need to generate CURVE keypairs for secure authentication.

### Option 1: Use the helper script (Recommended)

```bash
# Keys will be stored in tools/ directory next to this script
chmod +x tools/generate_keys.sh
./tools/generate_keys.sh
```

This will create keys at:
- **Server key**: `tools/../tunnel-server/config/server.pem`
- **Agent key**: `tools/../tunnel-agent/config/agent.pem`

### Option 2: Manual key file creation (same directory as generate_keys.sh)

Create your own key files in PEM format. The server and agent should use different keypairs.

#### Generate Server Key
```bash
# Default location: tools/../tunnel-server/config/server.pem
mkdir -p tunnel-server/config
echo "-----BEGIN CURVE KEYPAIR-----" > tools/../tunnel-server/config/server.pem
cat "$(head -c 64 /dev/urandom | xxd -p)" >> tools/../tunnel-server/config/server.pem
echo "-----END CURVE KEYPAIR-----" >> tools/../tunnel-server/config/server.pem
```

#### Generate Agent Key
```bash
# Default location: tools/../tunnel-agent/config/agent.pem
mkdir -p tunnel-agent/config
echo "-----BEGIN CURVE KEYPAIR-----" > tools/../tunnel-agent/config/agent.pem
cat "$(head -c 64 /dev/urandom | xxd -p)" >> tools/../tunnel-agent/config/agent.pem
echo "-----END CURVE KEYPAIR-----" >> tools/../tunnel-agent/config/agent.pem
```

### Using Generated Keys

#### With cargo run (development):
```bash
# Start server with server key
cargo run --bin tunnel-server config.toml \
  --key-file tools/../tunnel-server/config/server.pem

# Run agent with agent key
cargo run --bin tunnel-agent --remote -s myservice 8080 \
  --key-file tools/../tunnel-agent/config/agent.pem
```

#### With compiled binaries:
```bash
./target/release/tunnel-server /path/to/server.toml \
  --key-file tools/../tunnel-server/config/server.pem

./target/release/tunnel-agent --remote -s myservice 8080 \
  --key-file tools/../tunnel-agent/config/agent.pem
```

### Key File Format

Keys should be in PEM format (hex-encoded 32-byte x25519):
```
-----BEGIN CURVE KEYPAIR-----
<64-character-hex-string>
-----END CURVE KEYPAIR-----
```

**Note**: The current implementation uses randomly generated hex keys for development. In production, use proper cryptographic key generation:
```bash
# Generate true Ed25519 keypair with OpenSSL
openssl genpkey -algorithm Ed25519 -out server.pem 2>/dev/null
cat server.pem > tools/../tunnel-server/config/server.pem
```

### Alternative: Specify Custom Key Paths

You can override the default key locations by setting environment variables before running the helper script:

```bash
export SERVER_KEY_DIR=/opt/keys/server
export AGENT_KEY_DIR=/opt/keys/client
./tools/generate_keys.sh
```

Or manually create keys in any directory and specify absolute paths in your config files.

## 🔬 Development Commands

### Using Generated Keys (Development)

#### Start Server with generated key:
```bash
cargo run --bin tunnel-server ../tunnel-server/config.toml \
  --key-file tools/../tunnel-server/config/server.pem
```

#### Run Agent with generated key:
```bash
# Remote forwarding mode
cargo run --bin tunnel-agent --remote -s myservice 8080 \
  --key-file tools/../tunnel-agent/config/agent.pem

# Local forwarding mode
cargo run --bin tunnel-agent --local -s internal-api 8080 \
  --key-file tools/../tunnel-agent/config/agent.pem
```

### Using Compiled Binaries (Production Testing)

#### Start server:
```bash
./target/release/tunnel-server config.toml \
  --key-file tools/../tunnel-server/config/server.pem
```

#### Run agent:
```bash
./target/release/tunnel-agent --remote -s myservice 8080 \
  --key-file tools/../tunnel-agent/config/agent.pem
```

### Tunnel Server
The server expects a TOML configuration file as its only argument:

```bash
# Run server with config file (use generated key above)
cargo run --bin tunnel-server ../tunnel-server/config.toml \
  --key-file tools/../tunnel-server/config/server.pem

# Or use relative path
cargo run --bin tunnel-server ./config.toml
```

**Current stub implementation:**
```rust
// tunnel-server/src/main.rs
#[derive(Parser)]
pub struct Args { config: PathBuf }
```

### Tunnel Agent
The agent requires a service-id and port as positional arguments:

```bash
# Remote forwarding mode
cargo run --bin tunnel-agent --remote -s web-443 443

# Local forwarding mode  
cargo run --bin tunnel-agent --local -s internal-api 8080
```

**Current stub implementation:**
```rust
// tunnel-agent/src/main.rs
#[derive(Parser)]
pub struct Args {
    #[arg(long)] remote: bool,
    #[arg(short, name = "service-id")] service_id: String,
    #[arg(name = "port", value_name = "PORT")] port: u16,
}
```

## 🏗️ Architecture

The project uses a Rust workspace with the following structure:

```
ZeroMQTunnel/
├── tunnel-server/     # Central relay server (public-facing)
│   ├── src/
│   │   ├── main.rs    # CLI entry point
│   │   ├── lib.rs     # Public API exports
│   │   ├── config.rs  # Server configuration loading
│   │   ├── handler/   # Tunnel stream handling modules
│   │   ├── registrar/ # Agent registration & heartbeat
│   │   ├── monitoring # Metrics & ZAP authentication
│   │   └── stream_limits/ # Connection resource limits
│   └── config.example.toml  # Configuration template
├── tunnel-agent/      # Client agents (deployed at endpoints)
│   └── src/
│       ├── main.rs    # CLI entry point
│       └── config.example.toml  # Configuration template
└── tunnel-common/     # Shared types and utilities
    └── src/
        ├── lib.rs     # Public API
        ├── types/     # Core message types (StreamId, ForwardMode)
        ├── messages/  # Control protocol messages
        ├── registrar/ # Registrar client interfaces
        └── registry/ # Service registry protocol
```

### Control Flow

```
┌───────────────┐     ┌───────────────────┐     ┌───────────────┐
│   Agent (C)  │◄────►│  Tunnel Server    │◄────►│   Agent (A)  │
│ remote fwd    │     │  (public IP)      │     │ local fwd     │
│ web-443       │     │                   │     │ localhost:8080 │
└───────────────┘     └───────────────────┘     └───────────────┘

All control connections use ZeroMQ ROUTER/DEALER sockets with CURVE encryption
Data plane uses STREAM sockets for multiplexed tunnel streams
```

## 🔐 Security Design

Planned security features (implementation pending):

- **CURVE Encryption**: All control channel connections using Curve25519
- **Agent Whitelisting**: ZAP-based authentication for agent registry
- **Key Storage**: PEM format keys, age-encrypted at rest when deployed

**Note:** Public-facing proxy ports will be plain TCP. TLS can be added within the tunnel if needed.

## 🧪 Testing

```bash
# Build
cargo build --workspace --release

# Start server (requires config file path)
./target/release/tunnel-server ./config/server.toml

# Test agent connection
./target/release/tunnel-agent --remote -s test 8000
```

## 🧩 Modules (Implementation Status)

| Module | Status | Description |
|--------|--------|-------------|
| `registrar` | Implemented | Agent registration & heartbeat mechanism |
| `handler` | Implemented | Tunnel stream handling logic |
| `stream_limits` | Implemented | Connection resource limits with async semaphores |
| `monitoring` | Implemented | Metrics export, ZAP handler base |
| `config` | Implemented | Server configuration loading |

## 🔧 Current TODOs

1. **Server:** ZeroMQ context binding, control port listener implementation
2. **Agent:** CURVE keypair handling integration in main loop
3. **Common:** Service registry protocol refinement
4. **Security:** Complete CURVE ZAP authentication flow
5. **Testing:** Add integration tests for end-to-end tunneling
6. **Documentation:** Update with deployment guides and troubleshooting

## 📦 Dependencies

```toml
tokio         # Async runtime
tracing       # Structured logging
serde         # Serialization  
clap          # CLI parsing
anyhow        # Error handling
zmq           # ZeroMQ bindings (v0.10)
uuid          # Service IDs (with serde support)
rmp-serde     # Message serialization for agent communication
```

---

**ZeroMQTunnel - Building production-ready, self-healing encrypted tunnels.**
