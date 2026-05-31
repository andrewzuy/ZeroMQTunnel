// Phase 3 Server
use tokio::signal;
pub async fn run(_port: u16) -> Result<(),Box<dyn std::error::Error>> {
    let ctrl = signal::ctrl_c().await.ok();
    Ok(())
}
