use anyhow::Result;

use serialport::SerialPort;
use std::io::ErrorKind::BrokenPipe;
use std::{
    io::{BufRead, BufReader},
    mem::MaybeUninit,
    sync::{Arc, Mutex, Once},
};
use tokio::task::JoinError;

use crate::service;
use crate::utils::{map_lock_error, SingletonService};

pub struct AcquisitionServiceSettings {
    pub port: String,
    pub baud_rate: u32,
}

pub struct AcquisitionService {
    inner: Arc<futures::lock::Mutex<AcquisitionServiceInner>>,
}

static mut SINGLETON: MaybeUninit<AcquisitionService> = MaybeUninit::uninit();

impl AcquisitionService {
    pub fn new(settings: AcquisitionServiceSettings) -> Result<&'static AcquisitionService> {
        unsafe {
            SINGLETON = MaybeUninit::new(AcquisitionService {
                inner: Arc::new(futures::lock::Mutex::new(AcquisitionServiceInner::new(
                    settings,
                ))),
            });
        }

        Ok(AcquisitionService::get_service().ok_or(anyhow::anyhow!("Service not initialized"))?)
    }
}

impl SingletonService<AcquisitionService, anyhow::Error> for AcquisitionService {
    fn get_service() -> Option<&'static AcquisitionService> {
        if unsafe { SINGLETON.as_ptr().is_null() } {
            None
        } else {
            unsafe { Some(SINGLETON.assume_init_ref()) }
        }
    }

    async fn shutdown(&self) -> Result<()> {
        self.inner.lock().await.shutdown()
    }

    async fn run(&self) -> Result<()> {
        self.inner.lock().await.acquire().await
    }

    async fn is_alive(&self) -> Result<bool> {
        Ok(true)
    }
}

struct AcquisitionServiceInner {
    settings: AcquisitionServiceSettings,
}

impl AcquisitionServiceInner {
    pub fn new(settings: AcquisitionServiceSettings) -> Self {
        AcquisitionServiceInner { settings: settings }
    }

    pub fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    pub async fn acquire(&self) -> Result<()> {
        let port = self.settings.port.clone();
        let baud_rate = self.settings.baud_rate.clone();
        tokio::spawn(async move {
            let serial_port = serialport::new(port, baud_rate)
                .open()
                .map_err(|e| anyhow::anyhow!("Unable to open serial port: {}", e))
                .unwrap();
            let serial_port = Arc::new(Mutex::new(BufReader::new(serial_port)));

            loop {
                // sleep for 500 ms
                //

                tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                let line = match tokio::time::timeout(
                    std::time::Duration::from_millis(2000),
                    AcquisitionServiceInner::serial_read(serial_port.clone()),
                )
                .await
                {
                    Ok(result) => match result {
                        Ok(result) => match result {
                            Ok(result) => result,
                            Err(e) => {
                                log::error!("Error reading from serial port: {}", e);
                                continue;
                            }
                        },
                        Err(e) => {
                            log::error!("Error joining serial read task: {}", e);
                            continue;
                        }
                    },
                    Err(_) => {
                        log::error!("Timeout reading from serial port");
                        continue;
                    }
                };

                log::debug!("Read line: {}", line);
            }
        });

        Ok(())
    }

    pub async fn serial_read(
        serial_port: Arc<Mutex<BufReader<Box<dyn SerialPort>>>>,
    ) -> Result<Result<String>, JoinError> {
        let mut buffer = String::new();
        tokio::task::spawn_blocking(move || {
            let mut serial_port = serial_port.lock().unwrap();
            let mut line = String::new();

            match serial_port.read_line(&mut line) {
                Ok(count) => {
                    log::debug!("Read {} bytes", count);
                }
                Err(e) => {
                    if e.kind() == BrokenPipe {
                        return Err(anyhow::anyhow!("Unable to connect to data collection port"));
                    }
                    return Err(anyhow::anyhow!("Internal error: {}", e));
                }
            }

            Ok(line)
        })
        .await
    }
}
