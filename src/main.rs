use log::*;
use linux_embedded_hal as hal;
use bme680::{Bme680, I2CAddress, PowerMode, SettingsBuilder, OversamplingSetting, IIRFilterSize};
use core::result;
use core::time::Duration;
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::blocking::i2c;
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

#[tokio::main]
async fn main() -> result::Result<(), bme680::Error<<hal::I2cdev as i2c::Read>::Error, <hal::I2cdev as i2c::Write>::Error>> {
    env_logger::init();

    let i2c = hal::I2cdev::new("/dev/i2c-1").unwrap();

    let mut dev = Bme680::init(i2c, hal::Delay {}, I2CAddress::Primary)?;
    let mut delay = hal::Delay {};

    let settings = SettingsBuilder::new()
        .with_humidity_oversampling(OversamplingSetting::OS2x)
        .with_pressure_oversampling(OversamplingSetting::OS4x)
        .with_temperature_oversampling(OversamplingSetting::OS8x)
        .with_temperature_filter(IIRFilterSize::Size3)
        .with_gas_measurement(Duration::from_millis(1500), 320, 25)
        .with_run_gas(true)
        .build();

    dev.set_sensor_settings(settings)?;
    dev.set_sensor_mode(PowerMode::ForcedMode)?;

    let sensor_settings = dev.get_sensor_settings(settings.1);
    info!("Sensor settings: {:?}", sensor_settings);

    let addr = SocketAddr::from(([0, 0, 0, 0], 4242));

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

    let server = Server::bind(&addr).serve(make_svc);

    thread::spawn(move || {
        loop {
            delay.delay_ms(5000u32);
            info!("Retrieving sensor data");
            dev.set_sensor_mode(PowerMode::ForcedMode).unwrap();
            let (data, _state) = dev.get_sensor_data().unwrap();
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

    // Run this server for... forever!
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }

    Ok(())
}
