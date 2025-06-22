use library::error_handeler::Printer;
#[allow(unused_imports)]
use library::error_handeler::{self, LedNumber};
#[allow(unused_imports)]
use library::error_handeler::{ErrorOperation, LedOption, RGB};
use library::rustyline::highlight::Highlighter;
use library::rustyline::hint::Hinter;
use library::rustyline::validate::Validator;
use library::rustyline::{self, Helper};
use std::collections::HashMap;
use std::vec;

use crate::{private_lib::Manager, Mode};

pub fn initialise_cli() -> HashMap<&'static str, Box<dyn Fn(&[&str], &mut Manager, &Printer)>> {
    let mut cli = HashMap::new();
    cli.insert(
        "start",
        Box::new(start_app) as Box<dyn Fn(&[&str], &mut Manager, &Printer)>,
    );
    cli.insert(
        "stop",
        Box::new(stop_app) as Box<dyn Fn(&[&str], &mut Manager, &Printer)>,
    );
    cli.insert(
        "status",
        Box::new(get_status) as Box<dyn Fn(&[&str], &mut Manager, &Printer)>,
    );
    cli.insert(
        "list",
        Box::new(list_apps) as Box<dyn Fn(&[&str], &mut Manager, &Printer)>,
    );
    cli.insert(
        "help",
        Box::new(help) as Box<dyn Fn(&[&str], &mut Manager, &Printer)>,
    );
    cli.insert(
        "settings",
        Box::new(settings) as Box<dyn Fn(&[&str], &mut Manager, &Printer)>,
    );
    cli.insert(
        "led",
        Box::new(led) as Box<dyn Fn(&[&str], &mut Manager, &Printer)>,
    );

    cli
}

fn start_app(args: &[&str], open_threads: &mut Manager, printer: &Printer) {
    if args.len() == 0 {
        printer.print(
            "No thread name provided. Please specify a thread name.",
            RGB::WHITE(),
        );
        return;
    } else if args.len() > 1 {
        printer.print(
            "Multiple thread names provided. Please specify only one thread name.",
            RGB::WHITE(),
        );
        return;
    }
    printer.print(
        &format!("Attempting to start thread: {}", args[0]),
        RGB::WHITE(),
    );

    if args[0] == error_handeler::light_dimmer_thread::PLUGIN_NAME {
        open_threads.start_light_dimmer();
    } else {
        if let Err(err) = open_threads.start_new_thread(args[0].to_string()) {
            printer.print(&format!("{err}"), RGB::WHITE());
            return;
        }
    }
    if open_threads.is_running(args[0]) {
        printer.print(&format!("Thread {} is running", args[0]), RGB::SUCCESS());
    } else {
        printer.print(&format!("Thread {} failed to start", args[0]), RGB::ERROR());
    }
}

fn stop_app(args: &[&str], open_threads: &mut Manager, printer: &Printer) {
    if args.len() == 0 {
        printer.print(
            "No thread name provided. Please specify a thread name.",
            RGB::WHITE(),
        );
        return;
    } else if args.len() > 1 {
        printer.print(
            "Multiple thread names provided. Please specify only one thread name.",
            RGB::WHITE(),
        );
        return;
    }
    if args[0] == "all" {
        printer.print("Attempting to stop all threads", RGB::WHITE());
        open_threads.stop_all_threads();
        printer.print(
            "Note: 'errorThread' is a permanent system thread and cannot be stopped.",
            RGB::TRACE(),
        );
        #[cfg(feature = "led")]
        printer.print(
            &format!(
                "{} can't be stopped this way use the manual command to stop it.",
                error_handeler::light_dimmer_thread::PLUGIN_NAME
            ),
            RGB::TRACE(),
        );
        printer.print("All threads stopped", RGB::SUCCESS());
        return;
    }
    if args[0] == "errorThread" {
        printer.print("Can't stop error thread", RGB::ERROR());
        return;
    }
    printer.print(
        &format!("Attempting to stop thread: {}", args[0]),
        RGB::WHITE(),
    );
    if let Err(err) = open_threads.stop_thread(args[0].to_string()) {
        printer.print(&format!("{err}"), RGB::WHITE());
        return;
    }
    if open_threads.is_running(args[0]) {
        printer.print(&format!("Thread {} failed to stop", args[0]), RGB::ERROR());
    } else {
        printer.print(&format!("Thread {} stopped", args[0]), RGB::SUCCESS());
    }
}

