static PULSE_COUNT: AtomicU32 = AtomicU32::new(0);
const MEASUREMENT_INTERVAL: u64 = 3;
const PULSES_PER_LITER_PER_MINUTE: f32 = 4.8;

use anyhow::Result;
use esp_idf_hal::gpio::*;
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_svc::timer::*;
use esp_idf_sys::*;
use std::sync::{atomic::*, Arc, Mutex};
use std::time::Duration;

pub struct FlowMeter<P>
where
    P: Pin,
{
    flow: f32,
    _pin: PinDriver<'static, P, Input>,
}

impl<P: InputPin + OutputPin> FlowMeter<P> {
    pub fn new(pin: impl Peripheral<P = P> + 'static) -> Result<Self> {
        Ok(Self {
            flow: 0.0,
            _pin: subscribe_pin(pin, count_pulse)?,
        })
    }

    pub fn get_flow(&self) -> f32 {
        self.flow
    }

    fn set_flow(&mut self, flow: f32) {
        self.flow = flow;
    }
}

pub fn set_measurement_timer<P: InputPin + OutputPin>(
    flowmeter_arc: Arc<Mutex<FlowMeter<P>>>,
) -> Result<EspTimer, EspError> {
    let periodic_timer = EspTimerService::new()?.timer(move || {
        let cnt = PULSE_COUNT.fetch_and(0, Ordering::Relaxed);
        let mut flowmeter = flowmeter_arc.lock().unwrap();
        flowmeter.set_flow(
            cnt as f32 / (PULSES_PER_LITER_PER_MINUTE * (MEASUREMENT_INTERVAL as u32) as f32),
        );
    })?;

    periodic_timer.every(Duration::from_secs(MEASUREMENT_INTERVAL))?;

    Ok(periodic_timer)
}

fn count_pulse() {
    PULSE_COUNT.fetch_add(1, Ordering::Relaxed);
}

fn subscribe_pin<'d, P: InputPin + OutputPin>(
    pin: impl Peripheral<P = P> + 'd,
    notify: impl Fn() + 'static,
) -> Result<PinDriver<'d, P, Input>> {
    let mut pin = PinDriver::input(pin)?;

    pin.set_interrupt_type(InterruptType::NegEdge)?;

    unsafe {
        pin.subscribe(notify)?;
    }
    Ok(pin)
}
