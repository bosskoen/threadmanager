mod led_controller;

use std::{error, io::{self, Write}, process::exit, sync::{mpsc::Receiver, Arc, Mutex}, time::Duration};
use chrono::{DateTime, Local};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
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
    Print(String, String, RGB),
    ChangLed(RGB),
    BlickLed(RGB, Duration),
    PrintAndChangeLed(String,String,RGB, RGB),
    PrintAndBlinkLed(String,String, RGB, RGB,Duration),
    NonErrorPrint(String,String, RGB),
    StopErr
}

pub fn error_catchloop(resever:Receiver<ErrorOperation>,status: Arc<Mutex<Box<dyn Status>>>) -> Result<(), Box<dyn error::Error>>{
    *(status.lock().unwrap_or_else(|_| {
        eprintln!("couldn't initialise error status"); 
        exit(100)})
    ) = Box::new(ErrorStatus::new());

    let stdout_is_same_as_stderr: bool =stdout_is_same_as_stderr(); 
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    //TODO set led to 0x28db09

    for error in resever.iter(){
        match error {
            ErrorOperation::Print(plugin,mesiges, color) => {
                print_error(&plugin, &mesiges, stdout_is_same_as_stderr, color, &mut stderr);
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
            ErrorOperation::PrintAndChangeLed(plugin,mesiges, color, rgb) => {
                //TODO cangled
                print_error(&plugin, &mesiges, stdout_is_same_as_stderr, color, &mut stderr);
                update_status(&status, ChangeColor::Yes(rgb));
            },
            ErrorOperation::PrintAndBlinkLed(plugin,mesiges, color ,rgb, time) => {
                //TODO cangled
                print_error(&plugin, &mesiges, stdout_is_same_as_stderr, color, &mut stderr);
                update_status(&status, ChangeColor::Yes(rgb));
            },
            ErrorOperation::StopErr => break,
            ErrorOperation::NonErrorPrint(plugin, messige, rgb) => {
                let (r, g, b) = rgb.to_tuple();

                let mut color_spec = ColorSpec::new();
                color_spec.set_fg(Some(Color::Rgb(r, g, b)));
                if let Err(err) = stdout.set_color(&color_spec) {
                    eprintln!("Failed to set color for stdout: {}", err);
                }
                if let Err(err) = writeln!(&mut stdout, "\n{} from {}", messige, plugin) {
                    eprintln!("Failed to write to stdout: {}", err);
                }
                if let Err(err) = stdout.reset() {
                    eprintln!("Failed to reset stdout: {}", err);
                }
                print!("> ");
                if let Err(err) = io::stdout().flush(){
                    eprint!("Failed to flush stdout: {}", err);
                }
            },
        }
    }
    Ok(())
}

fn print_error(plugin: &str, message: &str, out_is_equal: bool, color: RGB, stderr: &mut StandardStream){ 
    let (r, g, b) = color.to_tuple();

    let mut color_spec = ColorSpec::new();
    color_spec.set_fg(Some(Color::Rgb(r, g, b))); 

    if out_is_equal {
        stderr.set_color(&color_spec).unwrap();
        if let Err(err) = writeln!(stderr, "\n{} in {}", message, plugin) {
            eprintln!("Failed to write to stderr: {}", err);
        }
        if let Err(err) = stderr.reset() {
            eprintln!("Failed to reset stderr: {}", err);
        }

        print!("> ");
        if let Err(err) = io::stdout().flush() {
            eprintln!("Failed to flush stdout: {}", err);
        }
        } else {
        if let Err(err) = stderr.set_color(&color_spec) {
            eprintln!("Failed to set color for stderr: {}", err);
        }
        if let Err(err) = writeln!(stderr, "{} in {}", message, plugin) {
            eprintln!("Failed to write to stderr: {}", err);
        }
        if let Err(err) = stderr.reset() {
            eprintln!("Failed to reset stderr: {}", err);
        }
    }
}

fn update_status(status: &Arc<Mutex<Box<dyn Status>>>,new_color: ChangeColor){

    if let Some(status) = (*status.lock().unwrap_or_else(|_| {
        eprintln!("couldn't lock error status");
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

            if stdout_is_console && stderr_is_console {
                return true;
            } else {
                return false;
            }
        } else {
            return false;
        }
    }
}