fn get_status(args: &[&str], open_threads: &mut Manager, printer: &Printer) {
    if args.len() == 0 {
        printer.print(
            "No thread name provided. Please specify a thread name.",
            RGB::WHITE(),
        );
        return;
    } else if args.len() > 1 {
        printer.print(
            "Multiple thread names provided. Please specify only one thread name.",
            RGB::WHITE(),
        );
        return;
    }
    if let Err(err) = open_threads.get_status(args[0].to_string()) {
        printer.print(&format!("{err}"), RGB::WHITE());
    }
}

fn list_apps(args: &[&str], open_threads: &mut Manager, printer: &Printer) {
    if args.len() == 0 {
        printer.print(
            "No argument provided. Please specify 'running', 'stopped', or 'all'.",
            RGB::WHITE(),
        );
        return;
    } else if args.len() > 1 {
        printer.print(
            "Invalid argument. Please specify 'running', 'stopped', or 'all'.",
            RGB::WHITE(),
        );
        return;
    }
    match args[0] {
        "running" => open_threads.list_threads(Mode::Running),
        "stopped" => open_threads.list_threads(Mode::Stopped),
        "all" => open_threads.list_threads(Mode::All),
        _ => printer.print(
            "Invalid argument. Please specify 'running', 'stopped', or 'all'.",
            RGB::WHITE(),
        ),
    }
}

fn help(args: &[&str], open_threads: &mut Manager, printer: &Printer) {
    if args.len() == 0 {
        // General help message for all commands
        printer.print("Available commands:", RGB::NOTICE());

        printer.print(
            "start <thread name> - Start a thread\n\
            stop <thread name> - Stop a thread\n\
            status <thread name> - Get the status of a thread\n\
            list <running/stopped/all> - List all threads\n\
            led <command> [args] - Control the LED system\n\
            settings <command> - Configure settings\n\
            help <command> || [app <app name>] - Get help for a specific command",
            RGB::WHITE(),
        );
        return;
    } else if args.len() > 1 {
        if args[0] == "app" {
            if args.len() == 2 {
                // Provide help for a specific app
                if let Err(er) = open_threads.help_message(args[1].to_string()) {
                    printer.print(&format!("{er}"), RGB::WHITE());
                }
            } else {
                printer.print("Invalid argument. Please specify 'app <app name>' to get help for a specific app.", RGB::WHITE());
            }
            return;
        } else {
            // Invalid argument case
            printer.print(
                "Invalid argument. Please specify 'app <app name>' to get help for a specific app.",
                RGB::WHITE(),
            );
            return;
        }
    }
    // Specific help for a command
    match args[0] {
        "start" => printer.print("start <thread name> - Start a thread with the specified name", RGB::WHITE()),
        "stop" => printer.print("stop <thread name> - Stop the thread with the specified name", RGB::WHITE()),
        "status" => printer.print("status <thread name> - Get the status of a running thread", RGB::WHITE()),
        "list" => printer.print("list <running/stopped/all> - List threads based on their status (running, stopped, or all)", RGB::WHITE()),
        "settings" => printer.print("settings reload - Reload configuration settings", RGB::WHITE()),
        "led" => printer.print("led <on/off/reset/color/brightness> [args] - Control LED colors and brightness\n\
                        Commands:\n\
                        on/off/reset [red/green/blue/all] - Control specific colors\n\
                        color <hex color (0xRRGGBB)> - Set LED color\n\
                        brightness <0-255> - Set brightness level", RGB::WHITE()),
        "help" => printer.print("help <command> || [app <app name>] - Get help for a specific command or app", RGB::WHITE()),
        "exit" => printer.print("exit - Exit the CLI", RGB::WHITE()),
        _ => printer.print("Invalid command. Please specify 'start', 'stop', 'status', 'list', 'led', 'settings', 'help' or 'app <app name>'", RGB::WHITE()),
    }
}

fn settings(args: &[&str], open_threads: &mut Manager, printer: &Printer) {
    if args.len() == 0 {
        printer.print(
            "No argument provided. Available command:\n\
               reload - Reload configuration settings",
            RGB::WHITE(),
        );
        return;
    } else if args.len() > 1 {
        printer.print(
            "Invalid argument. Only 'reload' is supported.",
            RGB::WHITE(),
        );
        return;
    }

    match args[0] {
        "reload" => open_threads.reload_settings(),
        _ => printer.print("Invalid argument. Please specify 'reload'.", RGB::WHITE()),
    }
}

