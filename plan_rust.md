# Production‑Ready Rust Implementation Plan: Resilient Encrypted TCP Tunnels with ZeroMQ

## 1. Overview & Goals

Build a production‑grade, self‑healing TCP tunnel broker in Rust that allows clients behind NAT to securely reach services on other NATed hosts. The system uses a central, publicly reachable server that mediates all connections, and supports **both local and remote port forwarding** (as in OpenSSH) so that users can decide where the listening port lives.

**Target features:**
- **Remote forwarding** (`-R` style): A service‑side agent (e.g., on host C) instructs the server to listen on a public port and relay all incoming TCP connections back to C’s local service.
- **Local forwarding** (`-L` style): A client‑side agent (e.g., on host A) opens a local TCP listener; connections to that port are tunnelled through the server and forwarded to a remote service (e.g., C:443) that is reachable via the server.
- **All communication encrypted** end‑to‑end using Curve25519‑based key exchange and authenticated encryption (ZeroMQ CURVE).
- **Automatic reconnection** on network disruption, with seamless re‑registration and session cleanup.
- **Concurrent connection multiplexing** – multiple independent TCP streams share the same encrypted tunnel.
- **Robust error handling** – no silent data loss, proper backpressure, resource limits, graceful shutdown.

The prototype proved the architecture in Python; this plan translates it to a fast, safe, and maintainable Rust implementation with async I/O.

---

## 2. Technology Stack (Rust Edition)

| Component               | Choice                                             | Rationale                                                                               |
|-------------------------|----------------------------------------------------|-----------------------------------------------------------------------------------------|
| Async runtime           | **Tokio** (latest stable)                          | Industry standard, rich ecosystem, best‑in‑class TCP/UDP, timers, and I/O drivers.      |
| ZeroMQ integration      | **`zmq` crate** + async bridge                    | Bindings to battle‑tested `libzmq`; `CURVE`, `ZMQ_STREAM`, auto‑reconnect all mature.   |
|                         | (Fallback: pure‑Rust `zeromq` crate for evaluation)| Might replace `zmq` if it matures, simplifying async and removing native dependency.    |
| Serialisation           | **`rmp‑serde` (MessagePack)**                      | Compact, binary, language‑agnostic; ideal for control messages.                         |
| Identity / crypto keys  | **`x25519‑dalek` + `ed25519‑dalek`**               | Generate and manage Curve25519 keypairs; keys exported in standard formats.             |
| Unique session IDs      | **`uuid` crate (v4)**                              | Uncoordinated, collision‑resistant identifiers for streams.                             |
| Configuration           | **`toml` + `serde`**                               | Human‑readable config files for server and agents.                                      |
| CLI                     | **`clap` v4**                                      | Ergonomic argument parsing with derive macros.                                          |
| Logging / Metrics       | **`tracing` / `opentelemetry`**                    | Structured, async‑aware diagnostics; metrics exported for monitoring.                   |
| Error handling          | **`anyhow` / `thiserror`**                         | Convenient error propagation and custom error types.                                    |

---

## 3. Architecture Design

### 3.1 Modes of Operation – Local vs. Remote Forwarding

The agent can be started in one of two forwarding modes:

#### Remote Forwarding (`-R`)
*Used when the service is behind NAT and you want to expose it on the public server.*
- Agent runs on the **service host** (e.g., computer C).
- It registers with the tunnel server, saying: *“Please listen on public port **P** and forward all connections to my local **host:port**”*.
- The server opens a TCP listener on `0.0.0.0:P`.
- External clients (A, B) simply connect to `server:P` as if it were the real service.

#### Local Forwarding (`-L`)
*Used when the client is behind NAT and you want a local port that reaches a remote service via the tunnel.*
- Agent runs on the **client host** (e.g., computer A).
- It opens a local TCP listener on `localhost:L`.
- On startup it tells the server: *“I’ll handle streams for a remote service **R:port**”* (the server must already have a way to reach that service, typically via another agent running remote forwarding).
- When a local app connects to `localhost:L`, the agent creates a new tunnel session, sends the data to the server, and the server forwards it to the ultimate destination (e.g., C:443) using its routing table.

> In practice, the server always acts as a session broker. An agent can be either a *producer* of sessions (remote forward) or a *consumer* (local forward). The protocol design unifies both.

### 3.2 Component Diagram
```
┌──────────────┐ ┌─────────────────────┐ ┌──────────────┐
│ Agent (C) │◄────►│ Tunnel Server │◄────►│ Agent (A) │
│ remote fwd │ │ (public IP) │ │ local fwd │
│ │ │ │ │ │
│ local:443 ◄──┤ │ public:1443 (rem.) │ │ local:8080◄──┤
│ │ │ │ │ │
└──────────────┘ └─────────────────────┘ └──────────────┘
```
### 3.3 Control Plane

