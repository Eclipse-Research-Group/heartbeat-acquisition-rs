use std::{fs, time::{Duration, SystemTime}};

use colored::*;
use log::Level;
use serde::Deserialize;
use serial::{Frame, SecTickModule};
use services::local::{LocalService, LocalServiceConfig};
use tokio::signal;
use writer::Writer;

mod serial;
mod writer;
mod services;

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
        .level(log::LevelFilter::Debug)
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

    let mut serial = SecTickModule::new(config.serial_port, 1_000_000, Duration::from_secs(5));

    serial.open().unwrap();

    let (tx, _) = tokio::sync::broadcast::channel(16);

    // let mut local = LocalService::new(LocalServiceConfig {
    //     port: 8080
    // }, tx.clone());

    let rx = tx.subscribe();

    let mut writer = writer::hdf5::HDF5Writer::new("test5.h5".into())?;

    let token = tokio_util::sync::CancellationToken::new();

    let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    let shutdown_token = token.clone();
    let tx_arc = tx.clone();
    tokio::spawn(async move {
        match signal::ctrl_c().await {
            Ok(()) => {
                log::info!("Shutting down, waiting for services...");
                shutdown_token.cancel();
                tx_arc.send(services::ServiceMessage::Shutdown).unwrap();
            },
            Err(err) => {
                eprintln!("Unable to listen for shutdown signal: {}", err);
                // we also shut down in case of error
            },
        }
    });

    // local.start().await?;

    while !token.is_cancelled() {
        let line = match serial.read_line().await {
            Ok(line) => line,
            Err(e) => {
                log::error!("Error reading line: {:?}", e);
                continue;
            }
        };

        log::trace!("Received line: {}", line);

        if line.starts_with("#") {
            log::info!("Received comment: {}", line);
            continue;
        }

        let frame = match Frame::parse(&line) {
            Ok(frame) => frame,
            Err(e) => {
                log::error!("Failed to parse frame: {:?}\n{}", e, &line[..line.len().min(60)]);
                continue;
            }
        };

        writer.write_frame(&frame).await?;
        tx.send(services::ServiceMessage::NewFrame(frame))?;
    }

    local.stop();

    log::info!("All done!");

    Ok(())
}

