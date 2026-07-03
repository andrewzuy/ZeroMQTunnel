## Detailed Implementation Plan: TUN-to-ZeroMQ Bridge in Rust

### 1. Overview
The application creates a TUN virtual network interface (`/dev/tun0`), reads raw IP packets from it, and forwards them over ZeroMQ to a remote peer. Incoming ZMQ messages are written back to the TUN interface. This effectively tunnels IP traffic (Layer 3) over a ZMQ transport.

The program is designed as a **single-threaded async event loop** using `tokio` to monitor both the TUN file descriptor and the ZMQ socket simultaneously.

### 2. Prerequisites & Dependencies
- **Linux kernel** with TUN/TAP support (`CONFIG_TUN`).
- **Rust toolchain** (rustc, cargo) – stable channel, 1.70+.
- Standard crates: `tokio`, `zmq`, `nix`, `clap`, `libc`.
- Root privileges (CAP_NET_ADMIN) to create and configure the TUN interface.

Installation (Debian/Ubuntu example):
```bash
sudo apt install libzmq3-dev pkg-config
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### 3. Architecture Design
```
+-----------------+        ZMQ PAIR socket        +-----------------+
|   App Instance  | <--------------------------> |  Remote Peer    |
|                 |   tcp://<addr>:<port>         | (same or other  |
| TUN (tun0) <--> |   raw IP packets as ZMQ msgs  |    machine)     |
|  async read/write|                               |                 |
+-----------------+                                +-----------------+
```

- Each instance uses a **ZMQ PAIR** socket for reliable, bidirectional, exactly-one-to-one communication.
- The TUN interface is opened **without** packet info header (`IFF_NO_PI`) so that `read()`/`write()` directly exchange raw IP datagrams.
- One side **binds**, the other **connects**.
- Async runtime (`tokio`) replaces `zmq_poll` for cleaner I/O multiplexing.

### 4. Detailed Implementation Steps

#### 4.1 Project Structure
```
zmq_tun/
├── Cargo.toml
├── src/
│   ├── main.rs         # Entry point, CLI, async runtime, main loop
│   ├── tun.rs          # TUN device handling
│   └── zmq_comm.rs     # ZeroMQ setup and I/O wrapper
└── README.md
```

#### 4.2 Dependencies (`Cargo.toml`)
```toml
[package]
name = "zmq_tun"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full", "signal"] }
zmq = { version = "0.10", features = ["tcp"] }
nix = { version = "0.29", features = ["ioctl", "net", "socket", "uio"] }
clap = { version = "4", features = ["derive"] }
libc = "0.2"
anyhow = "1"
log = "0.4"
env_logger = "0.11"
```

#### 4.3 TUN Device Management (`src/tun.rs`)
1. **Open the TUN clone device** (blocking mode):
   ```rust
   use std::fs::File;
   use std::os::unix::fs::OpenOptionsExt;

   let tun_file = OpenOptions::new()
       .read(true)
       .write(true)
       .open("/dev/net/tun")?;
   ```
   - No `O_NONBLOCK` — blocking reads in `spawn_blocking` are more efficient. The OS parks the thread until data arrives, eliminating polling overhead and sleep-based backoff.

2. **Allocate a TUN interface** using `ioctl(TUNSETIFF)`:
   - Set interface name to `"tun0"` (or user-supplied).
   - Set flags: `IFF_TUN | IFF_NO_PI`.
   - Define the ioctl number manually, since `TUNSETIFF` is a write-write ioctl not covered by nix's convenience macros.

   ```rust
   use std::mem;
   use libc::ifreq;

   let mut ifr: ifreq = unsafe { mem::zeroed() };
   let name = b"tun0";
   for (i, &b) in name.iter().enumerate() {
       unsafe { *ifr.ifr_name.offset(i as isize) = b };
   }
   unsafe {
       ifr.ifr_flags = (libc::IFF_TUN | libc::IFF_NO_PI) as libc::__u16;
   }
   let fd = tun_file.as_raw_fd();
   let ret = unsafe { libc::ioctl(fd, libc::TUNSETIFF, &ifr) };
   anyhow::ensure!(ret == 0, "ioctl TUNSETIFF failed: {}", std::io::Error::last_os_error());
   ```

3. **Set MTU** on the TUN interface:
   ```rust
   const MTU: u32 = 1500;
   let ret = unsafe { libc::ioctl(fd, libc::TUNSETIFF as u32 | 0x40040000, &MTU) };
   // Or use SIOCSIFMTU via a net socket
   ```
   - MTU should match between TUN and ZMQ's `ZMQ_MAXMSGSIZE`. Default 1500 is safe; increase only if jumbo frames are needed.

4. **Configure the interface** (IP address, netmask, bring up) via `nix` ioctls:
   - Create an AF_INET socket for interface configuration ioctls.
   - Use `SIOCSIFADDR` to set IP, `SIOCSIFNETMASK` for netmask, `SIOCSIFFLAGS` to bring up.

   ```rust
   use nix::libc::{AF_INET, SOCK_DGRAM, ioctl, ifreq, sockaddr_in};
   use std::mem;

   let sock = unsafe { libc::socket(AF_INET, SOCK_DGRAM, 0) };
   anyhow::ensure!(sock >= 0, "socket() failed");

   // Helper to fill ifreq with interface name
   fn fill_ifr_name(ifr: &mut ifreq, name: &str) {
       for (i, &b) in name.as_bytes().iter().enumerate() {
           unsafe { *ifr.ifr_name.offset(i as isize) = b };
       }
   }

   // Set IP address (10.0.0.1/24)
   let mut ifr: ifreq = unsafe { mem::zeroed() };
   fill_ifr_name(&mut ifr, "tun0");
   let mut addr: sockaddr_in = unsafe { mem::zeroed() };
   addr.sin_family = AF_INET as u16;
   addr.sin_addr.s_addr = libc::htonl(0x0A000001); // 10.0.0.1
   unsafe {
       std::ptr::copy(
           &addr as *const _ as *const u8,
           ifr.ifr_addr.as_ptr() as *mut u8,
           std::mem::size_of::<sockaddr_in>(),
       );
   }
   let ret = unsafe { ioctl(sock, nix::libc::SIOCSIFADDR, &ifr) };
   anyhow::ensure!(ret == 0, "ioctl SIOCSIFADDR failed: {}", std::io::Error::last_os_error());

   // Set netmask (255.255.255.0)
   let mut ifr: ifreq = unsafe { mem::zeroed() };
   fill_ifr_name(&mut ifr, "tun0");
   let mut mask: sockaddr_in = unsafe { mem::zeroed() };
   mask.sin_family = AF_INET as u16;
   mask.sin_addr.s_addr = libc::htonl(0xFFFFFF00); // 255.255.255.0
   unsafe {
       std::ptr::copy(
           &mask as *const _ as *const u8,
           ifr.ifr_addr.as_ptr() as *mut u8,
           std::mem::size_of::<sockaddr_in>(),
       );
   }
   let ret = unsafe { ioctl(sock, nix::libc::SIOCSIFNETMASK, &ifr) };
   anyhow::ensure!(ret == 0, "ioctl SIOCSIFNETMASK failed: {}", std::io::Error::last_os_error());

   // Bring interface up
   let mut ifr: ifreq = unsafe { mem::zeroed() };
   fill_ifr_name(&mut ifr, "tun0");
   // Get current flags first
   let ret = unsafe { ioctl(sock, nix::libc::SIOCGIFFLAGS, &ifr) };
   anyhow::ensure!(ret == 0, "ioctl SIOCGIFFLAGS failed");
   unsafe {
       ifr.ifr_flags |= libc::IFF_UP as libc::__s16;
   }
   let ret = unsafe { ioctl(sock, nix::libc::SIOCSIFFLAGS, &ifr) };
   anyhow::ensure!(ret == 0, "ioctl SIOCSIFFLAGS failed");

   // Close the temp socket
   unsafe { libc::close(sock) };
   ```
   - This eliminates external process spawning entirely. All configuration is done in-process.
   - Parse the CIDR from CLI (`10.0.0.1/24`) to extract IP and prefix length, then compute netmask.

5. **Wrap in a `TunDevice` struct**:
   ```rust
   pub struct TunDevice {
       file: File,
       name: String,
   }
   ```
   - Implement `Drop` to log cleanup (kernel auto-removes the interface on fd close).

6. **I/O strategy**:
   - `tokio::fs::File` does NOT provide true async I/O for character devices like TUN — it falls back to blocking syscalls.
   - **Correct approach**: Use `spawn_blocking` with blocking `read()`/`write()` on the raw fd. The thread parks until data is available, which is efficient and avoids polling/sleep loops entirely.

#### 4.4 ZeroMQ Setup (`src/zmq_comm.rs`)
1. **Create a ZMQ context**:
   ```rust
   let ctx = zmq::Context::new();
   ```
   - The context must outlive all sockets created from it. Keep it in `main()` scope so it's dropped last.

2. **Create a PAIR socket**:
   ```rust
   let socket = zmq::Socket::new(&ctx, zmq::SocketType::PAIR);
   ```
   - Clone the socket for use in multiple tasks (`zmq::Socket` implements `Clone`).

3. **Set socket options**:
   ```rust
   socket.set_linger(0)?;
   socket.set_max_msg_size(65536)?;       // >= MTU, handles jumbo frames
   socket.set_sndhwm(128)?;              // limit outbound queue
   socket.set_rcvhwm(128)?;              // limit inbound queue
   ```
   - `ZMQ_SNDHWM`/`ZMQ_RCVHWM` prevent unbounded queue growth under load.

4. **Bind or connect** based on CLI argument:
   ```rust
   if mode == Mode::Server {
       socket.bind("tcp://*:5555")?;
   } else {
       socket.connect("tcp://server_ip:5555")?;
   }
   ```

5. **Encapsulate send/recv**:
   - Use **blocking** `socket.send(buf, 0)` and `socket.recv(buf, 0)` inside `spawn_blocking`.
   - The thread parks until the operation completes — no polling, no `EAGAIN` handling, no sleep loops.
   - For shutdown, close the ZMQ context (`zmq::Context::term()`) to unblock any pending operations with `ETERM`.

#### 4.5 Main Event Loop (async, `tokio` + `spawn_blocking`)
Architecture: Two blocking tasks (one for TUN, one for ZMQ) forward data via `mpsc` channels to a central async forwarding loop.

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{broadcast, mpsc};
use tokio::task::spawn_blocking;

// Shared shutdown signal
let shutdown = Arc::new(AtomicBool::new(false));
let (shutdown_tx, _) = broadcast::channel::<()>(1);

let (tun_tx, mut tun_rx) = mpsc::channel::<Vec<u8>>(64);
let (zmq_tx, mut zmq_rx) = mpsc::channel::<Vec<u8>>(64);

// --- TUN reader task ---
let tun_file = tun.device().try_clone();
let tun_shutdown = shutdown.clone();
let tun_shutdown_tx = shutdown_tx.subscribe();
tokio::spawn(async move {
    loop {
        tokio::select! {
            result = spawn_blocking({
                let file = tun_file.try_clone().unwrap();
                move || {
                    let mut buf = vec![0u8; 2048];
                    nix::unistd::read(file.as_raw_fd(), &mut buf)
                        .map(|n| { buf.truncate(n); buf })
                }
            }) => {
                match result {
                    Ok(Ok(data)) => { tun_tx.send(data).await.ok(); }
                    Ok(Err(e)) => {
                        log::error!("TUN read error: {}", e);
                        break;
                    }
                    Err(_) => { /* join error, task cancelled */ break; }
                }
            }
            _ = shutdown_tx.recv() => {
                // Shutdown signalled
                break;
            }
        }
    }
    log::info!("TUN reader exited");
});

// --- ZMQ reader task ---
let zmq_socket = socket.clone();
let zmq_shutdown = shutdown.clone();
let zmq_shutdown_tx = shutdown_tx.subscribe();
tokio::spawn(async move {
    loop {
        tokio::select! {
            result = spawn_blocking({
                let sock = zmq_socket.clone();
                move || {
                    let mut buf = vec![0u8; 65536];
                    match sock.recv_bytes(&mut buf, 0) {
                        Ok(n) => { buf.truncate(n); Ok(buf) }
                        Err(e) => Err(e),
                    }
                }
            }) => {
                match result {
                    Ok(Ok(data)) => { zmq_tx.send(data).await.ok(); }
                    Ok(Err(zmq::Errno::ETERM)) => break, // context terminated
                    Ok(Err(e)) => {
                        log::error!("ZMQ recv error: {}", e);
                        break;
                    }
                    Err(_) => break,
                }
            }
            _ = shutdown_tx.recv() => {
                break;
            }
        }
    }
    log::info!("ZMQ reader exited");
});

// --- Main forwarding loop ---
loop {
    tokio::select! {
        Some(data) = tun_rx.recv() => {
            // Forward TUN -> ZMQ
            spawn_blocking({
                let sock = socket.clone();
                move || sock.send(&data, 0)
            }).await.ok().ok();
        }
        Some(data) = zmq_rx.recv() => {
            // Forward ZMQ -> TUN
            spawn_blocking({
                let file = tun.device().try_clone().unwrap();
                move || nix::unistd::write(file.as_raw_fd(), &data)
            }).await.ok().ok();
        }
        _ = tokio::signal::ctrl_c() => {
            log::info!("Shutting down...");
            shutdown.store(true, Ordering::SeqCst);
            let _ = shutdown_tx.send(());
            break;
        }
    }
}

// Cleanup: drop socket, context, and TUN file via RAII
```

