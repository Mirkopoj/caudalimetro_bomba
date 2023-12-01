use esp_idf_hal::gpio::*;
use esp_idf_hal::peripheral::Peripheral;
use anyhow::Result;

pub fn subscribe_pin<'d, P: InputPin + OutputPin>(
    pin: impl Peripheral<P = P> + 'd,
    notify: impl Fn() + 'static,
) -> Result<PinDriver<'d, P, Input>> {
    let mut pin = PinDriver::input(pin)?;

    pin.set_pull(Pull::Down)?;
    pin.set_interrupt_type(InterruptType::NegEdge)?;

    unsafe {
        pin.subscribe(notify)?;
    }
    Ok(pin)
}

