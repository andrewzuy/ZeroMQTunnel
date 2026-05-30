# Implementation Plan: Self-Healing Encrypted TCP Tunnels with ZeroMQ

## 1. Overview and Requirements

We need a resilient, encrypted TCP tunneling solution that allows multiple clients behind NAT to access a service running on another NATed computer. A central, publicly reachable server mediates the connections and provides automatic reconnection when the underlying transport drops—overcoming the common fragility of SSH tunnels.

**Key goals:**
- **NAT traversal**: All endpoints (A, B, C) initiate outbound connections to a well-known server (Z). No inbound firewall rules required.
- **Encryption**: All tunneled traffic must be encrypted end-to-end.
- **Service exposure**: Computer C exposes its local port 443 to the server; computers A and B can then connect to a server-side port (e.g., 1443) as if they were connecting directly to C.
- **Self-healing**: Temporary network interruptions are handled automatically—the tunnel re-establishes itself and resumes accepting new connections without manual intervention.
- **Multiplexing**: Multiple concurrent TCP connections from A and B (or even from a single client) must be supported and properly isolated.

The functionality mirrors SSH remote port forwarding (`ssh -R`) but with built-in reliability.

---

## 2. Technology Stack

| Component          | Choice                                       | Rationale                                                                                   |
|---------------------|----------------------------------------------|---------------------------------------------------------------------------------------------|
| Messaging library   | **ZeroMQ (libzmq)**                         | Built‑in async I/O, message routing patterns, automatic reconnection, and CURVE security.  |
| Encryption          | **ZeroMQ CURVE** (curve25519 + salsa20)     | Integrated, peer‑to‑peer authenticated encryption without external PKI.                    |
| TCP proxying        | **ZMQ_STREAM** sockets                      | Native handling of raw TCP streams within ZeroMQ, preserving TCP framing.                  |
| Serialisation       | **MessagePack** or custom length‑prefixed   | Lightweight binary framing for control messages.                                            |
| Heartbeat / keep‑alive | ZeroMQ `ZMQ_HEARTBEAT` + application‑level pings | Detect dead connections and trigger re‑registration.                               |
| Language            | Python or C/C++ (reference implementation)  | Python’s `pyzmq` for rapid prototyping, C++ for production performance.                     |

---

## 3. Architecture Design

### 3.1 System Components
```
[Computer A] [Server Z] [Computer C]
localhost public IP localhost
┌──────────────┐ ┌────────────────┐ ┌──────────────┐
│ Agent (cli) │◄──control──►│ Tunnel Server │◄──control──►│ Agent (svc) │
│ │◄────data────│ (ROUTER/STREAM) │◄────data────│ │
│ TCP client → │ │ port 1443 ←TCP │ │ TCP → :443 │
└──────────────┘ └────────────────┘ └──────────────┘
```

- **Tunnel Server (Z)** – Central broker running a control plane and a data plane.
- **Agent (service side, C)** – Connects to the server, registers a forwarding rule, and accepts inbound tunneled streams to relay to the local service.
- **Agent (client side, A/B)** – Connects to the server, optionally requests a local listener (or uses a local TCP connector), and relays TCP streams to the server’s exposed port.

Note: In the simplest “remote forward” scenario, A and B do not need a special local agent—they just connect directly to server Z’s public port. However, if A/B also desire automatic reconnection on their side (e.g., to mask server restarts), they can run a thin local agent that provides a stable local endpoint and reconnects to Z.

### 3.2 Control Plane

- **Protocol**: REQ/REP or DEALER/ROUTER over TCP, encrypted with CURVE.
- **Server** binds a `ROUTER` socket on a known port (e.g., `5555`).
- **Agents** connect using a `DEALER` socket (asynchronous, allows heartbeats).
- **Message types** (framed in multi‑part ZeroMQ messages):
  1. `REGISTER_FORWARD <service_id> <local_host> <local_port>`
  2. `UNREGISTER_FORWARD <service_id>`
  3. `HEARTBEAT` / `HEARTBEAT_ACK`
  4. `STREAM_START <session_id>` (from server to service agent when a new incoming TCP connection arrives)
  5. `STREAM_DATA <session_id> <data_frame>` (bidirectional)
  6. `STREAM_CLOSE <session_id>`
  
- All control messages are authenticated and encrypted by the CURVE mechanism; the server validates client public keys against a whitelist (or uses the ZAP handler).

### 3.3 Data Plane

The data plane uses **ZMQ_STREAM** sockets to move raw TCP octets.

**On the server:**
- Bind a `ZMQ_STREAM` socket to the publicly exposed port (e.g., `0.0.0.0:1443`).
- This socket accepts TCP connections from external clients (e.g., A and B).
- ZeroMQ delivers each TCP connection’s data as multipart messages: `[identity, data]` (identity is a unique binary blob assigned by ZMQ for that connection).
- The server maintains a mapping `(server_side_identity → session_id)` and forwards data to the appropriate agent’s data channel.

