//! Core types for ZeroMQ Tunnel protocol

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Error types for tunnel operations
#[derive(Error, Debug)]
pub enum TunnelError {
    #[error("ZMQ error: {0}")]
    Zmq(String),

    #[error("Serialization error: {0}")]
    Serialize(#[from] rmp_serde::encode::Error),

    #[error("Deserialization error: {0}")]
    Deserialize(#[from] rmp_serde::decode::Error),

    #[error("Tunnel error: {0}")]
    Tunnel(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Forwarding mode: remote or local
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ForwardMode {
    Remote,
    Local,
}

impl ForwardMode {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "remote" => Some(Self::Remote),
            "local" => Some(Self::Local),
            _ => None,
        }
    }
}

/// Registration message payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationRequest {
    /// Unique service identifier (human-readable)
    pub service_id: String,
    /// Forwarding mode: remote or local
    #[serde(rename = "forward_type")]
    pub forward_mode: ForwardMode,
    /// For remote forwarding: server port to bind
    /// For local forwarding: client-side listener port
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_port: Option<u16>,
    /// Target address for remote forwarding (e.g., 127.0.0.1:443)
    /// For local forwarding, this is the destination service_id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_host: Option<String>,
    /// Target port for remote forwarding
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_port: Option<u16>,
}

impl RegistrationRequest {
    /// Create a new remote forwarding registration
    pub fn remote(
        service_id: impl Into<String>,
        host: impl Into<String>,
        port: u16,
    ) -> Self {
        Self {
            service_id: service_id.into(),
            forward_mode: ForwardMode::Remote,
            local_port: None,
            target_host: Some(host.into()),
            target_port: Some(port),
        }
    }

    /// Create a new local forwarding registration
    pub fn local(service_id: impl Into<String>, port: u16) -> Self {
        Self {
            service_id: service_id.into(),
            forward_mode: ForwardMode::Local,
            local_port: Some(port),
            target_host: None,
            target_port: None,
        }
    }

    /// Create from string-based legacy format (for backward compatibility)
    pub fn from_legacy(service_id: &str, is_remote: bool, port: u16) -> Self {
        if is_remote {
            Self::remote(service_id, "localhost", port)
        } else {
            Self::local(service_id.to_string(), port)
        }
    }
}

/// Registration response from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationResponse {
    /// Whether registration succeeded
    pub success: bool,
    /// Service ID that was registered (or will be handled)
    pub service_id: String,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Stream session identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct StreamId(pub Uuid);

impl StreamId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_str(&self) -> String {
        self.0.to_string()
    }
}

impl From<Uuid> for StreamId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

/// Stream start signal with routing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStartRequest {
    /// Unique stream identifier
    pub stream_id: StreamId,
    /// For local forwarding: the remote service this routes to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_service_id: Option<String>,
}

/// Stream close signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamCloseRequest {
    pub stream_id: StreamId,
    /// Close reason (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Heartbeat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat;

impl Default for Heartbeat {
    fn default() -> Self {
        Self
    }
}

/// Agent identity (derived from CURVE keypair)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentIdentity(String);

impl AgentIdentity {
    pub fn new(s: String) -> Self {
        Self(s)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Session state for tracking connections
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionState {
    Active,
    Closing,
    Closed,
}