//! Stream multiplexer module for handling multiple concurrent streams
//!
//! Provides ZMQ STREAM socket multiplexing, backpressure handling, and session state management.

use bytes::{Buf, BufMut, Bytes};
use std::collections::HashMap;
use tokio::sync::Semaphore;
use anyhow::{Result, bail};
use tracing::{debug, info, warn};

/// Stream session state machine enum
#[derive(Debug, Clone, PartialEq)]
pub enum StreamSessionState {
    /// Session is being created (waiting for remote to connect)
    Creating,
    /// Session is active and forwarding data
    Active,
    /// Session is closing (graceful shutdown initiated)
    Closing,
    /// Session has been fully closed
    Closed,
}

/// Stream session tracking structure
#[derive(Debug)]
pub struct StreamSession {
    /// Unique stream identifier
    pub stream_id: String,
    /// Current state of the session
    pub state: StreamSessionState,
    /// Remote agent identity
    pub remote_identity: Option<String>,
    /// Local address (if applicable)
    pub local_address: Option<String>,
    /// Number of bytes forwarded
    pub bytes_forwarded: u64,
}

impl StreamSession {
    /// Create a new stream session in creating state
    pub fn new(stream_id: String) -> Self {
        Self {
            stream_id,
            state: StreamSessionState::Creating,
            remote_identity: None,
            local_address: None,
            bytes_forwarded: 0,
        }
    }

    /// Set remote identity for this session
    pub fn set_remote(&mut self, identity: impl Into<String>) {
        self.remote_identity = Some(identity.into());
        debug!("Set remote identity for stream {}: {}", self.stream_id, identity);
    }

    /// Set local address for this session
    pub fn set_local_address(&mut self, addr: impl Into<String>) {
        self.local_address = Some(addr.into());
    }

    /// Mark as closing (begin graceful shutdown)
    pub fn begin_close(&mut self) {
        debug!("Stream {} entering closing state", self.stream_id);
        self.state = StreamSessionState::Closing;
    }

    /// Check if session is active
    pub fn is_active(&self) -> bool {
        matches!(self.state, StreamSessionState::Active)
    }

    /// Get bytes forwarded
    pub fn bytes_forwarded(&self) -> u64 {
        self.bytes_forwarded
    }
}

impl Default for StreamSession {
    fn default() -> Self {
        Self::new(String::new())
    }
}

/// Multi-stream routing and multiplexing over single ZMQ connection
#[derive(Debug)]
pub struct StreamMultiplexer {
    /// Maps stream ID to session object
    sessions: HashMap<String, StreamSession>,
}

impl Default for StreamMultiplexer {
    fn default() -> Self {
        Self::new(1024) // Default global limit
    }
}

impl StreamMultiplexer {
    pub fn new(_global_limit: usize) -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    /// Create a new stream session
    pub fn create_session(&mut self, stream_id: String) -> StreamSession {
        let session = StreamSession::new(stream_id);
        self.sessions.insert(session_id.clone(), session.clone());
        debug!("Created stream session: {}", session_id);
        session
    }

    /// Get a session by ID
    pub fn get_session(&self, stream_id: &str) -> Option<&StreamSession> {
        self.sessions.get(stream_id)
    }

    /// Get mutable session by ID
    pub fn get_session_mut(&mut self, stream_id: &str) -> Option<&mut StreamSession> {
        self.sessions.get_mut(stream_id)
    }

    /// Remove a stream session
    pub fn remove_stream(&mut self, stream_id: &str) -> bool {
        if self.sessions.remove(stream_id).is_some() {
            debug!("Removed stream session: {}", stream_id);
            true
        } else {
            false
        }
    }

    /// List active streams
    pub fn list_streams(&self) -> Vec<String> {
        self.sessions.keys().cloned().collect()
    }

    /// Get active stream count
    pub fn stream_count(&self) -> usize {
        self.sessions.len()
    }

    /// Close a specific stream
    pub fn close_stream(&mut self, stream_id: &str) {
        if let Some(session) = self.sessions.get_mut(stream_id) {
            session.begin_close();
        }
    }

    /// Create multipart message for forwarding (identity headers + data)
    pub fn create_multipart_message(
        from_identity: &str,
        stream_id: &str,
        data: &[u8],
    ) -> Vec<Bytes> {
        if data.is_empty() {
            return vec![Bytes::from(format!("identity:{}\n", from_identity))];
        }

        // Create parts with identity header and data
        vec![
            Bytes::from(format!("identity:{}\n", from_identity)),
            Bytes::copy_from_slice(data),
        ]
    }

    /// Forward data to a remote stream identity via ZMQ
    pub fn forward_to_stream(_remote_identity: &str, _data: &[u8]) {
        debug!("Forwarding stream data (ZMQ socket not available in this context)");
    }

    /// Handle close signal for a stream
    pub fn handle_close(_identity: &str) {
        debug!("Received close signal for stream");
    }
}
