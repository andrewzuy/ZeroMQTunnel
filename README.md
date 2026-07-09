# zmq_tun — Bridging Virtual Networks Over ZeroMQ

*A comprehensive guide to building a TUN-to-ZeroMQ bridge in Rust.*

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Architecture at a Glance](#2-architecture-at-a-glance)
3. [The Technology Stack](#3-the-technology-stack)
4. [TUN Device Management](#4-tun-device-management)
5. [ZeroMQ Communication Layer](#5-zeromq-communication-layer)
6. [The Async Event Loop](#6-the-async-event-loop)
7. [Packet Parsing & Inspection](#7-packet-parsing--inspection)
8. [The Terminal User Interface](#8-the-terminal-user-interface)
9. [Signal Handling & Graceful Shutdown](#9-signal-handling--graceful-shutdown)
10. [Installation & Usage](#10-installation--usage)
11. [Troubleshooting](#11-troubleshooting)
12. [Limitations & Future Work](#12-limitations--future-work)

---

## 1. Introduction

`zmq_tun` is a lightweight, bidirectional network tunnel written in Rust. It creates a virtual network interface on Linux using the TUN device, then forwards raw IP packets between that interface and remote peers over ZeroMQ ROUTER/DEALER sockets connected via TCP.

The server uses a ROUTER socket to multiplex connections from multiple clients, each using a DEALER socket. A client registry maps ZMQ identities to client IPs, enabling the server to route return traffic to the correct peer.

### What problem does it solve?

Traditional VPN solutions are heavy, require complex configuration, and often introduce unnecessary overhead. `zmq_tun` takes a different approach: it leverages ZeroMQ's reliable messaging fabric as a transport layer, and Rust's async runtime to keep the forwarding path fast and simple. The ROUTER/DEALER socket pattern enables a single server to serve multiple clients, with automatic packet routing based on destination IP.

### Typical use cases

- **Custom overlay networks** between multiple hosts in a lab environment
- **Network traffic analysis** — the built-in TUI logs every packet in Wireshark-style format
- **Tunneling IP traffic** through a messaging infrastructure
- **Prototyping** isolated network segments without physical hardware

---

## 2. Architecture at a Glance

```
+----------------------+     ZeroMQ ROUTER      +----------------------+
|     Machine A        |  <--------------->   |     Machine B        |
|   (Server Mode)      |   tcp://addr:port    |   (Client Mode)      |
|                      |                      |                      |
|  +----------------+  |                      |  +----------------+  |
|  |   TUN Device   |  |                      |  |   TUN Device   |  |
|  |   tun0         |  |                      |  |   tun0         |  |
|  |  10.0.0.1/24   |  |                      |  |  10.0.0.2/24   |  |
|  +-------+--------+  |                      |  +-------+--------+  |
|          |           |                      |          |           |
|   +------v------+    |                      |   +------v------+    |
|   | TUN Reader   |    |                      |   | TUN Reader   |    |
|   | (blocking)   |    |                      |   | (blocking)   |    |
|   +------+------ +    |                      |   +------+------ +    |
|          |           |                      |          |           |
|   +------v------+    |                      |   +------v------+    |
|   |  mpsc ch     |    |                      |   |  mpsc ch     |    |
|   +------+------ +    |                      |   +------+------ +    |
|          |           |                      |          |           |
|   +------v------+    |                      |   +------v------+    |
|   |  Main Loop   |<---+----->+              |   |  Main Loop   |    |
|   |  (tokio)     |    |    |   |              |   |  (tokio)     |    |
|   |  +Registry   |    |    |   +--------------+---+------+------ +    |
|   +------+------ +    |    |                      |          |        |
|          |            |    |                      |          |        |
|   +------v------+    |    |                      |   +------v------+  |
|   | ZMQ Writer   |    |    |                      |   | TUN Writer   |  |
|   +------+------ +    |    |                      |   +------+------ +  |
|          |            |    |                      |          |          |
|   +------v------+    |    |                      |   +------v------+  |
|   | ZMQ Reader   |    |    |                      |   | ZMQ Reader   |  |
|   | (blocking)   |    |    |                      |   | (blocking)   |  |
|   +------+------ +    |    |                      |   +------+------ +  |
|          |            |    |                      |          |          |
|   +------v------+    |    |                      |   +------v------+  |
|   |  mpsc ch     |    |    |                      |   |  mpsc ch     |  |
|   +-------------+     |    |                      |   +-------------+  |
+-----------------------+    |                      +-------------------+
                              |
               +--------------+--------------+
               |  TCP Network (Internet, LAN)|
               +-----------------------------+
```

### Data flow in words

The application runs four concurrent I/O paths, all coordinated by a central async event loop:

1. **TUN Reader** — A blocking task reads raw IP packets from the TUN device file descriptor and sends them through a `tokio::sync::mpsc` channel.
2. **ZMQ Reader** — A blocking task receives messages from the ZeroMQ socket and sends them through a separate `mpsc` channel. On the server side, it parses the 3-frame ROUTER envelope (`[identity][delimiter][data]`) to extract the sender's identity and the packet payload.
3. **Main Loop** — The central `tokio::select!` loop receives from both channels. On the client side, packets from TUN are sent directly to the ZMQ DEALER socket. On the server side, packets from TUN are routed by destination IP using the `ClientRegistry`, which maps client IPs to their ZMQ identities. Packets arriving from ZeroMQ are written to the local TUN device.
4. **TUI Task** — A separate task renders a live terminal dashboard showing packet statistics and a Wireshark-style packet log.

This design decouples I/O sources from forwarding logic, keeping the async loop clean and responsive.

---

## 3. The Technology Stack

Every library in this project has a specific responsibility. Here is the complete dependency breakdown:

### Core runtime: `tokio` (v1, features: `full`, `signal`)

Tokio is the async runtime that powers the entire application. It provides:

- **The async executor** — `#[tokio::main]` bootstraps the runtime and drives all async tasks.
- **`tokio::sync::mpsc`** — Multi-producer, single-consumer channels that decouple the blocking I/O readers from the async forwarding loop. Each channel has a capacity of 64 messages, providing bounded buffering.
- **`tokio::sync::broadcast`** — A broadcast channel used for shutdown coordination. All spawned tasks subscribe to this channel and exit when a shutdown message is received.
- **`tokio::task::spawn_blocking`** — Offloads blocking system calls (`read()`, `write()`, ZMQ `send()`/`recv()`) to tokio's dedicated blocking thread pool. This is critical because both the TUN device and the ZeroMQ socket perform blocking I/O.
- **`tokio::signal`** — Handles `Ctrl+C` (`SIGINT`) and `SIGTERM` for graceful shutdown.
- **`tokio::time::sleep`** — Provides async sleep for backoff loops and TUI refresh timing.

### Network messaging: `zmq` (v0.10)

The `zmq` crate provides safe Rust bindings to the ZeroMQ messaging library (libzmq). It is responsible for:

- Creating a **ZMQ context** — the top-level object that manages all sockets.
- Creating a **ROUTER socket** (server) or **DEALER socket** (client) — the ROUTER/DEALER pattern enables multi-client support with message routing via client identities.
- Configuring socket options: `ZMQ_LINGER` (0, for immediate shutdown), `ZMQ_MAXMSGSIZE` (65536, to accommodate jumbo frames), `ZMQ_SNDHWM` and `ZMQ_RCVHWM` (1024 each, to bound queue sizes), and `ZMQ_RCVTIMEO`/`ZMQ_SNDTIMEO` (100ms, for non-blocking behavior).
- Binding (server mode) or connecting (client mode) to a TCP endpoint.

The socket is wrapped in `Arc<Mutex<ZmqSocket>>` to allow safe sharing across multiple async tasks.

### Unix system calls: `nix` (v0.29, features: `ioctl`, `net`, `socket`, `uio`, `fs`)

The `nix` crate provides safe Rust bindings to POSIX system calls. In this project, it is used for:

- **`nix::unistd::read`** and **`nix::unistd::write`** — Low-level read/write on the TUN file descriptor.
- **`nix::fcntl::{fcntl, FcntlArg, OFlag}`** — Setting the TUN file descriptor to non-blocking mode via `F_SETFL(O_NONBLOCK)`.

### C library bindings: `libc` (v0.2)

The `libc` crate provides FFI definitions for C types and constants. It is used for:

- **`libc::ifreq`** — The interface request structure used with `ioctl()` calls to configure the TUN device.
- **`libc::ioctl`** — The raw `ioctl()` syscall for `TUNSETIFF` (creating the TUN interface) and `TUNSETMTU` (setting the MTU).
- **`libc::IFF_TUN`** and **`libc::IFF_NO_PI`** — Flags that specify this is a TUN (Layer 3/IP) device without packet info headers.

### CLI parsing: `clap` (v4, features: `derive`)

Clap provides derive-macro-based command-line argument parsing. The `Args` struct and `Mode` enum are annotated with `#[derive(Parser)]` and `#[derive(ValueEnum)]`, generating argument parsing, help text, and validation automatically.

### Error handling: `anyhow` (v1)

The `anyhow` crate provides the `Result<T>` type and the `?` operator with `.context()` for attaching error descriptions. It replaces manual error code checking throughout the codebase.

### Logging: `log` (v0.4) + `env_logger` (v0.11)

- **`log`** provides the logging facade (`info!`, `warn!`, `error!` macros).
- **`env_logger`** provides the backend that reads the `RUST_LOG` environment variable and formats log output. Default level is `warn`.

### Terminal UI: `ratatui` (v0.29) + `crossterm` (v0.28)

- **`ratatui`** is a Terminal User Interface framework built on the tui-rs ecosystem. It provides widgets (`Block`, `Paragraph`, `Table`, `Row`, `Cell`), layout management (`Layout`, `Constraint`), and styling (`Style`, `Color`, `Stylizable`).
- **`crossterm`** provides low-level terminal control: switching to alternate screen buffer, reading keyboard events, and managing cursor visibility.

Together, they render the live packet monitor dashboard.

---

## 4. TUN Device Management

**File:** `src/tun.rs` (127 lines)

The TUN (TUNnel) device is a virtual network interface provided by the Linux kernel. It operates at Layer 3 (network layer), meaning it handles raw IP packets rather than Ethernet frames.

### Opening the device

```
/dev/net/tun  ──(open read/write)──>  File descriptor
```

The `open_tun()` function opens `/dev/net/tun` with read and write permissions. This is the clone device — opening it creates a new virtual network interface.

### Interface allocation via `ioctl(TUNSETIFF)`

An `ifreq` structure is populated with:
- The interface name (e.g., `"tun0"`)
- Flags: `IFF_TUN | IFF_NO_PI`

The `IFF_TUN` flag specifies Layer 3 operation (IP packets only). The `IFF_NO_PI` flag disables the packet info header, so reads and writes exchange raw IP datagrams directly, without any prefix.

The `ioctl(TUNSETIFF)` syscall allocates the interface and associates it with the file descriptor.

### Setting the MTU

A custom ioctl constant `TUNSETMTU` (0x400454D3) is used to set the Maximum Transmission Unit. The default is 1500 bytes (standard Ethernet MTU), but it is configurable via the `--mtu` CLI flag.

### Non-blocking mode

The file descriptor is set to non-blocking mode using `fcntl(F_SETFL, O_NONBLOCK)`. This ensures that `read()` calls return `EAGAIN` immediately if no data is available, rather than blocking the thread indefinitely.

### Interface configuration

The `configure_interface()` function uses external commands to set up the network interface:

1. **`ip addr add <IP>/<prefix> dev <name>`** — Assigns the IP address and subnet.
2. **`ip link set up <name>`** — Brings the interface up.

These commands are executed via `std::process::Command`. If either fails, the error is propagated and the application exits.

### The `TunDevice` struct

```rust
pub struct TunDevice {
    file: File,
    name: String,
}
```

This struct holds the open file descriptor and the interface name. When dropped, the file descriptor is closed, and the kernel automatically removes the virtual interface.

---

## 5. ZeroMQ Communication Layer

**File:** `src/zmq_comm.rs` (201 lines)

### The ROUTER/DEALER socket model

ZeroMQ offers many socket types (PUB/SUB, REQ/REP, PUSH/PULL, etc.). The ROUTER/DEALER pattern is used here because it provides:

- **Bidirectional** communication — both peers can send and receive.
- **Ordered delivery** — messages arrive in the order they were sent.
- **Multi-client** support — a single ROUTER socket on the server can multiplex connections from many DEALER clients, routing messages by client identity.

### ROUTER message framing

Messages from the server's ROUTER socket to clients use a 3-frame envelope:

```
[Frame 1: client identity] [Frame 2: empty delimiter] [Frame 3: packet data]
```

The empty delimiter frame is mandatory for ROUTER sockets. The `send_to_client()` function sends all three frames. The `zmq_reader_loop` on the server parses incoming 3-frame messages to extract the sender's identity and the packet payload.

### Client registration

When a client connects, it sends a registration message prefixed with `0xFE` followed by its IP address. The server's `ClientRegistry` maps the client's ZMQ identity to its IP address, enabling return-traffic routing.

### Socket configuration

| Option | Value | Purpose |
|--------|-------|---------|
| `ZMQ_LINGER` | 0 | On shutdown, discard pending messages immediately rather than waiting. |
| `ZMQ_MAXMSGSIZE` | 65536 | Maximum message size. Accommodates jumbo frames (up to 64KB). |
| `ZMQ_SNDHWM` | 1024 | High-water mark for outbound queue. Drops messages if queue exceeds 1024. |
| `ZMQ_RCVHWM` | 1024 | High-water mark for inbound queue. Same backpressure mechanism. |
| `ZMQ_RCVTIMEO` | 100ms | Receive timeout. Prevents indefinite blocking on `recv()`. |
| `ZMQ_SNDTIMEO` | 100ms | Send timeout. Prevents indefinite blocking on `send()`. |

### Server vs. client mode

- **Server**: Calls `socket.bind(address)` to listen on the specified TCP endpoint. Sets `ZMQ_IMMEDIATE` to require peer identity before connecting.
- **Client**: Calls `socket.set_identity()` with its IP address, then `socket.connect(address)` to connect to the server.

The socket is wrapped in `Arc<Mutex<ZmqSocket>>` to allow safe concurrent access from multiple async tasks. The `Arc` provides reference counting (the socket stays alive as long as any task holds a reference), and the `Mutex` ensures only one task accesses the socket at a time.

---

## 6. The Async Event Loop

**File:** `src/main.rs`

The main function orchestrates the entire application. Here is the startup sequence:

### Phase 1: Initialization

1. **Logging** — `env_logger` is initialized with default filter level `warn`.
2. **CLI parsing** — `clap` parses command-line arguments into the `Args` struct.
3. **CIDR parsing** — The IP address string (e.g., `"10.0.0.1/24"`) is split into IP and prefix length.
4. **TUN creation** — `TunDevice::new()` opens and configures the virtual interface.
5. **ZMQ setup** — A ZMQ context and ROUTER/DEALER socket are created and configured via `ZmqChannel::new()`.
6. **Channel creation** — Two `mpsc` channels are created:
   - `tun_tx`/`tun_rx`: carries packets from TUN reader to the main loop.
   - `zmq_tx`/`zmq_rx`: carries packets from ZMQ reader to the main loop.
7. **TUI state** — A `TuiState` struct wrapped in `Arc<Mutex<>>` is created for shared state between the main loop and the TUI task.

### Phase 2: Task spawning

Three tasks are spawned:

| Task | Writer | Purpose |
|------|--------|---------|
| TUI Task | No | Renders the live dashboard. Subscribes to shutdown broadcast. |
| TUN Reader | `tun_tx` | Reads packets from TUN device, sends to main loop via `mpsc`. |
| ZMQ Reader | `zmq_tx` | Receives packets from ZMQ socket, sends to main loop via `mpsc`. On the server, parses the 3-frame ROUTER envelope. |

Both reader tasks use `spawn_blocking` to offload blocking syscalls to tokio's blocking thread pool. Each task loops inside a `tokio::select!` that races the I/O operation against the shutdown broadcast.

### Phase 3: The forwarding loop

The main `tokio::select!` loop multiplexes events:

```
tokio::select! {
    Some(data) = tun_rx.recv()      =>  Forward to ZMQ (TUN → ZMQ)
    Some(data) = zmq_rx.recv()      =>  Forward to TUN (ZMQ → TUN)
    _ = ctrl_c                     =>  Break (graceful shutdown)
    _ = sigterm.recv()             =>  Break (graceful shutdown)
    _ = shutdown_signal.recv()     =>  Break (TUI-initiated shutdown)
    else => sleep(10ms)            =>  Prevent busy-waiting
}
```

**Client side:** Packets from TUN are sent directly to the ZMQ DEALER socket. Packets from ZMQ are written to the local TUN device.

**Server side:** Packets from TUN have their destination IP extracted and are routed via `send_to_client()` using the `ClientRegistry` to look up the target client's ZMQ identity. Registration messages (`0xFE` prefix) from ZMQ are used to update the client registry. All other packets from ZMQ are written to the local TUN device.

Each forwarded packet is logged to the TUI state, recording direction, parsed IP headers, protocol, length, and status.

---

## 7. Packet Parsing & Inspection

**File:** `src/main.rs` (lines 111–216)

Every packet that traverses the tunnel is parsed to extract human-readable metadata for the TUI dashboard. The `parse_ip_packet()` function operates on raw IPv4 datagrams:

### IPv4 header parsing

The IPv4 header is parsed manually by reading bytes at fixed offsets:

| Offset | Field | Description |
|--------|-------|-------------|
| Byte 0 (high nibble) | Version | Expected: 4 (IPv4) |
| Byte 0 (low nibble) | IHL | Header length in 32-bit words |
| Bytes 2-3 | Total Length | Full packet length in bytes |
| Byte 9 | Protocol | 1=ICMP, 6=TCP, 17=UDP |
| Bytes 12-15 | Source IP | 32-bit IPv4 address |
| Bytes 16-19 | Destination IP | 32-bit IPv4 address |

### Transport layer details

Depending on the protocol field, additional parsing extracts:

- **ICMP** (proto 1): Type and code fields.
- **TCP** (proto 6): Source port, destination port, and TCP flags (SYN, FIN, ACK, RST, PSH, URG).
- **UDP** (proto 17): Source port, destination port, and payload length.

This parsing is done purely for display purposes — it does not affect packet forwarding. Packets are forwarded as raw bytes regardless of whether parsing succeeds.

---

## 8. The Terminal User Interface

**File:** `src/main.rs` (lines 351–563)

The TUI is powered by `ratatui` (widget rendering) and `crossterm` (terminal control).

### Layout

The screen is divided into two regions using `ratatui::Layout`:

1. **Upper panel** (8 rows): Split horizontally into two halves.
   - **Left**: Connection info — mode, address, uptime, packet counters (TUN→ZMQ and ZMQ→TUN), total logged packets.
   - **Right**: ZMQ status — connection state and exit instructions.
2. **Lower panel** (remaining rows): A `ratatui::Table` widget displaying the packet log.

### Packet log table

The table mimics Wireshark's packet list with columns:

| Column | Content |
|--------|---------|
| No. | Sequential packet number |
| Time | Elapsed time since startup (seconds, 3 decimal places) |
| Direction | `tun->zmq` (yellow) or `zmq->tun` (magenta) |
| Source | Source IP address |
| Destination | Destination IP address |
| Protocol | ICMP, TCP, UDP, or Other |
| Length | Packet length in bytes |
| Info | Status (OK/FAIL/CANCELLED) plus protocol-specific details |

### Color coding

- **Direction**: Yellow for outbound (TUN→ZMQ), Magenta for inbound (ZMQ→TUN).
- **Status**: Green for OK, Red for FAIL, Dark Gray for CANCELLED.
- **Header**: White, bold.
- **Panel borders**: Cyan.

### Refresh rate

The TUI refreshes every 200ms. Keyboard input is polled non-blockingly — pressing `Ctrl+Q` triggers a graceful shutdown by sending a message on the broadcast channel.

### Entry management

The `TuiState` maintains a ring buffer of up to 500 packet entries. When the limit is exceeded, the oldest entries are removed from the front of the vector.

---

## 9. Signal Handling & Graceful Shutdown

The application handles three shutdown triggers:

| Signal | Handler | Source |
|--------|---------|--------|
| `SIGINT` (Ctrl+C) | `tokio::signal::ctrl_c()` | Terminal |
| `SIGTERM` | `tokio::signal::unix::signal(SignalKind::terminate())` | System / container orchestrator |
| Broadcast message | `broadcast::Receiver` | TUI task (Ctrl+Q) |

### Shutdown sequence

1. The main loop breaks on any of the above signals.
2. A shutdown message is sent via the `broadcast` channel.
3. The TUN reader task and ZMQ reader task receive the message and exit their loops.
4. The TUI task receives the message, restores the terminal (shows cursor, leaves alternate screen), and exits.
5. The ZMQ context, socket, and TUN file descriptor are dropped via Rust's RAII mechanism, releasing all resources.

The `ZMQ_LINGER=0` option ensures the ZMQ socket does not block on shutdown.

---

## 10. Installation & Usage

### Prerequisites

- **Linux** with TUN/TAP kernel support (`CONFIG_TUN`)
- **Root privileges** (or `CAP_NET_ADMIN`)
- **Rust toolchain** (stable, 1.70+)
- **libzmq development headers**

#### Installing dependencies

```bash
# Debian / Ubuntu
sudo apt install libzmq3-dev pkg-config

# RHEL / Fedora
sudo dnf install zeromq-devel pkg-config

# Arch Linux
sudo pacman -S zeromq pkgconf
```

### Building

```bash
git clone <repository-url>
cd ZeroMQTunnel
cargo build --release
```

The binary is at `target/release/zmq_tun`.

### Running

```bash
# Server (binds on port 5555)
sudo ./target/release/zmq_tun --mode server

# Client (connects to server)
sudo ./target/release/zmq_tun --mode client --address tcp://<server_ip>:5555 --ip 10.0.0.2/24
```

### Command-line arguments

| Argument | Short | Default | Description |
|----------|-------|---------|-------------|
| `--mode` | `-m` | *required* | `server` or `client` |
| `--address` | `-a` | `tcp://0.0.0.0:5555` | ZeroMQ bind/connect address |
| `--tun-name` | `-t` | `tun0` | TUN interface name |
| `--ip` | | `10.0.0.1/24` | IP address and CIDR prefix |
| `--mtu` | | `1500` | Maximum Transmission Unit |

### Logging control

```bash
# Debug output
RUST_LOG=debug sudo ./target/release/zmq_tun --mode server

# Quiet mode
RUST_LOG=error sudo ./target/release/zmq_tun --mode server
```

---

## 11. Troubleshooting

### "failed to open /dev/net/tun"

The TUN device node is missing:
```bash
sudo modprobe tun
```

### "ioctl TUNSETIFF failed: Permission denied"

The process lacks `CAP_NET_ADMIN`. Run with `sudo`, or:
```bash
sudo setcap cap_net_admin+ep ./target/release/zmq_tun
```

### "failed to bind to ..."

Port 5555 is already in use:
```bash
ss -tlnp | grep 5555
```

### Client cannot connect to server

- Verify the server is running
- Check firewall rules: `sudo iptables -L -n`
- Ensure network reachability between the two hosts

---

## 12. Limitations & Future Work

### Current limitations

- **Linux-only** — relies on Linux-specific TUN/TAP ioctls.
- **No encryption** — traffic is sent in plaintext over TCP.
- **No authentication** — any client can connect to a listening server.
- **IPv4 only** — no IPv6 support.
- **Requires root** — TUN interface creation needs elevated privileges.
- **No client disconnect detection** — disconnected clients remain in the registry.

### Planned enhancements

- **Encryption** — add ZMQ CURVE or TLS support.
- **IPv6** — extend TUN configuration for dual-stack operation.
- **Configuration files** — TOML/YAML-based configuration.
- **Metrics** — Prometheus exporter for packet counts, latency, and error rates.
- **Non-root operation** — use Linux user namespaces for privilege reduction.
- **Packet compression** — optional compression for low-bandwidth links.
- **Client disconnect detection** — heartbeat or ZMQ monitoring to clean up stale registry entries.

---

*Built with Rust, tokio, ZeroMQ, ratatui, and a healthy respect for raw network packets.*
