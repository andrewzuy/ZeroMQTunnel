```markdown
# ZMQ Tunnel Tool — Implementation Plan

A tool providing **SSH-like local and remote port forwarding** over **CurveZMQ-encrypted, auto-reconnecting tunnels**, where all traffic between two clients is **mediated by a central server**.

---

## 1. Overview

### 1.1 Goal
Build a CLI tool (`zmqtunnel`) that runs in three roles:

- **Server (broker/relay):** Central mediator. All client-to-client traffic passes through it.
- **Client (local-forward mode):** Mimics `ssh -L` — listens locally, forwards to a remote target via the server.
- **Client (remote-forward mode):** Mimics `ssh -R` — server-side client exposes a listener; traffic is forwarded back to the originating client's target.

### 1.2 SSH Equivalents

| SSH Command | Tool Equivalent | Description |
|-------------|-----------------|-------------|
| `ssh -L 8080:target:80 server` | `zmqtunnel local -L 8080:target:80` | Local listener → forwarded to target |
| `ssh -R 9090:target:22 server` | `zmqtunnel remote -R 9090:target:22` | Remote listener on server side → forwarded to client's target |

### 1.3 Core Properties
- 🔒 **Encrypted:** CurveZMQ for all client↔server links.
- 🔁 **Reliable:** Auto-reconnect, heartbeats, message acknowledgment, session resumption.
- 🌐 **Mediated:** No direct client-to-client connections; server relays everything.
- 🧩 **Multiplexed:** Many TCP connections multiplexed over a single ZMQ link per client.

---

## 2. Architecture

### 2.1 Topology

```
                          ┌────────────────────────┐
                          │        SERVER          │
                          │   (ROUTER + Curve)     │
                          │   Session Registry     │
                          │   Connection Router    │
                          └───────────┬────────────┘
                                      │
                ┌─────────────────────┴─────────────────────┐
                │  encrypted ZMQ (CurveZMQ)                  │
        ┌───────┴────────┐                          ┌────────┴───────┐
        │   CLIENT A     │                          │   CLIENT B     │
        │ (DEALER+Curve) │                          │ (DEALER+Curve) │
        │ ZMQ_STREAM     │                          │ ZMQ_STREAM     │
        │ local listener │                          │ target dialer  │
        └───────┬────────┘                          └────────┬───────┘
                │                                            │
        ┌───────┴────────┐                          ┌────────┴───────┐
        │ Local TCP app  │                          │ Remote service │
        │ (e.g. browser) │                          │ (e.g. :80)     │
        └────────────────┘                          └────────────────┘
