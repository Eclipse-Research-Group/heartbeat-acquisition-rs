use std::time::Duration;

use super::ServiceMessage;

pub struct SecTickConfig {
    serial_port: String,
    baud_rate: u32,
    timeout: Duration
}

pub struct SecTickService {
    config: SecTickConfig,
    port: Option<std::sync::Arc<std::sync::Mutex<std::io::BufReader<Box<dyn serialport::SerialPort>>>>>
}

impl SecTickService {
    pub fn new(config: SecTickConfig,
    tx: tokio::sync::broadcast::Sender<ServiceMessage>) -> SecTickService {
        SecTickService { 
            config,
            port: None
        }
    }

    pub fn start(&self) -> anyhow::Result<()> {
        let port = serialport::new(self.config.serial_port.clone(), self.baud_rate)
        .timeout(self.config.timeout)
        .open()?;

        let port = std::sync::Arc::new(std::sync::Mutex::new(std::io::BufReader::new(port)));

        self.port = Some(port);


        tokio::spawn(async {

        });

        Ok(())
    }

}