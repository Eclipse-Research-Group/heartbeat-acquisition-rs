pub mod data;

use anyhow::Context;
pub use data::Frame;
use tokio::task::JoinHandle;
use std::io::BufRead;

use std::time::Duration;

pub struct SecTickData {
    pub timestamp: u64
}

pub struct SecTickModule {
    serial_port: String,
    baud_rate: u32,
    timeout: Duration,
    port: Option<std::sync::Arc<std::sync::Mutex<std::io::BufReader<Box<dyn serialport::SerialPort>>>>>
}

impl SecTickModule {
    
    pub fn new(serial_port: String, baud_rate: u32, timeout: Duration) -> SecTickModule {
        SecTickModule { serial_port, baud_rate, timeout, port: None }
    }

    pub fn open(&mut self) -> anyhow::Result<()> {
        log::info!("Opening serial port: {} at baud rate: {}", self.serial_port, self.baud_rate);

        // Open serial port
        let port = serialport::new(self.serial_port.clone(), self.baud_rate)
            .timeout(self.timeout)
            .open()?;

        let port = std::sync::Arc::new(std::sync::Mutex::new(std::io::BufReader::new(port)));

        self.port = Some(port);

        Ok(())
    }

    pub async fn read_line(&mut self) -> anyhow::Result<String> {
        let port = self.port.as_ref().context("No port open")?.clone();
        let serial_read_future: JoinHandle<anyhow::Result<String>> = tokio::task::spawn_blocking(move || {
            let mut line = String::new();
            let mut port = port.lock().map_err(|_| anyhow::anyhow!("Error locking mutex"))?;

            port.read_line(&mut line)?;

            Ok(line)
        });

        match tokio::time::timeout(self.timeout, serial_read_future).await {
            Ok(serial_read_future) => return serial_read_future?,
            Err(_) => return Err(anyhow::anyhow!("Timeout reading serial port"))
        }

    }

    pub async fn next_data(&mut self) -> anyhow::Result<SecTickData> {
        return Ok(SecTickData { timestamp: 0 });
    }

}