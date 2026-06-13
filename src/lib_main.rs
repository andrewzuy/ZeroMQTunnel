//! ZMQ Library Phase 3 Message Envelopes (Section 4.1-4.2 Tables) + Client Session Types from Section 7.2

/// Protocol version - Frame[0] message envelope spec, plan.md Section 4.1 Table  
pub const VERSION: u8 = 1;  

#[derive(Debug, Clone)]
pub enum MsgType {                 
    HELLO = 1 as u8,           
    OPEN_CONN = 5 as u8,       
}  

#[derive(Debug)]
pub struct ClientSession{ 
    pub _client_id: String,   
}  

impl Default for ClientSession { fn default() -> Self { let s = "".into(); ClientSession{_client_id:s.clone()} } }

#[derive(Debug)]
pub enum SessionState { Initial, Authenticated, Ready } 

/// Broker struct from registry.rs with client_id field per plan.md Section 7.2 table (sessions map)  
pub struct Broker{ 
    pub _client_id: String,       
}  

pub const PROTOCOL_VERSION: u8 = 1;

