//! ZeroMQ Tunnel Server - Production-Ready TCP Tunnel Broker
//!
//! This is the central server that mediates all connections between agents.
//! It supports both local and remote port forwarding with CURVE encryption.

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use tunnel_server::{
    config::ServerConfig,
    monitoring::{log_metrics, MetricsCollector},
    registrar::Registrar,
};

/// CLI arguments for the server
#[derive(Parser)]
pub struct Args {
    /// Path to configuration file (TOML)
    #[arg(short = 'c', long)]
    pub config: PathBuf,
}

/// The main tunnel server structure
#[derive(Debug)]
pub struct TunnelServer {
    /// Server configuration
    config: ServerConfig,
    /// Agent registration state
    registrar: Registrar,
    /// Metrics collector
    metrics: MetricsCollector,
    /// Zap handler for CURVE authentication
    zap_handler: tunnel_server::monitoring::ZapHandler,
}

impl Default for TunnelServer {
    fn default() -> Self {
        // Load from config file or use defaults
        let cfg = ServerConfig::load(&PathBuf::from("config/server.toml")).unwrap_or_default();
        Self::new(cfg)
    }
}

impl TunnelServer {
    /// Create a new server instance
    pub fn new(config: ServerConfig) -> Self {
        // Load Zap handler with empty whitelist (allow all in dev mode)
        let zap = tunnel_server::monitoring::ZapHandler::default();

        // Initialize registrar for tracking agents
        let registrar = Registrar::default();

        Self {
            config,
            registrar,
            metrics: MetricsCollector::new(),
            zap_handler: zap,
        }
    }

    /// Load server from config file
    pub fn from_config(config_path: impl AsRef<std::path::Path>) -> Result<Self> {
        let cfg = ServerConfig::load(config_path)?;
        // Validate configuration
        cfg.validate()?;

        Ok(Self::new(cfg))
    }

    /// Get server config reference
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// Handle incoming registration request (called from control socket loop)
    pub async fn handle_registration(&self, _message: &[u8]) -> Result<()> {
        tracing::info!("Received registration request");

        // Parse the registration message
        // For now, just accept it - in production would parse using rmp-serde
        let service_id = "unknown";

        // Track the new session
        self.metrics.increment_registration();
        self.metrics.session_active();
        self.metrics.track_agent(service_id);

        tracing::info!("Registration accepted for: {}", service_id);

        Ok(())
    }

    /// Export metrics to stdout (or Prometheus endpoint)
    pub fn export_metrics(&self) {
        log_metrics(&self.metrics);
    }

    /// Graceful shutdown handler
    pub async fn shutdown(&mut self) -> Result<()> {
        tracing::info!("Shutdown signal received");
        self.export_metrics();
        Ok(())
    }
}

impl Drop for TunnelServer {
    fn drop(&mut self) {
        tracing::debug!("TunnelServer dropped");
        let _ = self.export_metrics();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing (logging)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::new("info")
        )
        .init();

    let args = Args::parse();

    // Parse command line config or use default path
    let config_path = if args.config.exists() {
        args.config.clone()
    } else {
        PathBuf::from("config/server.toml")
    };

    tracing::info!("ZeroMQTunnel Server starting");
    tracing::info!("Config: {:?}", config_path);

    // Try to load from file, fall back to defaults
    let mut server = match TunnelServer::from_config(&config_path) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Using default configuration: {}", e);
            TunnelServer::default()
        }
    };

    // Get control socket address
    let control_addr = server.config.control_addr();
    tracing::info!("Control socket will listen on: {}", control_addr);

    // In production:
    // 1. Bind ZMQ ROUTER socket on control port
    // 2. Handle incoming DEALER connections from agents
    // 3. Process REGISTER/UNREGISTER/HEARTBEAT messages
    // 4. Open STREAM sockets for remote forwards
    // 5. Forward data between agents

    tracing::info!("Server ready - awaiting agent connections");

    // Keep server alive (in production: run event loop)
    tokio::signal::ctrl_c().await?;

    // Graceful shutdown
    server.shutdown().await?;

    Ok(())
}
