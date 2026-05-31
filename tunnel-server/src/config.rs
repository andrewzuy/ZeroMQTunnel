// Server Configuration

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub control_port: u16,
    pub listen_address: String,
    pub key_file: String,
    pub global_max_connections: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            control_port: 5555,
            listen_address: "0.0.0.0:1443".to_string(),
            key_file: "/etc/tunnel/server.pem".to_string(),
            global_max_connections: 1000,
        }
    }
}
