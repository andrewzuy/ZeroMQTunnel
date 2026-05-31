// Tunnel Server CLI Entry Point for Phase 4 Production Deployment

use std::path::PathBuf;
use clap::Parser;

#[derive(Parser)] pub struct Args { config: PathBuf }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    println!("ZeroMQTunnel Server starting");
    println!("Config: {}", args.config.display());
    Ok(())
}
