use std::io::Write;
mod input;
use input::{Input, InputEvent, Interrupter};

pub struct ReactiveInput{
    buffer: String,
    cursor_pos: usize,
    history: History,
    setting: Setting,
    inputs: Input,
    interrupter: Interrupter
}

struct History{
    history_list: Vec<String>,
    history_index: usize,
}

pub struct Setting{
    history_capt: bool,
    max_history_size: usize,
    auto_add_history: bool,
}

pub enum ReadlineResult{
    Output(String),
    Eof,    // Ctrl-D
    Interrupted // Ctrl-C
}

impl ReactiveInput{
    pub fn new() ->  (Self , Interrupter) {
      Self::internal_new( Setting {
        history_capt: false,
        max_history_size: 100,
        auto_add_history: false,
        })
    }
    pub fn with_setting(setting: Setting) ->  (Self , Interrupter) {
        if(setting.history_capt){
            Self::internal_new(setting)
        } else {
            Self::new()
        }
    }

    fn internal_new(setting: Setting) -> (Self , Interrupter) {
        let (inputs, interrupter) = Input::new().unwrap();
        (Self {
            buffer: String::new(),
            cursor_pos: 0,
            history: History {
                history_list: Vec::with_capacity(setting.max_history_size),
                history_index: 0,
            },
            setting,
            inputs,
            interrupter : interrupter.clone(),
        } , interrupter)
    }
    pub fn add_to_history(&mut self, command: String) {
        if self.setting.history_capt {
            if(self.history.history_list.len() >= self.setting.max_history_size) {
                self.history.history_list[self.history.history_index] = command;
            }else{
                self.history.history_list.push(command);
            }
            self.history.history_index += 1;
            self.history.history_index %= self.setting.max_history_size;
        }else{
            self.history.history_list.push(command);
            self.history.history_index += 1;
        }
    }
    pub fn readline(&mut self, input: &str) -> ReadlineResult{
        self.buffer.clear();
        self.cursor_pos = 0;
        std::io::stdout().write(input.as_bytes());
        std::io::stdout().flush();

        loop{
            if let InputEvent::Input(key) = self.inputs.get_next_signal().unwrap(){
                match key {
                    input::Key::Char(_) => todo!(),
                    input::Key::Enter => break,
                    input::Key::Tab => todo!(),
                    input::Key::Backspace => todo!(),
                    input::Key::Escape => todo!(),
                    input::Key::Delete => todo!(),
                    input::Key::Home => todo!(),
                    input::Key::End => todo!(),
                    input::Key::ArrowUp => todo!(),
                    input::Key::ArrowDown => todo!(),
                    input::Key::ArrowLeft => todo!(),
                    input::Key::ArrowRight => todo!(),
                    input::Key::CtrlC => return ReadlineResult::Interrupted ,
                    input::Key::CtrlD => return ReadlineResult::Eof,
                    input::Key::Unknown => todo!(),
                }
            }else{
                //redraw
            }
        }

        if self.setting.auto_add_history {
            self.add_to_history(self.buffer.clone());
        }
        ReadlineResult::Output(self.buffer.clone())
    }


}

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