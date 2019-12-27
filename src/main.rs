use log::*;
use linux_embedded_hal as hal;
use bme680::{Bme680, I2CAddress, PowerMode, SettingsBuilder, OversamplingSetting, IIRFilterSize};
use core::time::Duration;
use embedded_hal::blocking::delay::DelayMs;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::{thread};
use hyper::{header::CONTENT_TYPE, Body, Request, Response, Server};
use hyper::service::{make_service_fn, service_fn};
use lazy_static::lazy_static;
use prometheus::{Encoder, Gauge, TextEncoder};

// Y?!
#[macro_use]
extern crate prometheus;

lazy_static! {
    static ref TMP_GAUGE: Gauge = register_gauge!("bme680_temp", "degrees celsius").unwrap();
    static ref PRS_GAUGE: Gauge = register_gauge!("bme680_pressure", "hPa").unwrap();
    static ref HUM_GAUGE: Gauge = register_gauge!("bme680_humidity", "precentage").unwrap();
    static ref GAS_GAUGE: Gauge = register_gauge!("bme680_gas", "resistance Ω").unwrap();
}

const PORT: u16 = 4242;

#[tokio::main]
async fn main() {
    info!("Initializing");
    env_logger::init();

    let i2c = hal::I2cdev::new("/dev/i2c-1").expect("Error in I2cdev::new()");

    let mut dev = Bme680::init(i2c, hal::Delay {}, I2CAddress::Primary).expect("could not init bme680 device");
    let mut delay = hal::Delay {};

    let settings = SettingsBuilder::new()
        .with_humidity_oversampling(OversamplingSetting::OS2x)
        .with_pressure_oversampling(OversamplingSetting::OS4x)
        .with_temperature_oversampling(OversamplingSetting::OS8x)
        .with_temperature_filter(IIRFilterSize::Size3)
        .with_gas_measurement(Duration::from_millis(1500), 320, 25)
        .with_run_gas(true)
        .build();

    dev.set_sensor_settings(settings).expect("Could not set sensor settings");
    dev.set_sensor_mode(PowerMode::ForcedMode).expect("Could not set sensor mode");

    let sensor_settings = dev.get_sensor_settings(settings.1);
    info!("Sensor settings: {:?}", sensor_settings);

    async fn prom(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
        let encoder = TextEncoder::new();
        let mut buffer = vec![];
        let metric_families = prometheus::gather();
        encoder.encode(&metric_families, &mut buffer).unwrap();

        let response = Response::builder()
            .status(200)
            .header(CONTENT_TYPE, encoder.format_type())
            .body(Body::from(buffer))
            .unwrap();

        Ok(response)
    }

    let make_svc = make_service_fn(|_conn| async {
        Ok::<_, Infallible>(service_fn(prom))
    });

    info!("Starting poll loop");
    thread::spawn(move || {
        loop {
            delay.delay_ms(5000u32);
            info!("Retrieving sensor data");

            if let Err(_) = dev.set_sensor_mode(PowerMode::ForcedMode) {
                error!("Could not set sensor mode");
                continue;
            }

            let data = match dev.get_sensor_data() {
                Ok(sensor_data) => sensor_data.0,
                Err(_) => {
                    error!("Could not get sensor data");
                    continue;
                }
            };

            info!("Temperature {}°C",   data.temperature_celsius());
            info!("Pressure {}hPa",     data.pressure_hpa());
            info!("Humidity {}%",       data.humidity_percent());
            info!("Gas Resistence {}Ω", data.gas_resistance_ohm());

            TMP_GAUGE.set(data.temperature_celsius() as f64);
            PRS_GAUGE.set(data.pressure_hpa() as f64);
            HUM_GAUGE.set(data.humidity_percent() as f64);
            GAS_GAUGE.set(data.gas_resistance_ohm() as f64);
        }
    });

    info!("Starting server on port {}", PORT);
    let addr = SocketAddr::from(([0, 0, 0, 0], PORT));
    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
