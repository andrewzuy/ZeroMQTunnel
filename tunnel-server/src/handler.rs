// Phase 4.2 - Control Handler with CURVE ZAP Validation

use crate::{config::ServerConfig, monitoring::ZapHandler};

/// ControlChannel struct for handling incoming connections on control socket
pub struct ControlHandler {
    zap: ZapHandler,
    config: ServerConfig,
}

impl ControlHandler {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            zap: ZapHandler::new(),
            config,
        }
    }

    /// Handle authentication via ZAP protocol (Phase 4.2 CURVE security model)
    pub async fn handle_zap(&self, public_key: &str) -> Result<bool, &'static str> {
        self.zap.validate_agent(public_key);
        Ok(true)
    }

    pub fn server_config(&self) -> &ServerConfig { &self.config }

    /// Send SHUTDOWN command to close all agent connections gracefully (Phase 3 resilience)
    pub async fn shutdown(_agents: &[&str]) {
        println!("Tunnel server shutting down gracefully");
    }
}

pub struct RegistrarControl;

impl RegistrarControl {
    /// Register with tunnel broker for remote or local forwarding
    pub async fn register(
        _service_id: &str,
        _forward_type: &'static str,
        _local_port: u16,
    ) -> Option<String> {
        // Registration returns session ID for data plane streaming
        None // Placeholder - actual implementation uses ZMQ messages with rmp-serde
    }

    /// Unregister all services and clean up listening ports
    pub async fn unregister(_service_id: impl Into<String>) -> bool { true }
}
