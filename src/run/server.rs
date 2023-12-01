use anyhow::Result;
use embedded_svc::{http::Method, io::Write};
use esp_idf_svc::http::server::{Configuration, EspHttpServer};
use std::sync::{Arc, Mutex};
use esp_idf_hal::gpio::*;

use super::flowmeter::FlowMeter;

pub fn begin<'d, P: InputPin + OutputPin>(
    server_state_viewer: Arc<Mutex<FlowMeter<P>>>,
) -> Result<EspHttpServer> {
    // 1.Create a `EspHttpServer` instance using a default configuration
    let mut server = EspHttpServer::new(&Configuration::default())?;

    // 2. Write a handler that returns the index page
    server.fn_handler("/", Method::Get, move |request| {
        let current_state = server_state_viewer.lock().unwrap();
        let html = index_html(current_state.get_flow());
        let mut response = request.into_ok_response()?;
        response.write_all(html.as_bytes())?;
        Ok(())
    })?;

    println!("Server awaiting connection");
    Ok(server)
}

fn templated(content: impl AsRef<str>) -> String {
    format!(
        r#"
<!DOCTYPE html>
<html>
    <head>
        <meta charset="utf-8">
        <title>esp-rs web server</title>
    </head>
    <h1>
        {}
    </h1>
</html>
"#,
        content.as_ref()
    )
}

fn index_html(caudal: f32) -> String {
    templated(format!("{} L/min", caudal))
}
