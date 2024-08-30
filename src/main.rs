use std::{fs, time::{Duration, Instant, SystemTime}};

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
mod led;

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
    node_id: String,
    file_duration_mins: i64,
    gzip_level: i8,
    output_dir: String,
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
    let mut led = led::LED::new(19, 20, 21)?;
    led.set_color(led::LedColor::White)?;

    // Check for writability to the output directory
    let output_dir = std::path::Path::new(&config.output_dir);
    if !output_dir.exists() {
        log::error!("Output directory does not exist: {}", config.output_dir);
        std::process::exit(1);
    }

    if !output_dir.is_dir() {
        log::error!("Output directory is not a directory: {}", config.output_dir);
        std::process::exit(1);
    }

    // Test by writing a file
    let test_file = output_dir.join("test_file");
    match fs::write(&test_file, "test") {
        Ok(_) => {
            fs::remove_file(&test_file)?;
        },
        Err(e) => {
            log::error!("Unable to write to output directory: {}", e);
            std::process::exit(1);
        }
    }

    log::info!("Starting Heartbeat node with node_id=\"{}\"", config.node_id);
    log::debug!("Serial port: {}", config.serial_port);

    let mut serial = SecTickModule::new(config.serial_port, 1_000_000, Duration::from_secs(5));

    serial.open().unwrap();

    let (tx, _) = tokio::sync::broadcast::channel(16);

    let mut local = LocalService::new(LocalServiceConfig {
        port: 8767
    }, tx.clone());

    let rx = tx.subscribe();

    let writer_config = writer::hdf5::HDF5WriterConfig {
        node_id: config.node_id.clone(),
        output_path: config.output_dir.into(),
        gzip_level: config.gzip_level,
    };
    let mut writer = writer::hdf5::HDF5Writer::new(writer_config.clone())?;

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::broadcast::channel::<()>(4);
    let tx_arc = tx.clone();
    tokio::spawn(async move {
        match signal::ctrl_c().await {
            Ok(()) => {
                log::info!("Shutting down, waiting for services...");
                shutdown_tx.send(()).unwrap();
                tx_arc.send(services::ServiceMessage::Shutdown).unwrap();
            },
            Err(err) => {
                eprintln!("Unable to listen for shutdown signal: {}", err);
                // we also shut down in case of error
            },
        }
    });

    local.start().await?;

    let mut last_start = Instant::now();

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                led.set_color(led::LedColor::Yellow)?;
                break;
            },
            line = serial.read_line() => {
                let when = chrono::Utc::now();
                match line {
                    Ok(line) => {
                        if last_start.elapsed() > Duration::from_secs(config.file_duration_mins as u64 * 60) {
                            writer = writer::hdf5::HDF5Writer::new(writer_config.clone())?;
                            last_start = Instant::now();
                        }

                        if line.starts_with("#") {
                            led.set_color(led::LedColor::Blue)?;
                            writer.write_comment(&line).await?;
                            continue;
                        }
                
                        let frame = match Frame::parse(&line) {
                            Ok(frame) => frame,
                            Err(e) => {
                                led.set_color(led::LedColor::Red)?;
                                log::error!("Failed to parse frame: {:?}\n{}", e, &line[..line.len().min(60)]);
                                continue;
                            }
                        };
                
                        writer.write_frame(when, &frame).await?;
                        tx.send(services::ServiceMessage::NewFrame(frame))?;
                        led.set_color(led::LedColor::Green)?;
                    },
                    Err(e) => {
                        log::error!("Error reading line: {:?}", e);
                        led.set_color(led::LedColor::Red)?;
                        continue;
                    }
                }
            }
        }   

        
    }

    local.stop();

    log::info!("All done!");

    led.set_color(led::LedColor::Off);

    Ok(())
}

