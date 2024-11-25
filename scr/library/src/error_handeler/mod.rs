mod led_controller;

use std::{process::exit, sync::{mpsc::Receiver, Arc, Mutex}};
use chrono::{DateTime, Local};
use crate::{format_duration, Status};

pub use led_controller::RGB;

enum ChangeColor {
    Yes(RGB),
    No
}
struct ErrorStatus{
    errors: usize,
    color: RGB,
    start_time: DateTime<Local>
}
impl Status for ErrorStatus {
    fn format(&self) -> String{
        format!(
            "Error thread prossesed {} errors.\n
            Led color is now {}.\n
            thread started at {} and is now running for {}", 
            self.errors, self.color.to_hex(), 
            self.start_time.format("%Y %m-%d; %H:%M:%S"), 
           format_duration(self.start_time, Local::now()))
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
impl ErrorStatus {
    fn new() -> Self{
        Self{
            errors: 0,
            color: RGB::from_hex(0x28db09),
            start_time: Local::now()
        }
    }
}

pub enum ErrorOperation {
    Print(String),
    ChangLed(RGB),
    BlickLed(RGB, f32),
    PrintAndChangeLed(String, RGB),
    PrintAndBlinkLed(String, RGB, f32),
    StopErr
}

pub fn error_catchloop(resever:Receiver<ErrorOperation>,status: Arc<Mutex<Box<dyn Status>>>){
    *(status.lock().unwrap_or_else(|_| {
        eprintln!("couldn't initialise error status"); 
        exit(100)})
    ) = Box::new(ErrorStatus::new());

    //TODO set led to 0x28db09

    for error in resever.iter(){
        match error {
            ErrorOperation::Print(mesiges) => {
                eprintln!("{mesiges}");
                update_status(&status, ChangeColor::No);
            },
            ErrorOperation::ChangLed(rgb) => {
                //TODO cangled
                update_status(&status, ChangeColor::Yes(rgb));
            },
            ErrorOperation::BlickLed(rgb, _) => {
                //TODO cangled
                update_status(&status, ChangeColor::Yes(rgb));
            }
            ErrorOperation::PrintAndChangeLed(mesiges, rgb) => {
                //TODO cangled
                eprintln!("{mesiges}");
                update_status(&status, ChangeColor::Yes(rgb));
            },
            ErrorOperation::PrintAndBlinkLed(mesiges, rgb, _) => {
                //TODO cangled
                eprintln!("{mesiges}");
                update_status(&status, ChangeColor::Yes(rgb));
            },
            ErrorOperation::StopErr => break
        }
    }
}

fn update_status(status: &Arc<Mutex<Box<dyn Status>>>,new_color: ChangeColor){

    if let Some(status) = (*status.lock().unwrap_or_else(|_| {
        eprintln!("couldn't initialize error status");
        exit(101);
    })).as_any_mut().downcast_mut::<ErrorStatus>(){
        status.errors += 1;
        if let ChangeColor::Yes(rgb) = new_color {
            status.color = rgb;
        }
    }else{
        eprintln!("Status isn't of type ErrorStarus");
        exit(102);
    }
}