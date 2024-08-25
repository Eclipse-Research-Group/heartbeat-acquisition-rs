pub mod data;

pub use data::Frame;

use std::time::Duration;

pub struct SecTickData {
    pub timestamp: u64
}

pub struct SecTickModule {
    serial_port: String,
    baud_rate: u32,
    timeout: Duration,
    port: Option<Box<dyn serialport::SerialPort>>
}

impl SecTickModule {
    
    pub fn new(serial_port: String, baud_rate: u32, timeout: Duration) -> SecTickModule {
        SecTickModule { serial_port, baud_rate, timeout, port: None }
    }

    pub fn open(&mut self) -> anyhow::Result<()> {
        println!("Opening serial port: {} at baud rate: {}", self.serial_port, self.baud_rate);

        // Open serial port
        let port = serialport::new(self.serial_port.clone(), self.baud_rate)
            .timeout(self.timeout)
            .open()?;

        self.port = Some(port);

        Ok(())
    }

    pub fn close(&mut self) -> anyhow::Result<()> {
        println!("Closing serial port: {}", self.serial_port);

        if let Some(port) = self.port.take() {
            drop(port);
        }

        Ok(())
    }

    async fn read_line(&mut self) -> anyhow::Result<String> {
        // let serial_read_future = tokio::task::spawn_blocking(move || {

        // });

        Ok("".to_string())
    }

    pub async fn next_data(&mut self) -> anyhow::Result<SecTickData> {
        return Ok(SecTickData { timestamp: 0 });
    }

}