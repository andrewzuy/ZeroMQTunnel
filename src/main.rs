use std::os::fd::AsRawFd;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use log::{error, info, warn};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use ratatui::Terminal;
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

#[derive(Clone)]
struct PacketEntry {
    number: u64,
    time_offset: f64,
    direction: String,
    source: String,
    destination: String,
    protocol: String,
    length: usize,
    info: String,
}

struct TuiState {
    entries: Vec<PacketEntry>,
    tun_to_zmq: AtomicU64,
    zmq_to_tun: AtomicU64,
    start_time: Instant,
    zmq_connected: bool,
    max_entries: usize,
}

impl TuiState {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            tun_to_zmq: AtomicU64::new(0),
            zmq_to_tun: AtomicU64::new(0),
            start_time: Instant::now(),
            zmq_connected: false,
            max_entries: 500,
        }
    }

    fn add_entry(&mut self, direction: &str, data: &[u8], status: &str) {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let (src, dst, proto, extra) = parse_ip_packet(data);
        let number = self.entries.len() as u64 + 1;

        if direction == "tun->zmq" {
            self.tun_to_zmq.fetch_add(1, Ordering::Relaxed);
        } else {
            self.zmq_to_tun.fetch_add(1, Ordering::Relaxed);
        }

        self.entries.push(PacketEntry {
            number,
            time_offset: elapsed,
            direction: direction.to_string(),
            source: src,
            destination: dst,
            protocol: proto,
            length: data.len(),
            info: format!("{} {}", status, extra),
        });

        while self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
    }
}

fn parse_ip_packet(data: &[u8]) -> (String, String, String, String) {
    if data.len() < 20 {
        return ("?".into(), "?".into(), "Unknown".into(), "too short".into());
    }

    let version = (data[0] >> 4) & 0xf;
    if version != 4 {
        return (
            "?".into(),
            "?".into(),
            format!("v{}", version),
            "not IPv4".into(),
        );
    }

    let ihl = ((data[0] & 0xf) as usize) * 4;
    let total_len = if data.len() >= ihl {
        u16::from_be_bytes([data[2], data[3]]) as usize
    } else {
        data.len()
    };
    let protocol = data[9];
    let src_ip = format!(
        "{}.{}.{}.{}",
        data[12], data[13], data[14], data[15]
    );
    let dst_ip = format!(
        "{}.{}.{}.{}",
        data[16], data[17], data[18], data[19]
    );

    let proto_name = match protocol {
        1 => "ICMP",
        6 => "TCP",
        17 => "UDP",
        _ => "Other",
    };

    let extra = match protocol {
        1 => {
            if data.len() >= ihl + 4 {
                let icmp_type = data[ihl];
                let icmp_code = data[ihl + 1];
                format!("type={} code={}", icmp_type, icmp_code)
            } else {
                "ICMP".into()
            }
        }
        6 => {
            if data.len() >= ihl + 20 {
                let sport = u16::from_be_bytes([data[ihl], data[ihl + 1]]);
                let dport = u16::from_be_bytes([data[ihl + 2], data[ihl + 3]]);
                let flags = data[ihl + 13];
                let flag_str = format_flags(flags);
                format!(
                    "{}:{} > {}:{} {}",
                    src_ip, sport, dst_ip, dport, flag_str
                )
            } else {
                "TCP".into()
            }
        }
        17 => {
            if data.len() >= ihl + 8 {
                let sport = u16::from_be_bytes([data[ihl], data[ihl + 1]]);
                let dport = u16::from_be_bytes([data[ihl + 2], data[ihl + 3]]);
                format!(
                    "{}:{} > {}:{} len={}",
                    src_ip, sport, dst_ip, dport, total_len.saturating_sub(ihl)
                )
            } else {
                "UDP".into()
            }
        }
        _ => format!("proto={}", protocol),
    };

    (src_ip, dst_ip, proto_name.into(), extra)
}