- **Protocol**: DEALER/ROUTER over TCP, secured with CURVE.
- **Server** binds a `ROUTER` socket on a control port (e.g., `5555`).
- **Agent** connects using a `DEALER` socket (asynchronous, non‑blocking).
- **Message frames** (multi‑part ZMQ message):
  1. Empty frame (delimiter for ROUTER compatibility)
  2. Message type (string): `REGISTER`, `UNREGISTER`, `HEARTBEAT`, `STREAM_START`, `STREAM_DATA`, `STREAM_CLOSE`, etc.
  3. Payload: serialised with MessagePack.
- **Registration payload** includes:
  - `forward_type`: `"remote"` | `"local"`
  - `service_id`: persistent, human‑readable identifier (e.g., `"web‑443"`)
  - For remote: `public_port` to listen on the server, `local_host`, `local_port`.
  - For local: `remote_service_id` (the `service_id` of a remote‑forwarded service), `local_port`.

### 3.4 Data Plane

- **Server**:
  - For remote forwards: binds a `ZMQ_STREAM` socket on the requested public port.
  - For all agent communication: binds a `ROUTER` socket (`5556`) used for streaming data and control (the same socket can carry both, or separate; we use the same for simplicity).
- **Agent**:
  - Always connects a `DEALER` data socket to `tcp://server:5556`, using CURVE.
  - For remote forward: manages TCP connections to the local target; maps `session_id` ↔ `TcpStream`.
  - For local forward: runs a `TcpListener`; for each incoming connection, it creates a new `session_id` and begins piping data to/from the data socket.

### 3.5 Security Model

- All ZMQ connections authenticated via CURVE: server public key is well‑known; agents’ public keys are whitelisted on the server (using ZAP or custom handler).
- The public‑facing `ZMQ_STREAM` ports (remote forwards) are **not encrypted**; they accept plain TCP. The agent‑to‑server hop is always CURVE‑encrypted. Trust the server (it can see plaintext).
- For true end‑to‑end encryption, applications can add TLS inside the tunnel—this is out of scope but architecturally trivial.

---

## 4. Phased Implementation Plan

### Phase 1 – Foundation & Control Protocol (Weeks 1‑3)

**Objective:** Establish a secure control channel, key management, and service registration for both forwarding modes.

1. **Project skeleton**
   - Cargo workspace with three crates: `tunnel-server`, `tunnel-agent`, `tunnel-common`.
   - Dependencies: `zmq`, `tokio`, `rmp-serde`, `serde`, `clap`, `tracing-subscriber`, `uuid`, `anyhow`.
   - Simple CLI for server and agent with `--config` flag.

2. **Key generation & storage**
   - Use `x25519-dalek` to generate long‑term keypairs for server and each agent.
   - Store keys in age‑encrypted PEM‑like files; provide a `genkey` subcommand.
   - Convert keys to Z85 format (`zmq::CurveKeyPair`) for use with `libzmq`.

3. **Control socket implementation**
   - Server: `zmq::ROUTER` bound to `tcp://*:5555`, set `ZMQ_CURVE_SERVER=1`, load secret key.
     - Custom ZAP handler (or allow‑all for testing, later implement strict whitelist).
   - Agent: `zmq::DEALER` connecting to server, configured with `ZMQ_CURVE_SERVERKEY` and its own keypair.
   - Implement a simple request‑response protocol over DEALER/ROUTER with message framing.
     - Helper functions in `tunnel-common` to send/receive typed messages using `rmp-serde`.

4. **Service registration**
   - Define `RegistrationRequest` and `RegistrationResponse` structs.
   - Agent sends `REGISTER` with its forwarding details.
   - Server maintains an in‑memory registry: `HashMap<ServiceId, ForwardingRule>`.
   - On successful registration, server opens the public listener if the rule is a remote forward.
   - Return `OK` or error.
   - `UNREGISTER` removes the rule and closes listener.

5. **Basic heartbeat skeleton**
   - Agent spawns a periodic timer that sends `HEARTBEAT` message; server replies `HEARTBEAT_ACK`.
   - Agent measures round‑trips; if no ACK after 3 intervals, declare connection dead and begin reconnection.

**Deliverable:** Server and agent can establish a CURVE‑encrypted control session, register a remote forward (server opens a TCP port) and a local forward (server updates routing), and respond to heartbeats.

### Phase 2 – Data Plane & Stream Multiplexing (Weeks 4‑6)

