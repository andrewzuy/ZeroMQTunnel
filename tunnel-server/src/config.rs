//! Server configuration loading and validation

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Deserialize;

/// Configuration for the tunnel server
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// Control port for ZMQ ROUTER socket
    pub control_port: u16,
    /// Data port for ZMQ STREAM sockets
    pub data_port: Option<u16>,
    /// Listen address (default: 0.0.0.0)
    pub listen_address: String,
    /// Path to server CURVE secret key
    pub key_file: PathBuf,
    /// Maximum number of connections
    pub global_max_connections: usize,
    /// Port range for remote forwards
    pub port_range_start: u16,
    /// Port range end for remote forwards
    pub port_range_end: u16,
    /// Heartbeat timeout in seconds
    pub heartbeat_timeout: u64,
    /// Connection idle timeout in seconds
    pub connection_idle_timeout: u64,
    /// Logging level (info, debug, trace)
    pub log_level: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self::from_defaults()
    }
}

impl ServerConfig {
    /// Create from default values
    pub fn from_defaults() -> Self {
        Self {
            control_port: 5555,
            data_port: Some(5556),
            listen_address: "0.0.0.0".to_string(),
            key_file: PathBuf::from("/etc/tunnel/server.pem"),
            global_max_connections: 1000,
            port_range_start: 1024,
            port_range_end: 65535,
            heartbeat_timeout: 30,
            connection_idle_timeout: 300,
            log_level: "info".to_string(),
        }
    }

    /// Create from TOML file
    pub fn from_file<P>(path: P) -> Result<Self>
    where
        P: AsRef<std::path::Path>,
    {
        let path = path.as_ref();
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;

        Self::from_toml(&contents)
    }

    /// Create from TOML string
    pub fn from_toml(toml_str: &str) -> Result<Self> {
        #[derive(Debug, Deserialize)]
        struct RawConfig {
            control_port: Option<u16>,
            data_port: Option<u16>,
            listen_address: Option<String>,
            key_file: Option<PathBuf>,
            global_max_connections: Option<usize>,
            port_range_start: Option<u16>,
            port_range_end: Option<u16>,
            heartbeat_timeout: Option<u64>,
            connection_idle_timeout: Option<u64>,
            log_level: Option<String>,
        }

        let raw: RawConfig = toml::from_str(toml_str)?;

        Ok(Self {
            control_port: raw.control_port.unwrap_or(5555),
            data_port: raw.data_port,
            listen_address: raw.listen_address.unwrap_or_else(|| "0.0.0.0".to_string()),
            key_file: raw.key_file.unwrap_or_else(|| PathBuf::from("/etc/tunnel/server.pem")),
            global_max_connections: raw.global_max_connections.unwrap_or(1000),
            port_range_start: raw.port_range_start.unwrap_or(1024),
            port_range_end: raw.port_range_end.unwrap_or(65535),
            heartbeat_timeout: raw.heartbeat_timeout.unwrap_or(30),
            connection_idle_timeout: raw.connection_idle_timeout.unwrap_or(300),
            log_level: raw.log_level.unwrap_or_else(|| "info".to_string()),
        })
    }

    /// Load from file with fallback to defaults if not found
    pub fn load<P>(path: P) -> Result<Self>
    where
        P: AsRef<std::path::Path>,
    {
        match Self::from_file(path) {
            Ok(cfg) => Ok(cfg),
            Err(e) => {
                tracing::warn!("Config file not found, using defaults: {}", e);
                Ok(Self::from_defaults())
            }
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), anyhow::Error> {
        if self.control_port < 1024 {
            return Err(anyhow::anyhow!("control_port must be >= 1024 (non-privileged port)"));
        }
        if let Some(port) = self.data_port {
            if port < 1024 {
                return Err(anyhow::anyhow!("data_port must be >= 1024 (non-privileged port)"));
            }
        }
        Ok(())
    }

    /// Get control socket address
    pub fn control_addr(&self) -> String {
        format!("tcp://{}:{}", self.listen_address, self.control_port)
    }

    /// Get data socket address (if configured)
    pub fn data_addr(&self) -> Option<String> {
        self.data_port.map(|port| format!("tcp://{}:{}", self.listen_address, port))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = ServerConfig::default();
        assert_eq!(cfg.control_port, 5555);
        assert_eq!(cfg.global_max_connections, 1000);
    }
}