```

### 2.2 Socket Strategy

| Component | ZMQ Socket | Purpose |
|-----------|-----------|---------|
| Server control plane | `ROUTER` (Curve server) | Receives from all clients, routes by identity |
| Client control plane | `DEALER` (Curve client) | Persistent link to server |
| Edge TCP termination | `ZMQ_STREAM` | Terminate raw TCP at listener / dialer side |

> **Note:** `DEALER`/`ROUTER` is used for the encrypted core because CurveZMQ does **not** work with `ZMQ_STREAM`. The `ZMQ_STREAM` sockets live entirely inside each client process and never touch the network — they only bridge local TCP to the in-process logic.

---

## 3. Data Flow

### 3.1 Local Forwarding (`ssh -L`)

```
1. Client A binds ZMQ_STREAM listener on local port 8080.
2. Browser connects → ZMQ_STREAM emits connect event (stream_id).
3. Client A sends OPEN_CONN{tunnel_id, conn_id, target} to Server via DEALER.
4. Server forwards OPEN_CONN to Client B (owner of target).
5. Client B dials target:80 via outbound ZMQ_STREAM, maps conn_id ↔ stream_id.
6. Bytes flow:  Browser → ZMQ_STREAM → DATA frame → Server → DATA frame → Client B → target.
7. Reverse path symmetric.
8. On TCP close, CLOSE_CONN propagates and tears down the mapping.
```

### 3.2 Remote Forwarding (`ssh -R`)

```
1. Client A requests Server to open a listener on the server side (or on Client B).
2. Server/Client B binds ZMQ_STREAM listener on remote port 9090.
3. Inbound connection → OPEN_CONN routed back to Client A.
4. Client A dials its local target (e.g. localhost:22).
5. Bidirectional relay as above.
```

---

## 4. Protocol Design

### 4.1 Message Envelope

Every relayed message is multipart. Use a compact binary header (e.g., MessagePack or a custom struct).

```
Frame 0: [protocol_version : uint8]
Frame 1: [msg_type        : uint8]
Frame 2: [header          : msgpack dict]
Frame 3: [payload         : raw bytes]   (optional, only for DATA)
```

### 4.2 Message Types

| Type | Direction | Header Fields | Description |
|------|-----------|---------------|-------------|
| `HELLO` | client → server | `client_id`, `auth_token`, `resume_session?` | Register / authenticate |
| `HELLO_ACK` | server → client | `session_id`, `assigned_id` | Confirm registration |
| `REGISTER_FORWARD` | client → server | `mode (L/R)`, `bind_addr`, `target`, `peer_id` | Declare a forward rule |
| `FORWARD_ACK` | server → client | `tunnel_id`, `status` | Forward accepted/rejected |
| `OPEN_CONN` | bidirectional | `tunnel_id`, `conn_id`, `target` | New TCP connection opened |
| `OPEN_ACK` | bidirectional | `conn_id`, `status` | Target dial succeeded/failed |
| `DATA` | bidirectional | `conn_id`, `seq` | Raw stream bytes (payload frame) |
| `CLOSE_CONN` | bidirectional | `conn_id`, `reason` | Connection torn down |
| `PING` / `PONG` | bidirectional | `timestamp` | Heartbeat / liveness |
| `ERROR` | server → client | `code`, `message` | Protocol or routing error |

### 4.3 Identifiers
- `client_id`: stable, derived from client's Curve public key (e.g., Z85-encoded).
- `session_id`: server-assigned, used for resumption after reconnect.
- `tunnel_id`: per forward rule.
- `conn_id`: per TCP connection within a tunnel (UUID or monotonic counter scoped to tunnel).

---

## 5. Security (CurveZMQ)

### 5.1 Key Management
- Each **server** has a long-term Curve keypair. Public key is distributed to clients out-of-band (config file / QR / paste).
- Each **client** has a long-term Curve keypair. Public key registered server-side for authorization.

### 5.2 Authentication Model
1. **Transport layer:** CurveZMQ encrypts + authenticates the link (client must know server's public key; server validates client public key).
2. **Application layer:** Use `ZAP` (ZeroMQ Authentication Protocol) via `ThreadAuthenticator`:
   - `configure_curve(domain='*', location=allowed_keys_dir)` — only whitelisted client public keys may connect.
3. **Optional:** Additional `auth_token` in `HELLO` for revocable session-level auth.

### 5.3 Authorization (Forward Rules)
- Server maintains an ACL: which `client_id` may open which forwards / reach which peers.
- Reject `REGISTER_FORWARD` if not permitted.

### 5.4 Config Layout
```
~/.zmqtunnel/
├── server_public.key
├── client_secret.key
├── client_public.key
└── config.yaml
```

---

## 6. Reliability & Reconnection

### 6.1 ZMQ Built-in Resilience
- `DEALER` auto-reconnects on drop. Configure:
  - `ZMQ_RECONNECT_IVL` (e.g., 250 ms)
  - `ZMQ_RECONNECT_IVL_MAX` (exponential backoff cap, e.g., 30 s)
  - `ZMQ_HEARTBEAT_IVL`, `ZMQ_HEARTBEAT_TIMEOUT`, `ZMQ_HEARTBEAT_TTL` (ZMTP heartbeats)
  - `ZMQ_SNDHWM` / `ZMQ_RCVHWM` (tune for backpressure)
  - `ZMQ_TCP_KEEPALIVE` settings

### 6.2 Application-Level Heartbeats
- Send `PING`/`PONG` every N seconds as a higher-level liveness signal independent of ZMTP.
- If no `PONG` within timeout → mark link dead → trigger reconnect logic.

### 6.3 Session Resumption
- On reconnect, client sends `HELLO` with previous `session_id`.
- Server attempts to rebind existing tunnel state to the new ZMQ identity.
- **Important caveat:** TCP connections (`conn_id`) cannot survive a true link outage cleanly because in-flight bytes may be lost. Strategy:
  - **Option A (simple):** Drop all active `conn_id`s on reconnect; only forward *rules* (`tunnel_id`) persist. New connections work immediately. (Recommended for v1.)
  - **Option B (advanced):** Add per-connection sequence numbers + buffered resend for short outages. Higher complexity.

### 6.4 Backpressure & Flow Control
- ZMQ buffers can grow unbounded → memory risk. Implement:
  - Per-`conn_id` send window / credit-based flow control, OR
  - Monitor HWM and pause reading from the corresponding ZMQ_STREAM socket when downstream is congested.

---

## 7. Module Breakdown

### 7.1 Suggested Project Structure
```
zmqtunnel/
├── Cargo.toml
├── src/
│   ├── main.rs             # CLI entrypoint (subcommands: local, remote, server)
│   ├── cli.rs              # argument parsing with clap
│   ├── config.rs           # config + key loading
│   ├── protocol.rs         # message types, encode/decode
│   ├── crypto.rs           # curve keypair gen, ZAP setup
│   ├── server/
│   │   ├── broker.rs       # ROUTER loop, routing table
│   │   ├── registry.rs     # sessions, tunnels, ACL
│   │   └── auth.rs         # ZAP authenticator wiring
│   ├── client/
│   │   ├── agent.rs        # DEALER loop + reconnection FSM
│   │   ├── local_fwd.rs    # ZMQ_STREAM listener side (-L)
│   │   ├── remote_fwd.rs   # ZMQ_STREAM dialer side (-R)
│   │   └── conn_mgr.rs     # conn_id <-> stream_id mapping
│   ├── stream_bridge.rs    # ZMQ_STREAM helpers (events, framing)
│   └── reliability.rs      # heartbeats, backoff, flow control
├── benches/                 # benchmarks
└── tests/
```

### 7.2 Key Responsibilities

**`protocol.py`**
- `encode(msg_type, header, payload) -> list[bytes]`
- `decode(frames) -> Message`
- Constants for all message types.

**`server/broker.py`**
- Main `ROUTER` poll loop.
- Route incoming frames to destination client by looking up registry.
- Prepend correct ZMQ identity for outgoing routing.

**`server/registry.py`**
- `sessions: {session_id: ClientSession}`
- `tunnels: {tunnel_id: TunnelSpec}`
- `routes: {(client_id, conn_id): peer_client_id}`
- ACL checks.

**`client/agent.py`**
- Establish Curve-encrypted DEALER connection.
- Reconnection state machine (CONNECTING → AUTHENTICATING → READY → RECONNECTING).
- Dispatch inbound messages to local/remote forward handlers.

**`client/conn_mgr.py`**
- Bidirectional map: `conn_id ↔ zmq_stream_identity`.
- Lifecycle: open → active → closing → closed.

---

## 8. Event Loop Strategy

Choose one approach (be consistent):

- **Option 1: `zmq.Poller`** — classic, synchronous, single-threaded poll over all sockets.
- **Option 2: `asyncio` + `zmq.asyncio`** — cleaner for concurrent connection handling. **Recommended.**

### Recommended (asyncio) loop per client:
```
async def run():
    await connect_with_curve()
    await asyncio.gather(
        zmq_recv_loop(),       # server → local
        stream_recv_loop(),    # local TCP → server
        heartbeat_loop(),
        reconnect_supervisor(),
    )
