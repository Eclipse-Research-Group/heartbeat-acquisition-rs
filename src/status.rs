use std::{sync::{Arc, Mutex, RwLock}, thread};
use actix_web::{App, HttpServer};
use prometheus_client::{encoding::text::encode, registry::{Metric, Registry}};

#[cfg(target_os = "linux")]
use led::LED;

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

#[cfg(target_os = "linux")]
mod led {
    use rppal::gpio::{Gpio, OutputPin};
    use std::error::Error;
    use crate::status::Color;

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

        pub fn set_color(&mut self, color: Color) -> Result<(), Box<dyn Error>> {
            match color {
                Color::Red => {
                    self.pin_red.set_high();
                    self.pin_green.set_low();
                    self.pin_blue.set_low();
                },
                Color::Green => {
                    self.pin_red.set_low();
                    self.pin_green.set_high();
                    self.pin_blue.set_low();
                },
                Color::Blue => {
                    self.pin_red.set_low();
                    self.pin_green.set_low();
                    self.pin_blue.set_high();
                },
                Color::Cyan => {
                    self.pin_red.set_low();
                    self.pin_green.set_high();
                    self.pin_blue.set_high();
                },
                Color::Magenta => {
                    self.pin_red.set_high();
                    self.pin_green.set_low();
                    self.pin_blue.set_high();
                },
                Color::Yellow => {
                    self.pin_red.set_high();
                    self.pin_green.set_high();
                    self.pin_blue.set_low();
                },
                Color::White => {
                    self.pin_red.set_high();
                    self.pin_green.set_high();
                    self.pin_blue.set_high();
                },
                Color::Off => {
                    self.pin_red.set_low();
                    self.pin_green.set_low();
                    self.pin_blue.set_low();
                },
            }
            self.color = color; // Save the current color state
            Ok(())
        }
    }

}

pub struct StatusManager {
    inner: Arc<Mutex<StatusManagerInner>>
}

impl StatusManager {
    pub fn new() -> StatusManager {
        StatusManager {
            inner: Arc::new(Mutex::new(StatusManagerInner::new()))
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

impl Clone for StatusManager {
    fn clone(&self) -> Self {
        StatusManager {
            inner: self.inner.clone()
        }
    }
}

struct StatusManagerInner {
    registry: Registry,

    #[cfg(target_os = "linux")]
    led: led::LED
}

impl StatusManagerInner {
    fn new() -> StatusManagerInner {

        #[cfg(target_os = "linux")]
        let mut led = {
            let mut led = led::LED::new(19, 20, 21).unwrap();
            led.set_color(Color::White);
            led
        };

        StatusManagerInner {
            registry: Registry::default(),

            #[cfg(target_os = "linux")]
            led: led
        }
    }

    #[cfg(target_os = "linux")]
    fn set_led_color(&mut self, color: Color) -> () {
        self.led.set_color(color).unwrap();
    }
}