//! Metrics export & ZAP handler for CURVE authentication

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use tracing::info;

/// Zap handler for validating agent connections (CURVE security model)
#[derive(Debug)]
pub struct ZapHandler {
    /// Whitelist of authorized agent public keys
    pub whitelist: HashSet<String>,
    /// Server's own identity key
    pub server_identity: String,
}

impl Default for ZapHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ZapHandler {
    pub fn new() -> Self {
        Self {
            whitelist: HashSet::new(), // Empty initially - all allowed in dev
            server_identity: "server".to_string(),
        }
    }

    /// Load from a file with optional password (for production)
    pub fn load_from_file(_path: &str, _password: Option<&str>) -> Self {
        info!("Loading agent whitelist from {_path}");
        // In production: read PEM keys, validate against server's known list
        // For now, allow all connections in development mode
        Self::new()
    }

    /// Validate agent public key against whitelist
    pub fn validate_agent(&self, public_key: &str) -> bool {
        if self.whitelist.is_empty() {
            // Whitelist not configured - allow (development mode)
            true
        } else if self.whitelist.contains(public_key) {
            info!("Agent authorized: {}", public_key);
            true
        } else {
            info!("Agent rejected (not in whitelist): {}", public_key);
            false
        }
    }

    /// Add a key to the whitelist
    pub fn add_whitelisted(&mut self, key: &str) -> &HashSet<String> {
        self.whitelist.insert(key.to_string());
        &self.whitelist
    }

    /// Remove a key from the whitelist
    pub fn remove_from_whitelist(&mut self, key: &str) -> bool {
        self.whitelist.remove(key)
    }

    /// Get current whitelist count
    pub fn whitelisted_count(&self) -> usize {
        self.whitelist.len()
    }
}

/// Metrics collector for tunnel statistics
#[derive(Debug)]
pub struct MetricsCollector {
    active_sessions: Arc<RwLock<usize>>,
    bytes_transferred_total: Arc<RwLock<u64>>,
    reconnect_count_total: Arc<RwLock<usize>>,
    registrations_total: Arc<RwLock<usize>>,
    agent_identities: Arc<RwLock<HashSet<String>>>,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            active_sessions: Arc::new(RwLock::new(0)),
            bytes_transferred_total: Arc::new(RwLock::new(0)),
            reconnect_count_total: Arc::new(RwLock::new(0)),
            registrations_total: Arc::new(RwLock::new(0)),
            agent_identities: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Increment active sessions
    pub fn session_active(&self) {
        let mut count = self.active_sessions.write().unwrap();
        *count += 1;
    }

    /// Decrement active sessions
    pub fn session_closed(&self) {
        let mut count = self.active_sessions.write().unwrap();
        *count -= 1;
    }

    /// Record bytes transferred
    pub fn add_bytes(&self, bytes: u64) {
        let mut total = self.bytes_transferred_total.write().unwrap();
        *total += bytes;
    }

    /// Increment reconnect count
    pub fn increment_reconnect(&self) {
        let mut count = self.reconnect_count_total.write().unwrap();
        *count += 1;
    }

    /// Increment registration count
    pub fn increment_registration(&self) {
        let mut count = self.registrations_total.write().unwrap();
        *count += 1;
    }

    /// Add an agent identity to tracking
    pub fn track_agent(&self, identity: &str) {
        let mut agents = self.agent_identities.write().unwrap();
        agents.insert(identity.to_string());
    }

    /// Get metrics snapshot
    pub fn get_metrics(&self) -> HashMap<String, u64> {
        let active = *self.active_sessions.read().unwrap() as u64;
        let bytes = *self.bytes_transferred_total.read().unwrap();
        let reconnects = *self.reconnect_count_total.read().unwrap() as u64;
        let registrations = *self.registrations_total.read().unwrap() as u64;

        HashMap::from([
            ("active_sessions".to_string(), active),
            ("bytes_transferred_total".to_string(), bytes),
            ("reconnect_count_total".to_string(), reconnects),
            ("registrations_total".to_string(), registrations),
        ])
    }

    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let sessions = self.active_sessions.read().unwrap();
        let bytes = self.bytes_transferred_total.read().unwrap();
        let reconnects = self.reconnect_count_total.read().unwrap();
        info!(
            "Metrics exported: active_sessions={}, bytes_transferred={}, reconnects={}",
            sessions, bytes, reconnects
        );

        format!(
            "# HELP tunnel_active_sessions Number of active tunnel sessions\n\
             # TYPE tunnel_active_sessions gauge\n\
             tunnel_active_sessions {}\n\
             \n\
             # HELP tunnel_bytes_transferred_total Total bytes transferred\n\
             # TYPE tunnel_bytes_transferred_total counter\n\
             tunnel_bytes_transferred_total {}\n\
             \n\
             # HELP tunnel_reconnect_count_total Total number of reconnections\n\
             # TYPE tunnel_reconnect_count_total counter\n\
             tunnel_reconnect_count_total {}",
            sessions, bytes, reconnects
        )
    }
}

/// Log current metrics for monitoring
pub fn log_metrics(collector: &MetricsCollector) {
    info!("Metrics snapshot: {:?}", collector.get_metrics());
    let active = *collector.active_sessions.read().unwrap();
    let bytes = *collector.bytes_transferred_total.read().unwrap();
    println!("Metrics exported: active_sessions={}, bytes_transferred={}", active, bytes);
}
