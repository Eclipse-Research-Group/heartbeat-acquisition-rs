use anyhow::Result;

#[cfg(target_os = "linux")]
pub fn setup_pins() -> Result<()> {
    use rppal::gpio::{Gpio, OutputPin};
    use std::error::Error;


    // Just in case, set raspberry pi pins to input
    let gpio = Gpio::new()?;
    let pin_gps_fix_led = gpio.get(26)?.into_input_pullup();
    let pin_gps_fix = gpio.get(18)?.into_input_pullup();
    let pin_gps_fix = gpio.get(4)?.into_input_pullup();
    let pin_gps_fix = gpio.get(17)?.into_input_pullup();
    let pin_gps_fix = gpio.get(16)?.into_input_pullup();

    Ok(())
}


#[cfg(not(target_os = "linux"))]
pub fn setup_pins() -> Result<()> {
    Ok(())
}