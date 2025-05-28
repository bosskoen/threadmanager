use std::io;
#[cfg(unix)]
use std::os::{unix::io::{AsRawFd, RawFd, BorrowedFd}, fd::{OwnedFd, AsFd}};

use std::ptr::null;
use std::sync::Arc;

#[cfg(unix)]
use nix::unistd::{pipe, read, write, close};

#[cfg(unix)]
use nix::fcntl::{fcntl, FcntlArg, OFlag};

#[cfg(unix)]
use nix::sys::termios::{tcgetattr, Termios, cfmakeraw, tcsetattr, SetArg::TCSANOW};
#[cfg(unix)]
use nix::libc;

#[cfg(windows)]
use winapi::um::consoleapi::*;
#[cfg(windows)]
use winapi::um::handleapi::*;
#[cfg(windows)]
use winapi::um::minwinbase::OVERLAPPED;
#[cfg(windows)]
use winapi::um::synchapi::CreateEventW;
#[cfg(windows)]
use winapi::um::synchapi::ResetEvent;
#[cfg(windows)]
use winapi::um::synchapi::SetEvent;
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

mod keys;
pub use keys::Key;

pub enum InputEvent {
    Input(Key),
    Interrupt(),
}

#[cfg(windows)]
struct ManualResetEvent(HANDLE);

pub struct Input {
    #[cfg(unix)]
    stdin_fd: RawFd,
    #[cfg(unix)]
    orig_termios: Termios,
    #[cfg(unix)]
    pipe_read: OwnedFd,
    #[cfg(unix)]
    old_flags: i32,

    #[cfg(windows)]
    stdin_handle: HANDLE,
    #[cfg(windows)]
    orig_mode: DWORD,
    #[cfg(windows)]
    interupt_event: Arc<ManualResetEvent>,
}

pub struct Interrupter {
    #[cfg(unix)]
    pipe_write: OwnedFd,

    #[cfg(windows)]
    interupt_event: Arc<ManualResetEvent>,
}

