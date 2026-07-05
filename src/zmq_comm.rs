use std::sync::Arc;

use anyhow::{Context, Result};
use log::info;
use zmq::{Context as ZmqContext, Socket as ZmqSocket, SocketType};

pub struct ZmqChannel {
    socket: Arc<std::sync::Mutex<ZmqSocket>>,
}

impl ZmqChannel {
    pub fn new(ctx: &ZmqContext, mode: &str, address: &str) -> Result<Self> {
        let socket = ctx
            .socket(SocketType::PAIR)
            .context("failed to create ZMQ PAIR socket")?;

        socket.set_linger(0).context("failed to set ZMQ_LINGER")?;
        socket
            .set_maxmsgsize(65536)
            .context("failed to set ZMQ_MAXMSGSIZE")?;
        socket
            .set_sndhwm(128)
            .context("failed to set ZMQ_SNDHWM")?;
        socket
            .set_rcvhwm(128)
            .context("failed to set ZMQ_RCVHWM")?;
        socket
            .set_rcvtimeo(100)
            .context("failed to set ZMQ_RCVTIMEO")?;
        socket
            .set_sndtimeo(100)
            .context("failed to set ZMQ_SNDTIMEO")?;

        match mode {
            "server" => {
                let bind_addr = if address.ends_with(":5555") || address.contains("*") {
                    address.to_string()
                } else {
                    "tcp://*:5555".to_string()
                };
                socket
                    .bind(&bind_addr)
                    .context(format!("failed to bind to {}", bind_addr))?;
                info!("ZMQ bound to {}", bind_addr);
            }
            "client" => {
                socket
                    .connect(address)
                    .context(format!("failed to connect to {}", address))?;
                info!("ZMQ connected to {}", address);
            }
            _ => anyhow::bail!("unknown mode: {}", mode),
        }

        Ok(Self {
            socket: Arc::new(std::sync::Mutex::new(socket)),
        })
    }

    pub fn socket_handle(&self) -> Arc<std::sync::Mutex<ZmqSocket>> {
        self.socket.clone()
    }

    pub fn send_packet(&self, data: &[u8]) -> Result<()> {
        self.socket
            .lock()
            .map_err(|_| anyhow::anyhow!("failed to lock socket"))?
            .send(data, 0)
            .context("failed to send packet over ZMQ")?;
        Ok(())
    }

    pub fn recv_packet(&self) -> Result<Vec<u8>> {
        self.socket
            .lock()
            .map_err(|_| anyhow::anyhow!("failed to lock socket"))?
            .recv_bytes(0)
            .context("failed to recv packet from ZMQ")
    }
}
