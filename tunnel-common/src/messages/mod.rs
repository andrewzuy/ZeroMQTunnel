//! Stream messages for data plane communication
use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug)]
pub enum ControlMessage {
    Register(RegisterData),
}

impl<'de> Deserialize<'de> for ControlMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        Ok(ControlMessage::Register(RegisterData::deserialize(deserializer)?))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterData {
    pub service_id: String,
    #[serde(rename = "forward_type")]
    pub forward_type: String,
    pub remote_port: Option<u16>,
    pub local_ip: std::net::Ipv4Addr,
    pub local_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationResponse {
    pub success: bool,
    pub service_id: String,
}
