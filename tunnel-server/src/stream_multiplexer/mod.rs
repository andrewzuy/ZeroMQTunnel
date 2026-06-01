use anyhow::Result;
use bytes::{Bytes, BufMut};

pub struct StreamMultiplexor {
    // Internal channel for streaming frames from zmq to tokio tasks
    pub rx: Option<tokio::sync::mpsc::Receiver<Bytes>>,
}

impl StreamMultiplexor {
    pub fn new() -> Self {
        let (rx, _tx) = tokio::sync::mpsc::channel::<Bytes>(1024);
        Self { rx: Some(rx) }
    }
    
    pub fn handle_frame(&self, bytes: &[u8]) -> Bytes {
        if bytes.is_empty() {
            Bytes::new() // Close signal
        } else {
            Bytes::copy_from_slice(bytes)
        }
    }
}

impl Default for StreamMultiplexor {
    fn default() -> Self {
        Self::new()
    }
}
