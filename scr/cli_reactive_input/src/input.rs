use std::io::{self, Read, Write};
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, RawFd};
use std::os::windows::io::FromRawHandle;
#[cfg(windows)]
use std::fs::File;

#[cfg(unix)]
use nix::unistd::{pipe, read, write, close};

#[cfg(unix)]
use termios::*;

#[cfg(windows)]
use winapi::um::consoleapi::*;
#[cfg(windows)]
use winapi::um::handleapi::*;
#[cfg(windows)]
use winapi::um::minwinbase::OVERLAPPED;
#[cfg(windows)]
use winapi::um::winbase::*;
#[cfg(windows)]
use winapi::um::wincon::*;
#[cfg(windows)]
use winapi::um::processenv::*;
#[cfg(windows)]
use winapi::um::fileapi::*;
#[cfg(windows)]
use winapi::shared::minwindef::*;
#[cfg(windows)]
use winapi::um::winnt::HANDLE;
#[cfg(windows)]
use winapi::um::synchapi::WaitForMultipleObjects;
#[cfg(windows)]
use std::ptr::null_mut;
#[cfg(windows)]
use std::os::windows::io::AsRawHandle;

#[cfg(windows)]
use winapi::um::namedpipeapi::CreatePipe;

mod keys;
pub use keys::Key;

pub enum InputEvent {
    Input(Key),
    Interrupt(String),
}

pub struct Input {
    #[cfg(unix)]
    stdin_fd: RawFd,
    #[cfg(unix)]
    orig_termios: Termios,
    #[cfg(unix)]
    pipe_read: RawFd,

    #[cfg(windows)]
    stdin_handle: HANDLE,
    #[cfg(windows)]
    orig_mode: DWORD,
    #[cfg(windows)]
    pipe_read: File,
}

pub struct Interrupter {
    #[cfg(unix)]
    pipe_write: RawFd,

    #[cfg(windows)]
    pipe_write: File,
}

