use std::{
    error::Error,
    fmt::Display,
    io::{stdout, Write},
};
mod history;
mod input;
use history::{History, HistorySettings};
use input::{Input, InputEvent, Interrupter};

pub struct ReactiveInput {
    buffer: String,
    cursor_pos: usize,
    history: History,
    setting: Setting,
    inputs: Input,

    completer: Option<Box<dyn Completer>>,
    auto_completes: Vec<String>,
}

pub struct Setting {
    history_capt: bool,
    max_history_size: usize,
    auto_add_history: bool,

    handle_control_signals: bool,
}

pub trait Completer {
    fn complete(&self, input: &str, corsor: usize) -> Vec<String>;
}

pub struct StaticCompleter {}
impl Completer for StaticCompleter {
    fn complete(&self, input: &str, corsor: usize) -> Vec<String> {
        vec!["help".to_string()]
            .into_iter()
            .filter(|cmd| cmd.starts_with(input))
            .collect()
    }
}
pub enum ReadlineResult {
    Output(String),
    Eof,         // Ctrl-D
    Interrupted, // Ctrl-C
}

impl ReactiveInput {
    pub fn new() -> (Self, Interrupter) {
        Self::internal_new(Setting::new(), None)
    }
    pub fn with_setting(setting: Setting) -> (Self, Interrupter) {
        Self::internal_new(setting, None)
    }
    pub fn with_completer<Cmpl: Completer + 'static>(completer: Cmpl) -> (Self, Interrupter) {
        Self::internal_new(Setting::new(), Some(Box::new(completer)))
    }
    pub fn with_setting_and_completer<Cmpl: Completer + 'static>(
        setting: Setting,
        completer: Cmpl,
    ) -> (Self, Interrupter) {
        Self::internal_new(setting, Some(Box::new(completer)))
    }
    fn internal_new(
        setting: Setting,
        completer: Option<Box<dyn Completer>>,
    ) -> (Self, Interrupter) {
        let (inputs, interrupter) = Input::new(setting.handle_control_signals).unwrap();
        (
            Self {
                buffer: String::new(),
                cursor_pos: 0,
                history: History::new(HistorySettings {
                    is_capped: setting.history_capt,
                    capacity: setting.max_history_size,
                }),
                setting,
                inputs,
                completer: completer,
                auto_completes: Vec::new(),
            },
            interrupter,
        )
    }
    pub fn set_completer<Cmpl: Completer + 'static>(&mut self, completer: Cmpl){
        self.completer = Some(Box::new(completer));
    }
    pub fn remove_completer(&mut self){
        self.completer = None
    }
    pub fn set_setting(&mut self, setting: Setting) {
        self.setting = setting;

        self.inputs
            .set_handle_contorl_signal(self.setting.handle_control_signals);
        self.history.set_setting(HistorySettings {
            is_capped: self.setting.history_capt,
            capacity: self.setting.max_history_size,
        });
    }
    pub fn add_to_history(&mut self, command: String) {
        self.history.add_to_history(command);
    }

    pub fn readline(&mut self, input: &str) -> Result<ReadlineResult, ReadlineError> {
        self.buffer.clear();
        let mut scratch_buffer = None;
        self.cursor_pos = 0;
        let mut re_run_completer = true;

        std::io::stdout().write(input.as_bytes());
        std::io::stdout().flush().unwrap();

        loop {
            match self.inputs.get_next_signal() {
                Ok(x) => {
                    if let InputEvent::Input(key) = x {
                        std::io::stdout().flush().unwrap();
                        match key {
                            input::Key::Char(char) => {
                                if scratch_buffer.is_some() {
                                    scratch_buffer = None; // Exit history mode
                                }
                                self.buffer.insert(self.cursor_pos, char);
                                self.cursor_pos += 1;
                                self.redraw_line(input);
                                re_run_completer = true;
                            }
                            input::Key::Enter => {
                                println!("");
                                self.auto_completes.clear();
                                break;
                            }
                            input::Key::Tab => todo!(),  //TODO run completer
                            input::Key::ShiftTab => todo!(),
                            input::Key::Backspace => {
                                if (self.cursor_pos != 0) {
                                    self.buffer.remove(self.cursor_pos - 1);
                                    self.cursor_pos -= 1;
                                    self.redraw_line(input);
                                    re_run_completer = true;
                                }
                            }
                            input::Key::Escape => {
                                self.buffer.clear();
                                scratch_buffer = None;
                                self.history.set_to_begining();
                                self.cursor_pos = 0;
                                re_run_completer = true;
                                self.redraw_line(input);
                            },
                            input::Key::Delete => {
                                if (self.cursor_pos < self.buffer.len()) {
                                    self.buffer.remove(self.cursor_pos);
                                    self.redraw_line(input);
                                    re_run_completer = true;
                                }
                            }
                            input::Key::Home => {
                                print!("\r");
                                self.cursor_pos = 0;
                                stdout().flush();
                                re_run_completer = true;
                            }
                            input::Key::End => {
                                print!("\r\x1B[{}C", self.buffer.len() + input.len());
                                self.cursor_pos = self.buffer.len();
                                stdout().flush();
                                re_run_completer = true;
                            }
                            input::Key::ArrowUp => {
                                if scratch_buffer.is_none() {
                                    scratch_buffer = Some(self.buffer.clone());
                                }
                                if let Some(entry) = self.history.previous() {
                                    self.buffer = entry;
                                    self.cursor_pos = self.buffer.len();
                                    self.redraw_line(input);
                                }
                                re_run_completer = true;
                            }
                            input::Key::ArrowDown => {
                                if let Some(entry) = self.history.next() {
                                    self.buffer = entry;
                                } else if let Some(original) = scratch_buffer.take() {
                                    self.buffer = original;
                                } // else stay on current buffer
                                self.cursor_pos = self.buffer.len();
                                self.redraw_line(input);
                                re_run_completer = true;
                            }
                            input::Key::ArrowLeft => {
                                if self.cursor_pos > 0 {
                                    self.cursor_pos -= 1;
                                    print!("\x1B[1D");
                                    std::io::stdout().flush();
                                    re_run_completer = true;
                                }
                            }
                            input::Key::ArrowRight => {
                                if self.cursor_pos < self.buffer.len() {
                                    self.cursor_pos += 1;
                                    print!("\x1B[1C");
                                    std::io::stdout().flush();
                                    re_run_completer = true;
                                }
                            }
                            input::Key::CtrlC => return Ok(ReadlineResult::Interrupted),
                            input::Key::CtrlD => return Ok(ReadlineResult::Eof),
                            input::Key::Unknown => todo!(),
                        }
                    } else {
                        //redraw
                        print!("interrupted");
                        std::io::stdout().flush().unwrap();
                    }
                }
                Err(e) => println!("Error: {}", e),
            }
        }

        if self.setting.auto_add_history {
            self.add_to_history(self.buffer.clone());
        }

        Ok(ReadlineResult::Output(self.buffer.clone()))
    }

    fn redraw_line(&self, input_char: &str) {
        print!(
            "\x1B[2K\r{}{}\r\x1B[{}C",
            input_char,
            self.buffer,
            self.cursor_pos + input_char.len()
        );
        stdout().flush();
    }
}

