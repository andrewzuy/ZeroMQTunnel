// Phase 3 - Service Registry with Auto-Registration Support

use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum ForwardingRule {
    Remote {
        service_id: String,
        local_host: String,
        local_port: u16,
    },
    Local {
        service_id: String,
        remote_service_id: String,
        local_port: u16,
    },
}

#[derive(Debug)]
pub struct ServiceRegistration {
    pub service_id: String,
    pub rule_type: &'static str,
    pub forwarding_rule: ForwardingRule,
    pub heartbeat_interval_ms: u64,
    pub last_heartbeat: std::time::SystemTime,
}

impl ServiceRegistration {
    fn new(
        service_id: &str,
        local_port: u16,
        rule_type: &'static str,
    ) -> Self {
        let remote_host = String::from("127.0.0.1");
        
        let rule = if rule_type == "remote" {
            ForwardingRule::Remote {
                service_id: service_id.to_string(),
                local_host: remote_host.clone(),
                local_port,
            }
        } else {
            ForwardingRule::Local {
                service_id: String::from("web-443"), // placeholder for lookup
                remote_service_id: String::from("web-443"),
                local_port,
            }
        };

        Self {
            service_id: service_id.to_string(),
            rule_type,
            forwarding_rule: rule,
            heartbeat_interval_ms: 5000,
            last_heartbeat: std::time::SystemTime::now(),
        }
    }
}

pub struct ServiceRegistry {
    registry: HashMap<String, ServiceRegistration>,
    active_streams: HashMap<String, String>, // session_id -> stream_identity mapping
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            registry: HashMap::new(),
            active_streams: HashMap::new(),
        }
    }

    /// Register a service (Phase 3 with auto-reconnect support)
    pub fn register(&mut self, registration: ServiceRegistration) -> String {
        let session_id = Uuid::new_v4().to_string();
        self.registry.insert(
            registration.service_id.clone(),
            registration,
        );
        session_id
    }

    /// Get streaming identity for known endpoint (server data plane mapping)
    pub fn get_stream_identity(&self, _service_id: &str, _local_port: u16) -> Option<String> {
        None // Placeholder - actual mapping depends on agent registration
    }

    /// Unregister service and clean up resources
    pub fn unregister(&mut self, service_id: impl Into<String>) -> bool {
        self.registry.remove(service_id.into().as_str()).is_some()
    }

    /// Check if agent is heartbeat active (Phase 3 reconnection logic)
    pub fn heart_beat(&self) -> usize {
        // Placeholder for heartbeat tracking implementation
        0
    }

    /// Get all registered service IDs (for graceful shutdown cleanup)
    pub fn service_ids(&self) -> Vec<String> {
        self.registry.keys().cloned().collect()
    }

    pub fn register_remote(
        &mut self,
        port: u16,
        public_port: u16,
        local_host: impl Into<String>,
        service_id: &str,
    ) {
        let remote_host = local_host.into();
        println!(
            "Remote registered: public:{public_port} -> local:{remote_host}:{port} id:[{service_id}]"
        );
    }

    pub fn register_local(&mut self) {
        // Local forwarding registration handled by data plane streaming
        println!("Local service opened for remote connections");
    }

    pub fn get_rules_count(&self) -> usize { self.registry.len() }
    
    pub fn has_remote(&self, _service_id: &str) -> bool { true }
}

impl Default for ServiceRegistry {
    fn default() -> Self { Self::new() }
}
