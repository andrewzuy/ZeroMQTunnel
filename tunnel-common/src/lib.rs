//! ZeroMQ Tunnel Common Library
//!
//! Shared types and utilities used by both server and agent.

pub mod messages;
pub mod registrar;
pub mod registry;
pub mod utils;
pub mod types;

pub use messages::{ControlMessage, RegistrationResponse};
pub use registrar::AgentRegistrar;
pub use registry::ServiceRegistry;
pub use types::{
    ForwardMode, RegistrationRequest, RegistrationResponse as RegResponse, SessionState,
    StreamId, TunnelError, AgentIdentity, Heartbeat, StreamStartRequest, StreamCloseRequest,
};

// Re-export rmp-serde serialization functions for convenience
pub use rmp_serde::to_vec;
