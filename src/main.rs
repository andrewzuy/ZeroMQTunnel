use std::os::fd::AsRawFd;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use log::{error, info, warn};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table};
use ratatui::Terminal;
use tokio::sync::{broadcast, mpsc};
use tokio::task::spawn_blocking;
use tokio::time::{sleep, Duration};

mod encryption;
mod tun;
mod zmq_comm;

use encryption::AesConfig;
use tun::TunDevice;
use zmq_comm::{ClientRegistry, ZmqChannel};

#[derive(Parser)]
#[command(name = "zmq_tun", about = "TUN-to-ZeroMQ bridge", long_about = "Linux TUN-to-ZeroMQ bridge that forwards IP packets between a TUN interface and a ZeroMQ ROUTER/DEALER socket pair.

Examples:
  # Start server
  sudo zmq_tun run --mode server -a tcp://0.0.0.0:5555 --passphrase mysecret

  # Start client
  sudo zmq_tun run --mode client -a tcp://192.168.1.100:5555 --passphrase mysecret --ip 10.0.0.2/24")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run as server or client
    #[command(
        about = "Run as server or client",
        long_about = "Run the TUN-to-ZeroMQ bridge in server or client mode.\n\nExamples:\n  # Start server\n  sudo zmq_tun run --mode server -a tcp://0.0.0.0:5555 --passphrase mysecret\n\n  # Start client\n  sudo zmq_tun run --mode client -a tcp://192.168.1.100:5555 --passphrase mysecret --ip 10.0.0.2/24"
    )]
    Run(RunArgs),
}

#[derive(Parser)]
struct RunArgs {
    /// Run mode
    #[arg(short, long, value_enum)]
    mode: Mode,

    /// Listen/connect address (e.g. tcp://0.0.0.0:5555)
    #[arg(short, long)]
    address: String,

    /// TUN interface name
    #[arg(short, long, default_value = "tun0")]
    tun_name: String,

    /// TUN interface IP with prefix (e.g. 10.0.0.1/24)
    #[arg(long, default_value = "10.0.0.1/24")]
    ip: String,

    /// TUN interface MTU
    #[arg(long, default_value_t = 1500)]
    mtu: u32,

    /// Passphrase for AES-256 encryption
    #[arg(long)]
    passphrase: String,
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
    client_count: AtomicU64,
    scroll_offset: usize,
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
            client_count: AtomicU64::new(0),
            scroll_offset: 0,
        }
    }

    fn set_client_count(&mut self, count: usize) {
        self.client_count.store(count as u64, Ordering::Relaxed);
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

    match args.command {
        Command::Run(run_args) => {
            run_tunnel(run_args).await?;
            return Ok(());
        }
    }
}

