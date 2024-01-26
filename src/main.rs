pub mod data;
pub mod capture;

use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::io::BufRead;
use data::DataPointFlags;
use toml::toml;
use serde_derive::Deserialize;
use serialport;
use log::{debug, error, log_enabled, info, Level};


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





fn main() {
    let config_contents= match fs::read_to_string("config.toml") {
        Ok(contents) => contents,
        Err(e) => panic!("Unable to open the config file: {:?}", e),
    };

    let config: ConfigFile = match toml::from_str(&config_contents) {
        Ok(data) => data,
        Err(e) => panic!("Unable to parse the config file: {:?}", e),
    };
    
    println!("Found node id: {}", config.acquire.node_id);

    println!("Opening serial port: {}", config.acquire.serial_port);
    let mut port = serialport::new(config.acquire.serial_port, config.acquire.baud_rate)
        .timeout(std::time::Duration::from_millis(10000))
        .open()
        .expect("Failed to open serial port");

    let mut reader = BufReader::new(port);

    let output_file = File::create("output.txt").expect("Failed to create output file");
    let mut writer = BufWriter::new(output_file);

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(_) => {},
            Err(e) => println!("Error: {:?}", e)
        }       

        let print_line = line.chars().skip(1).take(40).collect::<String>();
        println!("Line: {}", print_line);

        let parts = line.split(",").collect::<Vec<&str>>();
        let line = line.chars().skip(1).collect::<String>();
        writer.write_all(line.as_bytes()).unwrap();
    }


}