impl Setting {
    pub fn new() -> Self {
        Self {
            history_capt: false,
            max_history_size: 100,
            auto_add_history: false,
            handle_control_signals: true,
        }
    }
    pub fn from(is_caped: bool, max_size: usize, auto_hist: bool, handle_conrtols: bool) -> Self {
        Self {
            history_capt: is_caped,
            max_history_size: max_size,
            auto_add_history: auto_hist,
            handle_control_signals: handle_conrtols,
        }
    }

    // Fluent-style setters
    pub fn set_hist_len(mut self, size: usize) -> Self {
        self.max_history_size = size;
        self
    }

    pub fn change_hist_len(&mut self, size: usize) {
        self.max_history_size = size;
    }

    pub fn uncap_hist(mut self) -> Self {
        self.history_capt = false;
        self
    }

    pub fn cap_hist(mut self) -> Self {
        self.history_capt = true;
        self
    }

    pub fn change_hist_cap(&mut self, caped: bool) {
        self.history_capt = caped;
    }

    pub fn enable_auto_add_history(mut self) -> Self {
        self.auto_add_history = true;
        self
    }

    pub fn disable_auto_add_history(mut self) -> Self {
        self.auto_add_history = false;
        self
    }

    pub fn set_auto_add_history(&mut self, enabled: bool) {
        self.auto_add_history = enabled;
    }

    pub fn enable_signal_handling(mut self) -> Self {
        self.handle_control_signals = true;
        self
    }

    pub fn disable_signal_handling(mut self) -> Self {
        self.handle_control_signals = false;
        self
    }

    pub fn set_signal_handling(&mut self, enabled: bool) {
        self.handle_control_signals = enabled;
    }
}
#[derive(Debug)]
pub enum ReadlineError {
    StdOutError(std::io::Error),
}
impl Display for ReadlineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadlineError::StdOutError(e) => write!(f, "StdOutError: {}", e),
        }
    }
}

impl Error for ReadlineError {}
//TODO set escape key behavior

/* Things I decided my system must do:
User can type commands and they get processed when Enter is pressed

When prints happen (from other threads), they interrupt cleanly and restore the input line

The input supports:

Cursor movement (arrows, home/end, etc.)

Command history

(Later) optional linter/highlight system

Must have low CPU overhead (because it runs idle most of the time)

Must run well on Raspberry Pi (resource-constrained)

Must be portable (also works on Windows for development)


Chosen approach:
Use Unix poll() system call as the event loop

Poll will wait for:

Key presses (stdin becomes readable)

Timeouts (so redraws can happen on timer)

(Optional) messages from threads (using pipe/eventfd/self-pipe)

Why I chose poll():
✅ Low overhead (no busy waiting)

✅ Keypresses are instant (poll wakes on key)

✅ Redraws can happen on timer (set by poll timeout)

✅ Other threads can send messages to wake the loop

✅ Fully event-driven design

✅ Works on Unix (Linux, Raspberry Pi), and can have Windows alternative

❌ Requires writing my own line-editing logic (cursor move, history,


 Things I haven't fully decided yet:
Redraw signaling system:

Options:

Use poll timeout to check periodically (simple)

Use a pipe or eventfd that other threads write to (more responsive)

Use signal handlers (harder, less recommended)

Use async runtime (Tokio/async-std) — more overhead
*/
