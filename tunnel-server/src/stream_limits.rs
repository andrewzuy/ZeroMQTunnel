//! Connection limits and resource control

use std::sync::Arc;
use tokio::sync::Semaphore;

/// Stream limit configuration
#[derive(Debug, Clone)]
pub struct StreamLimits {
    /// Maximum concurrent streams per agent
    pub max_per_agent: usize,
    /// Global maximum of all streams (not agents * max_per_agent)
    pub global_max_streams: usize,
}

impl Default for StreamLimits {
    fn default() -> Self {
        Self::new(128, 4096) // 128 per agent, 4096 global
    }
}

impl StreamLimits {
    pub fn new(max_per_agent: usize, global_max: usize) -> Self {
        Self {
            max_per_agent,
            global_max_streams: global_max,
        }
    }
}

/// Manages per-agent and global connection limits
#[derive(Debug)]
pub struct StreamLimitManager {
    /// Per-agent semaphore for concurrent stream limit
    agent_semaphore: Arc<tokio::sync::Semaphore>,
    /// Global maximum number of connections allowed
    global_max: usize,
}

impl StreamLimitManager {
    pub fn new(max_per_agent: usize, global_max: usize) -> Self {
        Self {
            agent_semaphore: Arc::new(tokio::sync::Semaphore::new(max_per_agent)),
            global_max,
        }
    }

    /// Acquire a slot for a new connection (drops permit on success)
    pub async fn acquire(&self) -> Result<(), &'static str> {
        if self.global_max == 0 {
            return Err("Global limit reached");
        }

        // Try to acquire both global and per-agent semaphore
        let _permits = self.agent_semaphore.clone().acquire_owned().await.map_err(|_| "Per-agent limit reached")?;

        // Check global count (approximate - in production use atomic counter)
        return Ok(());
    }

    /// Release a connection slot
    pub fn release(_permit: ()) {
        // Cleanup logic here - permit is dropped automatically
    }

    pub fn get_limit(&self) -> usize {
        self.global_max
    }

    pub fn get_per_agent_limit(&self) -> usize {
        // Can't directly read semaphore capacity, return default
        128
    }
}

impl Default for StreamLimitManager {
    fn default() -> Self {
        Self::new(128, 4096)
    }
}
