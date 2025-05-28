use std::{
    error::Error,
    fmt::Display,
    io::{stdout, Write},
};
mod history;
mod input;
use history::{History, HistorySettings};
use input::{Input, InputEvent, Interrupter};

#[macro_export]
macro_rules! raw_println {
    ($($arg:tt)*) => {{
        use std::io::{Write, stdout};

        let mut output = format!($($arg)*);

        output = output.replace("\n", "\r\n");
        output.push_str("\r\n");


        // Print and flush
        let mut out = stdout();
        write!(out, "{}", output).unwrap();
        out.flush().unwrap();
    }};
}


pub struct ReactiveInput {
    buffer: String,
    cursor_pos: usize,
    history: History,
    setting: Setting,
    inputs: Input,

    completer: Option<Box<dyn Completer>>,
    auto_completes: Vec<String>,
    auto_complete_index: usize,
}

pub struct Setting {
    history_capt: bool,
    max_history_size: usize,
    auto_add_history: bool,
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
        let (inputs, interrupter) = Input::new().unwrap();
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
                auto_complete_index: 0,
            },
            interrupter,
        )
    }
    pub fn set_completer<Cmpl: Completer + 'static>(&mut self, completer: Cmpl) {
        self.completer = Some(Box::new(completer));
    }
    pub fn remove_completer(&mut self) {
        self.completer = None
    }
    pub fn set_setting(&mut self, setting: Setting) {
        self.setting = setting;

        self.history.set_setting(HistorySettings {
            is_capped: self.setting.history_capt,
            capacity: self.setting.max_history_size,
        });
    }
    pub fn get_setting(&mut self) -> &mut Setting{
        &mut self.setting
    }
    pub fn reload_setting(&mut self){
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
                                println!("\r");
                                self.auto_completes.clear();
                                self.history.set_to_begining();
                                break;
                            }
                            input::Key::Tab => {
                                if re_run_completer {
                                    if let Some(completer) = &self.completer {
                                        self.auto_completes =
                                            completer.complete(&self.buffer, self.cursor_pos);
                                    } else {
                                        self.auto_completes.clear();
                                    }
                                    self.auto_completes.insert(0, self.buffer.clone());
                                    self.auto_complete_index = 0;
                                    re_run_completer = false;
                                }
                                if !self.auto_completes.is_empty() {
                                    scratch_buffer = None; // Exit history mode

                                    
                                    self.auto_complete_index =
                                        (self.auto_complete_index + 1) % self.auto_completes.len();
                                    self.buffer =
                                        self.auto_completes[self.auto_complete_index].clone();
                                    self.redraw_line(input);
                                    self.cursor_to_end(input);
                                }
                            }
                            input::Key::ShiftTab => {
                                if re_run_completer {
                                    if let Some(completer) = &self.completer {
                                        self.auto_completes =
                                            completer.complete(&self.buffer, self.cursor_pos);
                                    } else {
                                        self.auto_completes.clear();
                                    }
                                    self.auto_completes.insert(0, self.buffer.clone());
                                    self.auto_complete_index = 0;
                                    re_run_completer = false;
                                }
                                if !self.auto_completes.is_empty() {
                                    scratch_buffer = None; // Exit history mode
                                    self.auto_complete_index =
                                        (self.auto_complete_index + self.auto_completes.len() - 1)
                                            % self.auto_completes.len();
                                    self.buffer =
                                        self.auto_completes[self.auto_complete_index].clone();
                                    self.cursor_to_end(input);
                                    self.redraw_line(input);
                                }
                            }
                            input::Key::Backspace => {
                                if (self.cursor_pos != 0) {
                                    if scratch_buffer.is_some() {
                                        scratch_buffer = None; // Exit history mode
                                    }
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
                            }
                            input::Key::Delete => {
                                if (self.cursor_pos < self.buffer.len()) {
                                    if scratch_buffer.is_some() {
                                        scratch_buffer = None; // Exit history mode
                                    }
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
                                self.cursor_to_end(input);
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
                        self.redraw_line(input);
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

    fn cursor_to_end(&mut self, input: &str) {
        print!("\r\x1B[{}C", self.buffer.len() + input.len());
        self.cursor_pos = self.buffer.len();
        stdout().flush();
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
        }
    }
    pub fn from(is_caped: bool, max_size: usize, auto_hist: bool) -> Self {
        Self {
            history_capt: is_caped,
            max_history_size: max_size,
            auto_add_history: auto_hist,
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

//TODO check for ^C and ^D when to polling maby throw a other thread