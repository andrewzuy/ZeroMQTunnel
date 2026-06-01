//! Service registry for managing tunnel service registrations

use std::collections::HashMap;

use crate::types::ForwardMode;

/// Manages the mapping between service IDs and their forwarding configurations
#[derive(Debug)]
pub struct ServiceRegistry {
    /// Maps service ID to its configuration
    services: HashMap<String, ServiceConfig>,
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }

    /// Register a service with its configuration
    pub fn register(&mut self, service_id: impl Into<String>, config: ServiceConfig) {
        self.services.insert(service_id.into(), config);
    }

    /// Get a service by ID, returning None if not found
    pub fn get(&self, service_id: &str) -> Option<&ServiceConfig> {
        self.services.get(service_id)
    }

    /// Get a mutable reference to a service configuration
    pub fn get_mut(&mut self, service_id: &str) -> Option<&mut ServiceConfig> {
        self.services.get_mut(service_id)
    }

    /// Remove a service from the registry
    pub fn remove(&mut self, service_id: &str) -> bool {
        self.services.remove(service_id).is_some()
    }

    /// Check if a service exists
    pub fn has(&self, service_id: &str) -> bool {
        self.services.contains_key(service_id)
    }

    /// Get all registered services (snapshot)
    pub fn iter(&self) -> impl Iterator<Item = (&String, &ServiceConfig)> {
        self.services.iter()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.services.is_empty()
    }

    /// Get count of registered services
    pub fn len(&self) -> usize {
        self.services.len()
    }
}

/// Configuration for a registered service
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// Forwarding mode (remote or local)
    pub forward_mode: ForwardMode,
    /// For remote forwarding: target host and port
    pub remote_target: Option<(String, u16)>,
    /// For local forwarding: listener port
    pub local_port: Option<u16>,
    /// Session limit for this service
    pub session_limit: Option<usize>,
}

impl ServiceConfig {
    /// Create a new remote forwarding configuration
    pub fn remote(host: impl Into<String>, port: u16) -> Self {
        Self {
            forward_mode: ForwardMode::Remote,
            remote_target: Some((host.into(), port)),
            local_port: None,
            session_limit: None,
        }
    }

    /// Create a new local forwarding configuration
    pub fn local(port: u16) -> Self {
        Self {
            forward_mode: ForwardMode::Local,
            remote_target: None,
            local_port: Some(port),
            session_limit: None,
        }
    }

    /// Create from string-based format (legacy support)
    pub fn from_legacy(is_remote: bool, port: u16) -> Self {
        if is_remote {
            let host = String::from("localhost");
            Self::remote(host, port)
        } else {
            Self::local(port)
        }
    }
}

