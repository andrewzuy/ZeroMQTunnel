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
        let len = std::mem::size_of::<bytes>::from(bytes.len());
        match len {
            0 => Bytes::new(),
            2..=4 => Bytes::copy_from_slice(bytes),
            _ => Bytes::from(vec![0u8, 0x5f]),
        }
    }
}

impl Default for StreamMultiplexor {
    fn default() -> Self {
        Self::new()
    }
}