impl Input {
    pub fn new() -> io::Result<(Self, Interrupter)> {
        #[cfg(unix)]
        {
            use nix::fcntl::{fcntl, FcntlArg, OFlag};

            let stdin_fd = io::stdin().as_raw_fd();

            // Save current termios
            let orig_termios = tcgetattr(unsafe {BorrowedFd::borrow_raw(stdin_fd)})?;
            let flags = fcntl(unsafe {BorrowedFd::borrow_raw(stdin_fd)}, FcntlArg::F_GETFL)?; // get current flags
            fcntl(unsafe {BorrowedFd::borrow_raw(stdin_fd)}, FcntlArg::F_SETFL(OFlag::from_bits_truncate(flags) | OFlag::O_NONBLOCK))?;
            let mut raw = orig_termios.clone();
            cfmakeraw(&mut raw);
            tcsetattr(unsafe {BorrowedFd::borrow_raw(stdin_fd)}, TCSANOW, &raw)?;

            let (read_fd, write_fd) = pipe()?;

            Ok((
                Input {
                    stdin_fd,
                    orig_termios,
                    pipe_read: read_fd,
                    old_flags: flags,
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
                let raw_mode;
                raw_mode = mode & !(ENABLE_PROCESSED_INPUT| ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT);
                let vt_mode = raw_mode | ENABLE_VIRTUAL_TERMINAL_INPUT;
                if SetConsoleMode(stdin_handle, vt_mode) == 0 {
                    return Err(io::Error::last_os_error());
                }

                let interupt_event: HANDLE = CreateEventW(null_mut(), TRUE, FALSE, null());
                let interupt_event = Arc::new(ManualResetEvent(interupt_event));
                let intupt_clone = Arc::clone(&interupt_event);

                Ok((
                    Input {
                        stdin_handle,
                        orig_mode,
                        interupt_event,
                    },
                    Interrupter {
                        interupt_event: intupt_clone,
                    },
                ))
            }
        }
    }

    /// Read form the pipe
    fn reset_interupter(&mut self) -> io::Result<()> {

        #[cfg(unix)]
        {
            let mut buf = [0; 32];
            read(self.pipe_read.as_fd(), &mut buf).map_err(|e| io::Error::from_raw_os_error(e as i32))?;
        }

        #[cfg(windows)]
        { 
            unsafe {
                ResetEvent(self.interupt_event.0); //TODO error
            }
        }
        Ok(())
    }

    /// Read a single byte from stdin
    fn read_byte_stdin(&mut self) -> io::Result<Option<u8>> {
        #[cfg(unix)]
        {
            let mut buf = [0; 1];
            match read(unsafe {BorrowedFd::borrow_raw(self.stdin_fd)}, &mut buf) {
            Ok(0) => Ok(None), // EOF
            Ok(_) => Ok(Some(buf[0])),
            Err(e) if e as i32 == libc::EAGAIN || e as i32 == libc::EWOULDBLOCK => Ok(None), // no data ready
            Err(e) => Err(io::Error::from_raw_os_error(e as i32)),
        }
        }

        #[cfg(windows)]
        {
            let mut record = [unsafe { std::mem::zeroed::<INPUT_RECORD>() }; 1];
            let mut events_read = 0;
             let ret = unsafe { PeekConsoleInputW(self.stdin_handle, record.as_mut_ptr(), 1, &mut events_read) };
            if ret == 0 {
                return Err(io::Error::last_os_error());
            }

            if events_read > 0 {
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
            }else{
                Ok(None)
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
            libc::pollfd { fd: self.pipe_read.as_raw_fd(), events: libc::POLLIN, revents: 0 },
        ];

        let ret = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as u64, -1) };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        for fd in &fds {
            if fd.revents & libc::POLLIN != 0 {
                if fd.fd == self.stdin_fd {
                    return Ok(InputEvent::Input(self.parce_key()?));
                } else if fd.fd == self.pipe_read.as_raw_fd() {
                    // pipe is ready
                    self.reset_interupter();
                    return Ok(InputEvent::Interrupt());
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
                                        0x5A => return Ok(Key::ShiftTab),
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
}//TODO see if this can be improved through looking stdin isnt beign written to, i.e.d. if bytes are comming but arnt in the buffer and you read a empty buffer and tink unknown key, but the bytes ant in the buffer yet
//poll with a realy small timeout


#[cfg(windows)]
    /// Wait for input from stdin or the pipe
    /// Returns the input event
    /// If the input is from stdin, it returns the key
    /// If the input is from the pipe, it returns the interrupt
pub fn get_next_signal(&mut self) -> io::Result<InputEvent> {

    let handles = [self.stdin_handle, self.interupt_event.0];

    let ret = unsafe { WaitForMultipleObjects(
        handles.len() as u32,
        handles.as_ptr(),
        FALSE,  // wait for any
        winapi::um::winbase::INFINITE
    ) };


    const WAIT_STDIN: u32 = winapi::um::winbase::WAIT_OBJECT_0;
    const WAIT_EVENT: u32 = winapi::um::winbase::WAIT_OBJECT_0 + 1;
    match ret {
        WAIT_STDIN => {
            // stdin is ready
            return Ok(InputEvent::Input(self.parce_key()?));
        },
        WAIT_EVENT => {
            // pipe is ready
            self.reset_interupter();

            Ok(InputEvent::Interrupt())
        },
        _ => Err(io::Error::last_os_error()),
    }
}
}

#[cfg(windows)]
impl Clone for Interrupter {
    fn clone(&self) -> Self {
        Interrupter { interupt_event: Arc::clone(&self.interupt_event) } // Handle is Copy
    }
}

#[cfg(unix)]
impl Clone for Interrupter {
    fn clone(&self) -> Self {
        let new_pipe = self.pipe_write.try_clone().unwrap();
        Interrupter { pipe_write: new_pipe } // OwnedFd is Clone
    }
}

impl Interrupter {
    pub fn interrupt(&mut self) -> io::Result<()> {
        #[cfg(unix)]
        {
            
                write(self.pipe_write.as_fd(), &[b'x']).map(|_| ()).map_err(|e| io::Error::from_raw_os_error(e as i32))?
            
        }

        #[cfg(windows)]
        {
            unsafe {
                SetEvent(self.interupt_event.0); //TODO error
            }
        }
        Ok(())
    }
}

impl Drop for Input {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            let _ = tcsetattr(unsafe{ BorrowedFd::borrow_raw(self.stdin_fd)}, TCSANOW, &self.orig_termios);
            let _ = fcntl(unsafe{ BorrowedFd::borrow_raw(self.stdin_fd)}, FcntlArg::F_SETFL(OFlag::from_bits_truncate(self.old_flags)));
        }

        #[cfg(windows)]
        {
            unsafe {
                SetConsoleMode(self.stdin_handle, self.orig_mode);
            }
        }
    }
}

#[cfg(windows)]
impl Drop for ManualResetEvent {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
    
}

//TODO error handling check
//TODO check on linux
//TODO cleanup

