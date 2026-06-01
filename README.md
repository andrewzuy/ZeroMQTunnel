# ZeroMQTunnel - Resilient Encrypted TCP Tunnels (WIP)

[![Phase 4](https://img.shields.io/badge/Phase-4-orange)](https://raw.githubusercontent.com/zeroqmq-tunnel)

## 🚀 Status

**In Development** - Core architecture implemented, CLI requires additional functionality.

Current capabilities:
- Basic CLI argument parsing (server takes config file, agent has remote/local modes)
- Project structure for ZeroMQ CURVE tunnel system
- Modules prepared for: config loading, ZAP handler, stream limits, session tracking

## 📦 Build

```bash
# Debug build
cargo build --workspace

# Release build  
cargo build --workspace --release
```

Binaries will be in `target/debug/` and `target/release/`.

## 🔬 Development Commands

### Tunnel Server
The server expects a TOML configuration file as its only argument:

```bash
# Run server with config file
cargo run --bin tunnel-server /etc/tunnel/server.toml

# Or use relative path
cargo run --bin tunnel-server ./config/server.toml
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
│   └── src/
│       ├── main.rs    # CLI entry point (config file argument)
│       ├── lib.rs     # Public API
│       ├── config.rs  # Server configuration
│       ├── registrar/ # Agent registration & heartbeat
│       ├── handler/   # Tunnel stream handling
│       ├── monitoring # Metrics & tracing
│       └── stream_limits/ # Connection resource limits
├── tunnel-agent/      # Client agents (deployed at endpoints)
│   └── src/
│       └── main.rs    # CLI entry point (remote/local mode)
└── tunnel-common/     # Shared types and utilities
    └── src/
        ├── lib.rs     # Public API
        ├── types/     # Core message types
        ├── messages/  # Protocol messages
        ├── registrar/ # Registrar client types
        └── registry/  # Service registry types
```

### Control Flow

```
┌───────────────┐     ┌───────────────────┐     ┌───────────────┐
│   Agent (C)  │◄────►│  Tunnel Server    │◄────►│   Agent (A)  │
│ remote fwd    │     │  (public IP)      │     │ local fwd     │
│ web-443       │     │                   │     │ localhost:8080 │
└───────────────┘     └───────────────────┘     └───────────────┘

All control connections use ZeroMQ (CURVE encryption TBD)
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

## 🧩 Modules (Work in Progress)

| Module | Status | Description |
|--------|--------|-------------|
| `registrar` | Stub | Agent registration & heartbeat mechanism |
| `handler` | Stub | Tunnel stream handling logic |
| `session_tracker` | Not created | Session lifecycle tracking |
| `stream_limits` | Stub | Connection resource limits |
| `monitoring` | Implemented | Metrics export, ZAP handler base |
| `config` | Implemented | Server configuration loading |

## 🔧 Current TODOs

1. **Server:** Implement config parsing, ZeroMQ context binding, control port listener
2. **Agent:** Implement ZeroMQ connection, CURVE keypair handling, remote/local mode logic  
3. **Common:** Implement shared message types, service registry protocol
4. **Security:** Add CURVE ZAP authentication, agent whitelist validation
5. **CLI:** Add `genkey` subcommand for key generation

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
