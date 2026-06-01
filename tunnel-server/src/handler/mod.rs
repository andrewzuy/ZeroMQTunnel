//! Handler module for managing tunnel stream connections
//!
//! Provides control socket handling, stream session management, and data forwarding.

use std::collections::HashMap;
use bytes::Bytes;
use anyhow::Result;
use tracing::debug;

pub use tunnel_common::{
    ControlMessage, RegistrationResponse, StreamId, ForwardMode,
};

/// Manages stream sessions and data forwarding
#[derive(Debug)]
pub struct StreamHandler {
    /// Maps session ID to connection info
    sessions: HashMap<String, StreamSession>,
}

impl Default for StreamHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamHandler {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    /// Create a new stream session with initial state
    pub fn create_session(&mut self, session_id: &str) -> &StreamSession {
        let session = StreamSession {
            active: true,
            remote_identity: None,
            local_address: None,
            bytes_transferred: 0,
        };
        self.sessions.insert(session_id.to_string(), session);
        debug!("Created stream session: {}", session_id);
        &self.sessions[session_id]
    }

    /// Get a session by ID (immutable)
    pub fn get_session(&self, session_id: &str) -> Option<&StreamSession> {
        self.sessions.get(session_id)
    }

    /// Get mutable session by ID
    pub fn get_session_mut(&mut self, session_id: &str) -> Option<&mut StreamSession> {
        self.sessions.get_mut(session_id)
    }

    /// Remove a session (close it)
    pub fn remove_session(&mut self, session_id: &str) -> bool {
        if let Some(session) = self.sessions.remove(session_id) {
            debug!("Closed and removed session: {}", session_id);
            true
        } else {
            false
        }
    }

    /// Check if session exists
    pub fn has_session(&self, session_id: &str) -> bool {
        self.sessions.contains_key(session_id)
    }

    /// List all active session IDs
    pub fn list_sessions(&self) -> Vec<String> {
        self.sessions.keys().cloned().collect()
    }

    /// Get session count
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Update session bytes transferred
    pub fn record_bytes(&mut self, session_id: &str, bytes: usize) {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.bytes_transferred += bytes;
        }
    }
}

/// Session state for tracking connections
#[derive(Debug, Default)]
pub struct StreamSession {
    /// Whether the session is active
    pub active: bool,
    /// Remote agent identity (CURVE public key)
    pub remote_identity: Option<String>,
    /// Local stream address (if applicable)
    pub local_address: Option<String>,
    /// Bytes transferred counter
    pub bytes_transferred: usize,
}

impl StreamSession {
    /// Mark session as inactive and begin cleanup
    pub fn close(&mut self) {
        self.active = false;
        debug!("Session marked as closed");
    }

    /// Set remote agent identity
    pub fn set_remote_identity(&mut self, identity: impl Into<String>) {
        self.remote_identity = Some(identity.into());
    }

    /// Set local address for this session
    pub fn set_local_address(&mut self, addr: impl Into<String>) {
        self.local_address = Some(addr.into());
    }
}

/// Stream multiplexor for handling multiple concurrent streams
#[derive(Debug)]
pub struct StreamMultiplexor;

impl StreamMultiplexor {
    pub fn new() -> Self {
        Self
    }

    /// Create a multipart ZMQ message with session ID and data
    /// Format: [identity_placeholder, stream_id, data]
    pub fn create_stream_message(
        stream_id: impl Into<String>,
        data: &[u8],
    ) -> Vec<Bytes> {
        if data.is_empty() {
            return vec![Bytes::new()];
        }

        let stream_id_str = stream_id.into();
        vec![
            Bytes::from(""),                      // delimiter (empty identity placeholder)
            Bytes::from(format!("stream:{}", stream_id_str)),
            Bytes::copy_from_slice(data),         // raw data payload
        ]
    }

    /// Forward data to a remote stream identity
    pub fn forward_to_stream(_remote_identity: &str, _data: &[u8]) {
        debug!("Forwarding stream data (would use ZMQ socket in production)");
    }

    /// Handle zero-length frame (stream close signal)
    pub fn handle_close(_identity: &str) {
        debug!("Received close signal for stream");
    }
}

/// Full data plane handler with stream multiplexing and backpressure
#[derive(Debug)]
pub struct StreamDataHandler {
    /// Session state tracking
    sessions: HashMap<String, StreamSession>,
}

impl Default for StreamDataHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamDataHandler {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    /// Create a stream session (stub - actual implementation uses ZMQ)
    pub async fn create_stream(_agent_id: &str) -> Result<String, TunnelError> {
        let session_id = uuid::Uuid::new_v4().to_string();
        debug!("Created stream session: {}", session_id);
        Ok(session_id)
    }

    /// Send data from agent to remote target (stub - needs ZMQ integration)
    pub async fn send_to_remote(
        _session_id: &str,
        _data: &[u8],
        _target_identity: &str,
    ) -> Result<(), TunnelError> {
        debug!("Sending stream data to remote");
        Ok(())
    }

    /// Close a stream session
    pub fn close_session(&mut self, session_id: &str) {
        self.sessions.remove(session_id);
        debug!("Closed session: {}", session_id);
    }

    /// Get current session count
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Handle incoming stream data frame
    pub fn handle_data_frame(
        &mut self,
        _stream_id: String,
        _data: Bytes,
    ) -> Result<(), TunnelError> {
        debug!("Handling data frame");
        Ok(())
    }
}

/// Tunnel error type for stream operations
#[derive(Debug)]
pub enum TunnelError {
    Io(std::io::Error),
    Serde(rmp_serde::decode::Error),
    Zmq(String),
}