#[cfg(not(feature = "led"))]
fn led(_args: &[&str], _open_threads: &mut Manager, printer: &Printer) {
    printer.print("LED's are disabled in this build", RGB::WHITE());
}

#[cfg(feature = "led")]
fn led(args: &[&str], _open_threads: &mut Manager, printer: &Printer) {
    if args.len() == 0 {
        printer.print("Available LED commands:", RGB::NOTICE());
        printer.print(
            "led on [red/green/blue/all] [0-4/all]\n\
        led off [red/green/blue/all] [0-4/all]\n\
        led reset [red/green/blue/all] [0-4/all]\n\
        led color <hex (0xRRGGBB)||normal> [0-4/all]\n\
        led brightness <0-16> [0-4/all]",
            RGB::WHITE(),
        );
        return;
    } else if args.len() > 3 {
        printer.print("To manny arguments.", RGB::WHITE());
        return;
    }
    match args[0] {
        "off" => {
            if args.len() == 3 {
                let oper;
                let led_num;
                match args[1] {
                    "red" => oper = LedOption::Red,
                    "green" => oper = LedOption::Green,
                    "blue" => oper = LedOption::Blue,
                    "all" => oper = LedOption::All,
                    _ => {
                        printer.print("Invalid argument. please specify a color and LED\nled off [red/green/blue/all] [1-4/all]", RGB::WHITE());
                        return;
                    }
                }
                match args[2] {
                    "0" => led_num = LedNumber::LED1,
                    "1" => led_num = LedNumber::LED2,
                    "2" => led_num = LedNumber::LED3,
                    "3" => led_num = LedNumber::LED4,
                    "4" => led_num = LedNumber::LED5,
                    "all" => led_num = LedNumber::ALL,
                    _ => {
                        printer.print("Invalid argument. please specify a color and LED\nled off [red/green/blue/all] [1-4/all]", RGB::WHITE());
                        return;
                    }
                }
                if let Err(_) = printer.send(ErrorOperation::OffColor(oper, led_num), "main") {
                    printer.print_error(
                        "main",
                        "failed to send led command",
                        RGB::CRITICAL_ERROR(),
                    );
                }
            } else {
                printer.print("Invalid argument. please specify a color and LED\nled off [red/green/blue/all] [1-4/all]", RGB::WHITE());
                return;
            }
        }
        "on" => {
            if args.len() == 3 {
                let oper;
                let led_num;
                match args[1] {
                    "red" => oper = LedOption::Red,
                    "green" => oper = LedOption::Green,
                    "blue" => oper = LedOption::Blue,
                    "all" => oper = LedOption::All,
                    _ => {
                        printer.print("Invalid argument. please specify a color and LED\nled on [red/green/blue/all] [1-4/all]", RGB::WHITE());
                        return;
                    }
                }
                match args[2] {
                    "0" => led_num = LedNumber::LED1,
                    "1" => led_num = LedNumber::LED2,
                    "2" => led_num = LedNumber::LED3,
                    "3" => led_num = LedNumber::LED4,
                    "4" => led_num = LedNumber::LED5,
                    "all" => led_num = LedNumber::ALL,
                    _ => {
                        printer.print("Invalid argument. please specify a color and LED\nled on [red/green/blue/all] [1-4/all]", RGB::WHITE());
                        return;
                    }
                }
                if let Err(_) = printer.send(ErrorOperation::OnColor(oper, led_num), "main") {
                    printer.print_error(
                        "main",
                        "failed to send led command",
                        RGB::CRITICAL_ERROR(),
                    );
                }
            } else {
                printer.print("Invalid argument. please specify a color and LED\nled on [red/green/blue/all] [1-4/all]", RGB::WHITE());
                return;
            }
        }
        "reset" => {
            if args.len() == 3 {
                let oper;
                let led_num;
                match args[1] {
                    "red" => oper = LedOption::Red,
                    "green" => oper = LedOption::Green,
                    "blue" => oper = LedOption::Blue,
                    "all" => oper = LedOption::All,
                    _ => {
                        printer.print("Invalid argument. please specify a color and LED\nled reset [red/green/blue/all] [1-4/all]", RGB::WHITE());
                        return;
                    }
                }
                match args[2] {
                    "0" => led_num = LedNumber::LED1,
                    "1" => led_num = LedNumber::LED2,
                    "2" => led_num = LedNumber::LED3,
                    "3" => led_num = LedNumber::LED4,
                    "4" => led_num = LedNumber::LED5,
                    "all" => led_num = LedNumber::ALL,
                    _ => {
                        printer.print("Invalid argument. please specify a color and LED\nled reset [red/green/blue/all] [1-4/all]", RGB::WHITE());
                        return;
                    }
                }
                if let Err(_) = printer.send(ErrorOperation::RestColor(oper, led_num), "main") {
                    printer.print_error(
                        "main",
                        "failed to send led command",
                        RGB::CRITICAL_ERROR(),
                    );
                }
            } else {
                printer.print("Invalid argument. please specify a color and LED\nled reset [red/green/blue/all] [1-4/all]", RGB::WHITE());
                return;
            }
        }
        "color" => {
            if args.len() == 3 {
                let color;
                let led_num;
                match args[2] {
                    "0" => led_num = LedNumber::LED1,
                    "1" => led_num = LedNumber::LED2,
                    "2" => led_num = LedNumber::LED3,
                    "3" => led_num = LedNumber::LED4,
                    "4" => led_num = LedNumber::LED5,
                    "all" => led_num = LedNumber::ALL,
                    _ => {
                        printer.print("Invalid argument. please specify a color and LED\nled off (0xrrggbb|| 0XRRGGBB)||normal [1-4/all]", RGB::WHITE());
                        return;
                    }
                }
                if args[1] == "normal" {
                    color = RGB::GREEN(); //TODO what color should be used here?
                } else if args[1].len() == 8 && args[1].to_lowercase().starts_with("0x") {
                    let trimde = &args[1][2..];
                    color = RGB::from_hex(u32::from_str_radix(trimde, 16).unwrap_or(0));
                } else {
                    printer.print(
                        "Invalid argument. Please specify a color as hex (0xrrggbb|| 0XRRGGBB).",
                        RGB::WHITE(),
                    );
                    return;
                }
                if let Err(_) =
                    printer.send(ErrorOperation::ChangeLed(color, false, led_num), "main")
                {
                    printer.print_error(
                        "main",
                        "failed to send led command",
                        RGB::CRITICAL_ERROR(),
                    );
                }
            } else {
                printer.print("Invalid argument. please specify a color and LED\nled off (0xrrggbb|| 0XRRGGBB)[1-4/all]", RGB::WHITE());
                return;
            }
        }
        "brightness" => {
            if args.len() == 3 {
                let new_level;
                let led_num;
                match args[2] {
                    "0" => led_num = LedNumber::LED1,
                    "1" => led_num = LedNumber::LED2,
                    "2" => led_num = LedNumber::LED3,
                    "3" => led_num = LedNumber::LED4,
                    "4" => led_num = LedNumber::LED5,
                    "all" => led_num = LedNumber::ALL,
                    _ => {
                        printer.print("Invalid argument. please specify a brightness and LED\nled off [0-16] [1-4/all]", RGB::WHITE());
                        return;
                    }
                }
                match args[1].parse::<u8>() {
                    Ok(value) => new_level = value,
                    Err(_) => {
                        printer.print("invalit input, it needs to be a number", RGB::WHITE());
                        return;
                    }
                }
                if let Err(_) =
                    printer.send(ErrorOperation::CangeBrighness(new_level, led_num), "main")
                {
                    printer.print_error(
                        "main",
                        "failed to send led command",
                        RGB::CRITICAL_ERROR(),
                    );
                }
            } else {
                printer.print("Invalid argument. Please specify a brightness level and LED [0 - 16].\nled brightness [0-16] [0-4/all]", RGB::WHITE());
                return;
            }
        }
        "help" => {
            printer.print("LED Command Help:\n", RGB::NOTICE());
            printer.print(
                "led on/off/reset [red/green/blue/all] [0-4/all] - Control specific colors\n\
                   led color <hex (0xRRGGBB)||noraml> [0-4/all] - Set LED color or sets it to the no problems color\n\
                   led brightness <0-16> [0-4/all] - Set brightness level",
                RGB::WHITE(),
            );
        }
        _ => printer.print(
            "Invalid LED command. Use 'led help' to see available commands.",
            RGB::WHITE(),
        ),
    }
}

