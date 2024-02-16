#[derive(PartialEq, Copy, Clone)]
pub enum LedColor {
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
pub mod led {
    use rppal::gpio::{Gpio, OutputPin};
    use std::error::Error;
    use super::LedColor;

    pub struct LED {
        pin_red: OutputPin,
        pin_green: OutputPin,
        pin_blue: OutputPin,
        color: LedColor
    }

    impl LED {
        pub fn new(pin_red: u8, pin_green: u8, pin_blue: u8) -> Result<LED, Box<dyn Error>> {
            Ok(LED {
                pin_red: Gpio::new()?.get(pin_red)?.into_output(),
                pin_green: Gpio::new()?.get(pin_green)?.into_output(),
                pin_blue: Gpio::new()?.get(pin_blue)?.into_output(),
                color: LedColor::Off
            })
        }

        pub fn set_color(&mut self, color: LedColor) -> Result<(), Box<dyn Error>> {
            match color {
                LedColor::Red => {
                    self.pin_red.set_high();
                    self.pin_green.set_low();
                    self.pin_blue.set_low();
                },
                LedColor::Green => {
                    self.pin_red.set_low();
                    self.pin_green.set_high();
                    self.pin_blue.set_low();
                },
                LedColor::Blue => {
                    self.pin_red.set_low();
                    self.pin_green.set_low();
                    self.pin_blue.set_high();
                },
                LedColor::Cyan => {
                    self.pin_red.set_low();
                    self.pin_green.set_high();
                    self.pin_blue.set_high();
                },
                LedColor::Magenta => {
                    self.pin_red.set_high();
                    self.pin_green.set_low();
                    self.pin_blue.set_high();
                },
                LedColor::Yellow => {
                    self.pin_red.set_high();
                    self.pin_green.set_high();
                    self.pin_blue.set_low();
                },
                LedColor::White => {
                    self.pin_red.set_high();
                    self.pin_green.set_high();
                    self.pin_blue.set_high();
                },
                LedColor::Off => {
                    self.pin_red.set_low();
                    self.pin_green.set_low();
                    self.pin_blue.set_low();
                },
            }
            self.color = color; // Save the current color state
            Ok(())
        }

        pub fn get_color(&self) -> LedColor {
            self.color
        }
    }

}

#[cfg(not(target_os = "linux"))]
pub mod led {
    use std::error::Error;
    use super::LedColor;

    pub struct LED {
        color: LedColor
    }

    impl LED {
        pub fn new(_pin_red: u8, _pin_green: u8, _pin_blue: u8) -> Result<LED, Box<dyn Error>> {
            Ok(LED {
                color: LedColor::Off
            })
        }

        pub fn set_color(&mut self, _color: LedColor) -> Result<(), Box<dyn Error>> {
            Ok(())
        }

        pub fn get_color(&self) -> LedColor {
            self.color
        }
    }

}

pub use led::LED;