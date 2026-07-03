# zmq_tun — TUN-to-ZeroMQ Bridge

A lightweight, bidirectional network tunnel that forwards raw IP packets between a Linux TUN device and a ZeroMQ PAIR socket over TCP. Built in Rust using `tokio` for async I/O.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Features](#features)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Usage](#usage)
- [Command-Line Arguments](#command-line-arguments)
- [Examples](#examples)
- [How It Works](#how-it-works)
- [Logging](#logging)
- [Shutdown & Signal Handling](#shutdown--signal-handling)
- [Performance & Tuning](#performance--tuning)
- [Troubleshooting](#troubleshooting)
- [Limitations](#limitations)
- [Future Enhancements](#future-enhancements)
- [License](#license)

## Overview

`zmq_tun` creates a virtual network interface (TUN) on Linux and bridges it to a ZeroMQ PAIR socket. This allows two machines to exchange raw IP packets over a TCP connection using ZeroMQ as the transport layer.

Typical use cases:
- Custom VPN or overlay network between two hosts
- Network traffic analysis and experimentation
- Tunneling IP traffic through a messaging fabric
- Lab environments where you need isolated network segments

## Architecture

```
+------------------+     ZeroMQ PAIR      +------------------+
|  Machine A       |  <--------------->   |  Machine B       |
|                  |   tcp://addr:port    |                  |
|  TUN (tun0)      |                      |  TUN (tun0)      |
|  10.0.0.1/24     |                      |  10.0.0.2/24     |
|                  |                      |                  |
|  [TUN reader] --+--->[mpsc]---> [loop]--+--->[mpsc]--->[ZMQ reader]
|  [ZMQ reader] --+--->[mpsc]---> [loop]--+--->[mpsc]--->[TUN reader]
+------------------+                      +------------------+
```

### Data Flow

1. **TUN → ZMQ**: Packets read from the TUN device are sent via `mpsc` channel to the main loop, then forwarded to the remote peer over ZeroMQ.
2. **ZMQ → TUN**: Packets received from ZeroMQ are sent via `mpsc` channel to the main loop, then written to the TUN device.
3. **Reader Tasks**: Two dedicated blocking tasks handle I/O — one reads from TUN, one reads from ZMQ. Both forward data through `tokio::sync::mpsc` channels.
4. **Main Loop**: A central `tokio::select!` loop multiplexes incoming data from both directions and handles signals for graceful shutdown.

### Key Design Decisions

- **ZMQ PAIR socket**: Provides reliable, bidirectional, one-to-one communication — ideal for point-to-point tunneling.
- **`IFF_NO_PI` flag**: TUN device operates without packet info headers, so `read()`/`write()` exchange raw IP datagrams directly.
- **`spawn_blocking` for I/O**: Blocking reads/writes are offloaded to tokio's blocking thread pool. The OS parks threads until data arrives, eliminating polling overhead.
- **`mpsc` channels**: Decouple I/O sources from forwarding logic, keeping the async loop clean and responsive.
- **`broadcast` channel**: Coordinates shutdown across all tasks.

## Features

- Bidirectional IP packet forwarding between TUN and ZeroMQ
- Configurable TUN device name, IP address, subnet mask, and MTU
- Server/client mode with flexible TCP addressing
- Async runtime with non-blocking I/O via tokio
- Graceful shutdown on `Ctrl+C` or `SIGTERM`
- Structured logging with configurable verbosity
- Queue bounds to prevent unbounded memory growth under load

## Prerequisites

### System Requirements

- **Linux** kernel with TUN/TAP support (`CONFIG_TUN`)
- **Root privileges** (or `CAP_NET_ADMIN` capability) to create and configure the TUN interface
- **Rust toolchain** (stable channel, 1.70+)

### System Dependencies

#### Debian / Ubuntu
```bash
sudo apt install libzmq3-dev pkg-config
```

#### RHEL / CentOS / Fedora
```bash
sudo dnf install zeromq-devel pkg-config
```

#### Arch Linux
```bash
sudo pacman -S zeromq pkgconf
```

### Rust Installation

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

## Installation

```bash
git clone <repository-url>
cd ZeroMQTunnel
cargo build --release
```

The binary will be located at `target/release/zmq_tun`.

### Quick Test

```bash
# Build and run in debug mode
sudo cargo run -- --mode server
```

## Usage

Run with root privileges. The binary must be started with `sudo` or equivalent to create the TUN interface.

```bash
sudo ./target/release/zmq_tun [OPTIONS] --mode <MODE>
```

## Command-Line Arguments

| Argument | Short | Default | Description |
|----------|-------|---------|-------------|
| `--mode` | `-m` | *(required)* | Operating mode: `server` or `client` |
| `--address` | `-a` | `tcp://0.0.0.0:5555` | ZeroMQ bind/connect address |
| `--tun-name` | `-t` | `tun0` | Name of the TUN interface |
| `--ip` | | `10.0.0.1/24` | IP address and CIDR prefix for the TUN interface |
| `--mtu` | | `1500` | Maximum Transmission Unit for the TUN interface |

### Mode Details

- **`server`**: Binds a ZeroMQ PAIR socket to the specified address and waits for a client to connect.
- **`client`**: Connects to the server at the specified address.

## Examples

### Basic Server-Client Setup

**Machine A (Server)** — binds on port 5555, gets IP `10.0.0.1`:
```bash
sudo ./target/release/zmq_tun --mode server --address tcp://0.0.0.0:5555 --ip 10.0.0.1/24
```

**Machine B (Client)** — connects to Machine A, gets IP `10.0.0.2`:
```bash
sudo ./target/release/zmq_tun --mode client --address tcp://<machine_a_ip>:5555 --ip 10.0.0.2/24
```

**Verify connectivity** from Machine A:
```bash
ping 10.0.0.2
```

### Custom TUN Interface

Use a different interface name and MTU:
```bash
sudo ./target/release/zmq_tun --mode server --tun-name tun1 --ip 192.168.100.1/16 --mtu 9000
```

### Using Development Build

```bash
sudo cargo run -- --mode server --address tcp://0.0.0.0:5555
```

## How It Works

### Startup Sequence

1. Parse CLI arguments via `clap`
2. Initialize structured logging via `env_logger`
3. Open `/dev/net/tun` and create the TUN interface via `ioctl(TUNSETIFF)`
4. Configure the interface: set IP address, netmask, and bring it up via socket ioctls
5. Create a ZMQ context and PAIR socket
6. Bind (server) or connect (client) the ZMQ socket
7. Spawn TUN reader and ZMQ reader tasks
8. Enter the main async forwarding loop

### Module Structure

| File | Purpose |
|------|---------|
| `src/main.rs` | Entry point, CLI parsing, async runtime, main forwarding loop, signal handling |
| `src/tun.rs` | TUN device creation, configuration, and I/O helpers |
| `src/zmq_comm.rs` | ZeroMQ context/socket setup, send/recv wrappers |

### TUN Device Management (`tun.rs`)

1. Opens `/dev/net/tun` with read/write access
2. Allocates a TUN interface using `ioctl(TUNSETIFF)` with flags `IFF_TUN | IFF_NO_PI`
3. Sets MTU via `ioctl(TUNSETMTU)`
4. Configures IP address via `ioctl(SIOCSIFADDR)`
5. Sets netmask via `ioctl(SIOCSIFNETMASK)`
6. Brings interface up via `ioctl(SIOCSIFFLAGS)`

All configuration is done in-process — no external command spawning.

### ZeroMQ Communication (`zmq_comm.rs`)

- Creates a PAIR socket with configured limits:
  - `ZMQ_LINGER = 0` (immediate shutdown)
  - `ZMQ_MAXMSGSIZE = 65536` (accommodates jumbo frames)
  - `ZMQ_SNDHWM = 128` (outbound queue limit)
  - `ZMQ_RCVHWM = 128` (inbound queue limit)
- Socket is wrapped in `Arc<Mutex<>>` for safe sharing across tasks

## Logging

Uses `env_logger` with `log` crate. Control verbosity via the `RUST_LOG` environment variable:

```bash
# Default (info level)
sudo ./target/release/zmq_tun --mode server

# Debug level
RUST_LOG=debug sudo ./target/release/zmq_tun --mode server

# Quiet (only errors)
RUST_LOG=error sudo ./target/release/zmq_tun --mode server
```

Supported levels: `error`, `warn`, `info`, `debug`, `trace`.

## Shutdown & Signal Handling

The application handles the following signals gracefully:

- **`Ctrl+C`** (`SIGINT`): Caught by `tokio::signal::ctrl_c()`
- **`SIGTERM`**: Caught via `tokio::signal::unix::signal()`

On shutdown:
1. Main loop breaks on signal receipt
2. A shutdown message is sent via the `broadcast` channel
3. TUN reader and ZMQ reader tasks exit their loops
4. All resources (TUN file descriptor, ZMQ socket, ZMQ context) are cleaned up via RAII `Drop`

## Performance & Tuning

### Default Settings

| Parameter | Value | Notes |
|-----------|-------|-------|
| TUN MTU | 1500 | Standard Ethernet MTU |
| ZMQ_MAXMSGSIZE | 65536 | Supports jumbo frames |
| ZMQ_SNDHWM | 128 | Outbound message queue limit |
| ZMQ_RCVHWM | 128 | Inbound message queue limit |
| mpsc buffer | 64 | Internal channel capacity |

### Tuning for High Throughput

- Increase `--mtu` for jumbo frames (requires network path support)
- Higher `ZMQ_SNDHWM`/`ZMQ_RCVHWM` values reduce backpressure but increase memory usage
- For low-latency scenarios, consider reducing channel buffer sizes

## Troubleshooting

### "failed to open /dev/net/tun"

Ensure the TUN device node exists:
```bash
ls -l /dev/net/tun
# If missing:
sudo modprobe tun
sudo mknod /dev/net/tun c 10 200
```

### "ioctl TUNSETIFF failed: Permission denied"

The process needs `CAP_NET_ADMIN` capability. Run with `sudo` or grant the capability:
```bash
sudo setcap cap_net_admin+ep ./target/release/zmq_tun
```

### "failed to bind to ..."

The port may already be in use. Check with:
```bash
ss -tlnp | grep 5555
```

### "TUN read error" or "ZMQ recv error"

Check logs at debug level for more details:
```bash
RUST_LOG=debug sudo ./target/release/zmq_tun --mode server
```

### Client Cannot Connect to Server

- Verify the server is running and bound to the correct address
- Check firewall rules: `sudo iptables -L -n`
- Ensure the server address is reachable from the client

### Ping Works But Higher-Level Traffic Doesn't

- Verify both sides are on the same subnet (matching CIDR prefix)
- Check that routing is correct on both machines
- Ensure no other network policies are interfering

## Limitations

- **Linux-only**: Relies on Linux-specific TUN/TAP device and ioctls
- **Point-to-point only**: Uses ZMQ PAIR socket, which supports exactly one peer
- **No encryption**: Traffic is sent in plaintext over TCP
- **No authentication**: Any client can connect to a server
- **IPv4 only**: Current implementation does not support IPv6
- **Requires root**: Creating TUN interfaces requires elevated privileges

## Future Enhancements

- **Multiple clients**: Switch to ROUTER/DEALER pattern for multiplexing
- **Encryption**: Add ZMQ CURVE or TLS support
- **IPv6 support**: Extend TUN configuration for dual-stack operation
- **Config file**: Support TOML/YAML configuration files
- **Metrics**: Prometheus exporter for packet counts, latency, and error rates
- **Non-root operation**: Use user namespaces for privilege reduction
- **Compression**: Optional packet compression for low-bandwidth links

## License

*License information to be added.*
