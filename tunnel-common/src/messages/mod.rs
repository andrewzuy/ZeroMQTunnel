//! Control protocol messages for ZeroMQ Tunnel

use serde::{Deserialize, Serialize};

use crate::types::ForwardMode;

/// Control message types for the protocol - simplified for MessagePack serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlMessage {
    /// Agent registration with forwarding details
    Register(RegisterRequest),
    /// Server response to registration
    RegisterResponse(RegistrationResponse),
    /// Stream session started
    StreamStart(StreamStart),
    /// Stream data frame (binary payload)
    StreamData(Vec<u8>),
    /// Close stream session
    StreamClose(StreamClose),
    /// Heartbeat from agent (empty marker)
    Heartbeat, // Unit variant for simplicity in protocol
    /// Heartbeat acknowledgment
    HeartbeatAck, // Unit variant for simplicity in protocol
    /// Unregistration request
    Unregister(UnregisterRequest),
    /// Server shutdown notice
    Shutdown, // Unit variant for simplicity in protocol
}

impl ControlMessage {
    /// Serialize to MessagePack bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec_named(self)
    }

    /// Create from raw MessagePack bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        rmp_serde::from_slice(bytes).ok()
    }
}

/// Agent registration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub service_id: String,
    #[serde(rename = "forward_type")]
    pub forward_mode: ForwardMode,
    /// For local forwarding: the port agent listens on
    #[serde(skip_serializing_if = "Option::is_none")]
    pub listener_port: Option<u16>,
}

/// Server response to registration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationResponse {
    pub success: bool,
    pub service_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Default for RegistrationResponse {
    fn default() -> Self {
        Self {
            success: false,
            service_id: String::new(),
            error: None,
        }
    }
}

/// Stream session start request (local forwarding)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStart {
    pub stream_id: String, // Using string for simplicity with MessagePack
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_service_id: Option<String>,
}

/// Stream data frame (payload marker - actual data is in the variant)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamData {
    /// Stream identifier for this data frame
    pub stream_id: String,
}

impl StreamData {
    /// Create stream data with a given stream ID
    pub fn new(stream_id: impl Into<String>) -> Self {
        Self { stream_id: stream_id.into() }
    }
}

/// Stream close request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamClose {
    pub stream_id: String,
    /// Optional reason for closing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Unregister request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnregisterRequest {
    pub service_id: String,
    /// Optional message about why unregistering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Helper function for creating registration message from string (legacy CLI format)
pub fn register_msg(service_id: &str, is_remote: bool, port: u16) -> RegistrationResponse {
    let request = if is_remote {
        RegisterRequest {
            service_id: service_id.to_string(),
            forward_mode: ForwardMode::Remote,
            listener_port: None,
        }
    } else {
        RegisterRequest {
            service_id: service_id.to_string(),
            forward_mode: ForwardMode::Local,
            listener_port: Some(port),
        }
    };

    RegistrationResponse {
        success: true,
        service_id: request.service_id.clone(),
        error: None,
    }
}
