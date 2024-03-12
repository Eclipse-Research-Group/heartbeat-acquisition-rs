pub mod capture;
pub mod service;
pub mod utils;
pub mod vendor;

use anyhow::Result;
use chrono::DateTime;
use chrono::DurationRound;
use chrono::Utc;
use colored::*;
use humantime::format_duration;
use log::{error, info, warn, Level};
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::Histogram;
use prometheus_client::registry::Registry;
use serde_derive::Deserialize;
use serialport;
use signal_hook::{consts::SIGINT, consts::SIGTERM, iterator::Signals};
use std::borrow::Cow;
use std::fs;
use std::io::BufRead;
use std::io::BufReader;
use std::io::ErrorKind::BrokenPipe;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Instant;
use std::time::SystemTime;
use uuid::Uuid;

use crate::capture::{CaptureFileMetadata, CaptureFileWriter, DataPoint};
use crate::service::storage;
use crate::service::storage::StorageServiceSettings;
use crate::service::{StatusService, StorageService, WebService};
use crate::utils::SingletonService;

#[derive(Deserialize)]
struct UserFile {
    node_id: String,
}

#[derive(Deserialize)]
struct ConfigFile {
    acquire: ConfigAcquire,
    storage: ConfigStorage,
}

#[derive(Deserialize)]
struct ConfigAcquire {
    serial_port: String,
    data_dir: String,
    baud_rate: u32,
    rotate_interval_seconds: u64,
}

#[derive(Deserialize)]
struct ConfigStorage {
    endpoint: String,
    secret: String,
    key: String,
    bucket: String,
}

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

