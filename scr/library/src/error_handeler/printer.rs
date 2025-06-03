use std::{
    io::{stderr, stdout, Write},
    sync::{atomic::AtomicBool, mpsc::Sender, LazyLock, Mutex},
    thread::JoinHandle,
};

use rustyline::ExternalPrinter;

use super::{ErrorOperation, RGB};

lazy_static! {
    static ref VARIABLES: Mutex<Variables> = Mutex::new(Variables::new());
}

static FORSED_SHUT_DOWN: LazyLock<AtomicBool> = LazyLock::new(|| AtomicBool::new(false));

struct Variables {
    stdout_is_same_as_stderr: bool,
    stdout_color: bool,
    stderr_color: bool,
    print_thread: Option<PrintThread>,
}

struct PrintThread {
    thread: JoinHandle<()>,
    sender: Sender<PrintOperation>,
}

impl Variables {
    fn new() -> Self {
        let stdout_is_same_as_stderr = stdout_is_same_as_stderr();
        let stdout_color = atty::is(atty::Stream::Stdout);
        let stderr_color = atty::is(atty::Stream::Stderr);

        Self {
            stdout_is_same_as_stderr,
            stdout_color,
            stderr_color,
            print_thread: None,
        }
    }

    fn set_print_thread(&mut self, thread: PrintThread) {
        if let Some(old_thread) = self.print_thread.take() {
            drop(old_thread);
        }
        self.print_thread = Some(thread);
    }
}

pub fn cleanup_static() {
    if let Ok(mut variables) = VARIABLES.lock() {
        if let Some(thread) = variables.print_thread.take() {
            drop(thread);
        }
    } else {
        eprintln!("couldn't lock print variables");
    }
}

impl Drop for PrintThread {
    fn drop(&mut self) {
        if let Err(e) = self.sender.send(PrintOperation::Close) {
            eprintln!("Failed to send close operation: {:?}", e);
        }
        let thread = std::mem::replace(&mut self.thread, std::thread::spawn(|| {}));
        if let Err(e) = thread.join() {
            eprintln!("Failed to join print thread: {:?}", e);
        }
    }
}

enum PrintOperation {
    Print(String),
    Close,
}

pub struct Printer {
    printer: Sender<PrintOperation>,
    sender: Sender<ErrorOperation>,
}

impl Printer {
    pub fn new<T: ExternalPrinter + Send + Sync + 'static>(
        mut printer: T,
        sender: Sender<ErrorOperation>,
    ) -> Self {
        let (print_tx, print_rx) = std::sync::mpsc::channel();

        let print_thread = std::thread::spawn(move || {
            for operation in print_rx {
                match operation {
                    PrintOperation::Print(message) => {
                        if let Err(e) = printer.print(message.clone()) {
                            eprintln!(
                                "Failed to print message: {}\nOriginal message: {}",
                                e, message
                            );
                        }
                    }
                    PrintOperation::Close => break,
                }
            }
        });

        match VARIABLES.lock() {
            Ok(mut c) => c.set_print_thread(PrintThread {
                thread: print_thread,
                sender: print_tx.clone(),
            }),
            Err(_) => {
                eprintln!("couldn't lock print variables")
            }
        };

        Self {
            printer: print_tx,
            sender,
        }
    }

    /// Sends a ^C signal to the program on unix that rustyline will cach, or sends a command like the user to exit on windows
    pub fn close_program() {
        FORSED_SHUT_DOWN.store(true, std::sync::atomic::Ordering::Release);
        #[cfg(unix)]
        unsafe {
            libc::raise(libc::SIGINT);
        }

        #[cfg(windows)]
        {
            unsafe {
               inject_text("\nexit\n");
            }
        }
    }

    pub fn is_forced_shutdown() -> bool {
        FORSED_SHUT_DOWN.load(std::sync::atomic::Ordering::Acquire)
    }
    pub fn reset_color() {
        let config = match VARIABLES.lock() {
            Ok(c) => c,
            Err(_) => {
                eprintln!("couldn't lock print variables");
                return;
            }
        };

        if config.stdout_is_same_as_stderr {
            if config.stdout_color || config.stderr_color {
                print!("\x1b[0m");
                let _ = stdout().flush();
            }
        } else {
            if config.stdout_color {
                print!("\x1b[0m");
                let _ = stdout().flush();
            }
            if config.stderr_color {
                eprint!("\x1b[0m");
                let _ = stderr().flush();
            }
        }
    }

    pub fn print(&self, message: &str, rgb: RGB) {
        let config = if let Ok(config) = VARIABLES.lock() {
            config
        } else {
            eprintln!("couldn't lock print variables");
            return;
        };

        let mut mesige = if config.stdout_color {
            let (r, g, b) = rgb.to_tuple();
            format!("\x1b[38;2;{r};{g};{b}m{message}\x1b[38;2;255;255;255m")
        } else {
            message.to_string()
        };
        mesige.push('\n');

        if let Err(e) = self.printer.send(PrintOperation::Print(mesige.clone())) {
            eprintln!("Failed to send print operation: {:?}", e);
            println!("{}", mesige);
        }
    }

    pub fn named_print(&self, plugin: &str, message: &str, rgb: RGB) {
        let config = if let Ok(config) = VARIABLES.lock() {
            config
        } else {
            eprintln!("couldn't lock print variables");
            return;
        };

        let mut mesige = if config.stdout_color {
            let (r, g, b) = rgb.to_tuple();
            format!("\x1b[38;2;{r};{g};{b}m\n{message} in {plugin}\x1b[38;2;255;255;255m")
        } else {
            format!("\n{} in {}", message, plugin)
        };
        mesige.push('\n');

        if let Err(e) = self.printer.send(PrintOperation::Print(mesige.clone())) {
            eprintln!("Failed to send print operation: {:?}", e);
            println!("{}", mesige);
        }
    }

    pub fn print_error(&self, plugin: &str, message: &str, rgb: RGB) {
        let config = if let Ok(config) = VARIABLES.lock() {
            config
        } else {
            eprintln!("couldn't lock print variables");
            return;
        };

        let formatted = format!("\n{} in {}\n", message, plugin);

        if config.stdout_is_same_as_stderr {
            let formatted = if config.stdout_color {
                let (r, g, b) = rgb.to_tuple();
                format!("\x1b[38;2;{r};{g};{b}m{formatted}\x1b[38;2;255;255;255m")
            } else {
                formatted
            };
            if let Err(e) = self.printer.send(PrintOperation::Print(formatted.clone())) {
                eprintln!(
                    "Failed to print message: {}\n Original message: {}",
                    e, message
                );
            }
        } else {
            if config.stderr_color {
                let (r, g, b) = rgb.to_tuple();
                eprint!("\x1b[38;2;{r};{g};{b}m{formatted}\x1b[38;2;255;255;255m",);
            } else {
                eprintln!("{}", formatted);
            }
        }
    }

    pub fn send(&self, operation: ErrorOperation, plugin: &str) -> Result<(), ()> {
        if let Err(e) = self.sender.send(operation) {
            self.print_error(
                plugin,
                &format!("Failed to send operation: {e}"),
                RGB::CRITICAL_ERROR(),
            );
            return Err(());
        }
        Ok(())
    }
}

