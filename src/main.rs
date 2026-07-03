use std::os::fd::AsRawFd;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use log::{error, info, warn};
use tokio::sync::{broadcast, mpsc};
use tokio::task::spawn_blocking;
use tokio::time::{sleep, Duration};

mod tun;
mod zmq_comm;

use tun::TunDevice;
use zmq_comm::ZmqChannel;

#[derive(Parser)]
#[command(name = "zmq_tun", about = "TUN-to-ZeroMQ bridge")]
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

#[derive(ValueEnum, Clone)]
enum Mode {
    Server,
    Client,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();

    let args = Args::parse();
    let mode_str = match args.mode {
        Mode::Server => "server",
        Mode::Client => "client",
    };

    info!(
        "Starting zmq_tun: mode={}, address={}, tun={}, ip={}, mtu={}",
        mode_str, args.address, args.tun_name, args.ip, args.mtu
    );

    let (ip, prefix_len) = parse_ip_cidr(&args.ip)?;

    let tun = TunDevice::new(&args.tun_name, &ip, prefix_len, args.mtu)?;

    let zmq_ctx = zmq::Context::new();
    let channel = ZmqChannel::new(&zmq_ctx, mode_str, &args.address)?;

    let (shutdown_tx, _) = broadcast::channel::<()>(1);

    let (tun_tx, mut tun_rx) = mpsc::channel::<Vec<u8>>(64);
    let (zmq_tx, mut zmq_rx) = mpsc::channel::<Vec<u8>>(64);

    let tun_to_zmq_count = Arc::new(AtomicU64::new(0));
    let zmq_to_tun_count = Arc::new(AtomicU64::new(0));

    let stats_tun_count = tun_to_zmq_count.clone();
    let stats_zmq_count = zmq_to_tun_count.clone();
    let mut stats_shutdown = shutdown_tx.subscribe();
    tokio::spawn(async move {
        let mode = mode_str;
        loop {
            tokio::select! {
                _ = sleep(Duration::from_secs(1)) => {
                    let sent = stats_tun_count.load(Ordering::Relaxed);
                    let recv = stats_zmq_count.load(Ordering::Relaxed);
                    eprintln!("[{mode}] tun->zmq: {sent} | zmq->tun: {recv}");
                }
                _ = stats_shutdown.recv() => {
                    break;
                }
            }
        }
    });

    let tun_file = tun.file().try_clone()?;
    let tun_shutdown_sub = shutdown_tx.subscribe();
    tokio::spawn(async move {
        if let Err(e) = tun_reader_loop(tun_file, tun_shutdown_sub, tun_tx).await {
            error!("TUN reader error: {}", e);
        }
        info!("TUN reader exited");
    });

    let zmq_socket_handle = channel.socket_handle();
    let zmq_shutdown_sub = shutdown_tx.subscribe();
    tokio::spawn(async move {
        if let Err(e) =
            zmq_reader_loop(zmq_socket_handle, zmq_shutdown_sub, zmq_tx).await
        {
            error!("ZMQ reader error: {}", e);
        }
        info!("ZMQ reader exited");
    });

    let mut sigterm = tokio::signal::unix::signal(
        tokio::signal::unix::SignalKind::terminate(),
    )
    .unwrap();

    let tun_write_file = tun.file().try_clone()?;

    loop {
        let mut shutdown_signal = shutdown_tx.subscribe();
        tokio::select! {
            Some(data) = tun_rx.recv() => {
                tun_to_zmq_count.fetch_add(1, Ordering::Relaxed);
                let sock = channel.socket_handle();
                if let Err(e) = spawn_blocking(move || {
                    sock.lock().map_err(|e| format!("mutex poisoned: {}", e))
                        .and_then(|s| s.send(&data, 0).map_err(|e| format!("zmq send error: {}", e)))
                }).await {
                    error!("Forward TUN->ZMQ failed: {:?}", e);
                }
            }
            Some(data) = zmq_rx.recv() => {
                zmq_to_tun_count.fetch_add(1, Ordering::Relaxed);
                let file = tun_write_file.try_clone().unwrap_or_else(|e| {
                    error!("Failed to clone tun file: {}", e);
                    std::process::exit(1);
                });
                if let Err(e) = spawn_blocking(move || {
                    nix::unistd::write(&file, &data)
                }).await {
                    warn!("Forward ZMQ->TUN failed: {:?}", e);
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Ctrl-C received, shutting down...");
                break;
            }
            _ = sigterm.recv() => {
                info!("SIGTERM received, shutting down...");
                break;
            }
            _ = shutdown_signal.recv() => {
                break;
            }
            else => {
                sleep(Duration::from_millis(10)).await;
            }
        }
    }

    let _ = shutdown_tx.send(());

    info!("Cleanup complete");
    Ok(())
}

async fn tun_reader_loop(
    file: std::fs::File,
    mut shutdown_rx: broadcast::Receiver<()>,
    tx: mpsc::Sender<Vec<u8>>,
) -> Result<()> {
    loop {
        tokio::select! {
            result = spawn_blocking({
                let file = file.try_clone().unwrap();
                move || {
                    let mut buf = vec![0u8; 2048];
                    nix::unistd::read(file.as_raw_fd(), &mut buf)
                        .map(|n| { buf.truncate(n); buf })
                }
            }) => {
                match result {
                    Ok(Ok(data)) => {
                        if tx.send(data).await.is_err() {
                            break;
                        }
                    }
                    Ok(Err(e)) => {
                        error!("TUN read error: {}", e);
                        break;
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                break;
            }
        }
    }
    Ok(())
}

async fn zmq_reader_loop(
    socket: Arc<std::sync::Mutex<zmq::Socket>>,
    mut shutdown_rx: broadcast::Receiver<()>,
    tx: mpsc::Sender<Vec<u8>>,
) -> Result<()> {
    loop {
        tokio::select! {
            result = spawn_blocking({
                let sock = socket.clone();
                move || {
                    let msg = sock.lock()
                        .map_err(|_e| zmq::Error::EHOSTUNREACH)
                        .and_then(|s| s.recv_bytes(0));
                    msg
                }
            }) => {
                match result {
                    Ok(Ok(data)) => {
                        if tx.send(data).await.is_err() {
                            break;
                        }
                    }
                    Ok(Err(zmq::Error::ETERM)) => {
                        break;
                    }
                    Ok(Err(e)) => {
                        error!("ZMQ recv error: {}", e);
                        break;
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                break;
            }
        }
    }
    Ok(())
}

fn parse_ip_cidr(cidr: &str) -> Result<(String, u8)> {
    let parts: Vec<&str> = cidr.split('/').collect();
    let ip = parts[0].to_string();
    let prefix = if parts.len() > 1 {
        parts[1].parse::<u8>().context("invalid prefix length")?
    } else {
        24
    };
    Ok((ip, prefix))
}
