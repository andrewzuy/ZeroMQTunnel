# zmq_tun — Bridging Virtual Networks Over ZeroMQ

*A lightweight TUN-to-ZeroMQ bridge in Rust with optional CURVE encryption.*

---

## Table of Contents

1. [Quick Start](#1-quick-start)
2. [CURVE Encryption](#2-curve-encryption)
3. [Architecture at a Glance](#3-architecture-at-a-glance)
4. [Command-Line Reference](#4-command-line-reference)
5. [The Terminal User Interface](#5-the-terminal-user-interface)
6. [How It Works](#6-how-it-works)
7. [Troubleshooting](#7-troubleshooting)
8. [Limitations & Future Work](#8-limitations--future-work)

---

## 1. Quick Start

### Prerequisites

- **Linux** with TUN/TAP support
- **Root privileges** (or `CAP_NET_ADMIN`)
- **Rust toolchain** (stable, 1.70+)
- **libzmq development headers**

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

### Running without encryption

```bash
# Server
sudo ./target/release/zmq_tun run --mode server

# Client
sudo ./target/release/zmq_tun run --mode client --address tcp://<server_ip>:5555 --client-ip 10.0.0.2
```

### Running with encryption (recommended)

See [Section 2: CURVE Encryption](#2-curve-encryption) for a full walkthrough.

---

## 2. CURVE Encryption

ZeroMQ CURVE provides authenticated encryption for all traffic between server and clients. Every connection is encrypted end-to-end — no plaintext packets traverse the network.

### Step-by-step setup

#### Step 1: Generate a key pair for the server

```bash
./target/release/zmq_tun keygen -o server.key
```

This creates a `server.key` file with two lines:
```
<public key in Z85>
<secret key in Z85>
```

The **public key** is what clients need to authenticate the server. The **secret key** stays on the server and should never be shared.

Example output:
```
Public key (Z85):  ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij
Secret key (Z85):  1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZab
```

#### Step 2: Generate a key pair for each client

Each client gets its own key pair. The server does **not** need the clients' keys — CURVE on the server side accepts any authenticated client.

```bash
./target/release/zmq_tun keygen -o client1.key
./target/release/zmq_tun keygen -o client2.key
```

#### Step 3: Start the server with encryption

```bash
sudo ./target/release/zmq_tun run --mode server --enable-curve --curve-key-file server.key
```

The server loads its key pair from `server.key` and enables CURVE. Any client connecting must present the server's public key to authenticate.

#### Step 4: Connect clients with encryption

Copy the server's **public key** (first line of `server.key`) to each client machine, then start each client:

```bash
# Client 1
sudo ./target/release/zmq_tun run \
  --mode client \
  --address tcp://<server_ip>:5555 \
  --client-ip 10.0.0.2 \
  --enable-curve \
  --curve-key-file client1.key \
  --server-public-key ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij

# Client 2
sudo ./target/release/zmq_tun run \
  --mode client \
  --address tcp://<server_ip>:5555 \
  --client-ip 10.0.0.3 \
  --enable-curve \
  --curve-key-file client2.key \
  --server-public-key ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij
```

Replace `ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij` with the actual server public key from `server.key`.

### How it works

| Role | What it needs | What it shares |
|------|---------------|----------------|
| **Server** | Its own key pair (`server.key`) | Its public key (first line of `server.key`) |
| **Client** | Its own key pair + server's public key | Nothing — the client key stays local |

The server does **not** need client public keys. It runs in `ZMQ_CURVE_SERVER` mode, which accepts any client that can authenticate against its own public key. Each client proves its identity through the CURVE handshake, and all traffic is encrypted automatically.

### Key file format

A key file is plain text with exactly two lines:
```
<Z85 public key>
<Z85 secret key>
```

You can also generate keys without saving to a file (prints to stdout):
```bash
./target/release/zmq_tun keygen
```

### Security notes

- Keep `.key` files secure and never commit them to version control
- The server's secret key should only exist on the server machine
- Each client should have its own unique key pair
- If a key is compromised, generate a new one with `keygen`

---

## 3. Architecture at a Glance

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
|   +------+------ +    |                      |   +------+------ +    |
|          |           |                      |          |           |
|   +------v------+    |                      |   +------v------+    |
|   |  mpsc ch     |    |                      |   |  mpsc ch     |    |
|   +------+------ +    |                      |   +------+------ +    |
|          |           |                      |          |           |
|   +------v------+    |    |                      |   +------v------+  |
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

### Data flow

1. **TUN Reader** — Reads raw IP packets from the TUN device and sends them through a channel to the main loop.
2. **ZMQ Reader** — Receives messages from the ZeroMQ socket and sends them through a channel to the main loop. On the server, it parses the 3-frame ROUTER envelope to extract sender identity and packet data.
3. **Main Loop** — Receives from both channels. Client mode sends TUN packets directly to ZMQ. Server mode routes TUN packets by destination IP using the `ClientRegistry`. Packets from ZMQ are written to the local TUN device.
4. **TUI Task** — Renders a live dashboard with packet statistics and a Wireshark-style packet log.

---

## 4. Command-Line Reference

### Subcommands

| Subcommand | Description |
|------------|-------------|
| `run` | Start the tunnel (server or client mode) |
| `keygen` | Generate a CURVE key pair |

### `run` arguments

| Argument | Short | Default | Description |
|----------|-------|---------|-------------|
| `--mode` | `-m` | *required* | `server` or `client` |
| `--address` | `-a` | `tcp://0.0.0.0:5555` | ZeroMQ bind/connect address |
| `--tun-name` | `-t` | `tun0` | TUN interface name |
| `--ip` | | `10.0.0.1/24` | IP address and CIDR prefix |
| `--mtu` | | `1500` | Maximum Transmission Unit |
| `--client-ip` | | *(none)* | Client IP for routing (required in client mode) |
| `--enable-curve` | | `false` | Enable ZMQ CURVE encryption |
| `--curve-key-file` | | *(none)* | Path to Z85-encoded key file (public + secret) |
| `--server-public-key` | | *(none)* | Server's public key in Z85 (required for client + curve) |

### `keygen` arguments

| Argument | Short | Default | Description |
|----------|-------|---------|-------------|
| `--output` | `-o` | *stdout* | Write key pair to file |

### Logging

```bash
# Debug output
RUST_LOG=debug sudo ./target/release/zmq_tun run --mode server

# Quiet mode
RUST_LOG=error sudo ./target/release/zmq_tun run --mode server
```

---

## 5. The Terminal User Interface

The built-in TUI provides a live dashboard with:

- **Connection info** — mode, address, uptime, CURVE status
- **Packet counters** — TUN-to-ZMQ and ZMQ-to-TUN packet counts
- **Connected clients** — number of registered clients (server mode)
- **Packet log** — Wireshark-style table with source/destination IPs, protocol, length, and status

Press `Ctrl+Q` or `Ctrl+C` to exit.

### Color coding

- **Yellow** — outbound packets (TUN to ZMQ)
- **Magenta** — inbound packets (ZMQ to TUN)
- **Green** — OK status / CURVE enabled
- **Red** — FAIL status

---

## 6. How It Works

### Client registration

When a client connects, it sends a registration message (prefixed with `0xFE`) containing its IP address. The server's `ClientRegistry` maps the client's ZMQ identity to its IP, enabling return-traffic routing.

### Packet routing (server side)

The server extracts the destination IP from each packet arriving from the TUN device and looks up the target client in the registry. Packets are sent using the 3-frame ROUTER envelope: `[identity] [empty delimiter] [data]`.

### CURVE encryption

When `--enable-curve` is set, both sides perform a CURVE handshake before any data is exchanged. The server sets `ZMQ_CURVE_SERVER` and loads its own key pair. Each client loads its own key pair and the server's public key. All subsequent traffic is encrypted automatically by libzmq.

### Socket configuration

| Option | Value | Purpose |
|--------|-------|---------|
| `ZMQ_LINGER` | 0 | Discard pending messages on shutdown |
| `ZMQ_MAXMSGSIZE` | 65536 | Support jumbo frames up to 64KB |
| `ZMQ_SNDHWM` / `ZMQ_RCVHWM` | 1024 | Bound queue sizes for backpressure |
| `ZMQ_RCVTIMEO` / `ZMQ_SNDTIMEO` | 100ms | Prevent indefinite blocking |

---

## 7. Troubleshooting

### "failed to open /dev/net/tun"

```bash
sudo modprobe tun
```

### "ioctl TUNSETIFF failed: Permission denied"

Run with `sudo`, or grant the capability:
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
- Ensure network reachability between hosts

### CURVE handshake fails

- Confirm the client's `--server-public-key` matches the server's public key (first line of the server's key file)
- Confirm both sides use `--enable-curve` — mixing encrypted and unencrypted peers will not work
- Check that key files have correct permissions and contain valid Z85 keys

---

## 8. Limitations & Future Work

### Current limitations

- **Linux-only** — relies on Linux-specific TUN/TAP ioctls
- **IPv4 only** — no IPv6 support
- **Requires root** — TUN interface creation needs elevated privileges
- **No client disconnect detection** — disconnected clients remain in the registry

### Planned enhancements

- **IPv6** — dual-stack TUN configuration
- **Configuration files** — TOML/YAML-based config
- **Metrics** — Prometheus exporter for packet counts, latency, and errors
- **Non-root operation** — Linux user namespaces for privilege reduction
- **Packet compression** — optional compression for low-bandwidth links
- **Client disconnect detection** — heartbeat or ZMQ monitoring

---

*Built with Rust, tokio, ZeroMQ, ratatui, and a healthy respect for raw network packets.*
