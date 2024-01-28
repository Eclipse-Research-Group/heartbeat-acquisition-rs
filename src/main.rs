pub mod data;
pub mod capture;

use std::borrow::Cow;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::io::BufRead;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use std::sync::{Arc, RwLock};
use colored::*;
use futures::future::Shared;
use futures::lock::Mutex;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::Histogram;
use tokio::signal;
use toml::toml;
use serde_derive::Deserialize;
use serialport;
use log::{debug, error, info, log_enabled, warn, Level};
use uuid::Uuid;
use std::thread;
use prometheus_client::encoding::text::encode;
use std::io::ErrorKind::BrokenPipe;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::registry::{self, Registry};
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use futures::executor::block_on;
use tokio_util::sync::CancellationToken;
use std::time::SystemTime;

use crate::capture::CaptureFileMetadata;
use crate::data::DataPointFlags;
use crate::data::DataPoint;


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

#[get("/metrics")]
async fn metrics(data: web::Data<AppData>) -> impl Responder {
    info!("Prometheus metrics fetched");
    let registry = data.registry.read().unwrap();
    let mut buffer = String::new();
    encode(&mut buffer, &registry).unwrap();
    return HttpResponse::Ok().body(buffer);
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    setup_logger()?;

    let config_contents= match fs::read_to_string("config.toml") {
        Ok(contents) => contents,
        Err(e) => panic!("Unable to open the config file: {:?}", e),
    };

    let config: ConfigFile = match toml::from_str(&config_contents) {
        Ok(data) => data,
        Err(e) => panic!("Unable to parse the config file: {:?}", e),
    };
    
    info!("Found node id: {}", config.acquire.node_id);

    info!("Opening serial port: {}", config.acquire.serial_port);
    let port = serialport::new(config.acquire.serial_port, config.acquire.baud_rate)
        .timeout(std::time::Duration::from_millis(10000))
        .open()
        .expect("Failed to open serial port");

    let mut reader = BufReader::new(port);

    let output_file = File::create("output.txt").expect("Failed to create output file");
    let mut writer = BufWriter::new(output_file);

    let mut metadata = CaptureFileMetadata::new(Uuid::new_v4(), 20000.0);

    // Add custom metadata
    metadata.set("NODE_ID", &config.acquire.node_id);

    // Write header to file
    let metadata_string: String = metadata.to_string();
    writer.write_all(metadata_string.as_bytes()).unwrap();

    let labels = vec![
        (Cow::Borrowed("capture_id"), Cow::from(metadata.capture_id().to_string())),
        (Cow::Borrowed("node_id"), Cow::from(config.acquire.node_id)),
    ];
    let registry = Registry::with_labels(labels.into_iter());
    let shared_registry = Arc::new(RwLock::new(registry));
    

    let buckets = [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];
    let gauge_gps_sats: Gauge = Gauge::default();
    let hist_process_time = Histogram::new(buckets.into_iter());
    
    {
        let mut registry = shared_registry.write().unwrap();
        registry.register("gps_satellite_count", "Number of satellites in GPS fix", gauge_gps_sats.clone());
        registry.register("heartbeat_tick_time", "Number of seconds from start of capture", hist_process_time.clone());
    }


    let registry_for_thread = shared_registry.clone();
    let token = CancellationToken::new();

    let token_clone = token.clone();
    ctrlc::set_handler(move || {
        token_clone.cancel();
    })
    .expect("Error setting Ctrl-C handler");

    let token_clone = token.clone();
    thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .thread_name("heartbeat-acquisition-rs")
            .build()
            .unwrap()
            .block_on(async {
                let srv = HttpServer::new(move || {
                    App::new()
                        .service(metrics)
                        .app_data(web::Data::new(AppData {
                            registry: registry_for_thread.clone(),
                        }))
                })
                .bind(("0.0.0.0", 8003))
                .unwrap()
                .run();
                
                tokio::select! {
                    _ = srv => {},
                    _ = token_clone.cancelled() => {
                        info!("Shutting down...");
                    },
                }

            });
    });

    while !token.is_cancelled() {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(_) => {
                info!("Reading line");
            },
            Err(e) => {
                if e.kind() == BrokenPipe {
                    error!("Unable to connect to data collection port, exiting...");
                    break;
                }
            }
        } 

        let now = Instant::now();

        if !line.starts_with("$") {
            // Not a data line
            continue;
        }



        // We don't need to parse the line here, we can just write it to the output file, skipping the first character '$'
        let line = line.chars().skip(1).collect::<String>();
        writer.write_all(line.as_bytes()).unwrap();
        writer.flush();

        // Parse for data analysis
        let data_point = match DataPoint::parse(&line) {
            Ok(data_point) => data_point,
            Err(e) => {
                error!("Failed to parse data point: {:?}", e);
                continue;
            }
        };

        if !data_point.has_gps_fix() {
            warn!("No GPS fix, data may be misaligned for this second");
        }

        gauge_gps_sats.set(data_point.satellite_count() as i64);

        let end = now.elapsed();
        hist_process_time.observe(end.as_nanos() as f64);
        debug!("Time elapsed: {:?}", end);


        
    }

    Ok(())

}