impl Clone for Printer {
    fn clone(&self) -> Self {
        Printer {
            printer: self.printer.clone(),
            sender: self.sender.clone(),
        }
    }
}

#[cfg(unix)]
fn stdout_is_same_as_stderr() -> bool {
    use libc::{fstat, stat};
    use std::io::{stderr, stdout};
    use std::mem::MaybeUninit;
    use std::os::unix::io::AsRawFd;

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
    use winapi::um::consoleapi::GetConsoleMode;
    use winapi::um::fileapi::GetFileType;
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
            let stdout_is_console =
                unsafe { GetConsoleMode(stdout_handle as _, &mut stdout_mode) != 0 };
            let stderr_is_console =
                unsafe { GetConsoleMode(stderr_handle as _, &mut stdout_mode) != 0 };

            return stdout_is_console && stderr_is_console;
        } else {
            return false;
        }
    }
}

#[cfg(windows)]
unsafe fn inject_text(text: &str) {
    use winapi::um::winnt::HANDLE;
    use std::mem::zeroed;
    use winapi::um::winnt::WCHAR;

    use winapi::um::wincon::WriteConsoleInputW;
    use winapi::um::wincon::KEY_EVENT;
    use winapi::um::wincon::KEY_EVENT_RECORD;
    use winapi::um::{processenv::GetStdHandle, winbase::STD_INPUT_HANDLE, wincon::INPUT_RECORD};

    let stdin_handle: HANDLE = GetStdHandle(STD_INPUT_HANDLE);
    let mut input_records: Vec<INPUT_RECORD> = Vec::new();

    for c in text.chars() {
        

    let mut key_down: INPUT_RECORD = zeroed();
        key_down.EventType = KEY_EVENT;
        *key_down.Event.KeyEvent_mut() = KEY_EVENT_RECORD {
            bKeyDown: 1,
            wRepeatCount: 1,
            wVirtualKeyCode: 0,
            wVirtualScanCode: 0,
            uChar: unsafe { std::mem::transmute::<WCHAR, _>(c as u16) },
            dwControlKeyState: 0,
        };
        input_records.push(key_down);

        // Key up
        let mut key_up: INPUT_RECORD = zeroed();
        key_up.EventType = KEY_EVENT;
        *key_up.Event.KeyEvent_mut() = KEY_EVENT_RECORD {
            bKeyDown: 0,
            ..*key_down.Event.KeyEvent()
        };
        input_records.push(key_up);
        }

        let mut written = 0;
        WriteConsoleInputW(stdin_handle, input_records.as_mut_ptr(), input_records.len() as u32, &mut written);

}
