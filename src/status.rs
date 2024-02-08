use std::{sync::{Arc, Mutex, RwLock}, thread};
use actix_web::{App, HttpServer};
use prometheus_client::{encoding::text::encode, registry::{Metric, Registry}};

#[cfg(target_os = "linux")]
use led::{LED, Color};

#[cfg(target_os = "linux")]
mod led {
    use rppal::gpio::{Gpio, OutputPin};
    use std::error::Error;

    pub struct LED {
        pin_red: OutputPin,
        pin_green: OutputPin,
        pin_blue: OutputPin,
        color: Color
    }

    impl LED {
        pub fn new(pin_red: u8, pin_green: u8, pin_blue: u8) -> Result<LED, Box<dyn Error>> {
            Ok(LED {
                pin_red: Gpio::new()?.get(pin_red)?.into_output(),
                pin_green: Gpio::new()?.get(pin_green)?.into_output(),
                pin_blue: Gpio::new()?.get(pin_blue)?.into_output(),
                color: Color::Off
            })
        }
    }

    pub enum Color {
        Red,
        Green,
        Blue,
        Cyan,
        Magenta,
        Yellow,
        White,
        Off
    }

}

pub struct MetricManager {
    inner: Arc<Mutex<MetricManagerInner>>
}

impl MetricManager {
    pub fn new() -> MetricManager {
        MetricManager {
            inner: Arc::new(Mutex::new(MetricManagerInner::new()))
        }
    }

    pub fn register_metric(&self, name: &str, description: &str, metric: impl Metric) {
        self.inner.lock().unwrap().registry.register(name, description, metric);
    }

    pub fn prometheus_encode(&self) -> String {
        let mut encoded = String::new();
        encode(&mut encoded, &self.inner.lock().unwrap().registry, ).unwrap();
        encoded
    }

}

impl Clone for MetricManager {
    fn clone(&self) -> Self {
        MetricManager {
            inner: self.inner.clone()
        }
    }
}

struct MetricManagerInner {
    registry: Registry,

    #[cfg(target_os = "linux")]
    led: led::LED
}

impl MetricManagerInner {
    fn new() -> MetricManagerInner {
        MetricManagerInner {
            registry: Registry::default(),

            #[cfg(target_os = "linux")]
            led: led::LED::new(1,2,3).unwrap()
        }
    }
}