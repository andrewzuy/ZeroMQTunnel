//! Registrar module for handling agent registrations and heartbeats
//!
//! Manages service lifecycle, agent identity tracking, and heartbeat monitoring.

use std::collections::HashMap;
use std::sync::RwLock;
use anyhow::{Result, bail};
use tracing::{debug, info, warn};

pub use tunnel_common::{AgentIdentity, TunnelError, RegistrationResponse, StreamId, ForwardMode};

/// Manages agent registrations and session tracking
#[derive(Debug)]
pub struct Registrar {
    /// Maps service ID to agent identity (CURVE public key)
    services: RwLock<HashMap<String, String>>,
    /// Maps agent identity to their registered service IDs
    agents: RwLock<HashMap<String, Vec<String>>>,
    /// Heartbeat manager for liveness checks
    heartbeat_timeout_ms: u64,
}

impl Default for Registrar {
    fn default() -> Self {
        Self::new(30_000) // 30 second timeout in ms
    }
}

impl Registrar {
    /// Create new Registrar with specified heartbeat timeout
    pub fn new(heartbeat_timeout_ms: u64) -> Self {
        Self {
            services: RwLock::new(HashMap::new()),
            agents: RwLock::new(HashMap::new()),
            heartbeat_timeout_ms,
        }
    }

    /// Register a new service with an agent identity (CURVE public key)
    pub fn register(&self, service_id: &str, identity: &AgentIdentity) -> Result<()> {
        // Check for duplicate registration
        if self.services.read().unwrap().contains_key(service_id) {
            bail!("Service ID already registered: {}", service_id);
        }

        // Store the service->identity mapping
        {
            let mut services = self.services.write().map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
            services.insert(service_id.to_string(), identity.as_str().to_string());
        }

        // Track agent->services for cleanup
        {
            let mut agents = self.agents.write().map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
            agents.entry(identity.as_str().to_string())
                .or_insert_with(Vec::<String>::new)
                .push(service_id.to_string());
        }

        info!(
            "Registered service '{}' for agent: {}",
            service_id, identity.as_str()
        );
        Ok(())
    }

    /// Unregister a service (removes from tracking)
    pub fn unregister(&self, service_id: &str) -> Result<bool> {
        let removed = self.services.write().map_err(|_| anyhow::anyhow!("Lock poisoned"))?
            .remove(service_id)
            .is_some();

        if removed {
            debug!("Removed service {}", service_id);
        }

        Ok(removed)
    }

    /// Get agent identity for a service (clone needed since we can't return ref to RwLock content)
    pub fn get_agent(&self, service_id: &str) -> Option<String> {
        self.services.read().map_or(None, |s| s.get(service_id).cloned())
    }

    /// Check if service exists and is registered
    pub fn has_service(&self, service_id: &str) -> bool {
        self.services.read().map_or(false, |s| s.contains_key(service_id))
    }

    /// Get all registered services
    pub fn list_services(&self) -> Vec<String> {
        self.services.read()
            .map(|s| s.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Get agent identities (list of public keys)
    pub fn list_agents(&self) -> Vec<String> {
        self.agents.read()
            .map(|a| a.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Check if an agent is registered with any services
    pub fn has_agent(&self, identity: &str) -> bool {
        self.agents.read().map_or(false, |a| a.contains_key(identity))
    }

    /// Get services for a specific agent
    pub fn get_agent_services(&self, identity: &str) -> Option<Vec<String>> {
        self.agents.read()
            .map(|a| a.get(identity).cloned().unwrap_or_default())
            .ok()
    }

    /// Heartbeat callback - record agent liveness
    pub fn heartbeat(&self, identity: &AgentIdentity) {
        debug!("Heartbeat from agent: {}", identity.as_str());
        // In production, track in heartbeat manager
    }
}

/// Manages heartbeat tracking for agent liveness detection
#[derive(Debug)]
pub struct HeartbeatManager {
    /// Maps agent identity to last heartbeat time
    heartbeats: RwLock<HashMap<String, std::time::Instant>>,
    /// Maximum interval between heartbeats in milliseconds
    max_interval_ms: u64,
}

impl Default for HeartbeatManager {
    fn default() -> Self {
        Self::new(30_000) // 30 seconds default
    }
}

impl HeartbeatManager {
    pub fn new(max_interval_ms: u64) -> Self {
        Self {
            heartbeats: RwLock::new(HashMap::new()),
            max_interval_ms,
        }
    }

    /// Record heartbeat from an agent by identity string
    pub fn record(&self, identity: &str) {
        let now = std::time::Instant::now();
        {
            let mut hb = self.heartbeats.write().unwrap();
            hb.insert(identity.to_string(), now);
        }
        debug!("Heartbeat recorded for: {}", identity);
    }

    /// Check if agent is alive (heartbeat within timeout)
    pub fn is_alive(&self, identity: &str) -> bool {
        let now = std::time::Instant::now();
        match self.heartbeats.read().unwrap().get(identity) {
            Some(&last) => now.duration_since(last).as_millis() < self.max_interval_ms.into(),
            None => false,
        }
    }

    /// Get agents that missed their heartbeat (potentially dead agents)
    pub fn get_dead_agents(&self) -> Vec<String> {
        let now = std::time::Instant::now();
        self.heartbeats
            .read()
            .unwrap()
            .iter()
            .filter(|(_, &last)| now.duration_since(last).as_millis() >= self.max_interval_ms.into())
            .map(|(id, _): (&String, &std::time::Instant)| id.clone())
            .collect()
    }

    /// Clean up dead agents and their services
    pub fn cleanup_dead(&self, identity: &str) -> bool {
        let name = identity.to_string();
        let removed = self.heartbeats.write().unwrap().remove(&name).is_some();
        if removed {
            info!("Cleaned up dead agent: {}", identity);
        }
        removed
    }

    /// Get the heartbeat timeout in seconds
    pub fn timeout_seconds(&self) -> u64 {
        self.max_interval_ms / 1000
    }

    /// Check if specific agent is within heartbeat window
    pub fn check_agent(&self, identity: &str) -> bool {
        let alive = self.is_alive(identity);
        if !alive {
            warn!("Agent missed heartbeat: {}", identity);
        }
        alive
    }
}
