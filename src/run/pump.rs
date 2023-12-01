use super::flowmeter::FlowMeter;
use anyhow::Result;
use esp_idf_hal::gpio::*;
use esp_idf_hal::peripheral::Peripheral;
use std::sync::{Arc, Mutex};

pub struct Pump<P, I>
where
    P: Pin,
    I: Pin,
{
    state: Arc<Mutex<FlowMeter<I>>>,
    pin: PinDriver<'static, P, Output>,
    threshold_min: f32,
    threshold_max: f32,
}

fn min(a: f32, b: f32) -> f32 {
    if a < b {
        a
    } else {
        b
    }
}

fn max(a: f32, b: f32) -> f32 {
    if a > b {
        a
    } else {
        b
    }
}

impl<P: InputPin + OutputPin, I: InputPin + OutputPin> Pump<P, I> {
    pub fn new(
        state: Arc<Mutex<FlowMeter<I>>>,
        pin: impl Peripheral<P = P> + 'static,
        threshold_min: f32,
        threshold_max: f32,
    ) -> Result<Self> {
        Ok(Self {
            state,
            pin: PinDriver::output(pin)?,
            threshold_min: min(threshold_min, threshold_max),
            threshold_max: max(threshold_min, threshold_max),
        })
    }

    pub fn manage(&mut self) -> Result<()> {
        let flow = self.state.lock().unwrap().get_flow();
        if flow > self.threshold_max {
            self.pin.set_high()?;
        }
        if flow < self.threshold_min {
            self.pin.set_low()?;
        }
        Ok(())
    }
}
