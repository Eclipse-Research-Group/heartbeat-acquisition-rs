use std::{fs, time::{Duration, SystemTime}};

use colored::*;
use log::Level;
use serde::Deserialize;
use serial::SecTickModule;

mod serial;

fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            let color = match record.level() {
                Level::Error => "red",
                Level::Warn => "yellow",
                Level::Info => "green",
                Level::Debug => "blue",
                Level::Trace => "magenta",
            };

            let colored_level = format!("{}", record.level()).color(color);
            out.finish(format_args!(
                "[{} {} {}] {}",
                humantime::format_rfc3339_millis(SystemTime::now()),
                colored_level,
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}

#[derive(Deserialize)]
struct HeartbeatConfig {
    serial_port: String,
    node_id: String
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
async fn main() -> anyhow::Result<()> {
    setup_logger()?;

    let config = load_config();

    log::info!("Starting Heartbeat node with node_id=\"{}\"", config.node_id);
    log::debug!("Serial port: {}", config.serial_port);

    let mut serial = SecTickModule::new(config.serial_port, 9600, Duration::from_secs(5));

    serial.open().unwrap();

    loop {
        let line = serial.read_line().await.unwrap();
        log::trace!("Received: {}", line);
    }

    serial.close().unwrap();

    Ok(())
}

