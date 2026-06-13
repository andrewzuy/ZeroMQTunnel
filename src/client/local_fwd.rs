//! Local Forwarding Mode Client (-L mode) - Phase 4
// Implements plan.md Section 3.1 local forwarding table: "Browser → ZMQ_STREAM → DEALER"

use zmq::{Socket as zmq_socket};

/// Local forwarder for SSH -L equivalent (Section 3.1 plan.md)  
pub struct LocalForwarder {
    pub socket: zmq_socket,
}

impl Default for LocalForwarder { 
    fn default() -> Self { 
        Self{socket: zmq_socket(zmq::Context::default(), zmq::SocketType::Stream).unwrap()}  
    }  
}
