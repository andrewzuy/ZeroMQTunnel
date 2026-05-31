use anyhow::Result;
use tokio::time;
pub async fn run(_port:u16)->Result<(),Box<dyn std::error::Error>> {
    println!("Phase 2 server"); 
    loop { time::sleep(time::Duration::from_millis(100)).await; }
}
SERVEREOF && cat /home/andrew/Development/ZeroMQTunnel/tunnel-server/src/server.rs
