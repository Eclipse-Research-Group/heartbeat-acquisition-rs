use std::{fs, time::Duration};

use serde::Deserialize;
use serial::SecTickModule;

mod serial;

#[derive(Deserialize)]
struct HeartbeatConfig {
    serial_port: String
}


fn load_config() -> HeartbeatConfig {
    let config_contents = match fs::read_to_string("config.toml") {
        Ok(contents) => contents,
        Err(e) => panic!("Unable to open the config file: {:?}", e),
    };

    let config: HeartbeatConfig = match toml::from_str(&config_contents) {
        Ok(data) => data,
        Err(e) => panic!("Unable to parse the config file: {:?}", e),
    };  

    return config;
}

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    let config = load_config();

    println!("Serial port: {}", config.serial_port);

    let mut serial = SecTickModule::new(config.serial_port, 9600, Duration::from_secs(5));

    serial.open().unwrap();

    loop {

    }

    serial.close().unwrap();

}
