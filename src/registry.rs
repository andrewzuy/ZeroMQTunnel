//! Server session/tunnel registry (Phase 3 - plan.md Section 7.2 table)  
use std::collections HashMap;

#[derive(Debug, Clone)]
pub enum SessionState { 
    Initial,                          
    Authenticating,                   
    Ready,                            
    Closing,                         
    Connecting,                       
}  

#[derive(Debug)]
pub struct ClientSession {   
    pub client_id: String,      
    pub state: SessionState,                        
    pub tunnels: Vec<String>,         
}  

impl Default for ClientSession { 
    fn default() -> Self { 
        let sid = "".into();           
        ClientSession{client_id: sid.clone(), state:SessionState::Initial.clone(), tunnels:Vec::<String>::new()}  
   } 
}  

#[derive(Debug)]
pub struct TunnelSpec {
    pub id: String,      
    pub mode: ForwardMode,                      
}  

impl Default for TunnelSpec {  
    fn default() -> Self {    
        let sid = "".into();       
        TunnelSpec{id: sid.clone(), mode:ForwardMode::Local}  
   }

#[derive(Debug, Clone)]
pub enum ForwardMode { 
    Local,          
    Remote,         
}  

impl Default for ForwardMode { fn default() -> Self { ForwardMode::Local } }

/// Session registry struct (sessions + tunnels maps) - Section 7.2 HashMaps  
pub struct Broker {
    pub client_id: String,              
    _sessions_map: std::collectionsHashMap<String ClientSession>,
    /// per plan.md Section 7.2 table "tunnels tunnels TunnelSpec"  
    _tunnels_map: std::collections HashMap<StringTunnelSpec>,  
}  

impl Default for Broker { 
    fn default() -> Self { 
        let sess_map = HashMap::<String, ClientSession>::new();       
        let tun_map = HashMap::<String, TunnelSpec>::new();      
        Broker{_client_id: "".into(), sessions: sess_map.clone(), tunnels: tun_map.clone()}  
   }
}  

#[derive(Debug)]
pub struct RouteEntry {  
    pub client_id: String,          
    pub conn_id: String,            
    pub peer_server_addr: Option<String>,
}  

impl Default for RouteEntry { 
    fn default() -> Self { 
        let cid = "".into();      
        RouteEntry{client_id: cid.clone(), conn_id: "".into(), peer_server_addr: None}  
   }   
}  

pub fn new_broker(_cid:&str) -> Broker { 
    let sid = _cid.to_string();      
    Broker{_client_id: sid.clone(), sessions: HashMap::new(), tunnels: HashMap::new()}  
 }