use std::sync::Mutex;
pub struct MyCompleter<'a> {
    threads: &'a Mutex<Manager>,
}

impl<'a> MyCompleter<'a> {
    pub fn new(threads: &'a Mutex<Manager>) -> Self {
        MyCompleter { threads }
    }
}

impl<'a> Helper for MyCompleter<'a> {}

impl<'a> Validator for MyCompleter<'a> {}

impl<'a> Highlighter for MyCompleter<'a> {}

impl<'a> Hinter for MyCompleter<'a> {
    type Hint = String;

    fn hint(&self, _line: &str, _pos: usize, _ctx: &rustyline::Context<'_>) -> Option<Self::Hint> {
        None
    }
}

impl<'a> rustyline::completion::Completer for MyCompleter<'a> {
    type Candidate = String;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _cont: &library::rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        #[cfg(feature = "led")]
        const COMMANDS: &[&str] = &[
            "start", "stop", "status", "list", "help", "settings", "led", "exit",
        ];
        #[cfg(not(feature = "led"))]
        const COMMANDS: &[&str] = &[
            "start", "stop", "status", "list", "help", "settings", "exit",
        ];

        #[cfg(feature = "led")]
        const LED_SUBS: &[&str] = &["on", "off", "reset", "color", "brightness"];
        #[cfg(feature = "led")]
        const LED_COLORS: &[&str] = &["red", "green", "blue", "all"];
        #[cfg(feature = "led")]
        const LED_INDICES: &[&str] = &["0", "1", "2", "3", "4", "all"];