fn load_config() -> (ConfigFile, UserFile) {
    let config_contents = match fs::read_to_string("config.toml") {
        Ok(contents) => contents,
        Err(e) => panic!("Unable to open the config file: {:?}", e),
    };

    let config: ConfigFile = match toml::from_str(&config_contents) {
        Ok(data) => data,
        Err(e) => panic!("Unable to parse the config file: {:?}", e),
    };

    let user_contents = match fs::read_to_string("user.toml") {
        Ok(contents) => contents,
        Err(e) => panic!("Unable to open the user file: {:?}", e),
    };

    let user: UserFile = match toml::from_str(&user_contents) {
        Ok(data) => data,
        Err(e) => panic!("Unable to parse the user file: {:?}", e),
    };

    return (config, user);
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_logger()?;
    vendor::setup_pins().unwrap();

    let app_start = Instant::now();

    let (config, user) = load_config();
    let data_dir = Path::new(&config.acquire.data_dir);
    let node_id = &user.node_id;

    info!("Using node id: {}", node_id.bold());

    let rotate_interval =
        chrono::TimeDelta::seconds(config.acquire.rotate_interval_seconds.clone() as i64);
    info!("Rotating every {} seconds", rotate_interval.num_seconds());

    let mut metadata = CaptureFileMetadata::new(Uuid::new_v4(), 20000.0);
    metadata.set("NODE_ID", node_id);

    // Create services
    let status_service: &StatusService =
        StatusService::get_service().expect("Failed to create status service");
    let web_service = WebService::get_service().expect("Failed to create web service");
    let storage_service = StorageService::new(StorageServiceSettings::new(
        config.storage.endpoint.clone(),
        config.storage.key.clone(),
        config.storage.secret.clone(),
        config.storage.bucket.clone(),
    ))
    .unwrap();

    status_service.set_led_color(service::status::led::LedColor::White);

    match storage_service.run() {
        Ok(_) => {
            info!("Storage service started");
        }
        Err(e) => {
            error!("Failed to start storage service: {:?}", e);
        }
    }

    // Configure prometheus registry
    let labels = vec![
        (
            Cow::Borrowed("capture_id"),
            Cow::from(metadata.capture_id().to_string()),
        ),
        (Cow::Borrowed("node_id"), Cow::from(node_id.clone())),
    ];
    let registry = Registry::with_labels(labels.into_iter());
    status_service.set_registry(registry);

    let buckets = [
        0.0, 0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 1.0, 10.0,
    ];
    let family = Family::<Vec<(String, String)>, Gauge>::default();
    let gauge_gps_sats: Gauge = Gauge::default();
    let gauge_latitude = Gauge::<f64, AtomicU64>::default();
    let gauge_longitude = Gauge::<f64, AtomicU64>::default();
    let hist_process_time = Histogram::new(buckets.into_iter());

    status_service.register_metric(
        "gps_satellite_count",
        "Number of satellites in GPS fix",
        gauge_gps_sats.clone(),
    );
    status_service.register_metric(
        "heartbeat_tick_time",
        "Number of seconds from start of capture",
        hist_process_time.clone(),
    );
    status_service.register_metric("value", "Value", family.clone());
    status_service.register_metric("latitude", "Latitude", gauge_latitude.clone());
    status_service.register_metric("longitude", "Longitude", gauge_longitude.clone());

    let shutdown = Arc::new(AtomicBool::new(false));

    let mut signals = Signals::new(&[SIGINT, SIGTERM])?;
    let shutdown_clone = shutdown.clone();
    thread::spawn(move || {
        let shutdown = shutdown_clone;
        for sig in signals.forever() {
            match sig {
                SIGINT | SIGTERM => {
                    log::info!("Shutting down...");
                    match storage_service.shutdown() {
                        Ok(_) => {
                            log::info!("Storage service shutdown gracefully");
                        }
                        Err(e) => {
                            log::error!("Failed to shutdown storage service gracefully: {:?}", e);
                        }
                    }

                    match web_service.shutdown() {
                        Ok(_) => {
                            log::info!("Web service shutdown gracefully");
                        }
                        Err(e) => {
                            log::error!("Failed to shutdown web service gracefully: {:?}", e);
                        }
                    }

                    shutdown.store(true, Ordering::Relaxed);
                    status_service.set_led_color(service::status::led::LedColor::Off);

                    log::info!("Exiting...");
                    std::process::exit(0);
                }
                _ => {}
            }
        }
    });

    // Start web server
    match web_service.run() {
        Ok(_) => {
            info!("Web service started");
        }
        Err(e) => {
            error!("Failed to start web service: {:?}", e);
        }
    }

    let mut writer = CaptureFileWriter::new(data_dir, &mut metadata)?;
    writer.init();

    let mut last_rotate = DateTime::from_timestamp(0, 0).unwrap();

    info!("Opening serial port: {}", config.acquire.serial_port.bold());
    let serial_port = match serialport::new(config.acquire.serial_port, config.acquire.baud_rate)
        .timeout(std::time::Duration::from_millis(10000))
        .open()
    {
        Ok(port) => port,
        Err(e) => {
            panic!("Unable to open serial port: {:?}", e);
        }
    };

    let serial_port = BufReader::new(serial_port);

    let serial_port = Arc::new(Mutex::new(serial_port));

    while !shutdown.load(Ordering::Relaxed) {
        // First check if we need to rotate files
        let tick_start = Utc::now();
        if tick_start.duration_round(rotate_interval.clone()).unwrap()
            == tick_start
                .duration_round(chrono::TimeDelta::seconds(1))
                .unwrap()
            && (tick_start - last_rotate).num_seconds() > rotate_interval.num_seconds()
        {
            last_rotate = tick_start;
            status_service.set_led_color(service::status::led::LedColor::Cyan);
            info!(
                "Collected {} seconds, rotating files",
                rotate_interval.num_seconds()
            );
            let object_path =
                Path::new(format!("{}/", node_id.clone()).as_str()).join(writer.filename());

            storage_service
                .queue_upload(
                    storage::UploadArgs::new(
                        config.storage.bucket.clone(),
                        writer.file_path(),
                        object_path.into_os_string().into_string().unwrap(),
                    )
                    .unwrap(),
                )
                .unwrap();
            drop(writer);

            writer = CaptureFileWriter::new(data_dir, &mut metadata)?;
            writer.init();
        }

        let serial_port = serial_port.clone();
        let serial_future = tokio::task::spawn_blocking(move || {
            let mut serial_port = serial_port.lock().unwrap();
            let mut line = String::new();

            match serial_port.read_line(&mut line) {
                Ok(count) => {
                    log::debug!("Read {} bytes", count);
                }
                Err(e) => {
                    if e.kind() == BrokenPipe {
                        return Err(anyhow::anyhow!("Unable to connect to data collection port"));
                    } else {
                        return Err(anyhow::anyhow!("Internal error"));
                    }
                }
            }

            return Ok(line);
        });

        let line = match tokio::time::timeout(std::time::Duration::from_millis(2000), serial_future)
            .await
        {
            Ok(result) => {
                // Didn't timeout
                match result {
                    Ok(result) => {
                        // Was able to join thread
                        match result {
                            Ok(line) => line,
                            Err(_) => {
                                // Wasn't able to get line
                                status_service.set_led_color(service::status::led::LedColor::Red);
                                log::error!("Internal error");
                                continue;
                            }
                        }
                    }
                    Err(_) => {
                        status_service.set_led_color(service::status::led::LedColor::Red);
                        log::error!("Internal error");
                        continue;
                    }
                }
            }
            Err(_) => {
                status_service.set_led_color(service::status::led::LedColor::Red);
                log::error!("Timeout reading from serial port");
                continue;
            }
        };

        if line.starts_with("#") {
            // Comment line
            writer.comment(&line);
            log::debug!("Comment: {}", line);
            continue;
        }

        if !line.starts_with("$") {
            continue;
        }

        // Parse for data analysis
        let data_point = match DataPoint::parse(&line) {
            Ok(data_point) => data_point,
            Err(e) => {
                status_service.set_led_color(service::status::led::LedColor::Red);
                error!("Failed to parse data point: {:?}", e);

                // Write line as it is, recover it later
                writer.write_line(&line);
                // TODO maybe manually add a newline

                writer.comment("! Failed to parse data point");

                continue;
            }
        };

        if data_point.timestamp().is_none() {
            warn!("Missing timestamp, including computer timestamp as a comment");
            writer.comment(
                format!("ERR Missing timestamp, including computer timestamp on next line")
                    .as_str(),
            );
            writer.comment(format!("{}", tick_start.timestamp_millis() as f64 / 1000.0).as_str());
        }

        let line = line.chars().skip(1).collect::<String>();
        writer.write_line(&line);

        status_service.push_data(&data_point).unwrap();

        // Warn user about missing GPS fix
        if !data_point.has_gps_fix() {
            warn!("No GPS fix, data may be misaligned for this second");
            status_service.set_led_color(service::status::led::LedColor::Magenta);
        } else {
            // Get start time
            status_service.set_led_color(service::status::led::LedColor::Green);
        }

        // GPS stuff
        gauge_gps_sats.set(data_point.satellite_count() as i64);
        gauge_latitude.set(data_point.latitude() as f64);
        gauge_longitude.set(data_point.longitude() as f64);
        // family
        //     .get_or_create(&vec![
        //         ("latitude".to_string(), data_point.latitude().to_string()),
        //         ("longitude".to_string(), data_point.longitude().to_string()),
        //     ])
        //     .set(data_point.satellites() as i64);

        // Update tick time
        let tick_end = Utc::now();
        let duration = tick_end.signed_duration_since(tick_start);
        hist_process_time.observe(duration.num_nanoseconds().unwrap() as f64 / 1_000_000_000.0);
    }

    info!(
        "Exiting, ran for {}",
        format_duration(app_start.elapsed()).to_string()
    );

    Ok(())
}
