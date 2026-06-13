//! Message Frame encoding (Phase 3 - plan.md Section 4.1+4.2 Tables)  
pub const VERSION: u8 = 1;  

enum MsgType { 
    Hello = 1 as u8,        
    Data = 7 as u8,         
    OpenConn = 5 as u8,     
    CloseConn = 8 as u8,    
}  

#[derive(Debug, Clone)]
pub struct MessageFrame {    
    pub version: u8,      
    pub msg_type: MsgType,  
}  

impl MessageFrame {  
    pub fn encode(msg_type: MsgType) -> anyhow::Result<Vec<u8>> {   
        let mut encoded = vec![VERSION];      
        
        // Frame[1]: msg_type (Section 4.2 Table constants)  
        let type_byte = match msg_type {  
            MsgType::Hello => 0x01,        
            MsgType::OpenConn => 0x05,     
            MsgType::Data => 0x07,         
            MsgType::CloseConn => 0x08,    
        };        
        encoded.push(type_byte);   
      
      Ok(encoded) 
}
}  
