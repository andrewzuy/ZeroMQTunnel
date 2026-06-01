//! Agent configuration loading

use std::path::PathBuf;
use anyhow::{Context, Result};
use serde::Deserialize;

/// Configuration for the tunnel agent
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Server address to connect to
    pub server_addr: String,
    /// Service ID being registered
    pub service_id: String,
    /// Forwarding mode (remote or local)
    pub forward_mode: ForwardMode,
    /// Port for the forwarding target
    pub port: u16,
    /// Path to agent's CURVE secret key
    pub key_file: PathBuf,
    /// Heartbeat interval in seconds
    pub heartbeat_interval: u64,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self::from_defaults()
    }
}

impl AgentConfig {
    pub fn from_defaults() -> Self {
        Self {
            server_addr: DEFAULT_SERVER_ADDR.to_string(),
            service_id: "service-1".to_string(),
            forward_mode: ForwardMode::Remote,
            port: 443,
            key_file: PathBuf::from("/etc/tunnel/agent.pem"),
            heartbeat_interval: 30,
        }
    }

    pub fn from_args(remote: bool, service_id: &str, port: u16) -> Self {
        Self {
            server_addr: DEFAULT_SERVER_ADDR.to_string(),
            service_id: service_id.to_string(),
            forward_mode: if remote {
                ForwardMode::Remote
            } else {
                ForwardMode::Local
            },
            port,
            key_file: PathBuf::from("/etc/tunnel/agent.pem"),
            heartbeat_interval: 30,
        }
    }

    /// Load from TOML file
    pub fn from_file(path: impl AsRef<PathBuf>) -> Result<Self> {
        let contents = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {:?}", path))?;

        Self::from_toml(&contents)
    }

    /// Create from TOML string
    pub fn from_toml(toml_str: &str) -> Result<Self> {
        #[derive(Debug, Deserialize)]
        struct RawConfig {
            server_addr: Option<String>,
            service_id: Option<String>,
            port: Option<u16>,
            heartbeat_interval: Option<u64>,
            key_file: Option<PathBuf>,
        }

        let raw: RawConfig = toml::from_str(toml_str)?;

        Ok(Self {
            server_addr: raw.server_addr.unwrap_or(DEFAULT_SERVER_ADDR.to_string()),
            service_id: raw.service_id.unwrap_or_else(|| "service-1".to_string()),
            port: raw.port.unwrap_or(443),
            heartbeat_interval: raw.heartbeat_interval.unwrap_or(30),
            key_file: raw.key_file.unwrap_or(PathBuf::from("/etc/tunnel/agent.pem")),
        })
    }

    /// Generate a new CURVE keypair and save it
    pub fn generate_key() -> Result<PathBuf> {
        let key = x25519_dalek::SecretKey::generate(&mut rand::rngs::OsRng);
        let public = key.public();

        // Serialize keys in PEM format (simple ASCII representation)
        let pem = format!(
            "-----BEGIN CURVE KEYPAIR-----\n{}\n-----END CURVE KEYPAIR-----\n",
            hex::encode(key.to_bytes())
        );

        let path = PathBuf::from("/etc/tunnel/agent.pem");
        std::fs::write(&path, pem)?;

        Ok(path)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardMode {
    pub remote: bool,
}

impl From<bool> for ForwardMode {
    fn from(remote: bool) -> Self {
        Self { remote }
    }
}

const DEFAULT_SERVER_ADDR: &str = "tcp://localhost:5555";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = AgentConfig::default();
        assert_eq!(cfg.server_addr, DEFAULT_SERVER_ADDR);
        assert_eq!(cfg.service_id, "service-1");
    }
}