**Key design decisions**:
- Blocking `read()`/`recv()` in `spawn_blocking` — no `O_NONBLOCK`, no `EAGAIN`, no sleep loops.
- `broadcast` channel for shutdown coordination — all tasks exit promptly.
- `AtomicBool` as secondary shutdown flag for any code that needs a quick check.
- Single buffer reused per direction; `Vec<u8>` handles variable packet sizes.
- No threading overhead beyond tokio's work-stealing pool for blocking tasks.

#### 4.6 Signal Handling & Shutdown
- `tokio::signal::ctrl_c()` in the main `select!` triggers shutdown.
- On signal: set `AtomicBool`, send on `broadcast` channel, break main loop.
- Blocking tasks check `broadcast` in their `select!` and exit on signal.
- Alternatively, `zmq::Context::term()` unblocks any pending `recv()` with `ETERM`.
- All resources (TUN file, ZMQ socket, ZMQ context) clean up via RAII `Drop`.
- Add `SIGTERM` handling via `tokio::signal::unix::signal(Signal::TERM)` for container/orchestrator compatibility.

#### 4.7 Command-Line Interface
Using `clap` with derive:
```rust
use clap::Parser;

#[derive(Parser)]
struct Args {
    #[arg(short, long, value_enum)]
    mode: Mode,

    #[arg(short, long, default_value = "tcp://0.0.0.0:5555")]
    address: String,

    #[arg(short, long, default_value = "tun0")]
    tun_name: String,

    #[arg(long, default_value = "10.0.0.1/24")]
    ip: String,

    #[arg(long, default_value_t = 1500)]
    mtu: u32,
}

#[derive(clap::ValueEnum, Clone)]
enum Mode { Server, Client }
```