impl Input {
    pub fn new() -> io::Result<(Self, Interrupter)> {
        #[cfg(unix)]
        {
            let stdin_fd = io::stdin().as_raw_fd();

            // Save current termios
            let orig_termios = tcgetattr(stdin_fd)?;
            let mut raw = orig_termios.clone();
            cfmakeraw(&mut raw);
            tcsetattr(stdin_fd, TCSANOW, &raw)?;

            let (read_fd, write_fd) = pipe()?;

            Ok((
                Input {
                    stdin_fd,
                    orig_termios,
                    pipe_read: read_fd,
                },
                Interrupter { pipe_write: write_fd },
            ))
        }

        #[cfg(windows)]
        {
            unsafe {
                let stdin_handle = GetStdHandle(STD_INPUT_HANDLE);
                if stdin_handle == INVALID_HANDLE_VALUE {
                    return Err(io::Error::last_os_error());
                }

                let mut mode: DWORD = 0;
                if GetConsoleMode(stdin_handle, &mut mode) == 0 {
                    return Err(io::Error::last_os_error());
                }

                const ENABLE_VIRTUAL_TERMINAL_INPUT: DWORD = 0x0200;

                // Save original mode and set raw mode
                let orig_mode = mode;
                let raw_mode = mode & !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT);
                let vt_mode = raw_mode | ENABLE_VIRTUAL_TERMINAL_INPUT;
                if SetConsoleMode(stdin_handle, vt_mode) == 0 {
                    return Err(io::Error::last_os_error());
                }

                let mut read_pipe = null_mut();
                let mut write_pipe = null_mut();
                if CreatePipe(&mut read_pipe, &mut write_pipe, null_mut(), 0) == 0 {
                    return Err(io::Error::last_os_error());
                }

                Ok((
                    Input {
                        stdin_handle,
                        orig_mode,
                        pipe_read: File::from_raw_handle(read_pipe as *mut std::ffi::c_void),
                    },
                    Interrupter {
                        pipe_write: File::from_raw_handle(write_pipe as *mut std::ffi::c_void),
                    },
                ))
            }
        }
    }

    /// Read form the pipe
    pub fn read_pipe(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        #[cfg(unix)]
        {
            read(self.pipe_read, buf).map_err(|e| io::Error::from_raw_os_error(e as i32))
        }

        #[cfg(windows)]
        {
            self.pipe_read.read(buf)
        }
    }

    /// Read a single byte from stdin
    pub fn read_byte_stdin(&mut self) -> io::Result<Option<u8>> {
        #[cfg(unix)]
        {
            let mut buf = [0; 1];
            let bytes_read = read(self.stdin_fd, &mut buf).map_err(|e| io::Error::from_raw_os_error(e as i32))?;
            if(bytes_read == 0) {
                Ok(None)
            }else{
                Ok(Some(buf[0]))
            }
        }

        #[cfg(windows)]
        {
            let mut buf = [0; 1];
            let mut bytes_read: DWORD = 0;
            let ret: BOOL = unsafe {
                ReadFile(self.stdin_handle, buf.as_mut_ptr() as *mut _,1, &mut bytes_read, null_mut() as *mut OVERLAPPED)
            };
            if ret != 0 {
                if bytes_read == 0 {
                    return Ok(None);
                }else{
                    Ok(Some(buf[0]))
                }
            } else {
                Err(io::Error::last_os_error())
            }
        }
    }

    #[cfg(unix)]
    /// Wait for input from stdin or the pipe
    /// Returns the input event
    /// If the input is from stdin, it returns the key
    /// If the input is from the pipe, it returns the interrupt
    pub fn get_next_signal(&mut self) -> io::Result<InputEvent> {
        let mut fds = [
            libc::pollfd { fd: self.stdin_fd, events: libc::POLLIN, revents: 0 },
            libc::pollfd { fd: self.pipe_read, events: libc::POLLIN, revents: 0 },
        ];

        let ret = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as u64, -1) };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        for fd in &fds {
            if fd.revents & libc::POLLIN != 0 {
                if fd.fd == self.stdin_fd {
                    return Ok(InputEvent::Input(self.parce_key()?));
                } else if fd.fd == self.pipe_read {
                    // pipe is ready
                    let mut buf = [0; 40];
                    self.read_pipe(&mut buf);
                    Ok(InputEvent::Interrupt(String::from_utf8_lossy(&buf).to_string()))
                }
            }
        }

        Err(io::Error::new(io::ErrorKind::Other, "Unknown event"))
    }

fn parce_key(&mut self) -> io::Result<Key>{
                    let first_byte = self.read_byte_stdin()?.unwrap();
                match first_byte {
                    0x0D => return Ok(Key::Enter),
                    0x0A => return Ok(Key::Enter),
                    0x09 => return Ok(Key::Tab),
                    0x7F => return Ok(Key::Backspace),
                    0x08 => return Ok(Key::Backspace),
                    0x03 => return Ok(Key::CtrlC),
                    0x04 => return Ok(Key::CtrlD),
                    0x1B =>{
                        if let Some(second_byte) = self.read_byte_stdin()?{
                            if second_byte == 0x5B{
                                if let Some(third_byte) = self.read_byte_stdin()?{
                                    match third_byte {
                                        0x41 => return Ok(Key::ArrowUp),
                                        0x42 => return Ok(Key::ArrowDown),
                                        0x43 => return Ok(Key::ArrowRight),
                                        0x44 => return Ok(Key::ArrowLeft),
                                        0x48 => return Ok(Key::Home),
                                        0x46 => return Ok(Key::End),
                                        0x33 => {
                                            if let Some(fourth_byte) = self.read_byte_stdin()?{
                                                if fourth_byte == 0x7E{
                                                    return Ok(Key::Delete);
                                                }else{
                                                    return Ok(Key::Unknown);
                                                }
                                            }else{
                                                return Ok(Key::Unknown);
                                            }
                                        },
                                        0x31 => {
                                            if let Some(fourth_byte) = self.read_byte_stdin()?{
                                                if fourth_byte == 0x7E{
                                                    return Ok(Key::Home);
                                                }else{
                                                    return Ok(Key::Unknown);
                                                }
                                            }else{
                                                return Ok(Key::Unknown);
                                            }
                                        },
                                        0x34 => {
                                            if let Some(fourth_byte) = self.read_byte_stdin()?{
                                                if fourth_byte == 0x7E{
                                                    return Ok(Key::End);
                                                }else{
                                                    return Ok(Key::Unknown);
                                                }
                                            }else{
                                                return Ok(Key::Unknown);
                                            }
                                        },
                                        _ => {return Ok(Key::Unknown);}
                                    }
                                }else{
                                    return Ok(Key::Unknown);
                                }
                            }else if second_byte == 0x4F{
                                if let Some(third_byte) = self.read_byte_stdin()?{
                                    match third_byte {
                                        0x41 => return Ok(Key::ArrowUp),
                                        0x42 => return Ok(Key::ArrowDown),
                                        0x43 => return Ok(Key::ArrowRight),
                                        0x44 => return Ok(Key::ArrowLeft),
                                        0x48 => return Ok(Key::Home),
                                        0x46 => return Ok(Key::End),
                                        _ => {return Ok(Key::Unknown);}
                                    }
                                } else{
                                    return Ok(Key::Unknown);
                                }
                            }else{
                                return Ok(Key::Unknown);
                            }
                        }else{
                            return Ok(Key::Escape);
                        }
                    },
                    _ => {
                        
                        if first_byte >= 0x20 && first_byte <= 0x7E{
                            return Ok(Key::Char(first_byte as char));
                        }else{
                            return Ok(Key::Unknown);
                        }
                    }
                }
}

