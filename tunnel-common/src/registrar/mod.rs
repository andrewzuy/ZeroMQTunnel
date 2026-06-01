//! Agent registrar client functionality

use crate::types::TunnelError;

/// Agent registrar client for communicating with tunnel server
#[derive(Debug)]
pub struct AgentRegistrar {
    /// Unique agent identity
    pub identity: String,
    /// Service ID being registered
    pub service_id: String,
}

impl AgentRegistrar {
    pub fn new(identity: impl Into<String>, service_id: impl Into<String>) -> Self {
        Self {
            identity: identity.into(),
            service_id: service_id.into(),
        }
    }

    /// Get service ID
    pub fn service_id(&self) -> &str {
        &self.service_id
    }

    /// Get agent identity
    pub fn identity(&self) -> &str {
        &self.identity
    }
}

/// Service registry for tracking active registrations
#[derive(Debug)]
pub struct ServiceRegistry {
    /// Maps agent identity to their registered services
    pub services: std::collections::HashMap<String, Vec<String>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            services: std::collections::HashMap::new(),
        }
    }

    /// Register a service for an agent
    pub fn register(&mut self, identity: &str, service_id: &str) {
        self.services
            .entry(identity.to_string())
            .or_insert_with(Vec::new)
            .push(service_id.to_string());
    }

    /// Unregister a service for an agent
    pub fn unregister(&mut self, identity: &str, service_id: &str) -> bool {
        if let Some(services) = self.services.get_mut(identity) {
            if let Some(idx) = services.iter().position(|s| s == service_id) {
                services.remove(idx);
                return true;
            }
        }
        false
    }

    /// Remove all services for an agent
    pub fn remove_agent(&mut self, identity: &str) -> bool {
        if let Some(services) = self.services.get_mut(identity) {
            *services = std::mem::take(services);
            return true;
        }
        false
    }

    /// Get services for an agent
    pub fn get_services(&self, identity: &str) -> Option<&Vec<String>> {
        self.services.get(identity)
    }

    /// Check if agent is registered
    pub fn has_agent(&self, identity: &str) -> bool {
        self.services.contains_key(identity)
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility function to generate new UUID v4 for stream IDs
pub fn uuid_v4() -> Result<uuid::Uuid, TunnelError> {
    Ok(uuid::Uuid::new_v4())
}
