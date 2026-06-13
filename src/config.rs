//! Config loading (Phase 1 - plan.md Section 2)
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct ServerConfig {
    pub _key_path: Option<String>,   // ~/.zmqtunnel/server_public.key  
}

impl Default for ServerConfig { 
    fn default() -> Self { 
        let kp = None;      
        ServerConfig{_key_path: kp.clone()}  
  }
}  

#[derive(Debug)]
pub struct ClientConfig {
    pub _key_path: Option<String>,     // ~/.zmqtunnel/client_secret.key  
    pub role: Option<String>,          // server | client from plan.md Table 2
}

impl Default for ClientConfig { 
    fn default() -> Self { 
        let kp = None;      
        ClientConfig{_key_path: kp.clone(), role: Some("client".into())}
    }
}  

#[derive(Debug, Clone)]
pub struct TunnelSpec {
    pub id: String,                   
    pub mode: ForwardMode,                      
}  
