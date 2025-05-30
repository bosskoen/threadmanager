#[cfg(feature = "led")]
mod led_controller;
#[cfg(feature = "led")]
use led_controller::{change_led_color,reset_color_led, color_on, color_off, change_led_brightness};
#[cfg(feature = "led")]
use chrono::Timelike;

mod rgb;

pub mod light_dimmer_thread;
mod printer;

pub use printer::{Printer, cleanup_static};

use std::{sync::{mpsc::Receiver, Arc, Mutex}};
use chrono::{DateTime, Local};
use crate::{format_duration, Status};

pub use rgb::RGB;

pub const LED_NIGHT_BRIGHTNESS:u8 = 5;
pub const LED_DAY_BRIGHTNESS: u8 = 14;

pub const TIME_TO_BRIGHTEN: u32 = 7; // in hours
pub const TIME_TO_DIM: u32= 21; // in hours

const INITIOLIZE_STATUS_ERROR:i32 = 100;
const ERROR_STATUS_LOCK_FAILED:i32 = 101;
const ERROR_STATUS_NOT_ERROR_STATUS:i32 = 102;


enum ChangeColor {
    Yes(RGB),
    No
}

struct ErrorStatus {
    errors: usize,
    color: RGB,
    start_time: DateTime<Local>
}

impl Status for ErrorStatus {
    fn format(&self) -> String {
        format!(
            "Error thread processed {} errors.\nLed color is now {}.\nThread started at {} and is now running for {}", 
            self.errors, self.color.to_hex(), 
            self.start_time.format("%Y %m-%d; %H:%M:%S"), 
            format_duration(self.start_time, Local::now())
        )
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl ErrorStatus {
    fn new() -> Self {
        Self {
            errors: 0,
            color: RGB::GREEN(),
            start_time: Local::now()
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum LedOption {
    Red,
    Green,
    Blue,
    All
}
impl std::fmt::Display for LedOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LedOption::Red => write!(f, "Red"),
            LedOption::Green => write!(f, "Green"),
            LedOption::Blue => write!(f, "Blue"),
            LedOption::All => write!(f, "All"),
        }
    }
    
}
pub enum PWMOption {
    On,
    Off
}
#[derive(Clone, Copy ,PartialEq)]
#[allow(dead_code)]
#[repr(u8)] 
pub enum LedNumber{
    LED1 = 0,
    LED2 = 1,
    LED3 = 2,
    LED4 = 3,
    LED5 = 4,
    ALL = 5,
}

impl std::fmt::Display for LedNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LedNumber::LED1 => write!(f, "LED 1"),
            LedNumber::LED2 => write!(f, "LED 2"),
            LedNumber::LED3 => write!(f, "LED 3"),
            LedNumber::LED4 => write!(f, "LED 4"),
            LedNumber::LED5 => write!(f, "LED 5"),
            LedNumber::ALL => write!(f, "ALL LEDs"),
        }
    }
}
impl std::ops::Mul<u8> for LedNumber{
    type Output = u8;

    fn mul(self, rhs: u8) -> Self::Output {
        self as u8 * rhs
    }
}

impl From<u8> for LedNumber{
    fn from(value: u8) -> Self {
        match value {
            0 => LedNumber::LED1,
            1 => LedNumber::LED2,
            2 => LedNumber::LED3,
            3 => LedNumber::LED4,
            4 => LedNumber::LED5,
            _ => panic!("Invalid LED number"),
        }
    }
}
impl From<usize> for LedNumber{
    fn from(value: usize) -> Self {
        match value {
            0 => LedNumber::LED1,
            1 => LedNumber::LED2,
            2 => LedNumber::LED3,
            3 => LedNumber::LED4,
            4 => LedNumber::LED5,
            _ => panic!("Invalid LED number"),
        }
    }
}

