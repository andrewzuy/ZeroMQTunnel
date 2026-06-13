//! Server CLI entry point (broker/router mode)
//! TODO: Implement ROUTER poll loop with CurveZMQ and ZAP authentication

use clap::Parser;  
use tracing::{info};

#[derive(Parser)]
pub struct ServerArgs {
    #[arg(short = "b", long = "bind-addr", default_value = "tcp://127.0.0.1:5093")] 
    pub bind_addr: String,
    
    #[arg(long)] 
    pub config: Option<String>,
}

impl ServerArgs{
    pub fn run(&self) -> anyhow::Result<()> {  
        info!("Starting ZMQ Tunnel server on {}", self.bind_addr);
        // TODO: Implement CurveZMQ ROUTER setup and poll loop here
        
        Ok(())  
    }
}

pub type ServerCli = ServerArgs;
