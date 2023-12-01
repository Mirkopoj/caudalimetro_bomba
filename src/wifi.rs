use std::{
    ffi::c_void,
    mem::{size_of, size_of_val},
    str,
    thread,
    time::Duration,
};

use anyhow::Result;
use embedded_svc::wifi::{AuthMethod, Configuration};
use esp_idf_hal::peripheral;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition, wifi::BlockingWifi, wifi::EspWifi,
};
use esp_idf_sys::*;

static mut S_WIFI_EVENT_GROUP: *mut c_void = std::ptr::null_mut();

static CONNECTED_BIT: u32 = BIT0;
static ESPTOUCH_DONE_BIT: u32 = BIT1;

pub fn wifi(
    modem: impl peripheral::Peripheral<P = esp_idf_hal::modem::Modem> + 'static,
    sysloop: EspSystemEventLoop,
    nvs_partition: EspDefaultNvsPartition,
) -> Result<Box<EspWifi<'static>>> {
    let mut _auth_method = AuthMethod::WPA2Personal;
    unsafe {
        if let Some(x_event_group_create) = g_wifi_osi_funcs._event_group_create {
            S_WIFI_EVENT_GROUP = x_event_group_create();
        }
    }

    let mut esp_wifi = EspWifi::new(modem, sysloop.clone(), Some(nvs_partition))?;

    let mut wifi = BlockingWifi::wrap(&mut esp_wifi, sysloop)?;

    let config = wifi.get_configuration()?;
    if let Configuration::Client(conf) = config {
        if !conf.ssid.is_empty() {
            println!("Conexion guardada {:?}", conf);
            wifi.start()?;
            wifi.connect()?;
            while !wifi.is_up()? {
                println!("Esperando a {:?}", conf);
                thread::sleep(Duration::from_secs(1));
            }
            println!("Conectado a {}", conf.ssid);
            return Ok(Box::new(esp_wifi));
        }
    }

    unsafe {
        esp!(esp_event_handler_register(
            WIFI_EVENT,
            ESP_EVENT_ANY_ID,
            Some(event_handler),
            std::ptr::null_mut(),
        ))?;
        esp!(esp_event_handler_register(
            IP_EVENT,
            ip_event_t_IP_EVENT_STA_GOT_IP as i32,
            Some(event_handler),
            std::ptr::null_mut(),
        ))?;
        esp!(esp_event_handler_register(
            SC_EVENT,
            ESP_EVENT_ANY_ID,
            Some(event_handler),
            std::ptr::null_mut(),
        ))?;
    }

    wifi.set_configuration(&Configuration::Client(Default::default()))?;

    wifi.start()?;

    println!("Esperando smartconfig");
    while !wifi.is_up()? {
        thread::sleep(Duration::from_secs(1));
    }

    Ok(Box::new(esp_wifi))
}

