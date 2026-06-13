//! Client config for local forwarding (Section 3.1 -L mode)
// SSH -L equivalent setup per plan.md Section 3.1 table

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Configuration for local forwarding client (-L mode)  
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalClientConfig {
    /// Local port to bind (e.g., 20232) per Section 3.1 table  
    pub local_port: u16,
    /// Target address to forward through tunnel  
    pub target_addr: String,  // Format: "target:port"  
}

impl Default for LocalClientConfig { 
    fn default() -> Self { 
        Self{local_port: 20232, target_addr: "localhost:80".into()}  
    }   
}