pub enum ErrorOperation {
    /// the first string is the plugin name, the second is the message, the third is the text color
    PrintError(String, String, RGB),
    /// the bool is to indicate if the color change is an error or not (true = error, false = non error)
    ChangeLed(RGB, bool, LedNumber),
    /// the first string is the plugin name, the second is the message, the third is the text color, the fourth is the led color and the fifth is the led number
    NonErrorPrintAndChangeLed(String, String, RGB, RGB,LedNumber),
    /// the first string is the plugin name, the second is the message, the third is the text color, the fourth is the led color and the fifth is the led number
    PrintAndChangeLedError(String, String, RGB, RGB, LedNumber),
    /// the first string is the plugin name, the second is the message, the third is the text color
    NonErrorPrint(String, String, RGB),
    CangeBrighness(u8, LedNumber),
    ///reset to color to use the pwm signal to be dimed again, undoes the OnColor and OffColor functions
    RestColor(LedOption, LedNumber),
    ///set the color to not folow the pwm signal and will turn full off
    OffColor(LedOption, LedNumber),
    /// set the color to not folow the pwm signal and will turn full on
    OnColor(LedOption, LedNumber),
    /// disables or enables the pwm controller for the leds
    PWM(PWMOption),
    StopErr
}


pub fn error_catchloop(receiver: Receiver<ErrorOperation>, mut printer: Printer, status: Arc<Mutex<Box<dyn Status>>>) {
    initialize_status(&status, &mut printer);

    #[cfg(feature = "led")]
    let mut led_controler = {
        let now = Local::now();
        if now.hour() >= TIME_TO_DIM || now.hour() < TIME_TO_BRIGHTEN {
            led_controller::led::LedController::new([RGB::GREEN(), RGB::BLACK(), RGB::BLACK(), RGB::BLACK(), RGB::BLACK()], [LED_NIGHT_BRIGHTNESS; 5])
        } else {
            led_controller::led::LedController::new([RGB::GREEN(), RGB::BLACK(), RGB::BLACK(), RGB::BLACK(), RGB::BLACK()], [LED_DAY_BRIGHTNESS; 5])
        }
    }.unwrap_or_else(|err| {
        printer.print_error("errorThread",&format!("couldn't initialize led controler: \n {}", err), RGB::CRITICAL_ERROR());
        led_controller::led::LedController::dummy()
    });

    for error in receiver.iter() {
        match error {
            ErrorOperation::PrintError(plugin, message, color) => {
                        printer.print_error(&plugin, &message, color);
                        update_status_error(&status, ChangeColor::No, &mut printer);
                    },
            ErrorOperation::ChangeLed(rgb, is_error, _led) => {
                        #[cfg(feature = "led")]
                        change_led_color(&mut led_controler, rgb, _led).unwrap_or_else(|err| {
                            printer.print_error(
                                "errorThread",
                                &format!("Failed to change {} color to {:?}: {}", _led,rgb, err),
                                RGB::CRITICAL_ERROR(),
                            );
                            ()}
                        );

                        update_status(&status, ChangeColor::Yes(rgb), is_error, &mut printer);
                    },
            ErrorOperation::PrintAndChangeLedError(plugin, message, color, rgb,_led) => {

                        #[cfg(feature = "led")]
                        change_led_color(&mut led_controler, rgb,_led).unwrap_or_else(|err| {
                            printer.print_error(
                                "errorThread",
                                &format!("Plugin '{}' failed to change {} color to {:?}: {}", plugin, _led,rgb, err),
                                RGB::CRITICAL_ERROR(),
                            );
                            ()}
                        );

                        printer.print_error(&plugin, &message, color);
                        update_status_error(&status, ChangeColor::Yes(rgb), &mut printer);
                    },
            ErrorOperation::StopErr => break,
            ErrorOperation::NonErrorPrint(plugin, message, rgb) => {
                        printer.named_print(&plugin, &message, rgb);
                    },
            ErrorOperation::CangeBrighness(_new_brightness, _led) => {
                        #[cfg(feature = "led")]
                        change_led_brightness(&mut led_controler, _new_brightness, _led).unwrap_or_else(|err| {
                            printer.print_error(
                                "errorThread",
                                &format!("Failed to change {} brightness to {}: {}", _led,_new_brightness, err),
                                RGB::CRITICAL_ERROR(),
                            );
                            ()
                        });
                    },
            ErrorOperation::RestColor(_led_option,_led) => {
                        #[cfg(feature = "led")]
                        reset_color_led(&mut led_controler, _led_option,_led).unwrap_or_else(|err| {
                           printer.print_error("errorThread", &format!("Failed to reset {} {} : {}", _led_option,_led,err), RGB::CRITICAL_ERROR());
                        });
                    },
            ErrorOperation::OffColor(_led_option,_led) => {
                        #[cfg(feature = "led")]
                        color_off(&mut led_controler, _led_option,_led).unwrap_or_else(|err| {
                            printer.print_error("errorThread", &format!("Failed to turn OFF {} {} : {}", _led_option,_led, err), RGB::CRITICAL_ERROR());
                        });
                    },
            ErrorOperation::OnColor(_led_option,_led) => {
                        #[cfg(feature = "led")]
                        color_on(&mut led_controler, _led_option,_led).unwrap_or_else(|err| {
                            printer.print_error("errorThread", &format!("Failed to turn ON {} {} : {}", _led_option,_led, err), RGB::CRITICAL_ERROR());
                        });
                    },
            ErrorOperation::PWM(_pwmoption) => {
                        #[cfg(feature = "led")]
                        if let PWMOption::Off = _pwmoption{
                            led_controler.off();
                        }else{
                            led_controler.on();
                        }
                    },
            ErrorOperation::NonErrorPrintAndChangeLed(plugin, message, color, rgb, _led_number) => {
                #[cfg(feature = "led")]
                change_led_color(&mut led_controler, rgb,_led_number).unwrap_or_else(|err| {
                    printer.print_error(
                        "errorThread",
                        &format!("Plugin '{}' failed to change {} color to {:?}: {}", plugin, _led_number,rgb, err),
                        RGB::CRITICAL_ERROR(),
                    );
                    ()}
                );

                printer.named_print(&plugin, &message, color);
                update_status(&status, ChangeColor::Yes(rgb), false, &mut printer);
            },
        }
    }
}

