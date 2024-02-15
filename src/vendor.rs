use anyhow::Result;

#[cfg(target_os = "linux")]
pub fn setup_pins() -> Result<()> {
    use rppal::gpio::{Gpio, OutputPin};
    use std::error::Error;

    let gpio = Gpio::new()?;
    let pin_red = gpio.get(17)?.into_output();
    let pin_green = gpio.get(27)?.into_output();
    let pin_blue = gpio.get(22)?.into_output();

    let pin_gps_fix_led = gpio.get(26)?.into_input_pullup();
    let pin_gps_fix = gpio.get(18)?.into_input_pullup();
    let pin_gps_fix = gpio.get(4)?.into_input_pullup();
    let pin_gps_fix = gpio.get(17)?.into_input_pullup();
    let pin_gps_fix = gpio.get(16)?.into_input_pullup();

    Ok(())
}


#[cfg(not(target_os = "linux"))]
pub fn setup_pins() {
    
}