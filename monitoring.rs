//! ZeroMQ Tunnel - Monitoring & ZAP Authentication Module
//!
//! Handles metrics collection, Prometheus export, and CURVE ZAP validation.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use anyhow::{Context, Result};
use tracing::{info, warn, debug};

/// Zap handler for validating agent connections via CURVE security model
#[derive(Debug)]
pub struct ZapHandler {
    /// Server's ed25519 keystore for CURVE operations (using ed25519_dalek)
    server_keypair: Option<ed25519_dalek::Keypair>,
    /// Whitelist of authorized agent public keys (optional, disabled by default)
    whitelist: RwLock<HashSet<String>>,
    /// Server identity for logging
    server_identity: String,
}

impl Default for ZapHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ZapHandler {
    /// Create new ZapHandler with empty whitelist (development mode)
    pub fn new() -> Self {
        Self {
            server_keypair: None, // Will be loaded from key file or generated
            whitelist: RwLock::new(HashSet::new()),
            server_identity: "tunnel-server".to_string(),
        }
    }

    /// Load ZapHandler configuration and optional key file
    pub fn load(key_path: &str) -> Result<Self> {
        let handler = Self::new();
        if std::path::Path::new(key_path).exists() {
            info!("Loading ZAP handler from key file: {}", key_path);
            // In production, parse PEM format and load ed25519 keystore
            // For now, skip loading - keys are generated on first run
        } else {
            warn!("Key file not found at {}: using development mode", key_path);
        }
        Ok(handler)
    }

    /// Validate agent public key against whitelist (or allow all in dev mode)
    pub fn validate_agent(&self, public_key: &str) -> Result<(), String> {
        if self.whitelist.read().unwrap().is_empty() {
            debug!("Agent connection validated (whitelist disabled): {}", public_key);
            Ok(())
        } else if self.whitelist.read().unwrap().contains(public_key) {
            info!("Agent authorized via whitelist: {}", public_key);
            Ok(())
        } else {
            let err = format!("Agent rejected (not in whitelist): {}", public_key);
            warn!("{}", err);
            Err(err)
        }
    }

    /// Add a key to the whitelist
    pub fn add_whitelisted(&self, key: &str) -> Result<(), String> {
        let mut wl = self.whitelist.write().map_err(|e| e.to_string())?;
        wl.insert(key.to_string());
        info!("Added agent to whitelist: {}", key);
        Ok(())
    }

    /// Remove a key from the whitelist
    pub fn remove_from_whitelist(&self, key: &str) -> bool {
        let mut wl = self.whitelist.write().map_err(|_| "PoisonError").unwrap();
        wl.remove(key)
    }

    /// Check if whitelist is enabled
    pub fn is_whitelisted(&self) -> bool {
        !self.whitelist.read().map_or(true, |s| s.is_empty())
    }

    /// Get current whitelist count
    pub fn whitelisted_count(&self) -> usize {
        self.whitelist.read().map_or(0, |s| s.len())
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
    /// Create new metrics collector with zeroed counters
    pub fn new() -> Self {
        Self {
            active_sessions: Arc::new(RwLock::new(0)),
            bytes_transferred_total: Arc::new(RwLock::new(0)),
            reconnect_count_total: Arc::new(RwLock::new(0)),
            registrations_total: Arc::new(RwLock::new(0)),
            agent_identities: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Session opened - increment counter
    pub fn session_opened(&self) {
        let mut count = self.active_sessions.write().unwrap();
        *count += 1;
        debug!("Active sessions: {}", count);
    }

    /// Session closed - decrement counter
    pub fn session_closed(&self) {
        let mut count = self.active_sessions.write().unwrap();
        *count = count.saturating_sub(1);
        debug!("Active sessions: {}", count);
    }

    /// Record bytes transferred (add to total)
    pub fn add_bytes(&self, bytes: u64) {
        let mut total = self.bytes_transferred_total.write().unwrap();
        *total += bytes;
    }

    /// Get bytes transferred (last interval or since start?)
    pub fn bytes_this_interval(&self, interval_size: u64) -> u64 {
        // Simplified - in production track per-interval counters
        let total = self.bytes_transferred_total.read().unwrap();
        total % interval_size  // Pseudo-reset for demo
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

    /// Track an agent identity for monitoring
    pub fn track_agent(&self, identity: &str) {
        let mut agents = self.agent_identities.write().unwrap();
        agents.insert(identity.to_string());
    }

    /// Get all active identities
    pub fn get_agent_ids(&self) -> Vec<String> {
        self.agent_identities.read().unwrap().clone()
    }

    /// Get metrics snapshot as HashMap
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

    /// Export metrics in Prometheus text format
    pub fn export_prometheus(&self) -> String {
        let sessions = *self.active_sessions.read().unwrap();
        let bytes = *self.bytes_transferred_total.read().unwrap();
        let reconnects = *self.reconnect_count_total.read().unwrap();

        format!(
            "# HELP tunnel_active_sessions Number of active tunnel sessions\n\
             # TYPE tunnel_active_sessions gauge\n\
             tunnel_active_sessions {sessions}\n\
             \n\
             # HELP tunnel_bytes_transferred_total Total bytes transferred\n\
             # TYPE tunnel_bytes_transferred_total counter\n\
             tunnel_bytes_transferred_total {bytes}\n\
             \n\
             # HELP tunnel_reconnect_count_total Total number of reconnections\n\
             # TYPE tunnel_reconnect_count_total counter\n\
             tunnel_reconnect_count_total {reconnects}",
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

/// Log current metrics for monitoring (deprecated, use MetricsCollector)
pub fn log_metrics(collector: &MetricsCollector) {
    info!("Metrics snapshot");
    let active = *collector.active_sessions.read().unwrap();
    let bytes = *collector.bytes_transferred_total.read().unwrap();
    println!("Metrics exported: active_sessions={}, bytes_transferred={}", active, bytes);
}
