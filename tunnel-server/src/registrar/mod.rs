//! Registrar module for handling agent registrations and heartbeats

use std::collections::HashMap;
use std::sync::RwLock;
use anyhow::Result;

pub use tunnel_common::{AgentIdentity, TunnelError};

/// Manages agent registrations and session tracking
#[derive(Debug)]
pub struct Registrar {
    /// Maps service ID to agent identity
    services: RwLock<HashMap<String, AgentIdentity>>,
    /// Maps agent identity to their registered service IDs
    agents: RwLock<HashMap<String, Vec<String>>>,
}

impl Default for Registrar {
    fn default() -> Self {
        Self::new()
    }
}

impl Registrar {
    pub fn new() -> Self {
        Self {
            services: RwLock::new(HashMap::new()),
            agents: RwLock::new(HashMap::new()),
        }
    }

    /// Register a new agent/service
    pub fn register(&self, service_id: &str, identity: &AgentIdentity) -> Result<()> {
        // Check for duplicate registration
        if self.services.read().unwrap().contains_key(service_id) {
            anyhow::bail!("Service ID already registered");
        }

        // Store the service->identity mapping
        {
            let mut services = self.services.write().unwrap();
            services.insert(service_id.to_string(), (*identity).clone());
        }

        // Track agent->services
        {
            let mut agents = self.agents.write().unwrap();
            agents.entry(identity.as_str().to_string())
                .or_insert_with(Vec::<String>::new)
                .push(service_id.to_string());
        }

        Ok(())
    }

    /// Unregister a service (note: identity is lost when service is removed from map)
    pub fn unregister(&self, service_id: &str) -> Result<bool> {
        let removed = self.services.write().unwrap().remove(service_id).is_some();

        if removed {
            tracing::debug!("Removed service {}", service_id);
        }

        Ok(removed)
    }

    /// Get agent identity for a service (note: clone needed since we can't return ref to RwLock content)
    pub fn get_agent(&self, service_id: &str) -> Option<AgentIdentity> {
        self.services.read().unwrap().get(service_id).cloned()
    }

    /// Check if service exists
    pub fn has_service(&self, service_id: &str) -> bool {
        self.services.read().unwrap().contains_key(service_id)
    }

    /// Get all registered services
    pub fn list_services(&self) -> Vec<String> {
        self.services.read()
            .unwrap()
            .keys()
            .cloned()
            .collect()
    }

    /// Get agent identities
    pub fn list_agents(&self) -> Vec<String> {
        self.agents.read()
            .unwrap()
            .keys()
            .cloned()
            .collect()
    }
}

/// Heartbeat manager for tracking agent liveness
#[derive(Debug)]
pub struct HeartbeatManager {
    /// Maps agent identity to last heartbeat time
    heartbeats: RwLock<HashMap<String, std::time::Instant>>,
}

impl HeartbeatManager {
    pub fn new(_max_interval: std::time::Duration) -> Self {
        Self {
            heartbeats: RwLock::new(HashMap::new()),
        }
    }

    /// Record heartbeat from an agent
    pub fn record(&self, identity: &str) {
        {
            let mut hb = self.heartbeats.write().unwrap();
            hb.insert(identity.to_string(), std::time::Instant::now());
        }
    }

    /// Check if agent is alive (heartbeat within timeout)
    pub fn is_alive(&self, identity: &str, max_interval: std::time::Duration) -> bool {
        match self.heartbeats.read().unwrap().get(identity) {
            Some(&last) => {
                std::time::Instant::now().duration_since(last) <= max_interval
            }
            None => false,
        }
    }

    /// Get agents that missed their heartbeat (dead agents)
    pub fn get_dead_agents(&self, max_interval: std::time::Duration) -> Vec<String> {
        let now = std::time::Instant::now();
        self.heartbeats
            .read()
            .unwrap()
            .iter()
            .filter(|(_, &last)| now.duration_since(last) > max_interval)
            .map(|(id, _): (&String, &std::time::Instant)| id.clone())
            .collect()
    }

    /// Clean up dead agents (optional)
    pub fn cleanup_dead(&self, identity: impl AsRef<str>) -> bool {
        let name = identity.as_ref().to_string();
        self.heartbeats.write().unwrap().remove(&name).is_some()
    }
}
