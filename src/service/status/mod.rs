pub mod led;


use std::{mem::MaybeUninit, sync::{Arc, Mutex, Once, RwLock}, thread};
use prometheus_client::{encoding::text::encode, registry::{Metric, Registry}};
use led::{LED, LedColor};
use rocket::Data;
use serde::{Deserialize, Serialize};

use crate::{capture::DataPoint, utils::SingletonService};

pub struct StatusService {
    inner: Arc<Mutex<StatusServiceInner>>
}


impl StatusService {
    fn new() -> StatusService {
        StatusService {
            inner: Arc::new(Mutex::new(StatusServiceInner::new()))
        }
    }

    pub fn set_registry(&self, registry: Registry) {
        self.inner.lock().unwrap().registry = registry;
    }

    pub fn register_metric(&self, name: &str, description: &str, metric: impl Metric) {
        self.inner.lock().unwrap().registry.register(name, description, metric);
    }

    pub fn prometheus_encode(&self) -> String {
        let mut encoded = String::new();
        encode(&mut encoded, &self.inner.lock().unwrap().registry, ).unwrap();
        encoded
    }

    pub fn push_data(&self, data: &DataPoint) -> Result<(), ()> {
        self.inner.lock().unwrap().last_data = data.clone();
        Ok(())
    }

    pub fn get_data(&self) -> DataPoint {
        self.inner.lock().unwrap().last_data.clone()
    }

    pub fn set_led_color(&self, color: LedColor) {
        self.inner.lock().unwrap().set_led_color(color);
    }

}

impl SingletonService<StatusService> for StatusService {
    fn get_service() -> &'static StatusService {
        static mut SINGLETON: MaybeUninit<StatusService> = MaybeUninit::uninit();
        static ONCE: Once = Once::new();

        unsafe {
            ONCE.call_once(|| {
                // Make it
                let singleton = StatusService::new();
                // Store it to the static var, i.e. initialize it
                SINGLETON.write(singleton);
            });

            // Now we give out a shared reference to the data, which is safe to use
            // concurrently.
            SINGLETON.assume_init_ref()
        }
    }
}

impl Clone for StatusService {
    fn clone(&self) -> Self {
        StatusService {
            inner: self.inner.clone()
        }
    }
}

struct StatusServiceInner {
    registry: Registry,
    led: led::LED,
    last_data: DataPoint
}

impl StatusServiceInner {
    fn new() -> StatusServiceInner {
        let mut led = led::LED::new(19, 20, 21).unwrap();
        led.set_color(LedColor::Off).expect("Could not set LED color");

        StatusServiceInner {
            registry: Registry::default(),
            led: led,
            last_data: DataPoint::default()
        }
    }


    fn set_led_color(&mut self, color: LedColor) -> () {
        self.led.set_color(color).unwrap();
    }


}