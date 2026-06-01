//! ZeroMQ Tunnel Agent - Client-side agent for forwarding TCP streams
//!
//! Supports both remote forwarding (expose local service) and local forwarding (tunnel to remote).

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;


/// CLI arguments for the agent
#[derive(Parser)]
pub struct Args {
    /// Forwarding mode: "remote" or "local"
    #[arg(long)]
    pub remote: bool,

    /// Local forwarding mode: "local"
    #[arg(long)]
    pub local: bool,

    /// Server address (default: tcp://localhost:5555)
    #[arg(short = 's', long)]
    pub server_addr: Option<String>,

    /// Service ID (e.g., "web-443" for remote or "internal-api" for local)
    #[arg(short, name = "service-id", default_value = "service-1")]
    pub service_id: String,

    /// Local port to listen/tunnel (for local mode) or target port (for remote)
    #[arg(name = "port", value_name = "PORT")]
    pub port: u16,

    /// Path to agent CURVE key file
    #[arg(short, long)]
    pub key_file: Option<PathBuf>,

    /// Heartbeat interval in seconds (default: 30)
    #[arg(long, default_value = "30")]
    pub heartbeat_interval: u64,
}

/// Default server address if not specified
const DEFAULT_SERVER_ADDR: &str = "tcp://localhost:5555";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if !args.remote && !args.local {
        eprintln!("Error: Must specify either --remote or --local mode");
        return Err(anyhow::anyhow!("Missing forwarding mode").into());
    }

    println!("ZeroMQ Tunnel Agent starting");
    println!("Mode: remote = {}, local = {}", args.remote, args.local);
    println!("Service ID: {}", args.service_id);
    println!("Port: {}", args.port);

    Ok(())
}
