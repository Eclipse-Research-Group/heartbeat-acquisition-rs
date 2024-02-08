use std::{sync::{Arc, Mutex, RwLock}, thread};

use actix_web::{App, HttpServer};
use prometheus_client::{encoding::text::encode, registry::{Metric, Registry}};

#[cfg(target_os = "linux")]
pub mod led;

#[cfg(target_os = "linux")]
use led::{LED, Color};

#[cfg(target_os = "linux")]
mod led {
    use rppal::gpio::{Gpio, OutputPin, Pin, Output, PullUp, PushPull};

    pub struct LED {
        pin_red: OutputPin<Output<PushPull>>,
        pin_green: OutputPin<Output<PushPull>>,
        pin_blue: OutputPin<Output<PushPull>>,
        color: Color
    }

    impl LED {
        pub fn new(pin_red: u8, pin_green: u8, pin_blue: u8) -> LED {
            LED {
                pin_red: Pin::new(pin_red).into_output(),
                pin_green: Pin::new(pin_green).into_output(),
                pin_blue: Pin::new(pin_blue).into_output(),
                color: Color::Off
            }
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
    registry: Registry
}

impl MetricManagerInner {
    fn new() -> MetricManagerInner {
        MetricManagerInner {
            registry: Registry::default()
        }
    }
}