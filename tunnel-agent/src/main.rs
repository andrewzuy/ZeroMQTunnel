use clap::Parser;

#[derive(Parser)]
pub struct Args {
    #[arg(long)] remote: bool,
    #[arg(short = "R")] service_id: String,
    #[arg(name = "port", value_name = "PORT")] port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    println!("ZeroMQ Tunnel Agent");
    Ok(())
}
