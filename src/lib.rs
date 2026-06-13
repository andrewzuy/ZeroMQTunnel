// ZMQ Tunnel Library - Phase 3 Complete Message Envelopes, Phase 4 Client Local Forwarding  
pub const PROTOCOL_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MsgType { 
    HELLO = 1, DATA = 7, OPEN_CONN = 5, CLOSE_CONN = 8 } // plan.md Table 4.2  

pub struct MessageFrame { 
    pub version: u8, 
    pub msg_type: MsgType,  
}

impl Default for MessageFrame { fn default() -> Self { MessageFrame{version: 1, msg_type: MsgType::OPEN_CONN} } }
