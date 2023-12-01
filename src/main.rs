use anyhow::{bail, Result};
use core::str;
use embedded_svc::{http::client::Client, io::Read};
use esp_idf_hal::{gpio::*, prelude::Peripherals, reset};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    http::client::{Configuration, EspHttpConnection},
    nvs::EspDefaultNvsPartition,
};
use subscription::subscribe_pin;

use log::info;
use std::{thread, time::Duration};

mod wifi;
use wifi::wifi;
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys::{self as _, esp, nvs_flash_erase};

use serde::{Deserialize, Serialize};

use semver::Version;

use crate::run::run;

mod run;

mod subscription;

#[derive(Serialize, Deserialize, Debug)]
struct UpdateJson {
    version: String,
    link: String,
}

#[derive(Debug)]
struct Update {
    version: Version,
    link: String,
}

impl Update {
    pub fn new(json: UpdateJson) -> Update {
        let version = Version::parse(&json.version).unwrap();
        let link = json.link;
        Update { version, link }
    }
}

fn reset_request() {
    let _ = esp!(unsafe { nvs_flash_erase() });
    reset::restart();
}

fn main() -> Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let nvs_partition = EspDefaultNvsPartition::take()?;

    let (pin0, pins) = Pines::new(peripherals.pins);
    let _reset = subscribe_pin(pin0, reset_request);

    let _wifi = wifi(peripherals.modem, sysloop, nvs_partition)?;

    let run_thread = thread::spawn(move || run(pins));

    let link = ota()?;

    ota_update(link)?;

    let _ = run_thread.join();

    Ok(())
}

fn ota() -> Result<String> {
    let json_link =
        "https://raw.githubusercontent.com/Mirkopoj/caudalimeto_bomba/master/update.json";

    let update = loop {
        if let Ok(update) = check_update(json_link) {
            break update;
        }
    };

    let version = update.version;

    loop {
        thread::sleep(Duration::from_secs(3600));
        if let Ok(update) = check_update(json_link) {
            println!("Version actual: {}", version);
            println!("Version leida: {}", update.version);
            if update.version > version {
                break;
            }
        }
    }

    Ok(update.link)
}

fn connect() -> Result<Client<EspHttpConnection>> {
    let connection = EspHttpConnection::new(&Configuration {
        use_global_ca_store: true,
        crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
        ..Default::default()
    })?;
    let client = Client::wrap(connection);

    Ok(client)
}

fn check_update(url: impl AsRef<str>) -> Result<Update> {
    let mut client = connect()?;
    let request = client.get(url.as_ref())?;
    let response = request.submit()?;
    let status = response.status();

    let update: Update;

    match status {
        200..=299 => {
            let mut buf = [0_u8; 256];
            let mut reader = response;
            let size = Read::read(&mut reader, &mut buf)?;
            if size == 0 {
                bail!("Zero sized message");
            }
            update = Update::new(serde_json::from_slice(&buf[..size])?);
        }
        _ => bail!("Unexpected response code: {}", status),
    }

    Ok(update)
}

fn ota_update(url: impl AsRef<str>) -> Result<()> {
    let mut client = connect()?;
    let request = client.get(url.as_ref())?;
    let response = request.submit()?;
    let status = response.status();
    let mut ota = esp_ota::OtaUpdate::begin()?;

    info!("Begin OTA");

    match status {
        200..=299 => {
            let mut buf = [0_u8; 256];
            let mut reader = response;
            loop {
                let size = Read::read(&mut reader, &mut buf)?;
                info!("Read {} bytes", size);
                if size == 0 {
                    break;
                }
                ota.write(&buf)?;
                info!("Wrote {} bytes", size);
            }
        }

        _ => bail!("Unexpected response code: {}", status),
    }

    let mut completed_ota = ota.finalize()?;
    completed_ota.set_as_boot_partition()?;
    info!("OTA Complete");
    completed_ota.restart();
}

