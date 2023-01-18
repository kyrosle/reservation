use std::path::Path;

use abi::Config;
use anyhow::{Ok, Result};
use reservation_service::start_server;

#[tokio::main]
async fn main() -> Result<()> {
    let filename = std::env::var("RESERVATION_CONFIG").unwrap_or_else(|_| {
        let p1 = Path::new("./reservation.yml");
        let path = shellexpand::tilde("~/.config/reservation.yml");
        let p2 = Path::new(path.as_ref());
        let test_path = shellexpand::tilde("~/.config/project_use/reservation.yml");
        let p3 = Path::new(test_path.as_ref());
        let p4 = Path::new("/etc/reservation.yml");

        match (p1.exists(), p2.exists(), p3.exists(), p4.exists()) {
            (true, _, _, _) => p1.to_str().unwrap().to_string(),
            (_, true, _, _) => p2.to_str().unwrap().to_string(),
            (_, _, true, _) => p3.to_str().unwrap().to_string(),
            (_, _, _, true) => p4.to_str().unwrap().to_string(),
            _ => panic!("no config file found"),
        }
    });
    let config = Config::load(filename)?;
    start_server(&config).await
}
