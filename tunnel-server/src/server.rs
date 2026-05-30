pub mod registrar {} pub async fn run(port: u16) { println!("Server starting on port {}", port); loop { std::thread::sleep(std::time::Duration::from_millis(100)); } }
