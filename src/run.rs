use anyhow::Result;
use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use crate::Pines;

use self::{
    flowmeter::{set_measurement_timer, FlowMeter},
    pump::Pump,
};

mod flowmeter;
mod pump;
mod server;

pub fn run(pins: Pines) -> Result<()> {
    let state = Arc::new(Mutex::new(FlowMeter::new(pins.gpio32)?));

    let _server = server::begin(state.clone())?;

    let _timer = set_measurement_timer(state.clone())?;

    let mut pump = Pump::new(state, pins.gpio2, 1.0, 5.0)?;

    loop {
        pump.manage()?;
        thread::sleep(Duration::from_secs(1));
    }
}
