#[cfg(feature = "led")]
mod led_controller;

mod rgb;

use std::{io::{self, IsTerminal, Write}, process::exit, sync::{mpsc::Receiver, Arc, Mutex}, time::Duration};
use chrono::{DateTime, Local};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use crate::{format_duration, Status};

pub use rgb::RGB;

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

pub enum ErrorOperation {
    Print(String, String, RGB),
    ChangeLed(RGB),
    BlinkLed(RGB, Duration),
    PrintAndChangeLed(String, String, RGB, RGB),
    PrintAndBlinkLed(String, String, RGB, RGB, Duration),
    NonErrorPrint(String, String, RGB),
    StopErr
}

pub fn error_catchloop(receiver: Receiver<ErrorOperation>, status: Arc<Mutex<Box<dyn Status>>>) {
    initialize_status(&status);

    // TODO: set led to RGB::GREEN()

    for error in receiver.iter() {
        match error {
            ErrorOperation::Print(plugin, message, color) => {
                print_error(&plugin, &message, color);
                update_status(&status, ChangeColor::No);
            },
            ErrorOperation::ChangeLed(rgb) => {
                // TODO: change led
                update_status(&status, ChangeColor::Yes(rgb));
            },
            ErrorOperation::BlinkLed(rgb, _) => {
                // TODO: blink led
                update_status(&status, ChangeColor::Yes(rgb));
            },
            ErrorOperation::PrintAndChangeLed(plugin, message, color, rgb) => {
                // TODO: change led
                print_error(&plugin, &message, color);
                update_status(&status, ChangeColor::Yes(rgb));
            },
            ErrorOperation::PrintAndBlinkLed(plugin, message, color, rgb, time) => {
                // TODO: blink led
                print_error(&plugin, &message, color);
                update_status(&status, ChangeColor::Yes(rgb));
            },
            ErrorOperation::StopErr => break,
            ErrorOperation::NonErrorPrint(plugin, message, rgb) => {
                print_interup(&plugin, &message, rgb);
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

fn update_status(status: &Arc<Mutex<Box<dyn Status>>>, new_color: ChangeColor) {
    if let Some(status) = (*status.lock().unwrap_or_else(|_| {
        print_error("errorThread","couldn't lock error status", RGB::CRITICAL_ERROR());
        exit(ERROR_STATUS_LOCK_FAILED);
    })).as_any_mut().downcast_mut::<ErrorStatus>() {
        status.errors += 1;
        if let ChangeColor::Yes(rgb) = new_color {
            status.color = rgb;
        }
    } else {
        print_error("errorThread","Status isn't of type ErrorStatus", RGB::CRITICAL_ERROR()); 
        exit(ERROR_STATUS_NOT_ERROR_STATUS);
    }
}

#[cfg(unix)]
fn stdout_is_same_as_stderr() -> bool {
    use std::os::unix::io::{AsRawFd, RawFd};
    use std::io::{stdout, stderr};

    let stdout_fd: RawFd = stdout().as_raw_fd();
    let stderr_fd: RawFd = stderr().as_raw_fd();
    stdout_fd == stderr_fd
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