pub struct Pines {
    #[cfg(not(feature = "riscv-ulp-hal"))]
    pub gpio1: Gpio1,
    pub gpio2: Gpio2,
    #[cfg(not(feature = "riscv-ulp-hal"))]
    pub gpio3: Gpio3,
    pub gpio4: Gpio4,
    #[cfg(not(feature = "riscv-ulp-hal"))]
    pub gpio5: Gpio5,
    #[cfg(not(feature = "riscv-ulp-hal"))]
    pub gpio6: Gpio6,
    #[cfg(not(feature = "riscv-ulp-hal"))]
    pub gpio7: Gpio7,
    #[cfg(not(feature = "riscv-ulp-hal"))]
    pub gpio8: Gpio8,
    #[cfg(not(feature = "riscv-ulp-hal"))]
    pub gpio9: Gpio9,
    #[cfg(not(feature = "riscv-ulp-hal"))]
    pub gpio10: Gpio10,
    #[cfg(not(feature = "riscv-ulp-hal"))]
    pub gpio11: Gpio11,
    pub gpio12: Gpio12,
    pub gpio13: Gpio13,
    pub gpio14: Gpio14,
    pub gpio15: Gpio15,
    #[cfg(not(feature = "riscv-ulp-hal"))]
    pub gpio16: Gpio16,
    #[cfg(not(feature = "riscv-ulp-hal"))]
    pub gpio17: Gpio17,
    #[cfg(not(feature = "riscv-ulp-hal"))]
    pub gpio18: Gpio18,
    #[cfg(not(feature = "riscv-ulp-hal"))]
    pub gpio19: Gpio19,
    #[cfg(not(feature = "riscv-ulp-hal"))]
    pub gpio21: Gpio21,
    #[cfg(not(feature = "riscv-ulp-hal"))]
    pub gpio22: Gpio22,
    #[cfg(not(feature = "riscv-ulp-hal"))]
    pub gpio23: Gpio23,
    pub gpio25: Gpio25,
    pub gpio26: Gpio26,
    pub gpio27: Gpio27,
    pub gpio32: Gpio32,
    pub gpio33: Gpio33,
    pub gpio34: Gpio34,
    pub gpio35: Gpio35,
    pub gpio36: Gpio36,
    pub gpio37: Gpio37,
    pub gpio38: Gpio38,
    pub gpio39: Gpio39,
}

impl Pines {
    pub fn new(pins: Pins) -> (Gpio0, Self) {
        (
            pins.gpio0,
            Self {
                #[cfg(not(feature = "riscv-ulp-hal"))]
                gpio1: pins.gpio1,
                gpio2: pins.gpio2,
                #[cfg(not(feature = "riscv-ulp-hal"))]
                gpio3: pins.gpio3,
                gpio4: pins.gpio4,
                #[cfg(not(feature = "riscv-ulp-hal"))]
                gpio5: pins.gpio5,
                #[cfg(not(feature = "riscv-ulp-hal"))]
                gpio6: pins.gpio6,
                #[cfg(not(feature = "riscv-ulp-hal"))]
                gpio7: pins.gpio7,
                #[cfg(not(feature = "riscv-ulp-hal"))]
                gpio8: pins.gpio8,
                #[cfg(not(feature = "riscv-ulp-hal"))]
                gpio9: pins.gpio9,
                #[cfg(not(feature = "riscv-ulp-hal"))]
                gpio10: pins.gpio10,
                #[cfg(not(feature = "riscv-ulp-hal"))]
                gpio11: pins.gpio11,
                gpio12: pins.gpio12,
                gpio13: pins.gpio13,
                gpio14: pins.gpio14,
                gpio15: pins.gpio15,
                #[cfg(not(feature = "riscv-ulp-hal"))]
                gpio16: pins.gpio16,
                #[cfg(not(feature = "riscv-ulp-hal"))]
                gpio17: pins.gpio17,
                #[cfg(not(feature = "riscv-ulp-hal"))]
                gpio18: pins.gpio18,
                #[cfg(not(feature = "riscv-ulp-hal"))]
                gpio19: pins.gpio19,
                #[cfg(not(feature = "riscv-ulp-hal"))]
                gpio21: pins.gpio21,
                #[cfg(not(feature = "riscv-ulp-hal"))]
                gpio22: pins.gpio22,
                #[cfg(not(feature = "riscv-ulp-hal"))]
                gpio23: pins.gpio23,
                gpio25: pins.gpio25,
                gpio26: pins.gpio26,
                gpio27: pins.gpio27,
                gpio32: pins.gpio32,
                gpio33: pins.gpio33,
                gpio34: pins.gpio34,
                gpio35: pins.gpio35,
                gpio36: pins.gpio36,
                gpio37: pins.gpio37,
                gpio38: pins.gpio38,
                gpio39: pins.gpio39,
            },
        )
    }
}