**Objective:** Move real TCP payloads through the tunnel, supporting concurrent streams in both forwarding modes.

1. **Server data socket**
   - Bind a second `zmq::ROUTER` socket on `tcp://*:5556` with CURVE.
   - For each remote forward rule already active, bind a `zmq::STREAM` socket on the advertised port (e.g., `0.0.0.0:1443`).
   - The `STREAM` sockets are polled together with the `ROUTER` sockets using a custom async‑compatible event loop: spawn a dedicated thread for `zmq::poll` and use a `tokio::sync::mpsc` channel to deliver events to the async world. *(We implement a lightweight `ZmqPoller` that runs a `zmq::Poller` on a background thread and sends readiness notifications.)*

2. **Session mapping**
   - On the server, when a new connection arrives on a `STREAM` socket, `zmq` sends `[identity, data]`. The identity is unique per connection.
   - Generate a new `session_id` (UUIDv4), map it to the `(agent_identity, stream_identity)`.
   - Send the first data frame to the appropriate agent over the data ROUTER socket, using multipart message: `[agent_identity, b"", session_id, data]`.
   - Forward subsequent frames the same way.
   - When an agent sends data for a session, look up the `stream_identity` and forward to the `STREAM` socket.
   - On agent‑initiated `STREAM_CLOSE`, send a zero‑length message to the `STREAM` identity to close the TCP connection.

3. **Agent data handling – remote forward**
   - Maintain a `HashMap<SessionId, tokio::net::TcpStream>`.
   - On receiving data from the server for an unknown session, create a new `TcpStream` to the configured local target (`127.0.0.1:443`).
   - Spawn a task that reads from the `TcpStream` and writes to the data DEALER socket (via an async‑safe sender channel that serialises ZMQ access).
   - Similarly, spawn a task that reads from the DEALER (via a receiver) and writes to the `TcpStream`.
   - When either side closes, send `STREAM_CLOSE` and clean up.

4. **Agent data handling – local forward**
   - The agent starts a `TcpListener` on the requested local port.
   - For each incoming connection, generate a new `session_id`.
   - Send a `STREAM_START` message to the server, containing the `session_id` and the `remote_service_id` (so the server knows where to route it).
   - Then proceed with the same bidirectional piping as above, but now the agent is the initiator of the stream.
   - The server, upon receiving `STREAM_START` for a local forward, looks up the target agent that registered the requested `remote_service_id` and relays data similarly, as if it were a remote forward stream.

5. **Backpressure & flow control**
   - Use bounded channels between the ZMQ background thread and async tasks. Apply `zmq::SNDHWM`/`RCVHWM` on sockets to avoid uncontrolled memory usage.
   - On `TcpStream` write errors, close the corresponding tunnel session.

**Deliverable:** A working bidirectional TCP tunnel for both remote and local forwarding modes, with multiple concurrent connections flowing correctly.

### Phase 3 – Resilience & Self‑Healing (Weeks 7‑9)

**Objective:** Make the system survive network outages, server restarts, and agent crashes without user intervention.

1. **Automatic reconnection**
   - Set ZeroMQ socket options: `ZMQ_RECONNECT_IVL=1000`, `ZMQ_RECONNECT_IVL_MAX=30000`, `ZMQ_TCP_KEEPALIVE=1`.
   - When the agent detects heartbeat failure (no ACKs for 3 consecutive intervals), it explicitly disconnects the socket to force a clean reconnect sequence.

2. **Re‑registration logic**
   - In the agent’s reconnection handler, after the DEALER socket successfully reconnects (or when the heartbeat resumes), automatically send a fresh `REGISTER` message for all previously active forwarding rules.
   - The server must treat a re‑REGISTER as a replace operation: atomically update the rule and, if necessary, rebind the public listener (closing old STREAM socket and opening a new one if the port changed).
   - Persist the agent’s active rules in memory; if the agent process is restarted, it reads its config and re‑registers from scratch.

3. **Session cleanup on disconnect**
   - On the server, if an agent’s heartbeat misses 3 intervals, consider the agent dead:
     - Remove all its registrations.
     - Close all associated `STREAM` connections (send zero‑length to each identity).
     - Discard any pending session mappings for that agent.
   - On the agent, when it detects disconnection, shut down all local `TcpStream`/`TcpListener` tasks with a custom error. This releases OS resources immediately. Applications connecting via the tunnel will see connection resets, but they can retry.

4. **Graceful shutdown**
   - Implement signal handling (`SIGINT`, `SIGTERM`) using `tokio::signal`.
   - On shutdown, server sends `SHUTDOWN` control message to all agents, closes all listening sockets, waits for pending data writes to complete, and then exits.
   - Agents clean up local resources and terminate.

