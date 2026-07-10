use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use log::info;
use zmq::{Context as ZmqContext, Socket as ZmqSocket, SocketType};

use crate::curve::CurveConfig;

/// Magic byte prefix for client registration messages.
pub const REGISTRATION_PREFIX: u8 = 0xFE;

/// Tracks connected clients and routes packets by IP.
pub struct ClientRegistry {
    registry: Arc<std::sync::RwLock<HashMap<String, String>>>,
}

impl ClientRegistry {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(std::sync::RwLock::new(HashMap::new())),
        }
    }

    pub fn register(&self, identity: &str, ip: &str) {
        let mut reg = self.registry.write().unwrap();
        reg.insert(identity.to_string(), ip.to_string());
        info!("Client registered: {} -> {}", identity, ip);
    }

    pub fn unregister(&self, identity: &str) {
        let mut reg = self.registry.write().unwrap();
        if let Some(ip) = reg.remove(identity) {
            info!("Client unregistered: {} ({})", identity, ip);
        }
    }

    pub fn get_identity(&self, dst_ip: &str) -> Option<String> {
        self.registry.read().unwrap()
            .iter()
            .find(|(_, ip)| ip.as_str() == dst_ip)
            .map(|(id, _)| id.clone())
    }

    pub fn is_empty(&self) -> bool {
        self.registry.read().unwrap().is_empty()
    }

    pub fn len(&self) -> usize {
        self.registry.read().unwrap().len()
    }

    pub fn clone(&self) -> Self {
        Self {
            registry: self.registry.clone(),
        }
    }
}

pub struct ZmqChannel {
    socket: Arc<std::sync::Mutex<ZmqSocket>>,
    mode: String,
    client_ip: Option<String>,
    client_registry: ClientRegistry,
    curve_enabled: bool,
}

impl Clone for ZmqChannel {
    fn clone(&self) -> Self {
        Self {
            socket: self.socket.clone(),
            mode: self.mode.clone(),
            client_ip: self.client_ip.clone(),
            client_registry: self.client_registry.clone(),
            curve_enabled: self.curve_enabled,
        }
    }
}

impl ZmqChannel {
    pub fn new(
        ctx: &ZmqContext,
        mode: &str,
        address: &str,
        client_ip: Option<&str>,
        curve_config: Option<&CurveConfig>,
    ) -> Result<Self> {
        let socket_type = if mode == "server" { SocketType::ROUTER } else { SocketType::DEALER };
        let socket = ctx
            .socket(socket_type)
            .context("failed to create ZMQ socket")?;

        socket.set_linger(0).context("failed to set ZMQ_LINGER")?;
        socket
            .set_maxmsgsize(65536)
            .context("failed to set ZMQ_MAXMSGSIZE")?;
        socket
            .set_sndhwm(1024)
            .context("failed to set ZMQ_SNDHWM")?;
        socket
            .set_rcvhwm(1024)
            .context("failed to set ZMQ_RCVHWM")?;
        socket
            .set_rcvtimeo(100)
            .context("failed to set ZMQ_RCVTIMEO")?;
        socket
            .set_sndtimeo(100)
            .context("failed to set ZMQ_SNDTIMEO")?;

        if let Some(config) = curve_config {
            apply_curve(&socket, mode, config).context("failed to configure ZMQ CURVE")?;
        }

        if mode == "server" {
            socket.set_immediate(true).context("failed to set ZMQ_IMMEDIATE")?;
            socket
                .bind(&address)
                .context(format!("failed to bind to {}", address))?;
            info!("ZMQ ROUTER bound to {}", address);
        } else {
            if let Some(ip) = client_ip {
                socket.set_identity(ip.as_bytes()).context("failed to set identity")?;
            }
            socket
                .connect(address)
                .context(format!("failed to connect to {}", address))?;
            info!("ZMQ DEALER connected to {}", address);
        }

        let channel = Self {
            socket: Arc::new(std::sync::Mutex::new(socket)),
            mode: mode.to_string(),
            client_ip: client_ip.map(|s| s.to_string()),
            client_registry: ClientRegistry::new(),
            curve_enabled: curve_config.is_some(),
        };

        if mode == "client" {
            if let Some(ref ip) = channel.client_ip {
                // Retry registration a few times to ensure connection is ready
                for attempt in 0..5 {
                    let mut reg_bytes = vec![REGISTRATION_PREFIX];
                    reg_bytes.extend(ip.as_bytes());
                    let msg = zmq::Message::from(reg_bytes);
                    match channel.socket.lock().map(|s| s.send(msg, 0)) {
                        Ok(Ok(())) => break,
                        Ok(Err(e)) => {
                            info!("Registration attempt {} failed: {}, retrying...", attempt + 1, e);
                            std::thread::sleep(std::time::Duration::from_millis(200));
                        }
                        Err(e) => {
                            info!("Registration attempt {} mutex poisoned: {}", attempt + 1, e);
                            break;
                        }
                    }
                }
            }
        }

        Ok(channel)
    }

    pub fn socket_handle(&self) -> Arc<std::sync::Mutex<ZmqSocket>> {
        self.socket.clone()
    }

    pub fn client_registry(&self) -> ClientRegistry {
        self.client_registry.clone()
    }

    pub fn mode(&self) -> &str {
        &self.mode
    }

    pub fn send_to_client(&self, identity: &str, data: &[u8]) -> Result<()> {
        let sock = self.socket.lock()
            .map_err(|_| anyhow::anyhow!("failed to lock socket"))?;

        let id_msg = zmq::Message::from(identity.as_bytes());
        sock.send(id_msg, zmq::SNDMORE)
            .context("failed to send identity frame")?;

        let delim_msg = zmq::Message::new();
        sock.send(delim_msg, zmq::SNDMORE)
            .context("failed to send delimiter frame")?;

        let data_msg = zmq::Message::from(data.to_vec());
        sock.send(data_msg, 0)
            .context("failed to send data frame")?;

        Ok(())
    }

    pub fn recv_with_identity(&self) -> Result<(String, Vec<u8>)> {
        let sock = self.socket.lock()
            .map_err(|_| anyhow::anyhow!("failed to lock socket"))?;

        let id_msg = sock.recv_msg(0)
            .context("failed to recv identity frame")?;
        let identity = String::from_utf8_lossy(&*id_msg).to_string();

        let data_msg = sock.recv_msg(0)
            .context("failed to recv data frame")?;
        let data = (*data_msg).to_vec();

        Ok((identity, data))
    }

    pub fn check_registration(data: &[u8]) -> Option<String> {
        if data.len() > 1 && data[0] == REGISTRATION_PREFIX {
            if let Ok(s) = std::str::from_utf8(&data[1..]) {
                let ip = s.trim();
                if !ip.is_empty() && ip.contains('.') {
                    return Some(ip.to_string());
                }
            }
        }
        None
    }
}

fn apply_curve(socket: &ZmqSocket, mode: &str, config: &CurveConfig) -> Result<()> {
    socket.set_curve_server(true).context("failed to set ZMQ_CURVE_SERVER")?;
    socket
        .set_curve_publickey(&config.own_keys.public_key)
        .context("failed to set ZMQ_CURVE_PUBLICKEY")?;
    socket
        .set_curve_secretkey(&config.own_keys.secret_key)
        .context("failed to set ZMQ_CURVE_SECRETKEY")?;

    if mode != "server" {
        if let Some(ref server_pk) = config.server_public_key {
            socket
                .set_curve_serverkey(server_pk)
                .context("failed to set ZMQ_CURVE_SERVERKEY")?;
        }
    }

    info!("ZMQ CURVE encryption enabled ({})", mode);
    Ok(())
}