**To communicate with agents, the server uses a second socket pair:**
- **Agent data socket**: Each agent opens a `DEALER` socket (or a `STREAM` client) to the server’s data port (e.g., `5556`).  
  - A `DEALER` socket gives fair‑queued routing and can carry arbitrary message frames; we can prefix each message with a `session_id` routing frame.
- The server uses a `ROUTER` socket bound to `5556` to receive agent connections. This socket identifies agents by their CURVE public key (or a configured identity).

**On the service agent (C):**
- Maintains a pool of ordinary TCP sockets to the local target (`127.0.0.1:443`).
- When a `STREAM_START <session_id>` control message arrives (or the first data frame for a new session), the agent opens a new TCP connection to the local service.
- Data frames from the server with that `session_id` are written to the TCP socket; data read from the TCP socket is wrapped in `STREAM_DATA <session_id> <data>` and sent back via the data `DEALER` to the server.

This design keeps TCP stream semantics intact: the `ZMQ_STREAM` on the server handles TCP framing, and the pair of `DEALER`/`ROUTER` sockets carry the stream as a series of messages.

**Alternative simplification** – If we want to avoid ZMQ_STREAM on the agent side and use a simpler approach, we can implement the data plane entirely with `DEALER`/`ROUTER` and perform the TCP bridging with plain TCP sockets + poller. The plan will describe both, but for production the `ZMQ_STREAM` on the server is essential.

### 3.4 Encryption

- **Control plane**: Uses ZMQ_CURVE with server key as well‑known public key, agents generate their own keypairs. The server’s ZAP handler authenticates agents based on their public key.
- **Data plane**: The same CURVE mechanism can be applied to the `ROUTER`/`DEALER` data socket. The server enforces encryption and authenticates the same agent keys. The `ZMQ_STREAM` socket on the server’s public port does **not** use CURVE (it receives plain TCP from external clients), but all data crossing the internet (server ↔ agent) is CURVE‑encrypted.

End-to-end security: The tunnel traffic is encrypted on the wire between the server and agents. The server can decrypt the data, so it must be trusted. For true end‑to‑end encryption (client A to C), we would need an additional layer (e.g., TLS inside the tunnel), which is out of scope for this first version.

---

## 4. Detailed Implementation Steps

### 4.1 Phase 1 – Core Messaging & Authentication

1. **Generate CURVE certificates** (or keypairs) for the server and each agent.
2. **Implement the server control socket** (ROUTER bound to `tcp://*:5555`).
   - On receiving a connection, the ZAP handler checks the client’s public key against a directory of authorised keys.
3. **Implement the agent control socket** (DEALER connecting to `tcp://server:5555`).
   - Set socket options: `ZMQ_CURVE_SERVERKEY`, `ZMQ_CURVE_PUBLICKEY`, `ZMQ_CURVE_SECRETKEY`.
4. **Define a simple framing protocol** for control messages:
   - Frame 0: empty delimiter (for ROUTER compatibility)
   - Frame 1: message type string (e.g., `b"REGISTER"`)
   - Subsequent frames: parameters
5. **Implement registration**: agent C sends `REGISTER_FORWARD` with `service_id="svc443"`, `local_host="127.0.0.1"`, `local_port=443`. Server records the mapping and replies `OK`.

### 4.2 Phase 2 – Data Plane / Stream Relaying

1. **Server data socket**:
   - Bind a `ROUTER` socket to `tcp://*:5556` with CURVE enabled.
   - Bind a `ZMQ_STREAM` socket to `tcp://*:1443` (the exposed port).
2. **Agent data socket**:
   - Open a `DEALER` socket connecting to `tcp://server:5556`, using the same CURVE keys.
3. **Session management** on the server:
   - Maintain a table: `{session_id: {agent_identity, stream_identity, ...}}`
   - When data arrives on the `ZMQ_STREAM` from an external client:
     - If the stream’s identity is new, generate a unique `session_id` (e.g., UUID4).
     - Map `(stream_identity -> session_id)`.
     - Send a `STREAM_START` control message to the agent responsible for the requested service (the server may need to know which agent handles which service; registration provided that).
     - Alternatively, skip explicit START and just forward the first data frame; the agent can be designed to react to unknown `session_id` by opening a new TCP connection.
   - Forward data frames to the agent’s `ROUTER` socket, routing by `agent_identity`, with the multipart message: `[agent_identity, b"", session_id, data]` (empty delimiter to mimic ROUTER/DEALER conventions).
4. **Agent data handling**:
   - Poll its `DEALER` socket and a set of TCP sockets to the local service.
   - When a new `session_id` appears, open a TCP connection to `127.0.0.1:443`.
   - Map `session_id -> tcp_socket`.
   - Data received from the server is written to the TCP socket.
   - Data read from a TCP socket is sent back via the `DEALER`: `[b"", session_id, data]`.
   - On TCP socket error or close, send `STREAM_CLOSE <session_id>` to the server and clean up.
