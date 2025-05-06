use std::{process::exit, sync::{atomic::{AtomicBool, Ordering}, mpsc::Sender, Arc, Mutex}, thread, time::Duration};
use chrono::{DateTime, Local, Timelike};

use crate::Status;

use super::{print_error, ErrorOperation, LED_DAY_BRIGHTNESS, LED_NIGHT_BRIGHTNESS, RGB, TIME_TO_BRIGHTEN, TIME_TO_DIM};

pub const PLUGIN_NAME: &str = "light_dimmer";
const STATUS_INITIALIZE_ERROR : i32 = 108;
const TIME_TO_SLEEP: Duration = Duration::from_secs(5);
const ERROR_THREAD_DOWN: i32 = 105;

mod light_dimmer_types{
    use crate::{error_handeler::format_duration, impl_status};

    use super::*;

    pub struct LightDimmerStatus {
        pub current_brightness: u8,
        pub start_time: DateTime<Local>,
    }
    impl_status!(LightDimmerStatus, | status: &LightDimmerStatus | {
            format!("Current Brightness: {}, Start Time: {}\nRunning for {}", status.current_brightness, status.start_time.format("%Y %m-%d; %H:%M:%S"), format_duration(status.start_time, Local::now()))
        }
    );

}
use light_dimmer_types::*;

fn init_status(status: &Arc<Mutex<Box<dyn Status>>>) {
    if let Ok(mut stat) = status.lock() {
        let new_status = LightDimmerStatus {
            current_brightness: 0,
            start_time: Local::now(),
        };
        (*stat) = Box::new(new_status);
    }else{
        print_error(PLUGIN_NAME, "failed to initiolanise the status", RGB::CRITICAL_ERROR());
        exit(STATUS_INITIALIZE_ERROR);
    }
}

pub fn start_light_dim(error_handel: Sender<ErrorOperation>, stopflag: Arc<AtomicBool>, status: Arc<Mutex<Box<dyn Status>>>){
    init_status(&status);

    let mut last_updated = Local::now();

    loop{
        let start_of_loop = Local::now();
        if stopflag.load(Ordering::Relaxed) {
            break;
        }
        if start_of_loop.hour() >= TIME_TO_DIM && last_updated.hour() < TIME_TO_DIM{
            if let Err(_) = error_handel.send(ErrorOperation::CangeBrighness(LED_NIGHT_BRIGHTNESS)){
                print_error(PLUGIN_NAME, "Failed to send mesige to error thread", RGB::CRITICAL_ERROR());
                exit(ERROR_THREAD_DOWN);
            }
        }
        else if start_of_loop.hour() >= TIME_TO_BRIGHTEN && last_updated.hour() < TIME_TO_BRIGHTEN{
            if let Err(_) = error_handel.send(ErrorOperation::CangeBrighness(LED_DAY_BRIGHTNESS)){
                print_error(PLUGIN_NAME, "Failed to send mesige to error thread", RGB::CRITICAL_ERROR());
                exit(ERROR_THREAD_DOWN);
            }
        } 
        last_updated = start_of_loop;
        thread::sleep(TIME_TO_SLEEP);
    }
}