#[cfg(windows)]
    /// Wait for input from stdin or the pipe
    /// Returns the input event
    /// If the input is from stdin, it returns the key
    /// If the input is from the pipe, it returns the interrupt
pub fn get_next_signal(&mut self) -> io::Result<InputEvent> {
    let handles = [self.stdin_handle, self.pipe_read.as_raw_handle() as HANDLE];

    let ret = unsafe { WaitForMultipleObjects(
        handles.len() as u32,
        handles.as_ptr(),
        0,  // wait for any
        winapi::um::winbase::INFINITE
    ) };

    const WAIT_STDIN: u32 = winapi::um::winbase::WAIT_OBJECT_0;
    const WAIT_PIPE: u32 = winapi::um::winbase::WAIT_OBJECT_0 + 1;
    match ret {
        WAIT_STDIN => {
            // stdin is ready
            return Ok(InputEvent::Input(self.parce_key()?));
        },
        WAIT_PIPE => {
            // pipe is ready
            let mut buf = [0; 40];
            self.read_pipe(&mut buf);
            Ok(InputEvent::Interrupt(String::from_utf8_lossy(&buf).to_string()))
        },
        _ => Err(io::Error::last_os_error()),
    }
}
}

#[cfg(windows)]
impl Clone for Interrupter {
    fn clone(&self) -> Self {
        let duplicated = self.pipe_write.try_clone().expect("Failed to clone pipe");
        Interrupter { pipe_write: duplicated }
    }
}

#[cfg(unix)]
impl Clone for Interrupter {
    fn clone(&self) -> Self {
        Interrupter { pipe_write: self.pipe_write } // RawFd is Copy
    }
}

impl Interrupter {
    pub fn interrupt(&mut self) -> io::Result<()> {
        #[cfg(unix)]
        {
            write(self.pipe_write, &[b'x']).map(|_| ()).map_err(|e| io::Error::from_raw_os_error(e as i32))
        }

        #[cfg(windows)]
        {
            self.pipe_write.write_all(&[b'x'])
        }
    }
}

impl Drop for Input {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            let _ = tcsetattr(self.stdin_fd, TCSANOW, &self.orig_termios);
            let _ = close(self.pipe_read);
        }

        #[cfg(windows)]
        {
            unsafe {
                SetConsoleMode(self.stdin_handle, self.orig_mode);
            }
            let _ = self.pipe_read.flush();
        }
    }
}

impl Drop for Interrupter {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            let _ = close(self.pipe_write);
        }

        #[cfg(windows)]
        {
            let _ = self.pipe_write.flush();
        }
    }
}


//TODO error handling check
//TODO check on linux
//TODO cleanup