```

---

## 9. Implementation Phases

### Phase 1 — Foundation
- [ ] Project scaffolding, CLI skeleton (`local` / `remote` / `server` subcommands).
- [ ] Curve keypair generation command (`zmqtunnel keygen`).
- [ ] Config loading.

### Phase 2 — Secure Transport
- [ ] Server `ROUTER` with Curve + ZAP whitelist.
- [ ] Client `DEALER` with Curve.
- [ ] `HELLO` / `HELLO_ACK` handshake.
- [ ] Echo test to verify encrypted round-trip.

### Phase 3 — Protocol & Routing
- [ ] Implement `protocol.py` (encode/decode all messages).
- [ ] Server registry + routing of `OPEN_CONN` / `DATA` / `CLOSE_CONN`.

### Phase 4 — Local Forwarding (`-L`)
- [ ] `ZMQ_STREAM` listener, connect/disconnect event handling.
- [ ] `conn_mgr` mapping.
- [ ] End-to-end: local listener → server → peer dialer → target.

### Phase 5 — Remote Forwarding (`-R`)
- [ ] Reverse direction listener setup.
- [ ] `REGISTER_FORWARD` with mode `R`.
- [ ] End-to-end remote forward test.

### Phase 6 — Reliability
- [ ] Tune ZMQ reconnect/heartbeat socket options.
- [ ] Application-level `PING`/`PONG`.
- [ ] Reconnection FSM + session resumption (Option A: drop conns, keep rules).
- [ ] Backpressure / flow control.

### Phase 7 — Hardening & UX
- [ ] ACL enforcement for forwards.
- [ ] Structured logging + metrics (active conns, bytes, reconnects).
- [ ] Graceful shutdown (drain + close conns).
- [ ] Multiple simultaneous forward rules per client.

### Phase 8 — Testing & Packaging
- [ ] Unit tests for `protocol`, `conn_mgr`, `registry`.
- [ ] Integration tests with real TCP echo / HTTP server.
- [ ] Chaos test: kill server mid-transfer, verify recovery.
- [ ] Package + distribute (PyPI / single binary via PyInstaller).

---

## 10. Testing Plan

| Test | Method |
|------|--------|
| Encryption works | Verify ZAP rejects unknown keys; capture traffic, confirm ciphertext |
| Local forward correctness | `curl` through `-L` tunnel to HTTP server |
| Remote forward correctness | Reverse-connect via `-R`, hit a service |
| Reconnection | Kill + restart server; confirm forward rules survive, new conns work |
| Concurrency | Many parallel connections (e.g., `ab`, `wrk`) |
| Large transfers | Stream multi-GB file; verify integrity (checksum) |
| Backpressure | Slow consumer; confirm no unbounded memory growth |
| Connection teardown | Abrupt client close; confirm `CLOSE_CONN` propagation |

---

## 11. Known Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| ZMQ buffer growth (no native backpressure to TCP) | Credit-based flow control per conn_id |
| In-flight data lost on reconnect | Document behavior; Option B resend buffer if needed |
| `ZMQ_STREAM` byte-stream framing complexity | Encapsulate all framing in `stream_bridge.py` |
| Curve key distribution | Provide `keygen` + clear docs; consider QR/paste helper |
| Single server = SPOF | Document; future: multiple brokers / failover |
| Head-of-line blocking on single DEALER link | Acceptable for v1; consider per-tunnel sockets later |

---

## 12. Future Enhancements
- 🔄 Multiple broker support / HA failover.
- 📊 Web dashboard for live tunnel monitoring.
- 🔐 Short-lived rotating session keys.
- 🗜️ Optional payload compression.
- 🪪 mTLS-style certificate model layered on top.
- 📦 UDP forwarding support.

---

## 14. Rust Implementation Notes

### 14.1 Async Model Choice
**Option 1: `tokio` with blocking `zmq` calls on single thread** — Recommended for v1. Matches the Python plan's poll loop structure but using tokio's executor:
```rust
#[tokio::main]
async fn run() -> Result<()> {
    let zmq = zmq::prelude::{Socket, SocketType};
    let ctx = Context::new()?;
    
    // Create ROUTER socket (blocking calls on tokio)
    let mut socket: Socket<SocketType::Router> = Socket::new(SocketType::Router, &ctx)?;
    socket.set_curve_server(true)?;
    socket.set_reconnect_ivl(250_ms())?;
    
    // Block until Ctrl-C
    blocking::block_on(server_loop(&mut socket))?;
    Ok(())
}
```

**Option 2: `tokio` async wrappers** — Available in dev builds (`zmq` crate + `zmq-async`). More complex but fully non-blocking.

**Recommendation:** Start with Option 1 (blocking calls on tokio thread pool). It's simpler, easier to debug, and scales to ~10k connections on a single core before hitting GIL-style limits. Migrate to async only if profiling shows bottlenecks.

---

### 14.2 ZMQ_STREAM in Rust
Rust's [`zmq`](https://docs.rs/zmq) crate currently treats `ZMQ_STREAM` as blocking-only. Options:
- Block on tokio threads (recommended for v1, acceptable scaling).
- Use `mio` + `tokio-io` custom wrappers for true async.
- Prototype first with a simple echo server to verify events (`connect()`, `accept()` callbacks).

**Tip:** Encapsulate all ZMQ_STREAM framing in `stream_bridge.rs` as planned, but use blocking poll in tokio tasks initially.

---

### 14.3 Auth & CurveZMQ Setup
The [`zmq`](https://docs.rs/zmq) crate supports these modes:
```rust
let socket = Socket::new(SocketType::Dealer, &ctx)?;
socket.set_curve_server(true)?;                    // Server mode
// or
socket.set_curve_client(true)?;                     // Client mode
socket.set_require_cert(CertOpt::AllowClient)?;     // Whitelist clients
```

For ZAP whitelist authentication:
- Store allowed client public keys in `~/.zmqtunnel/`.
- Use [`anyhow`](https://docs.rs/anyhow) for error propagation (e.g., "client_pubkey not in allowlist").

---

### 14.4 Session & Reconnect State Machine
Use Rust's `Enum` + `match` or [`enum_dispatch`] crate:
```rust
#[derive(Debug)]
enum ClientState {
    Connecting,
    Authenticating,
    Ready { session_id: String },
    // ... etc
}
```

On reconnect, drop all `conn_id`s (Option A) but preserve `tunnel_id` state in registry. This matches the plan's v1 design.

---

### 14.5 Flow Control Implementation
Per-`conn_id` send window approach:
```rust
struct ConnState {
    send_window: usize, // bytes allowed to queue
    sent_bytes: usize,
    paused: bool,
}
```
- When HWM is hit, drop the `RecvMessage` and signal pause upstream via a closure or channel.
- Resume reading when `recv()` succeeds again (credit-based flow control pattern).

---

### 14.6 Testing in Rust
Use [`tokio-test`] for async testing:
```rust
#[tokio::test]
async fn test_local_forward() {
    let server = start_server().await;
    let client = start_client_with_keys().await;
    
    spawn_local_listener(&client, "127.0.0.1:8080").await?;
    assert_eq!(send_data(&client, b"hello world"), Ok(12));
}
```

Integration tests use real HTTP servers (`actix-web` or `hyper`) as targets to verify end-to-end flows.

---

## 15. Comparison: Rust vs. Python for zmqtunnel v1

| Aspect | **Rust** ✓ | Python |
|--------|-----------|--------|
| **Async model** | tokio (flexible, scalable) | asyncio (simple) |
| **ZMQ binding** | `zmq` sys crate + `tokio` wrappers | `pyzmq` mature |
| **ZMQ_STREAM** | Blocking on threads (good enough) | Native support |
| **Memory safety** | ✓ Zero-cost, no GC | ✓ Refs/GC overhead |
| **Binary size** | ~5–10 MB single binary | ~30 MB (PyInstaller) |
| **Debugging** | Line-level stack traces + async proctets | Full backtrace + pdb |
| **Learning curve** | Steeper, but pays off long-term | Shallow |
| **Time to MVP** | 3–4 weeks | 1–2 weeks (can migrate later) |

**Rust wins here**: the complexity is in the protocol/relay logic, not low-level I/O. Rust's async model handles that beautifully, and zero-cost abstractions mean no hidden performance overheads on v1.
```

This gives you a complete, copy-ready blueprint. The trickiest parts will be **`ZMQ_STREAM` framing** (Phase 4) and **flow control** (Phase 6) — I'd suggest prototyping those early with a throwaway script before committing to the full structure.

