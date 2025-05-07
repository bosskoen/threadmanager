#[cfg(feature = "led")]
mod led_controller;
#[cfg(feature = "led")]
use led_controller::{change_led_color,reset_color_led, color_on, color_off, change_led_brightness};
#[cfg(feature = "led")]
use chrono::Timelike;


mod rgb;

pub mod light_dimmer_thread;

use std::{io::{self, IsTerminal, Write}, process::exit, sync::{mpsc::Receiver, Arc, Mutex}};
use chrono::{DateTime, Local};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use crate::{format_duration, Status};

pub use rgb::RGB;

pub const LED_NIGHT_BRIGHTNESS:u8 = 5;
pub const LED_DAY_BRIGHTNESS: u8 = 14;

pub const TIME_TO_BRIGHTEN: u32 = 7; // in hours
pub const TIME_TO_DIM: u32= 21; // in hours

const INITIOLIZE_STATUS_ERROR:i32 = 100;
const ERROR_STATUS_LOCK_FAILED:i32 = 101;
const ERROR_STATUS_NOT_ERROR_STATUS:i32 = 102;

lazy_static!{
    static ref VARIABLES: Mutex<Variables> = Mutex::new(Variables::new());
}

struct Variables {
    stdout_is_same_as_stderr: bool,
    stdout_color: bool,
    stderr_color: bool,
    stdout: StandardStream,
    stderr: StandardStream
}

impl Variables {
    fn new() -> Self {
        let stdout_is_same_as_stderr = stdout_is_same_as_stderr();
        let stdout_color = io::stdout().is_terminal();
        let stderr_color = io::stderr().is_terminal();
        let stdout = StandardStream::stdout(ColorChoice::Auto);
        let stderr = StandardStream::stderr(ColorChoice::Auto);

        Self {
            stdout_is_same_as_stderr,
            stdout_color,
            stderr_color,
            stdout,
            stderr
        }
    }
}

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
    Print(String, String, RGB),
    /// the bool is to indicate if the color change is an error or not (true = error, false = non error)
    ChangeLed(RGB, bool, LedNumber),
    /// the first string is the plugin name, the second is the message, the third is the text color, the fourth is the led color and the fifth is the led number
    PrintAndChangeLed(String, String, RGB, RGB,LedNumber),
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