#### 4.8 Compilation (Cargo)
```bash
cargo build --release
```

No Makefile needed — Cargo handles everything. For cross-compilation or static linking of libzmq, use `cross` crate or Docker.

#### 4.9 Testing
Same as C plan:
1. On machine A (server):
   ```bash
   sudo cargo run --release -- --mode server --address tcp://0.0.0.0:5555 --tun tun0 --ip 10.0.0.1/24
   ```
2. On machine B (client):
   ```bash
   sudo cargo run --release -- --mode client --address tcp://<server_ip>:5555 --tun tun0 --ip 10.0.0.2/24
   ```
3. Ping the remote TUN IP:
   ```bash
   ping 10.0.0.2   # from machine A
   ```

### 5. Error Handling & Robustness
- Use `anyhow::Result<T>` for fallible functions — concise error propagation with context.
- `anyhow` is already in `Cargo.toml`.
- Wrap all fallible operations with `?` and `.context("description")`.
- On TUN read error: log with `log::error!`, break reader task (interface removed or fd closed).
- On ZMQ recv error: `ETERM` → break loop (context terminated); other errors → log and break.
- On TUN write error: log and drop packet (tun buffer full). No retry — backpressure handled by `ZMQ_SNDHWM`.
- **MTU alignment**: TUN MTU set to 1500 (configurable). `ZMQ_MAXMSGSIZE` set to 64KB to accommodate overhead and jumbo frames.
- **Queue bounds**: `ZMQ_SNDHWM`/`ZMQ_RCVHWM` set to 128 to prevent unbounded memory growth.
- Use `Result` types throughout — no panics in the hot path.

