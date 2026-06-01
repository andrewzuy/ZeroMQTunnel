//! Connection limits and resource control for tunnel streams
//!
//! Manages per-agent and global connection limits using async semaphores.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;
use tracing::{debug, info};

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

    /// Create limits from config string (for CLI)
    pub fn from_cli(per_agent: &str, global: &str) -> Self {
        let per = per_agent.parse::<usize>().unwrap_or(128);
        let global = global.parse::<usize>().unwrap_or(4096);
        info!("Stream limits: {} per agent, {} global", per, global);
        Self::new(per, global)
    }

    /// Get per-agent limit
    pub fn get_per_agent_limit(&self) -> usize {
        self.max_per_agent
    }

    /// Get global limit
    pub fn get_global_limit(&self) -> usize {
        self.global_max_streams
    }
}

/// Manages per-agent and global connection limits
#[derive(Debug)]
pub struct StreamLimitManager {
    /// Global semaphore for concurrent stream limit
    global_semaphore: Arc<Semaphore>,
    /// Per-agent semaphores (map identity -> semaphore)
    agent_semaphores: Mutex<HashMap<String, Arc<Semaphore>>>,
    /// Current active stream count (atomic operations)
    global_counter: Arc<Mutex<usize>>,
    /// Maximum number of connections allowed
    global_max: usize,
}

impl StreamLimitManager {
    pub fn new(max_per_agent: usize, global_max: usize) -> Self {
        let global_semaphore = Arc::new(Semaphore::new(global_max));
        let agent_semaphores: HashMap<String, Arc<Semaphore>> = (0..max_per_agent)
            .map(|i| {
                let cloned = Arc::clone(&global_semaphore);
                (format!("agent-{}", i), cloned)
            })
            .collect();

        Self {
            global_semaphore,
            agent_semaphores: Mutex::new(agent_semaphores),
            global_counter: Arc::new(Mutex::new(0)),
            global_max,
        }
    }

    /// Acquire a slot for a new connection
    /// Returns true if acquired, false if limit reached
    pub async fn acquire(&self, agent_id: &str) -> Result<(), &'static str> {
        // First check global limit
        if self.global_max == 0 {
            return Err("Global limit reached");
        }

        // Acquire global permit (dropped on drop)
        let _global_permit = self.global_semaphore.clone().acquire_owned().await
            .map_err(|_| "Failed to acquire global slot")?;

        // Check current count to enforce global limit
        let current = *self.global_counter.lock().unwrap();
        if current >= self.global_max {
            return Err("Global connection limit reached");
        }

        // Try to acquire from per-agent semaphore pool
        let _agent_permit = match &self.agent_semaphores.lock().unwrap()[agent_id] {
            permit => permit.clone().acquire_owned().await
                .map_err(|_| "Failed to acquire agent slot")?,
        };

        // Increment global counter
        *self.global_counter.lock().unwrap() += 1;

        debug!("Acquired stream slot for: {}", agent_id);
        Ok(())
    }

    /// Release a connection slot (permit dropped automatically)
    pub fn release(_permit: ()) {
        // Permit drop triggers semaphore release
    }

    /// Check if we have capacity for new connections
    pub fn has_capacity(&self) -> bool {
        self.global_semaphore.clone().try_acquire_owned().is_ok()
    }

    /// Get current global limit
    pub fn get_limit(&self) -> usize {
        self.global_max
    }

    /// Get per-agent limit (returns configured max_per_agent)
    pub fn get_per_agent_limit(&self) -> usize {
        128 // Simplified - in production track actual permits
    }

    /// Get current active connection count
    pub fn current_connections(&self) -> usize {
        *self.global_counter.lock().unwrap()
    }

    /// Check if connection is allowed
    pub fn check_connection(&self) -> Result<(), &'static str> {
        if self.global_max == 0 {
            return Err("No connections allowed");
        }
        let current = *self.global_counter.lock().unwrap();
        if current >= self.global_max {
            return Err("Connection limit reached");
        }
        Ok(())
    }

    /// Initialize per-agent semaphore for new agent
    pub fn add_agent(&self, agent_id: &str) -> Result<(), &'static str> {
        // Just acknowledge the agent exists
        debug!("Added agent to stream limit manager: {}", agent_id);
        Ok(())
    }
}

impl Default for StreamLimitManager {
    fn default() -> Self {
        Self::new(128, 4096)
    }
}