unsafe extern "C" fn event_handler(
    _event_handler_arg: *mut ::core::ffi::c_void,
    event_base: esp_event_base_t,
    event_id: i32,
    event_data: *mut ::core::ffi::c_void,
) {
    if event_base == WIFI_EVENT && event_id == wifi_event_t_WIFI_EVENT_STA_START as i32 {
        thread::spawn(|| smartconfig_example_task());
    } else if event_base == WIFI_EVENT
        && event_id == wifi_event_t_WIFI_EVENT_STA_DISCONNECTED as i32
    {
        esp_wifi_connect();
        if let Some(x_event_group_clear_bits) = g_wifi_osi_funcs._event_group_clear_bits {
            x_event_group_clear_bits(S_WIFI_EVENT_GROUP, CONNECTED_BIT);
        }
    } else if event_base == IP_EVENT && event_id == ip_event_t_IP_EVENT_STA_GOT_IP as i32 {
        if let Some(x_event_group_set_bits) = g_wifi_osi_funcs._event_group_set_bits {
            x_event_group_set_bits(S_WIFI_EVENT_GROUP, CONNECTED_BIT);
        }
    } else if event_base == SC_EVENT && event_id == smartconfig_event_t_SC_EVENT_SCAN_DONE as i32 {
        println!("Scan done");
    } else if event_base == SC_EVENT
        && event_id == smartconfig_event_t_SC_EVENT_FOUND_CHANNEL as i32
    {
        println!("Found channel");
    } else if event_base == SC_EVENT
        && event_id == smartconfig_event_t_SC_EVENT_GOT_SSID_PSWD as i32
    {
        println!("Got SSID and password");

        let evt: *mut smartconfig_event_got_ssid_pswd_t =
            event_data as *mut smartconfig_event_got_ssid_pswd_t;
        let mut wifi_config: wifi_config_t = Default::default();
        let mut ssid: [u8; 33] = [0; 33];
        let mut password: [u8; 65] = [0; 65];
        let mut rvd_data: [u8; 33] = [0; 33];

        bzero(
            &mut wifi_config as *mut _ as *mut c_void,
            size_of::<wifi_config_t>() as u32,
        );
        memcpy(
            wifi_config.sta.ssid.as_mut_ptr() as *mut c_void,
            (*evt).ssid.as_ptr() as *const c_void,
            size_of_val(&wifi_config.sta.ssid) as u32,
        );
        memcpy(
            wifi_config.sta.password.as_mut_ptr() as *mut c_void,
            (*evt).password.as_ptr() as *const c_void,
            size_of_val(&wifi_config.sta.password) as u32,
        );
        wifi_config.sta.bssid_set = (*evt).bssid_set;
        if wifi_config.sta.bssid_set == true {
            memcpy(
                wifi_config.sta.bssid.as_mut_ptr() as *mut c_void,
                (*evt).bssid.as_ptr() as *const c_void,
                size_of_val(&wifi_config.sta.bssid) as u32,
            );
        }

        memcpy(
            ssid.as_mut_ptr() as *mut c_void,
            (*evt).ssid.as_ptr() as *const c_void,
            size_of_val(&(*evt).ssid) as u32,
        );
        memcpy(
            password.as_mut_ptr() as *mut c_void,
            (*evt).password.as_ptr() as *const c_void,
            size_of_val(&(*evt).password) as u32,
        );
        let ssid_s = match str::from_utf8(&ssid) {
            Ok(s) => s,
            Err(_) => "utf error",
        };
        let password_s = match str::from_utf8(&password) {
            Ok(s) => s,
            Err(_) => "utf error",
        };
        println!("SSID:{}", ssid_s);
        println!("PASSWORD:{}", password_s);
        if (*evt).type_ == smartconfig_type_t_SC_TYPE_ESPTOUCH_V2 {
            let _ = esp!(esp_smartconfig_get_rvd_data(
                rvd_data.as_mut_ptr() as *mut u8,
                size_of_val(&rvd_data) as u8
            ));
            println!("RVD_DATA:");
            for i in 0..33 {
                print!("{:02x} ", rvd_data[i]);
            }
            println!("");
        }

        let _ = esp!(esp_wifi_disconnect());
        let _ = esp!(esp_wifi_set_config(
            wifi_interface_t_WIFI_IF_STA,
            &mut wifi_config as *mut wifi_config_t
        ));
        esp_wifi_connect();
    } else if event_base == SC_EVENT
        && event_id == smartconfig_event_t_SC_EVENT_SEND_ACK_DONE as i32
    {
        if let Some(x_event_group_set_bits) = g_wifi_osi_funcs._event_group_set_bits {
            x_event_group_set_bits(S_WIFI_EVENT_GROUP, ESPTOUCH_DONE_BIT);
        }
    }
}

fn smartconfig_example_task() {
    unsafe {
        let mut ux_bits = 0;
        let _ = esp!(esp_smartconfig_set_type(
            smartconfig_type_t_SC_TYPE_ESPTOUCH
        ));
        let cfg: smartconfig_start_config_t = Default::default();
        let _ = esp!(esp_smartconfig_start(&cfg));
        loop {
            if let Some(x_event_group_wait_bits) = g_wifi_osi_funcs._event_group_wait_bits {
                ux_bits = x_event_group_wait_bits(
                    S_WIFI_EVENT_GROUP,
                    CONNECTED_BIT | ESPTOUCH_DONE_BIT,
                    true as i32,
                    false as i32,
                    u32::MAX,
                );
            }
            if ux_bits & CONNECTED_BIT != 0 {
                println!("WiFi Connected to ap");
            }
            if ux_bits & ESPTOUCH_DONE_BIT != 0 {
                println!("smartconfig over");
                esp_smartconfig_stop();
                vTaskDelete(std::ptr::null_mut());
            }
        }
    }
}