fn initialize_status(status: &Arc<Mutex<Box<dyn Status>>>, printer: &mut Printer) {
    match status.lock() {
        Ok(mut stat) => {
            *stat = Box::new(ErrorStatus::new());
        },
        Err(_) => {
            printer.print_error("errorThread","couldn't lock error status", RGB::CRITICAL_ERROR());
            printer.print(
                &format!("exited with exit code {}", INITIOLIZE_STATUS_ERROR),
                RGB::WHITE(),
            );
            Printer::close_program();
        }
        
    }
}


fn update_status(status: &Arc<Mutex<Box<dyn Status>>>, new_color: ChangeColor, is_error: bool, printer: &mut Printer) {

    match status.lock() {
        Ok(mut stat) => {
            if let Some(status) = (*stat).as_any_mut().downcast_mut::<ErrorStatus>() {
                if is_error {
                    status.errors += 1;
                }
                if let ChangeColor::Yes(rgb) = new_color {
                    status.color = rgb;
                }
            } else {
                printer.print_error("errorThread","Status isn't of type ErrorStatus", RGB::CRITICAL_ERROR()); 
                printer.print(
                    &format!("exited with exit code {}", ERROR_STATUS_NOT_ERROR_STATUS),
                    RGB::WHITE(),
                );
                Printer::close_program();
            }
        },
        Err(_) => {
            printer.print_error("errorThread","couldn't lock error status", RGB::CRITICAL_ERROR());
            printer.print(
                &format!("exited with exit code {}", ERROR_STATUS_LOCK_FAILED),
                RGB::WHITE(),
            );
            Printer::close_program();
        }
    }
}

fn update_status_error(status: &Arc<Mutex<Box<dyn Status>>>, new_color: ChangeColor, printer: &mut Printer) {
    update_status(status, new_color, true, printer);
}

