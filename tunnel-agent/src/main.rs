// Phase 3 Forwarding
#[derive(Parser)] pub struct Args { #[arg(long)] remote: bool, #[arg(short = "R", default_value = "")] service_id: String, #[arg(value_name = "PORT")] port: u16, }
pub struct RemoteForwarder { pub port: u16, pub service_id: String, }

pub struct LocalForwarder { pub local_port: u16, }