async fn run_tunnel(args: RunArgs) -> Result<()> {
    let mode_str = match args.mode {
        Mode::Server => "server",
        Mode::Client => "client",
    };

    let encryption = Some(AesConfig::from_passphrase(&args.passphrase));
    let encryption_enabled = true;

    info!(
        "Starting zmq_tun: mode={}, address={}, tun={}, ip={}, mtu={}, encryption={}",
        mode_str, args.address, args.tun_name, args.ip, args.mtu, encryption_enabled
    );

    let (ip, prefix_len) = parse_ip_cidr(&args.ip)?;

    let tun = TunDevice::new(&args.tun_name, &ip, prefix_len, args.mtu)?;

    let client_ip = if mode_str == "client" { Some(ip.as_str()) } else { None };

    let zmq_ctx = zmq::Context::new();
    let channel = ZmqChannel::new(
        &zmq_ctx,
        mode_str,
        &args.address,
        client_ip,
        encryption,
    )?;
    let client_registry = channel.client_registry();

    let (shutdown_tx, _) = broadcast::channel::<()>(1);

    let (tun_tx, mut tun_rx) = mpsc::channel::<Vec<u8>>(64);
    let (zmq_tx, mut zmq_rx) = mpsc::channel::<Vec<u8>>(64);

    let tui_state = Arc::new(std::sync::Mutex::new(TuiState::new()));

    let tui_state_clone = tui_state.clone();
    let tui_shutdown_tx = shutdown_tx.clone();
    let mut tui_shutdown = shutdown_tx.subscribe();
    let tui_mode = mode_str.to_string();
    let tui_addr = args.address.clone();
    tokio::spawn(async move {
        run_tui(tui_state_clone, tui_mode, tui_addr, encryption_enabled, tui_shutdown_tx, &mut tui_shutdown).await;
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
    let zmq_registry = client_registry.clone();
    let zmq_channel = channel.clone();
    tokio::spawn(async move {
        if let Err(e) = zmq_reader_loop(zmq_socket_handle, zmq_shutdown_sub, zmq_tx, zmq_tui_state, &zmq_mode, &zmq_registry, &zmq_channel).await {
            error!("ZMQ reader error: {}", e);
        }
        info!("ZMQ reader exited");
    });

    let mut sigterm = tokio::signal::unix::signal(
        tokio::signal::unix::SignalKind::terminate(),
    )
    .unwrap();

    let mut shutdown_signal = shutdown_tx.subscribe();

    let tun_write_file = tun.file().try_clone()?;
    let main_tui_state = tui_state.clone();
    let main_registry = client_registry.clone();
    let is_server = mode_str == "server";

    loop {
        tokio::select! {
            Some(data) = tun_rx.recv() => {
                let data_clone = data.clone();

                if is_server {
                    let dst_ip = extract_dst_ip(&data);
                    let registry = main_registry.clone();
                    let chan = channel.clone();
                    let result = spawn_blocking(move || {
                        if let Some(identity) = registry.get_identity(&dst_ip) {
                            chan.send_to_client(&identity, &data)
                        } else {
                            warn!("No client registered for IP {}, dropping packet", dst_ip);
                            anyhow::Ok(())
                        }
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
                } else {
                    let chan = channel.clone();
                    let result = spawn_blocking(move || {
                        chan.send_raw(&data)
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
    encryption_enabled: bool,
    shutdown_tx: broadcast::Sender<()>,
    shutdown: &mut broadcast::Receiver<()>,
) {
    let mut stdout = std::io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen
    )
    .unwrap();

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(t) => t,
        Err(_) => return,
    };

    loop {
        while crossterm::event::poll(Duration::from_millis(0)).unwrap_or(false) {
            if let Ok(crossterm::event::Event::Key(key_event)) = crossterm::event::read() {
                if key_event.kind != crossterm::event::KeyEventKind::Press {
                    continue;
                }
                match key_event.code {
                    crossterm::event::KeyCode::Char('q')
                        if key_event.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        let _ = shutdown_tx.send(());
                        break;
                    }
                    crossterm::event::KeyCode::Up => {
                        if let Ok(mut s) = state.lock() {
                            if s.scroll_offset > 0 {
                                s.scroll_offset -= 1;
                            }
                        }
                    }
                    crossterm::event::KeyCode::Down => {
                        if let Ok(mut s) = state.lock() {
                            if s.scroll_offset < s.entries.len().saturating_sub(1) {
                                s.scroll_offset += 1;
                            }
                        }
                    }
                    crossterm::event::KeyCode::PageUp => {
                        if let Ok(mut s) = state.lock() {
                            s.scroll_offset = s.scroll_offset.saturating_sub(10);
                        }
                    }
                    crossterm::event::KeyCode::PageDown => {
                        if let Ok(mut s) = state.lock() {
                            s.scroll_offset = (s.scroll_offset + 10).min(s.entries.len().saturating_sub(1));
                        }
                    }
                    crossterm::event::KeyCode::Home => {
                        if let Ok(mut s) = state.lock() {
                            s.scroll_offset = 0;
                        }
                    }
                    crossterm::event::KeyCode::End => {
                        if let Ok(mut s) = state.lock() {
                            s.scroll_offset = u32::MAX as usize;
                        }
                    }
                    _ => {}
                }
            }
        }

        tokio::select! {
            _ = shutdown.recv() => {
                break;
            }
            _ = sleep(Duration::from_millis(50)) => {}
        }

        let snapshot = {
            let s = state.lock().unwrap();
            let entries = s.entries.clone();
            let tun_zmq = s.tun_to_zmq.load(Ordering::Relaxed);
            let zmq_tun = s.zmq_to_tun.load(Ordering::Relaxed);
            let scroll_offset = s.scroll_offset;
            (entries, tun_zmq, zmq_tun, scroll_offset)
        };

        terminal.draw(|f| {
            f.render_widget(Clear, f.area());

            let size = f.area();

            let upper_height = 9;
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

            let (entries, tun_zmq, zmq_tun, scroll_offset) = snapshot;

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
                Line::from(Span::styled(
                    format!("Connected clients: {}", state.lock().unwrap().client_count.load(Ordering::Relaxed)),
                    Style::new().fg(Color::Cyan).bold(),
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
                    format!(
                        "AES-256 encryption: {}",
                        if encryption_enabled { "ENABLED" } else { "DISABLED" }
                    ),
                    Style::new()
                        .fg(if encryption_enabled { Color::Green } else { Color::Yellow })
                        .bold(),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Scroll: \u{2191}\u{2193} PgUp/PgDn Home/End | Ctrl+Q exit",
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

            let visible_rows = chunks[1].height as usize - 2;
            let clamped_offset = if scroll_offset >= entries.len() {
                entries.len().saturating_sub(visible_rows)
            } else {
                scroll_offset
            };
            let end = (clamped_offset + visible_rows).min(entries.len());
            let visible_entries = &entries[clamped_offset..end];

            let inner_chunks = Layout::horizontal([
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(chunks[1]);

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

            let rows: Vec<Row> = visible_entries.iter().map(|e| {
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
                    .borders(Borders::LEFT | Borders::TOP | Borders::BOTTOM)
                    .title(format!(" Packet Log [{}/{}]", clamped_offset + 1, entries.len()))
                    .border_style(Style::new().fg(Color::Cyan))
            )
            .column_spacing(1);

            f.render_widget(table, inner_chunks[0]);

            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
            let mut scrollbar_state = ScrollbarState::new(entries.len())
                .position(clamped_offset);
            f.render_stateful_widget(scrollbar, inner_chunks[1], &mut scrollbar_state);
        }).unwrap();
    }

    let _ = terminal.show_cursor();
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::LeaveAlternateScreen
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
                        if !data.is_empty() {
                            if tx.send(data).await.is_err() {
                                break;
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        if e == nix::Error::EAGAIN || e == nix::Error::EWOULDBLOCK {
                            tokio::time::sleep(Duration::from_millis(5)).await;
                            continue;
                        }
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
    mode: &str,
    registry: &ClientRegistry,
    channel: &ZmqChannel,
) -> Result<()> {
    loop {
        tokio::select! {
            result = spawn_blocking({
                let sock = socket.clone();
                move || {
                    sock.lock()
                        .map_err(|_e| zmq::Error::EHOSTUNREACH)
                        .and_then(|s| {
                            let id_msg = s.recv_msg(0)?;
                            let identity = String::from_utf8_lossy(&*id_msg).to_string();

                            let delim_msg = s.recv_msg(0)?;
                            if !delim_msg.is_empty() {
                                let data = (*delim_msg).to_vec();
                                return Ok((identity, data));
                            }

                            let data_msg = s.recv_msg(0)?;
                            let data = (*data_msg).to_vec();

                            Ok((identity, data))
                        })
                }
            }) => {
                match result {
                    Ok(Ok((identity, data))) => {
                        let decrypted = if let Some(dec) = channel.decrypt(&data) {
                            dec
                        } else {
                            continue;
                        };

                        if mode == "server" {
                            if let Some(client_ip) = ZmqChannel::check_registration(&decrypted) {
                                registry.register(&identity, &client_ip);
                                let count = registry.len();
                                info!("{} clients connected", count);
                                if let Ok(mut state) = tui_state.lock() {
                                    state.set_client_count(count);
                                }
                                continue;
                            }
                        }

                        let status = "RECV";
                        tui_state.lock().unwrap().add_entry("zmq_recv", &decrypted, status);
                        if tx.send(decrypted).await.is_err() {
                            break;
                        }
                    }
                    Ok(Err(zmq::Error::ETERM)) => {
                        break;
                    }
                    Ok(Err(zmq::Error::EAGAIN)) => {
                        tokio::time::sleep(Duration::from_millis(5)).await;
                        continue;
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

fn extract_dst_ip(data: &[u8]) -> String {
    if data.len() >= 20 && (data[0] >> 4) == 4 {
        format!("{}.{}.{}.{}", data[16], data[17], data[18], data[19])
    } else {
        "?".into()
    }
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