5. **Server relaying**:
   - When data arrives from an agent on the `ROUTER` data socket (message: `[agent_identity, b"", session_id, data]`), look up the corresponding `stream_identity` and forward `[stream_identity, data]` to the `ZMQ_STREAM` socket.
   - On receiving `STREAM_CLOSE` from an agent, close the associated external TCP connection (`ZMQ_STREAM` send identity + zero‑length message to signal close).

### 4.3 Phase 3 – Heartbeating & Automatic Reconnection

1. **Application‑level heartbeats**:
   - The agent sends a `HEARTBEAT` control message every 5 seconds.
   - The server replies with `HEARTBEAT_ACK`.
   - If the agent misses 3 consecutive acknowledgements, it considers the connection dead and enters a reconnection loop.
2. **ZeroMQ socket options for automatic reconnection**:
   - `ZMQ_RECONNECT_IVL=1000` (reconnect after 1 second)
   - `ZMQ_RECONNECT_IVL_MAX=30000`
   - `ZMQ_HEARTBEAT_IVL=5000` (optional, ZeroMQ 4.2+ can do TCP keep‑alive)
3. **Re‑registration on reconnect**:
   - When the agent’s DEALER socket successfully reconnects, the agent automatically sends a new `REGISTER_FORWARD` for each service it is responsible for. The server updates its routing table.
   - The agent shall use a **persistent service ID** so that the server can cleanly replace a stale registration.
4. **Cleanup of stale sessions**:
   - The server monitors agent heartbeats. If an agent is silent for more than 30 seconds, it forcefully closes all TCP streams associated with that agent (sends close notification on `ZMQ_STREAM`) and removes the registrations.
   - The agent, when it detects disconnection, closes all local TCP sockets and discards ongoing sessions. This is acceptable because TCP is stream‑oriented and broken tunnels cannot preserve in‑flight connections.

### 4.4 Phase 4 – Client‑Side Local Agent (Optional Enhancement)

If A and B want a resilient local endpoint that survives server restarts, they can run a mini‑agent:

- Binds a local `ZMQ_STREAM` listener on `127.0.0.1:1443`.
- Internally connects to the remote server’s `ZMQ_STREAM` port (or uses the data plane `DEALER`).
- The local agent’s control plane registers with the server to receive forwarding, or simply acts as a transparent TCP‑over‑ZMQ bridge.
- This local agent can buffer connection attempts while the server is unreachable and establish the upstream TCP connection once the server is available, making the local port always available to applications.

---

## 5. Security Considerations

- **Authentication**: Strict CURVE key whitelisting on the server. Unauthorised agents cannot register forwardings.
- **Encryption**: All traffic between agent and server (both control and data) is encrypted with Curve25519‑based key exchange and symmetric Salsa20/Poly1305 AEAD (via ZeroMQ CURVE).
- **Exposure**: The server’s public `ZMQ_STREAM` port is unencrypted and open. It should be firewalled to only intended users, or additional application‑level authentication (e.g., a simple token) can be implemented.
- **DoS**: Limit concurrent TCP connections per agent and globally. Use `ZMQ_MAX_SOCKETS` and rate limiting in the server logic.

---

## 6. Testing Strategy

1. **Unit tests**:
   - Control message parsing/serialisation.
   - Session mapping logic on the server.
   - Agent TCP pool management.
2. **Integration tests**:
   - Spin up server and two agents (C and a test client) on loopback.
   - Validate that a TCP connection to server port `1443` reaches agent C’s local service (e.g., a simple HTTP server).
   - Verify multiple concurrent connections work.
3. **Resilience tests**:
   - Kill the server process; confirm agents reconnect and re‑register; a new TCP connection works after server restart.
   - Simulate network delays/loss with `tc` netem; ensure heartbeats keep the tunnel alive or trigger clean reconnection.
   - Force agent process restart; ensure the server cleans up stale sessions and new registrations restore service.

---

## 7. Potential Alternatives and Enhancements

- **Alternatives to ZeroMQ**:  
  *Yamux* over TLS with a lightweight relay could achieve the same. However, ZeroMQ gives us robust messaging patterns and built‑in reconnection, reducing development time.
- **Direct peer‑to‑peer after initial mediation**:  
  After coordination via server, the agents could attempt UDP hole‑punching (using ZeroMQ’s `ZMQ_DGRAM` or STUN) to establish a direct encrypted tunnel, bypassing the server for data. This can be added later.
- **Full TCP session migration**:  
  To survive brief outages without dropping active TCP connections, we would need to buffer data and synchronise TCP sequence numbers—a complex feature (like Mosh for TCP). Keep this out of scope.
- **Load balancing**:  
  If multiple agents expose the same service, the server can round‑robin incoming TCP connections.

---

## 8. Conclusion

This plan outlines a practical, ZeroMQ‑based architecture for building a self‑healing encrypted TCP tunnel system. By leveraging ZMQ_STREAM for TCP proxying, CURVE for transport security, and automatic reconnection plus heartbeat‑driven re‑registration, we can deliver the reliability that SSH tunnels lack. The modular design allows iterative development, starting with a simple remote‑forward relay and growing into a robust, production‑grade overlay network.
