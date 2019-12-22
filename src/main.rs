extern crate bme680;
extern crate embedded_hal;
extern crate env_logger;
extern crate linux_embedded_hal as hal;
#[macro_use]
extern crate log;

// use crate::hal::*;
use bme680::{Bme680, I2CAddress, FieldData, PowerMode, SettingsBuilder, OversamplingSetting, IIRFilterSize};
use core::result;
use core::time::Duration;
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::blocking::i2c;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::{thread};
use hyper::{Body, Request, Response, Server};
use hyper::service::{make_service_fn, service_fn};
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> result::Result<(), bme680::Error<<hal::I2cdev as i2c::Read>::Error, <hal::I2cdev as i2c::Write>::Error>> {
    env_logger::init();

    let i2c = hal::I2cdev::new("/dev/i2c-1").unwrap();

    let mut dev = Bme680::init(i2c, hal::Delay {}, I2CAddress::Primary)?;
    let mut delay = hal::Delay {};

    let field_data = Arc::new(Mutex::new(None::<FieldData>));

    // let state = State { field_data: Arc::clone(&field_data) };

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

    let addr = SocketAddr::from(([127, 0, 0, 1], 4242));

    let make_svc = make_service_fn(|socket: &hyper::server::conn::AddrStream| {
        let remote_addr = socket.remote_addr();
        let field_data = field_data.clone();
        async move {
            let field_data = field_data.clone();
            Ok::<_, Infallible>(service_fn(move |_: Request<Body>| {
                let field_data = field_data.clone();
                async move {
                    let opt_data = field_data.lock().unwrap();

                    if let Some(data) = *opt_data {
                        info!("Temperature {}°C", data.temperature_celsius());
                        info!("Pressure {}hPa", data.pressure_hpa());
                        info!("Humidity {}%", data.humidity_percent());
                        info!("Gas Resistence {}Ω", data.gas_resistance_ohm());
                    } else {
                        warn!("No data");
                    }
                    Ok::<_, Infallible>(
                        Response::new(Body::from(format!("Hello, {}!", remote_addr)))
                    )
                }
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);

    let field_data = field_data.clone();
    thread::spawn(move || {
        loop {
            delay.delay_ms(5000u32);
            info!("Retrieving sensor data");
            dev.set_sensor_mode(PowerMode::ForcedMode).unwrap();
            let (data, _state) = dev.get_sensor_data().unwrap();
            info!("Sensor Data {:?}", data);
            info!("Temperature {}°C", data.temperature_celsius());
            info!("Pressure {}hPa", data.pressure_hpa());
            info!("Humidity {}%", data.humidity_percent());
            info!("Gas Resistence {}Ω", data.gas_resistance_ohm());

            let mut opt_data = field_data.lock().unwrap();

            *opt_data = Some(data)
        }
    });

    // Run this server for... forever!
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }

    Ok(())
}