        const LIST_ARGS: &[&str] = &["running", "stopped", "all"];
        const SETTINGS_ARGS: &[&str] = &["reload"];

        let words: Vec<&str> = line[..pos].trim().split_whitespace().collect();
        let end_with_space = line[..pos].ends_with(' ');
        let current_word = if end_with_space {
            ""
        } else {
            words.last().unwrap_or(&"")
        };
        let start = pos - current_word.len();

        let mut completions = Vec::new();

        match words.get(0) {
            None => {
                completions.extend(COMMANDS.iter().map(|&s| s.to_string()));
            }
            Some(&"start") => {
                if let Ok(lock) = self.threads.lock() {
                    let mut app = lock.get_list_stopped_apps();
                    if !lock.is_running("errorThread") {
                        app.push("errorThread".to_string());
                    }
                    #[cfg(feature = "led")]
                    if !lock.is_running(error_handeler::light_dimmer_thread::PLUGIN_NAME) {
                        app.push(error_handeler::light_dimmer_thread::PLUGIN_NAME.to_string());
                    }
                    completions.extend(
                        app.iter()
                            .filter(|s| s.starts_with(current_word))
                            .map(|s| s.to_string()),
                    );
                }
            }
            Some(&"status") => {
                if let Ok(lock) = self.threads.lock() {
                    let app = lock.get_list_running_apps();
                    completions.extend(
                        app.iter()
                            .filter(|s| s.starts_with(current_word))
                            .map(|s| s.to_string()),
                    );
                }
            }
            Some(&"settings") => {
                completions.extend(
                    SETTINGS_ARGS
                        .iter()
                        .filter(|s| s.starts_with(current_word))
                        .map(|s| s.to_string()),
                );
            }
            #[cfg(feature = "led")]
            Some(&"led") => {
                match words.len() {
                    1 => {
                        // Only "led" typed, suggest subcommands
                        completions.extend(
                            LED_SUBS
                                .iter()
                                .filter(|s| s.starts_with(current_word))
                                .map(|s| s.to_string()),
                        );
                    }
                    2 => {
                        let subcmd = words[1];
                        match subcmd {
                            "on" | "off" | "reset" => {
                                if end_with_space {
                                    completions.extend(LED_COLORS.iter().map(|s| s.to_string()));
                                } else {
                                    completions.extend(
                                        LED_COLORS
                                            .iter()
                                            .filter(|s| s.starts_with(subcmd))
                                            .map(|s| s.to_string()),
                                    );
                                }
                            }
                            "color" => {
                                if end_with_space {
                                    // After "led color ", suggest "normal" and hex format hint
                                    completions.push("normal".to_string());
                                    completions.push("0xrrbbgg".to_string()); // Optional placeholder suggestion
                                } else {
                                    // Partial input for the color value (e.g., "n" or "0x")
                                    if "normal".starts_with(current_word) {
                                        completions.push("normal".to_string());
                                    }
                                    if "0xrrbbgg"
                                        .to_lowercase()
                                        .starts_with(&current_word.to_lowercase())
                                    {
                                        completions.push("0xrrbbgg".to_string());
                                        // Optional guidance-style suggestion
                                    }
                                }
                            }
                            "brightness" => {
                                if end_with_space {
                                    completions.extend((0..=16).map(|n| n.to_string()));
                                } else {
                                    completions.extend(
                                        (0..=16).map(|n| n.to_string())
                                            .filter(|s| s.starts_with(subcmd))
                                            .map(|s| s.to_string()),
                                    );
                                }
                            }
                            _ => {
                                // Partial input like "led o" → match to subcommands
                                completions.extend(
                                    LED_SUBS
                                        .iter()
                                        .filter(|s| s.starts_with(subcmd))
                                        .map(|s| s.to_string()),
                                );
                            }
                        }
                    }

                    3 => {
                        let subcmd = words[1];

                        match subcmd {
                            "on" | "off" | "reset" => {
                                completions.extend(
                                    LED_INDICES
                                        .iter()
                                        .filter(|s| s.starts_with(current_word))
                                        .map(|s| s.to_string()),
                                );
                            }

                            "color" | "brightness" => {
                                // Expecting an index at this point
                                if end_with_space {
                                    completions.extend(LED_INDICES.iter().map(|s| s.to_string()));
                                } else {
                                    completions.extend(
                                        LED_INDICES
                                            .iter()
                                            .filter(|s| s.starts_with(current_word))
                                            .map(|s| s.to_string()),
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            Some(&"stop") => {
                let mut stop_args: Vec<String> = vec!["all".to_string()];
                if let Ok(lock) = self.threads.lock() {
                    stop_args.extend(lock.get_list_running_apps());
                }
                completions.extend(
                    stop_args
                        .iter()
                        .filter(|s| s.starts_with(current_word))
                        .map(|s| s.to_string()),
                );
            }
            Some(&"list") => {
                completions.extend(
                    LIST_ARGS
                        .iter()
                        .filter(|s| s.starts_with(current_word))
                        .map(|s| s.to_string()),
                );
            }
            Some(&"help") => {
                match words.get(1) {
                    None if end_with_space => {
                        let mut help_args: Vec<String> =
                            COMMANDS.iter().map(|x| x.to_string()).collect();
                        help_args.push("app".to_string());
                        completions.extend(help_args.iter().map(|s| s.to_owned()));
                    }

                    Some(&"app") => {
                        // Provide app/thread names as suggestions for the third word
                        if words.len() == 2 && end_with_space
                            || (words.len() == 3 && !end_with_space)
                        {
                            if let Ok(lock) = self.threads.lock() {
                                let mut name = lock.get_list_all_apps();
                                #[cfg(feature = "led")]
                                name.push(
                                    error_handeler::light_dimmer_thread::PLUGIN_NAME.to_string(),
                                );
                                name.push("errorThread".to_string());

                                completions.extend(
                                    name.iter()
                                        .filter(|s| s.starts_with(current_word))
                                        .map(|s| s.to_string()),
                                );
                            }
                        }
                    }

                    Some(arg) if words.len() == 2 && !end_with_space => {
                        let mut help_args: Vec<String> =
                            COMMANDS.iter().map(|x| x.to_string()).collect();
                        help_args.push("app".to_string());
                        completions.extend(
                            help_args
                                .iter()
                                .filter(|s| s.starts_with(arg))
                                .map(|s| s.to_string()),
                        );
                    }

                    _ => {}
                }
            }
            Some(_cmd) if words.len() == 1 && !end_with_space => {
                completions.extend(
                    COMMANDS
                        .iter()
                        .filter(|c| c.starts_with(current_word))
                        .map(|s| s.to_string()),
                );
            }
            _ => {}
        }

        Ok((start, completions))
    }
}

//TODO cleanup mach statmenst (not urgent)
