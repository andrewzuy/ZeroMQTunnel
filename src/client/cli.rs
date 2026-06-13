//! Client CLI entry point (Phase 4 Local Forward -L mode)
//! Implements plan.md Section 3.1 local forwarding table

use clap::{Parser, Subcommand};
use anyhow::Result;

#[derive(Debug, Clone, Parser)]
pub struct LocalArgs {
    #[arg(short = 'l', long, default_value = "20232")] 
    pub local_port: u16,         // Section 3.1 table bind port
    
    #[arg(short = 't', long, required = true, default_value = "localhost:80")]  
    pub target_addr: String,     // Target to forward (SSH -L equivalent)
}

impl Default for LocalArgs { 
    fn default() -> Self { 
        Self{local_port: 20232u16, target_addr: "localhost:80".to_string()}  
    }  
}
