# ZeroMQ Tunnel

A lightweight Linux TUN-to-ZeroMQ bridge that forwards IP packets between a virtual network interface and a ZeroMQ socket. Connect remote machines over any network using a server-client architecture with AES-256-CBC encryption.

## Quick Start

### Prerequisites

- Linux with TUN/TAP support
- Root privileges (or `CAP_NET_ADMIN`)
- Rust toolchain (stable, 1.70+)
- libzmq development headers

```bash
# Debian / Ubuntu
sudo apt install libzmq3-dev pkg-config

# RHEL / Fedora
sudo dnf install zeromq-devel pkg-config

# Arch Linux
sudo pacman -S zeromq pkgconf
```

### Build

```bash
git clone https://github.com/andrewzuy/ZeroMQTunnel.git
cd ZeroMQTunnel
cargo build --release
```

The binary is at `target/release/zmq_tun`.

### Run

```bash
# Server
sudo ./target/release/zmq_tun run --mode server -a tcp://0.0.0.0:5555 --passphrase mysecret

# Client
sudo ./target/release/zmq_tun run --mode client -a tcp://<server_ip>:5555 --passphrase mysecret --ip 10.0.0.2/24
```

---

## AES-256 Encryption

All traffic between server and clients is encrypted with AES-256-CBC. The 32-byte key is derived from a shared passphrase using SHA-256.

### How it works

| Component | Detail |
|-----------|--------|
| **Cipher** | AES-256-CBC (pure Rust) |
| **Key derivation** | `SHA-256(passphrase)` |
| **IV** | 16 random bytes per packet, prepended to ciphertext |
| **Padding** | PKCS7 |
| **Wire format** | `[16-byte IV][AES-256-CBC ciphertext]` |

Both server and client must use the same passphrase. Encryption is always enabled — `--passphrase` is required.

### Security notes

- SHA-256 of a raw passphrase is not a password-hardened KDF (no salt, no iterations)
- No authentication (MAC/HMAC) — ciphertext integrity is not verified
- The IV is random per packet but not authenticated
- For stronger security later, consider migrating to `argon2`/`pbkdf2` for key derivation and AES-GCM for authenticated encryption

---

## Architecture

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
2. **ZMQ Reader** — Receives messages from the ZeroMQ socket, decrypts them, and sends them through a channel to the main loop. On the server, it parses the 3-frame ROUTER envelope to extract sender identity and packet data.
3. **Main Loop** — Receives from both channels. Client mode encrypts and sends TUN packets directly to ZMQ. Server mode routes TUN packets by destination IP using the `ClientRegistry`. Packets from ZMQ are written to the local TUN device.
4. **TUI Task** — Renders a live dashboard with packet statistics and a Wireshark-style packet log.

---

## Command-Line Reference

### Subcommands

| Subcommand | Description |
|------------|-------------|
| `run` | Start the tunnel (server or client mode) |

### `run` arguments

| Argument | Short | Default | Description |
|----------|-------|---------|-------------|
| `--mode` | `-m` | *required* | `server` or `client` |
| `--address` | `-a` | *required* | ZeroMQ bind/connect address |
| `--passphrase` | | *required* | Shared passphrase for AES-256 encryption |
| `--tun-name` | `-t` | `tun0` | TUN interface name |
| `--ip` | | `10.0.0.1/24` | IP address and CIDR prefix |
| `--mtu` | | `1500` | Maximum Transmission Unit |

### Logging

```bash
# Debug output
RUST_LOG=debug sudo ./target/release/zmq_tun run --mode server -a tcp://0.0.0.0:5555 --passphrase mysecret

# Quiet mode
RUST_LOG=error sudo ./target/release/zmq_tun run --mode server -a tcp://0.0.0.0:5555 --passphrase mysecret
```

---

## Terminal User Interface

The built-in TUI provides a live dashboard with:

- **Connection info** — mode, address, uptime, encryption status
- **Packet counters** — TUN-to-ZMQ and ZMQ-to-TUN packet counts
- **Connected clients** — number of registered clients (server mode)
- **Packet log** — Wireshark-style table with source/destination IPs, protocol, length, and status

Press `Ctrl+Q` or `Ctrl+C` to exit.

### Scrolling

The packet log supports scrolling through the buffered entries (up to 500 packets):

| Key | Action |
|-----|--------|
| `Up` / `Down` | Scroll one row |
| `PageUp` / `PageDown` | Scroll 10 rows |
| `Home` | Jump to the first entry |
| `End` | Jump to the last entry |

The table title shows the current position (e.g., `[42/500]`). When scrolled to the bottom, new packets auto-scroll into view.

### Color coding

- **Yellow** — outbound packets (TUN to ZMQ)
- **Magenta** — inbound packets (ZMQ to TUN)
- **Green** — OK status / encryption enabled
- **Red** — FAIL status

---

## How It Works

### Client registration

When a client connects, it sends an encrypted registration message (prefixed with `0xFE`) containing its IP address. The server's `ClientRegistry` maps the client's ZMQ identity to its IP, enabling return-traffic routing.

### Packet routing (server side)

The server extracts the destination IP from each packet arriving from the TUN device and looks up the target client in the registry. Packets are encrypted, then sent using the 3-frame ROUTER envelope: `[identity] [empty delimiter] [encrypted data]`.

### Socket configuration

| Option | Value | Purpose |
|--------|-------|---------|
| `ZMQ_LINGER` | 0 | Discard pending messages on shutdown |
| `ZMQ_MAXMSGSIZE` | 65536 | Support jumbo frames up to 64KB |
| `ZMQ_SNDHWM` / `ZMQ_RCVHWM` | 1024 | Bound queue sizes for backpressure |
| `ZMQ_RCVTIMEO` / `ZMQ_SNDTIMEO` | 100ms | Prevent indefinite blocking |

---

## Troubleshooting

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
- Confirm both sides use the same passphrase

### Decryption fails

- Confirm both server and client use the exact same passphrase
- Check for typos — SHA-256 of different passphrases produces completely different keys

---

## Legacy builds

To build a binary compatible with older systems (e.g., Ubuntu 20.04), use the provided Dockerfile:

```bash
docker build -f Dockerfile.build -t zmq-tun-builder .
docker run --rm -v $(pwd):/usr/src/myapp zmq-tun-builder
# Built binary: target/legacy_gcc/zmq_tun
```

---

## Limitations & Future Work

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
- **Authenticated encryption** — migrate to AES-GCM for ciphertext integrity

---

*Built with Rust, tokio, ZeroMQ, and AES-256-CBC.*
