// Phase 4.1 - Connection Limits & Resource Control

use std::sync::Arc;
use tokio::sync::Semaphore;

/// Manages per-agent and global connection limits for resource control
pub struct StreamLimitManager {
    /// Per-agent semaphore for concurrent stream limit
    agent_semaphore: Arc<Semaphore>,
    /// Global maximum number of connections allowed
    global_max: usize,
}

impl StreamLimitManager {
    pub fn new(max_per_agent: usize) -> Self {
        Self {
            agent_semaphore: Arc::new(Semaphore::new(max_per_agent)),
            global_max: 500,
        }
    }

    /// Acquire a slot for a new connection (returns Ok(handle) or Err(Exhausted))
    pub async fn acquire(&self) -> Result<SemaphorePermit<'_>, &'static str> {
        if self.global_max == 0 { return Err("Global limit reached"); }
        match Arc::clone(&self.agent_semaphore).acquire_owned().await {
            Ok(permit) => Ok((permit, "agent")),
            Err(_) => Err("Per-agent limit reached"),
        }
    }

    /// Release a connection slot
    pub fn release(_slot: &()) {
        // Cleanup logic here
    }

    pub fn get_limit(&self) -> usize { self.global_max }
}

#[derive(Clone)]
pub struct SemaphorePermit<'a> {
    _phantom: std::marker::PhantomData<&'a()>,
}

impl Default for StreamLimitManager {
    fn default() -> Self { Self::new(256) }
}
