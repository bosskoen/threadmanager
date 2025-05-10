use std::io::Write;
mod input;
use input::Input;

pub struct ReactiveInput{
    buffer: String,
    cursor_pos: usize,
    history: Vec<String>,
    setting: Setting,
    history_index: usize,
    inputs: Input,
}

pub struct Setting{
    history_capt: bool,
    max_history_size: usize,
    auto_add_history: bool,
}

impl ReactiveInput{
    pub fn new() -> Self {
      Self::internal_new( Setting {
        history_capt: false,
        max_history_size: 100,
        auto_add_history: false,
        })
    }
    pub fn with_setting(setting: Setting) -> Self {
        if(setting.history_capt){
            Self::internal_new(setting)
        } else {
            Self::new()
        }
    }

    fn internal_new(setting: Setting) -> Self {
        Self {
            buffer: String::new(),
            cursor_pos: 0,
            history: Vec::with_capacity(setting.max_history_size),
            setting,
            history_index: 0,
        }
    }
    pub fn add_to_history(&mut self, command: String) {
        if self.setting.history_capt {
            if(self.history.len() >= self.setting.max_history_size) {
                self.history[self.history_index] = command;
            }else{
                self.history.push(command);
            }
            self.history_index += 1;
            self.history_index %= self.setting.max_history_size;
        }else{
            self.history.push(command);
            self.history_index += 1;
        }
    }
    pub fn readline(&mut self, input: &str) -> String{
        self.buffer.clear();
        self.cursor_pos = 0;
        std::io::stdout().write(input.as_bytes());
        std::io::stdout().flush();

        self.read_loop();

        if self.setting.auto_add_history {
            self.add_to_history(self.buffer.clone());
        }
        self.buffer.clone()
    }

    ///unix poll() system call as the event loop
    fn read_loop(&mut self) {

    }

    // windows
    /*fn readloop(&mut self) ->string{
    } */
}



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