pub fn error_catchloop(receiver: Receiver<ErrorOperation>, status: Arc<Mutex<Box<dyn Status>>>) {
    initialize_status(&status);

    #[cfg(feature = "led")]
    let mut led_controler = {
        let now = Local::now();
        if now.hour() >= TIME_TO_DIM || now.hour() < TIME_TO_BRIGHTEN {
            led_controller::led::LedController::new([RGB::GREEN(), RGB::BLACK(), RGB::BLACK(), RGB::BLACK(), RGB::BLACK()], [LED_NIGHT_BRIGHTNESS; 5])
        } else {
            led_controller::led::LedController::new([RGB::GREEN(), RGB::BLACK(), RGB::BLACK(), RGB::BLACK(), RGB::BLACK()], [LED_DAY_BRIGHTNESS; 5])
        }
    }.unwrap_or_else(|err| {
        print_error("errorThread",&format!("couldn't initialize led controler: \n {}", err), RGB::CRITICAL_ERROR());
        led_controller::led::LedController::dummy()
    });

    for error in receiver.iter() {
        match error {
            ErrorOperation::Print(plugin, message, color) => {
                        print_error(&plugin, &message, color);
                        update_status_error(&status, ChangeColor::No);
                    },
            ErrorOperation::ChangeLed(rgb, is_error, led) => {
                        #[cfg(feature = "led")]
                        change_led_color(&mut led_controler, rgb, led).unwrap_or_else(|err| {
                            print_error(
                                "errorThread",
                                &format!("Failed to change {} color to {:?}: {}", led,rgb, err),
                                RGB::CRITICAL_ERROR(),
                            );
                            ()}
                        );

                        update_status(&status, ChangeColor::Yes(rgb), is_error);
                    },
            ErrorOperation::PrintAndChangeLedError(plugin, message, color, rgb,led) => {

                        #[cfg(feature = "led")]
                        change_led_color(&mut led_controler, rgb,led).unwrap_or_else(|err| {
                            print_error(
                                "errorThread",
                                &format!("Plugin '{}' failed to change {} color to {:?}: {}", plugin, led,rgb, err),
                                RGB::CRITICAL_ERROR(),
                            );
                            ()}
                        );

                        print_error(&plugin, &message, color);
                        update_status_error(&status, ChangeColor::Yes(rgb));
                    },
            ErrorOperation::StopErr => break,
            ErrorOperation::NonErrorPrint(plugin, message, rgb) => {
                        print_interup(&plugin, &message, rgb);
                    },
            ErrorOperation::CangeBrighness(new_brightness, led) => {
                        #[cfg(feature = "led")]
                        change_led_brightness(&mut led_controler, new_brightness, led).unwrap_or_else(|err| {
                            print_error(
                                "errorThread",
                                &format!("Failed to change {} brightness to {}: {}", led,new_brightness, err),
                                RGB::CRITICAL_ERROR(),
                            );
                            ()
                        });
                    },
            ErrorOperation::RestColor(led_option,led) => {
                        #[cfg(feature = "led")]
                        reset_color_led(&mut led_controler, led_option,led).unwrap_or_else(|err| {
                            print_error("errorThread", &format!("Failed to reset {} {} : {}", led_option,led,err), RGB::CRITICAL_ERROR());
                        });
                    },
            ErrorOperation::OffColor(led_option,led) => {
                        #[cfg(feature = "led")]
                        color_off(&mut led_controler, led_option,led).unwrap_or_else(|err| {
                            print_error("errorThread", &format!("Failed to turn OFF {} {} : {}", led_option,led, err), RGB::CRITICAL_ERROR());
                        });
                    },
            ErrorOperation::OnColor(led_option,led) => {
                        #[cfg(feature = "led")]
                        color_on(&mut led_controler, led_option,led).unwrap_or_else(|err| {
                            print_error("errorThread", &format!("Failed to turn ON {} {} : {}", led_option,led, err), RGB::CRITICAL_ERROR());
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
            ErrorOperation::PrintAndChangeLed(plugin, message, color, rgb, led_number) => {
                #[cfg(feature = "led")]
                change_led_color(&mut led_controler, rgb,led_number).unwrap_or_else(|err| {
                    print_error(
                        "errorThread",
                        &format!("Plugin '{}' failed to change {} color to {:?}: {}", plugin, led_number,rgb, err),
                        RGB::CRITICAL_ERROR(),
                    );
                    ()}
                );

                print_error(&plugin, &message, color);
                update_status(&status, ChangeColor::Yes(rgb), false);
            },
        }
    }
}

fn initialize_status(status: &Arc<Mutex<Box<dyn Status>>>) {
    *(status.lock().unwrap_or_else(|_| {
        print_error("errorThread","couldn't initialize error status", RGB::CRITICAL_ERROR());
        exit(INITIOLIZE_STATUS_ERROR); 
    })) = Box::new(ErrorStatus::new());
}

fn print_message(stream: &mut StandardStream, message: &str, color: RGB) {
    let (r, g, b) = color.to_tuple();
    let mut color_spec = ColorSpec::new();
    color_spec.set_fg(Some(Color::Rgb(r, g, b)));

    if let Err(err) = stream.set_color(&color_spec) {
        eprintln!("Failed to set color: {}", err);
    }
    if let Err(err) = writeln!(stream, "{}", message) {
        eprintln!("Failed to write to stream: {}", err);
    }
    if let Err(err) = stream.set_color(ColorSpec::new().set_fg(Some(Color::Rgb(255,255,255)))) {
        eprintln!("Failed to reset stream: {}", err);
    }
    if let Err(err) = stream.flush() {
        eprintln!("Failed to flush stream: {}", err);
    }
}

pub fn reset_color() {
    let mut config = if let Ok(config) = VARIABLES.lock() {
        config
    } else {
        eprintln!("couldn't lock print variables");
        return;
    };

    if let Err(err) = config.stdout.reset(){
        eprintln!("Failed to reset stdout: {}", err);
    }
    if let Err(err) = config.stderr.reset(){
        eprintln!("Failed to reset stderr: {}", err);
    }
}

pub fn print_error(plugin: &str, message: &str, color: RGB) {
    let mut config = if let Ok(config) = VARIABLES.lock() {
        config
    } else {
        eprintln!("couldn't lock print variables");
        return;
    };

    let formatted_message = format!("\n{} in {}", message, plugin);

    if config.stdout_is_same_as_stderr {
        if config.stdout_color {
            print_message(&mut config.stderr, &formatted_message, color);
            print!("> ");
            if let Err(err) = io::stdout().flush() {
                eprintln!("Failed to flush stdout: {}", err);
            }
        } else {
            eprint!("{formatted_message}");
            print!("> ");
            if let Err(err) = io::stdout().flush() {
                eprintln!("Failed to flush stdout: {}", err);
            }
        }
    } else {
        if config.stderr_color {
            print_message(&mut config.stderr, &formatted_message, color);
        } else {
            eprint!("{} in {}", message, plugin);
        }
    }
}

/// Prints a message that wil end the message with a newline "> " so the user can see that the program is waiting for input 
pub fn print_interup(plugin: &str, message: &str, rgb: RGB) {
    let mut config = if let Ok(config) = VARIABLES.lock() {
        config
    } else {
        eprintln!("couldn't lock print variables");
        return;
    };

    let formatted_message = format!("\n{} from {}", message, plugin);

    if config.stdout_color {
        print_message(&mut config.stdout, &formatted_message, rgb);
    } else {
        println!("{} from {}", message, plugin);
    }
    print!("> ");
    if let Err(err) = io::stdout().flush() {
        eprint!("Failed to flush stdout: {}", err);
    }
}

pub fn print(message: &str, rgb: RGB) {
    let mut config = if let Ok(config) = VARIABLES.lock() {
        config
    } else {
        eprintln!("couldn't lock print variables");
        return;
    };

    if config.stdout_color {
        print_message(&mut config.stdout, message, rgb);
    } else {
        println!("{}", message);
    }
}

fn update_status(status: &Arc<Mutex<Box<dyn Status>>>, new_color: ChangeColor, is_error: bool) {
    if let Some(status) = (*status.lock().unwrap_or_else(|_| {
        print_error("errorThread","couldn't lock error status", RGB::CRITICAL_ERROR());
        exit(ERROR_STATUS_LOCK_FAILED);
    })).as_any_mut().downcast_mut::<ErrorStatus>() {
        if is_error{
        status.errors += 1;
        }
        if let ChangeColor::Yes(rgb) = new_color {
            status.color = rgb;
        }
    } else {
        print_error("errorThread","Status isn't of type ErrorStatus", RGB::CRITICAL_ERROR()); 
        exit(ERROR_STATUS_NOT_ERROR_STATUS);
    }
}

fn update_status_error(status: &Arc<Mutex<Box<dyn Status>>>, new_color: ChangeColor){
    update_status(status, new_color, true);
}

#[cfg(unix)]
fn stdout_is_same_as_stderr() -> bool {
    use std::os::unix::io::AsRawFd;
    use std::mem::MaybeUninit;
    use std::io::{stdout, stderr};
    use libc::{fstat, stat};

    unsafe {
        let mut stat_out = MaybeUninit::<stat>::uninit();
        let mut stat_err = MaybeUninit::<stat>::uninit();

        let stdout_fd = stdout().as_raw_fd();
        let stderr_fd = stderr().as_raw_fd();

        if fstat(stdout_fd, stat_out.as_mut_ptr()) != 0 {
            return false;
        }
        if fstat(stderr_fd, stat_err.as_mut_ptr()) != 0 {
            return false;
        }

        let stat_out = stat_out.assume_init();
        let stat_err = stat_err.assume_init();

        // Compare device and inode
        stat_out.st_dev == stat_err.st_dev && stat_out.st_ino == stat_err.st_ino
    }
}

#[cfg(windows)]
fn stdout_is_same_as_stderr() -> bool {
    use std::os::windows::io::{AsRawHandle, RawHandle};
    use winapi::um::fileapi::GetFileType;
    use winapi::um::consoleapi::GetConsoleMode;
    use winapi::um::winbase::FILE_TYPE_CHAR;

    let stdout_handle: RawHandle = std::io::stdout().as_raw_handle();
    let stderr_handle: RawHandle = std::io::stderr().as_raw_handle();

    let mut stdout_mode: u32 = 0;

    if stdout_handle == stderr_handle {
        return true;
    } else {
        let stdout_type = unsafe { GetFileType(stdout_handle as _) };
        let stderr_type = unsafe { GetFileType(stderr_handle as _) };

        if stdout_type == FILE_TYPE_CHAR && stderr_type == FILE_TYPE_CHAR {
            let stdout_is_console = unsafe { GetConsoleMode(stdout_handle as _, &mut stdout_mode) != 0 };
            let stderr_is_console = unsafe { GetConsoleMode(stderr_handle as _, &mut stdout_mode) != 0 };

            return stdout_is_console && stderr_is_console;
        } else {
           return false;
        }
    }
}
