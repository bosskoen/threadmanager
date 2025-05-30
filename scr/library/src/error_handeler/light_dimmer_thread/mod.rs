use chrono::{DateTime, Local, Timelike};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use crate::Status;

use super::{
    ErrorOperation, Printer, LED_DAY_BRIGHTNESS, LED_NIGHT_BRIGHTNESS, RGB, TIME_TO_BRIGHTEN,
    TIME_TO_DIM,
};

pub const PLUGIN_NAME: &str = "light_dimmer";
const STATUS_INITIALIZE_ERROR: i32 = 108;
const TIME_TO_SLEEP: Duration = Duration::from_secs(5);
const ERROR_THREAD_DOWN: i32 = 105;

mod light_dimmer_types {
    use crate::{error_handeler::format_duration, impl_status};

    use super::*;

    pub struct LightDimmerStatus {
        pub current_brightness: u8,
        pub start_time: DateTime<Local>,
    }
    impl_status!(LightDimmerStatus, |status: &LightDimmerStatus| {
        format!(
            "Current Brightness: {}, Start Time: {}\nRunning for {}",
            status.current_brightness,
            status.start_time.format("%Y %m-%d; %H:%M:%S"),
            format_duration(status.start_time, Local::now())
        )
    });
}
use light_dimmer_types::*;

fn init_status(status: &Arc<Mutex<Box<dyn Status>>>, printer: &mut Printer) {
    if let Ok(mut stat) = status.lock() {
        let new_status = LightDimmerStatus {
            current_brightness: 0,
            start_time: Local::now(),
        };
        (*stat) = Box::new(new_status);
    } else {
        printer.print_error(
            PLUGIN_NAME,
            "failed to initiolanise the status",
            RGB::CRITICAL_ERROR(),
        );
        printer.print(
            &format!("exited with exit code {}", STATUS_INITIALIZE_ERROR),
            RGB::WHITE(),
        );
        Printer::close_program();
    }
}

pub fn start_light_dim(
    mut printer: Printer,
    stopflag: Arc<AtomicBool>,
    status: Arc<Mutex<Box<dyn Status>>>,
) {
    init_status(&status, &mut printer);

    let mut last_updated = Local::now();

    loop {
        let start_of_loop = Local::now();
        if stopflag.load(Ordering::Relaxed) {
            break;
        }
        if start_of_loop.hour() >= TIME_TO_DIM && last_updated.hour() < TIME_TO_DIM {
            if let Err(_) = printer.send(
                ErrorOperation::CangeBrighness(LED_NIGHT_BRIGHTNESS, super::LedNumber::ALL),
                PLUGIN_NAME,
            ) {
                printer.print(
                    &format!("exited with exit code {}", ERROR_THREAD_DOWN),
                    RGB::WHITE(),
                );
                Printer::close_program();
            }
        } else if start_of_loop.hour() >= TIME_TO_BRIGHTEN && last_updated.hour() < TIME_TO_BRIGHTEN
        {
            if let Err(_) = printer.send(
                ErrorOperation::CangeBrighness(LED_DAY_BRIGHTNESS, super::LedNumber::ALL),
                PLUGIN_NAME,
            ) {
                printer.print(
                    &format!("exited with exit code {}", ERROR_THREAD_DOWN),
                    RGB::WHITE(),
                );
                Printer::close_program();
            }
        }
        last_updated = start_of_loop;
        thread::sleep(TIME_TO_SLEEP);
    }
}