**Deliverable:** System survives arbitrary network hiccups and server/agent restarts; new TCP connections can be established within seconds after recovery.

### Phase 4 – Production Hardening & Observability (Weeks 10‑12)

**Objective:** Prepare the system for real deployment with monitoring, performance tuning, and configuration flexibility.

1. **Connection limits & resource control**
   - Per‑agent limits on maximum concurrent streams (configurable).
   - Global limit on total `STREAM` connections the server will accept.
   - Set `ZMQ_MAX_SOCKETS` appropriately; enforce at application level with semaphores.

2. **CURVE whitelist (ZAP handler)**
   - Implement a real ZAP handler that reads authorised public keys from a configuration file (can be reloaded on SIGHUP).
   - Reject unauthorised agents with a clear error message.

3. **Structured logging & tracing**
   - Use `tracing` with spans for each session and agent identity.
   - Log key events: registration, stream start/end, errors, reconnections.
   - Export metrics (active sessions, bytes transferred, reconnect counts) via `opentelemetry` to Prometheus.

4. **Configuration file format (TOML)**
   - Server config: listen addresses, key paths, whitelist, port ranges for remote forwards, timeouts, connection limits.
   - Agent config: server address, forwarding rules (list of `{type, service_id, local/remote ports}`), heartbeat interval, key paths.

5. **Benchmarking & performance**
   - Use `criterion` to measure stream throughput and latency.
   - Optimise critical paths: reduce copies, pre‑allocate buffers, use `bytes` crate for data frames.
   - Ensure the ZMQ polling thread does not become a bottleneck—consider multiple poller threads for large numbers of streams.

**Deliverable:** A fully monitored, configurable, and resource‑safe tunnel broker ready for pilot deployment.

### Phase 5 – Testing & Security Audit (Weeks 13‑15)

**Objective:** Achieve confidence in reliability and security through rigorous testing.

1. **Unit & integration tests**
   - Extensive tests for message framing, session mapping, registration replacement, reconnection logic.
   - Integration tests that spin up a real server and two agents, create local/remote forwards, and pump traffic (HTTP requests) through them.

2. **Fault injection**
   - Use `tc` netem (or a Rust‑based chaos proxy) to simulate packet loss, latency, and connection drops.
   - Verify heartbeats trigger correctly, re‑registration succeeds, and no data corruption occurs.

3. **Fuzzing**
   - Fuzz the control message parser with `cargo-fuzz` (libfuzzer).
   - Send malformed multipart messages to the ROUTER sockets to ensure robust error handling.

4. **Security review**
   - Audit CURVE key handling: are keys zeroised on drop? (use `secrecy` crate).
   - Verify ZAP handler cannot be bypassed.
   - Check for potential resource exhaustion (DoS) via huge number of registrations or streams.
   - Validate that all `unsafe` blocks (if any, mainly in `zmq` bindings) are minimal and reviewed.

5. **Documentation & packaging**
   - Write man pages for server and agent.
   - Produce systemd unit files.
   - Provide Docker images for the server and a quick‑start guide.

**Deliverable:** A hardened, well‑tested release candidate with documentation suitable for production deployment.

---

## 5. Key Implementation Considerations in Rust

- **Async ↔ Synchronous bridging**: `zmq` is synchronous; we’ll design a dedicated `ZmqDriver` that runs `zmq::poll` on a `std::thread` and communicates with the Tokio world via `tokio::sync::mpsc`. All ZMQ socket operations happen on that thread to avoid contention. This is a well‑understood pattern and allows us to benefit from `libzmq`’s proven stability.
- **Memory safety**: Use `bytes::Bytes` for data frames to avoid unnecessary copies and maintain zero‑cost slicing. All TCP streams are handled with `tokio::net`, which is safe and async.
- **Error handling**: Custom `tunnel_common::TunnelError` enum that covers ZMQ, I/O, serialisation, and protocol errors. Every `Result` is propagated; all tasks that fail log the error and clean up resources gracefully.
- **Configuration reload**: For agent whitelist, we can implement an in‑place `Arc<RwLock<Whitelist>>` that gets updated on SIGHUP without restarting the server.

---

## 6. Conclusion

This phased plan delivers a production‑ready, Rust‑based replacement for SSH tunnels with the added reliability of automatic reconnection and the flexibility of both local and remote port forwarding. By building on mature technologies like `libzmq`, Tokio, and Curve25519, we can achieve high performance, strong security, and maintainability. The modular architecture allows incremental delivery: a working prototype after Phase 2, full resilience in Phase 3, and a battle‑hardened release in Phase 5.
