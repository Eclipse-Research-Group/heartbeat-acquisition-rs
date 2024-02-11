pub mod capture;
pub mod service;
pub mod utils;

use std::borrow::Cow;
use std::fs;
use std::io::BufReader;
use std::io::BufRead;
use std::path::Path;
use std::time::Instant;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use signal_hook::{consts::SIGINT, consts::SIGTERM, iterator::Signals};
use colored::*;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::Histogram;
use serde_derive::Deserialize;
use serialport;
use log::{error, info, warn, Level};
use uuid::Uuid;
use std::thread;
use std::io::ErrorKind::BrokenPipe;
use prometheus_client::registry::Registry;
use tokio_util::sync::CancellationToken;
use humantime::format_duration;


use crate::capture::{CaptureFileMetadata, CaptureFileWriter, DataPoint};
use crate::service::status::StatusService;
use crate::service::web::WebService;
use crate::utils::SingletonService;

#[derive(Clone)]
struct AppData {
    registry: Arc<RwLock<Registry>>,
}


#[derive(Deserialize)]
struct ConfigFile {
    acquire: ConfigAcquire
}

#[derive(Deserialize)]
struct ConfigAcquire {
    node_id: String,
    serial_port: String,
    data_dir: String,
    baud_rate: u32
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
                humantime::format_rfc3339_seconds(SystemTime::now()),
                colored_level,
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log")?)
        .apply()?;
    Ok(())
}

fn load_config() -> ConfigFile { 
    let config_contents= match fs::read_to_string("config.toml") {
        Ok(contents) => contents,
        Err(e) => panic!("Unable to open the config file: {:?}", e),
    };

    let config: ConfigFile = match toml::from_str(&config_contents) {
        Ok(data) => data,
        Err(e) => panic!("Unable to parse the config file: {:?}", e),
    };

    return config;
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    setup_logger()?;

    let app_start = Instant::now();

    let config = load_config();
    let data_dir = Path::new(&config.acquire.data_dir);
    
    info!("Using node id: {}", config.acquire.node_id.bold());

    info!("Opening serial port: {}", config.acquire.serial_port.bold());
    let serial_port = match serialport::new(config.acquire.serial_port, config.acquire.baud_rate)
        .timeout(std::time::Duration::from_millis(10000))
        .open() {
            Ok(port) => port,
            Err(e) => {
                panic!("Unable to open serial port: {:?}", e);
            }
        };

    let mut serial_port = BufReader::new(serial_port);

    let mut metadata = CaptureFileMetadata::new(Uuid::new_v4(), 20000.0);
    metadata.set("NODE_ID", &config.acquire.node_id);

    // Create services
    let status_service = StatusService::get_service();
    let web_service = WebService::get_service();

    status_service.set_led_color(crate::service::status::led::LedColor::Magenta);

    // Configure prometheus registry
    let labels = vec![
        (Cow::Borrowed("capture_id"), Cow::from(metadata.capture_id().to_string())),
        (Cow::Borrowed("node_id"), Cow::from(config.acquire.node_id)),
    ];
    let registry = Registry::with_labels(labels.into_iter());
    status_service.set_registry(registry);

    let buckets = [0.0, 0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 1.0, 10.0];
    let family = Family::<Vec<(String, String)>, Gauge>::default();
    let gauge_gps_sats: Gauge = Gauge::default();
    let hist_process_time = Histogram::new(buckets.into_iter());

    status_service.register_metric("gps_satellite_count", "Number of satellites in GPS fix", gauge_gps_sats.clone());
    status_service.register_metric("heartbeat_tick_time", "Number of seconds from start of capture", hist_process_time.clone());
    status_service.register_metric("value", "Value", family.clone());

    let token = CancellationToken::new();

    let token_clone = token.clone();
    ctrlc::set_handler(move || {
        info!("begin cancel.");
        token_clone.cancel();
        info!("end cancel.");
    })
    .expect("Error setting Ctrl-C handler");

    let mut signals = Signals::new(&[SIGINT, SIGTERM])?;
    thread::spawn(move || {
        for sig in signals.forever() {
            match sig {
                SIGINT => {
                    log::info!("Received SIGINT");
                },
                SIGTERM => {
                    log::info!("Received SIGTERM");
                },
                _ => {}
            }
        }
    });


    // Start web server
    web_service.start();

    let mut writer = CaptureFileWriter::new(data_dir, &mut metadata)?;
    writer.init();

    status_service.set_led_color(crate::service::status::led::LedColor::White);

    while !token.is_cancelled() {

        let mut line = String::new();
        match serial_port.read_line(&mut line) {
            Ok(_) => {
                if !line.starts_with('$') {
                    continue;
                }
            },
            Err(e) => {
                if e.kind() == BrokenPipe {
                    status_service.set_led_color(crate::service::status::led::LedColor::Red);
                    error!("Unable to connect to data collection port, exiting...");
                    
                    break;
                }
            }
        } 

        status_service.set_led_color(crate::service::status::led::LedColor::Green);

        // Start timer
        let tick_start = SystemTime::now();

        // Parse for data analysis
        let data_point = match DataPoint::parse(&line) {
            Ok(data_point) => data_point,
            Err(e) => {
                error!("Failed to parse data point: {:?}", e);

                // Write line as it is, recover it later
                writer.write_line(&line);
                // TODO maybe manually add a newline

                writer.comment("ERR Failed to parse data point");
                status_service.set_led_color(crate::service::status::led::LedColor::Red);


                continue;
            }
        };

        if data_point.timestamp() == -1 {
            writer.comment(format!("ERR Missing timestamp, time as of writing is {}", tick_start.duration_since(UNIX_EPOCH)?.as_secs_f64()).as_str());
        }

        let line = line.chars().skip(1).collect::<String>();
        writer.write_line(&line);
        if writer.lines_written() % 5 == 0 {
            info!("Rotating");
            status_service.set_led_color(crate::service::status::led::LedColor::Cyan);
            writer = CaptureFileWriter::new(data_dir, &mut metadata)?;
            writer.init();
        }

        status_service.push_data(&data_point).unwrap();

        // Warn user about missing GPS fix
        if !data_point.has_gps_fix() {
            warn!("No GPS fix, data may be misaligned for this second");
            status_service.set_led_color(crate::service::status::led::LedColor::Yellow);
        }

        // GPS stuff
        gauge_gps_sats.set(data_point.satellite_count() as i64);
        family.get_or_create(&vec![("latitude".to_string(), data_point.latitude().to_string()), ("longitude".to_string(), data_point.longitude().to_string())]).set(data_point.satellites() as i64);

        // Update tick time
        let tick_time = tick_start.elapsed()?;
        hist_process_time.observe(tick_time.as_secs_f64());
    }

    info!("Exiting, ran for {}", format_duration(app_start.elapsed()).to_string());

    Ok(())

}
