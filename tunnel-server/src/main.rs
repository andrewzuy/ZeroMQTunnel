use clap::Parser;
mod server;

#[derive(Debug, Parser)]
#[command(name = "tunnel-server")]
struct Opts {
    #[arg(short, long, default_value = "5555")]
    port: u16,
}

#[tokio::main]
async fn main() {
    let opts: Opts = Opts::parse();
    println!("Starting Tunnel Server on port {}...", opts.port);
    server::run(opts.port).await;
}