### 6. Potential Enhancements (Not Required for Initial Version)
- **Multiple TUN interfaces**: one per client using unique names.
- **Multiplexing**: use ROUTER/DEALER pattern for multiple remote peers.
- **Encryption**: ZMQ CURVE via the `zmq` crate's CurveAPI support.
- **Compression**: optional, generally not needed for IP packets.
- **Config file support**: add `serde` + `toml` for file-based config.
- **Non-root operation**: use user namespaces or custom TUN device nodes.
- **Metrics**: add prometheus exporter for packet counts, latency, errors.

### 7. Key Rust Advantages Over C
- **No manual memory management**: `Vec<u8>` buffers, RAII cleanup on drop.
- **Type-safe CLI**: `clap` derive macros generate help text and validation.
- **Structured logging**: `log` + `env_logger` for configurable log levels.
- **Error handling**: `anyhow` + `Result` propagation replaces error code checking.
- **Async I/O**: `tokio` provides cleaner I/O multiplexing than `zmq_poll`.
- **Channel-based communication**: `tokio::sync::mpsc` decouples I/O sources from forwarding logic.
- **Easier testing**: unit tests with `#[tokio::test]`, mock-friendly architecture.

### 8. Summary
This plan yields a minimal, functional TUN-over-ZeroMQ bridge in Rust:
- **Async single-threaded** with `tokio` avoids synchronization complexity.
- **PAIR socket** provides a simple bidirectional pipe for raw IP datagrams.
- **Raw TUN interface** (`IFF_NO_PI`) simplifies data handling.
- **CLI via `clap`** makes it easy to deploy and test.
- **Memory-safe** with no manual pointer arithmetic or buffer overflow risks.

The application can serve as a foundation for custom VPNs, overlay networks, or lab experiments where traffic must be transported over a messaging fabric.
