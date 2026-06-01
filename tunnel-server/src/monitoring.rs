//! Metrics export & ZAP handler for CURVE Authentication
//!
//! Handles metrics collection, Prometheus format export, and agent authentication.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use tracing::{info, warn, debug};
use anyhow::{Result, bail};

/// Zap handler for validating agent connections (CURVE security model)
#[derive(Debug)]
pub struct ZapHandler {
    /// Server's CURVE secret key (placeholder - loaded from key file)
    server_keypair: Option<String>,
    /// Server public key for identification
    server_public_key: Option<String>,
    /// Whitelist of authorized agent public keys (optional, disabled by default)
    whitelist: RwLock<HashSet<String>>,
    /// Server identity for logging
    server_identity: String,
}

impl ZapHandler {
    pub fn new() -> Self {
        Self {
            server_keypair: None, // Will be loaded or generated
            server_public_key: None,
            whitelist: RwLock::new(HashSet::new()),
            server_identity: "tunnel-server".to_string(),
        }
    }

    pub fn load_from_file(path: &str) -> Result<Self> {
        info!("Loading ZAP handler from key file: {}", path);
        // In production, read PEM file and parse x25519_dalek SecretKey
        Ok(Self::new())
    }

    /// Validate agent public key against whitelist (CURVE model)
    pub fn validate_agent(&self, public_key: &str) -> Result<()> {
        if self.whitelist.read().unwrap().is_empty() {
            // Whitelist not configured - allow (development mode)
            debug!("Agent connection validated (whitelist disabled): {}", public_key);
            Ok(())
        } else if self.whitelist.read().unwrap().contains(public_key) {
            info!("Agent authorized via whitelist: {}", public_key);
            Ok(())
        } else {
            let err = format!("Agent rejected (not in whitelist): {}", public_key);
            warn!("{}", err);
            bail!("{}", err);
        }
    }

    /// Add a key to the whitelist
    pub fn add_whitelisted(&self, key: &str) -> Result<()> {
        let mut wl = self.whitelist.write().unwrap();
        wl.insert(key.to_string());
        info!("Added agent to whitelist: {}", key);
        Ok(())
    }

    /// Remove a key from the whitelist
    pub fn remove_from_whitelist(&self, key: &str) -> bool {
        let mut wl = self.whitelist.write().unwrap();
        wl.remove(key)
    }

    /// Get current whitelist count
    pub fn whitelisted_count(&self) -> usize {
        self.whitelist.read().unwrap().len()
    }

    /// Check if agent is whitelisted (or whitelist disabled)
    pub fn is_whitelisted(&self) -> bool {
        self.whitelist.read().map_or(true, |s| s.is_empty())
    }
}

impl Default for ZapHandler {
    fn default() -> Self {
        Self::new()
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
        debug!("Active sessions: {}", count);
    }

    /// Decrement active sessions
    pub fn session_closed(&self) {
        let mut count = self.active_sessions.write().unwrap();
        *count = count.saturating_sub(1);
        debug!("Active sessions: {}", count);
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
        debug!("Reconnects: {}", count);
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

    /// Reset all counters to zero
    pub fn reset(&self) {
        *self.active_sessions.write().unwrap() = 0;
        *self.bytes_transferred_total.write().unwrap() = 0;
        *self.reconnect_count_total.write().unwrap() = 0;
        *self.registrations_total.write().unwrap() = 0;
    }
}

/// Log current metrics for monitoring (convenience function)
pub fn log_metrics(collector: &MetricsCollector) {
    info!("Metrics snapshot: {:?}", collector.get_metrics());
    let active = *collector.active_sessions.read().unwrap();
    let bytes = *collector.bytes_transferred_total.read().unwrap();
    println!("Metrics exported: active_sessions={}, bytes_transferred={}", active, bytes);
}