fn format_flags(flags: u8) -> String {
    let mut s = String::new();
    if flags & 0x02 != 0 {
        s.push_str("SYN");
    }
    if flags & 0x01 != 0 {
        s.push_str("FIN");
    }
    if flags & 0x08 != 0 {
        s.push_str("ACK");
    }
    if flags & 0x10 != 0 {
        s.push_str("RST");
    }
    if flags & 0x20 != 0 {
        s.push_str("PSH");
    }
    if flags & 0x04 != 0 {
        s.push_str("URG");
    }
    if s.is_empty() {
        ".".into()
    } else {
        s
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
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

    let tui_state = Arc::new(std::sync::Mutex::new(TuiState::new()));

    let tui_state_clone = tui_state.clone();
    let mut tui_shutdown = shutdown_tx.subscribe();
    let tui_mode = mode_str.to_string();
    let tui_addr = args.address.clone();
    tokio::spawn(async move {
        run_tui(tui_state_clone, tui_mode, tui_addr, &mut tui_shutdown).await;
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
    let zmq_tui_state = tui_state.clone();
    let zmq_mode = mode_str.to_string();
    tokio::spawn(async move {
        if let Err(e) = zmq_reader_loop(zmq_socket_handle, zmq_shutdown_sub, zmq_tx, zmq_tui_state, &zmq_mode).await {
            error!("ZMQ reader error: {}", e);
        }
        info!("ZMQ reader exited");
    });

    let mut sigterm = tokio::signal::unix::signal(
        tokio::signal::unix::SignalKind::terminate(),
    )
    .unwrap();

    let tun_write_file = tun.file().try_clone()?;
    let main_tui_state = tui_state.clone();

    loop {
        let mut shutdown_signal = shutdown_tx.subscribe();
        tokio::select! {
            Some(data) = tun_rx.recv() => {
                let data_clone = data.clone();
                let sock = channel.socket_handle();
                let result = spawn_blocking(move || {
                    sock.lock().map_err(|e| format!("mutex poisoned: {}", e))
                        .and_then(|s| s.send(&data, 0).map_err(|e| format!("zmq send error: {}", e)))
                }).await;

                let status = match result {
                    Ok(Ok(())) => "OK",
                    Ok(Err(ref e)) => {
                        error!("Forward TUN->ZMQ failed: {:?}", e);
                        "FAIL"
                    }
                    Err(_) => "CANCELLED",
                };

                main_tui_state.lock().unwrap().add_entry("tun->zmq", &data_clone, status);
            }
            Some(data) = zmq_rx.recv() => {
                let data_clone = data.clone();
                let file = tun_write_file.try_clone().unwrap_or_else(|e| {
                    error!("Failed to clone tun file: {}", e);
                    std::process::exit(1);
                });
                let result = spawn_blocking(move || {
                    nix::unistd::write(&file, &data)
                }).await;

                let status = match result {
                    Ok(Ok(_)) => "OK",
                    Ok(Err(ref e)) => {
                        warn!("Forward ZMQ->TUN failed: {:?}", e);
                        "FAIL"
                    }
                    Err(_) => "CANCELLED",
                };

                main_tui_state.lock().unwrap().add_entry("zmq->tun", &data_clone, status);
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

async fn run_tui(
    state: Arc<std::sync::Mutex<TuiState>>,
    mode: String,
    address: String,
    shutdown: &mut broadcast::Receiver<()>,
) {
    let mut stdout = std::io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )
    .unwrap();

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(t) => t,
        Err(_) => return,
    };

    loop {
        tokio::select! {
            _ = shutdown.recv() => {
                break;
            }
            _ = sleep(Duration::from_millis(200)) => {}
        }

        let snapshot = {
            let s = state.lock().unwrap();
            let entries = s.entries.clone();
            let tun_zmq = s.tun_to_zmq.load(Ordering::Relaxed);
            let zmq_tun = s.zmq_to_tun.load(Ordering::Relaxed);
            (entries, tun_zmq, zmq_tun)
        };

        terminal.draw(|f| {
            let size = f.area();

            let upper_height = 8;
            let chunks = Layout::vertical([
                Constraint::Length(upper_height),
                Constraint::Min(1),
            ])
            .split(size);

            let info_chunks = Layout::horizontal([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(chunks[0]);

            let (entries, tun_zmq, zmq_tun) = snapshot;

            let elapsed = state.lock().unwrap().start_time.elapsed();
            let uptime = format_uptime(elapsed);

            let info_text = vec![
                Line::from(Span::styled(
                    format!("Mode: {} | Address: {}", mode, address),
                    Style::new().fg(Color::Cyan),
                )),
                Line::from(Span::styled(
                    format!("Uptime: {}", uptime),
                    Style::new().fg(Color::Green),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!("TUN -> ZMQ packets: {}", tun_zmq),
                    Style::new().fg(Color::Yellow).bold(),
                )),
                Line::from(Span::styled(
                    format!("ZMQ -> TUN packets: {}", zmq_tun),
                    Style::new().fg(Color::Magenta).bold(),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!("Total packets logged: {}", entries.len()),
                    Style::new().fg(Color::White),
                )),
            ];

            let info_block = Block::default()
                .borders(Borders::ALL)
                .title(" zmq_tun - Live Packet Monitor ")
                .border_style(Style::new().fg(Color::Cyan));

            let info_para = Paragraph::new(info_text).block(info_block);
            f.render_widget(info_para, info_chunks[0]);

            let status_block = Block::default()
                .borders(Borders::ALL)
                .title(" ZMQ Status ")
                .border_style(Style::new().fg(Color::Cyan));

            let status_text = vec![
                Line::from(Span::styled(
                    "Connection: ACTIVE",
                    Style::new().fg(Color::Green).bold(),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Press Ctrl+C to exit",
                    Style::new().fg(Color::Gray),
                )),
            ];

            let status_para = Paragraph::new(status_text).block(status_block);
            f.render_widget(status_para, info_chunks[1]);

            if entries.is_empty() {
                let empty_block = Block::default()
                    .borders(Borders::ALL)
                    .title(" Waiting for packets... ")
                    .border_style(Style::new().fg(Color::Gray));
                f.render_widget(empty_block, chunks[1]);
                return;
            }

            let header = vec![
                "No.",
                "Time",
                "Direction",
                "Source",
                "Destination",
                "Protocol",
                "Length",
                "Info",
            ];

            let rows: Vec<Row> = entries.iter().map(|e| {
                let time_str = format!("{:.3}", e.time_offset);
                let dir_color = if e.direction == "tun->zmq" {
                    Color::Yellow
                } else {
                    Color::Magenta
                };
                let status_color = match e.info.split_whitespace().next() {
                    Some("OK") => Color::Green,
                    Some("FAIL") => Color::Red,
                    Some("CANCELLED") => Color::DarkGray,
                    _ => Color::Gray,
                };

                Row::new(vec![
                    Cell::from(format!("{}", e.number)),
                    Cell::from(time_str),
                    Cell::from(Span::styled(
                        e.direction.clone(),
                        Style::new().fg(dir_color).bold(),
                    )),
                    Cell::from(e.source.clone()),
                    Cell::from(e.destination.clone()),
                    Cell::from(e.protocol.clone()),
                    Cell::from(format!("{}", e.length)),
                    Cell::from(Span::styled(
                        e.info.clone(),
                        Style::new().fg(status_color),
                    )),
                ])
            }).collect();

            let widths = vec![
                Constraint::Min(5),
                Constraint::Min(7),
                Constraint::Min(9),
                Constraint::Min(14),
                Constraint::Min(14),
                Constraint::Min(8),
                Constraint::Min(6),
                Constraint::Min(20),
            ];

            let table = Table::new(
                rows,
                widths,
            )
            .header(
                Row::new(header)
                    .style(Style::new().bold().fg(Color::White))
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Packet Log (Wireshark-style) ")
                    .border_style(Style::new().fg(Color::Cyan))
            )
            .column_spacing(1);

            f.render_widget(table, chunks[1]);
        }).unwrap();
    }

    let _ = terminal.show_cursor();
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )
    .unwrap();
}

fn format_uptime(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    let hrs = secs / 3600;
    let mins = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", hrs, mins, s)
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
    tui_state: Arc<std::sync::Mutex<TuiState>>,
    _mode: &str,
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
                        let status = "RECV";
                        tui_state.lock().unwrap().add_entry("zmq_recv", &data, status);
                        if tx.send(data).await.is_err() {
                            break;
                        }
                    }
                    Ok(Err(zmq::Error::ETERM)) => {
                        break;
                    }
                    Ok(Err(ref e)) => {
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