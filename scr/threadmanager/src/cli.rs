use std::collections::HashMap;
use library::error_handeler::{self, LedNumber};
#[allow(unused_imports)]
use library::error_handeler::{print, print_error, ErrorOperation, LedOption, RGB};

use crate::{private_lib::Manager, Mode};

pub fn initialise_cli() -> HashMap<&'static str, Box<dyn Fn(&[&str], &mut Manager)>>{
    let mut cli = HashMap::new();
    cli.insert("start", Box::new(start_app) as Box<dyn Fn(&[&str], &mut Manager)>);
    cli.insert("stop", Box::new(stop_app) as Box<dyn Fn(&[&str], &mut Manager)>);
    cli.insert("status", Box::new(get_status) as Box<dyn Fn(&[&str], &mut Manager)>);
    cli.insert("list", Box::new(list_apps) as Box<dyn Fn(&[&str], &mut Manager)>);
    cli.insert("help", Box::new(help) as Box<dyn Fn(&[&str], &mut Manager)>);
    cli.insert("setting", Box::new(settings) as Box<dyn Fn(&[&str], &mut Manager)>);
    cli.insert("led", Box::new(led) as Box<dyn Fn(&[&str], &mut Manager)>);

    cli
}

fn start_app(args: &[&str], open_threads: &mut Manager){
    if args.len() == 0{
        print("No thread name provided. Please specify a thread name.", RGB::WHITE());
        return;
    }else if args.len() > 1{
        print("Multiple thread names provided. Please specify only one thread name.", RGB::WHITE());
        return;
    }
    print(&format!("Attempting to start thread: {}", args[0]), RGB::WHITE());

    if args[0] == error_handeler::light_dimmer_thread::PLUGIN_NAME{
        open_threads.start_light_dimmer();
    }else {
        if let Err(err) = open_threads.start_new_thread(args[0].to_string()){
            print(&format!("{err}"), RGB::WHITE());
            return;
        }
    }
    if open_threads.is_running(args[0]){
        print(&format!("Thread {} is running", args[0]), RGB::SUCCESS());
    }else{
        print(&format!("Thread {} failed to start", args[0]), RGB::ERROR());
    }
}

fn stop_app(args: &[&str],open_threads: &mut Manager){
    if args.len() == 0{
        print("No thread name provided. Please specify a thread name.", RGB::WHITE());
        return;
    }else if args.len() > 1{
        print("Multiple thread names provided. Please specify only one thread name.", RGB::WHITE());
        return;
    }
    if args[0] == "all"{
        print("Attempting to stop all threads", RGB::WHITE());
        open_threads.stop_all_threads();
        print("Note: 'errorThread' is a permanent system thread and cannot be stopped.", RGB::TRACE());
        #[cfg(feature = "led")]
        print(&format!("{} can't be stopped this way use the manual command to stop it.", error_handeler::light_dimmer_thread::PLUGIN_NAME), RGB::TRACE());
        print("All threads stopped", RGB::SUCCESS());
        return;
    }  if args[0] == "errorThread" {
        print("Can't stop error thread", RGB::ERROR());
        return;
    }
    print(&format!("Attempting to stop thread: {}", args[0]), RGB::WHITE());
    if let Err(err) = open_threads.stop_thread(args[0].to_string()){
        print(&format!("{err}"), RGB::WHITE());
        return;
    }
    if open_threads.is_running(args[0]){
        print(&format!("Thread {} failed to stop", args[0]), RGB::ERROR());
    }else{
        print(&format!("Thread {} stopped", args[0]), RGB::SUCCESS());
    }
}

fn get_status(args: &[&str], open_threads: &mut Manager){
    if args.len() == 0{
        print("No thread name provided. Please specify a thread name.", RGB::WHITE());
        return;
    }else if args.len() > 1{
        print("Multiple thread names provided. Please specify only one thread name.", RGB::WHITE());
        return;
    }
    if let Err(err) = open_threads.get_status(args[0].to_string()){
        print( &format!("{err}"), RGB::WHITE());
    }
}

fn list_apps(args: &[&str], open_threads: &mut Manager){
    if args.len() == 0{
        print("No argument provided. Please specify 'running', 'stopped', or 'all'.", RGB::WHITE());
        return;
    }else if args.len() > 1{
        print("Invalid argument. Please specify 'running', 'stopped', or 'all'.", RGB::WHITE());
        return;
    }
    match args[0] {
        "running" => open_threads.list_threads(Mode::Running),
        "stopped" => open_threads.list_threads(Mode::Stopped),
        "all" => open_threads.list_threads(Mode::All),
        _ => print("Invalid argument. Please specify 'running', 'stopped', or 'all'.", RGB::WHITE())
    }
}

fn help(args: &[&str], open_threads: &mut Manager) {
    if args.len() == 0 {
        // General help message for all commands
        print("Available commands:" , RGB::NOTICE());

        print("start <thread name> - Start a thread\n\
            stop <thread name> - Stop a thread\n\
            status <thread name> - Get the status of a thread\n\
            list <running/stopped/all> - List all threads\n\
            led <command> [args] - Control the LED system\n\
            settings <command> - Configure settings\n\
            help <command> || [app <app name>] - Get help for a specific command",
            RGB::WHITE());
        return;
    } else if args.len() > 1 {
        if args[0] == "app" {
            if args.len() == 2 {
                // Provide help for a specific app
                if let Err(er) = open_threads.help_message(args[1].to_string()) {
                    print(&format!("{er}"), RGB::WHITE());
                }
            } else {
                print("Invalid argument. Please specify 'app <app name>' to get help for a specific app.", RGB::WHITE());
            }
            return;
        } else {
            // Invalid argument case
            print("Invalid argument. Please specify 'app <app name>' to get help for a specific app.", RGB::WHITE());
            return;
        }
    }
    // Specific help for a command
    match args[0] {
        "start" => print("start <thread name> - Start a thread with the specified name", RGB::WHITE()),
        "stop" => print("stop <thread name> - Stop the thread with the specified name", RGB::WHITE()),
        "status" => print("status <thread name> - Get the status of a running thread", RGB::WHITE()),
        "list" => print("list <running/stopped/all> - List threads based on their status (running, stopped, or all)", RGB::WHITE()),
        "settings" => print("settings reload - Reload configuration settings", RGB::WHITE()),
        "led" => print("led <on/off/reset/color/brightness> [args] - Control LED colors and brightness\n\
                        Commands:\n\
                        on/off/reset [red/green/blue/all] - Control specific colors\n\
                        color <hex color (0xRRGGBB)> - Set LED color\n\
                        brightness <0-255> - Set brightness level", RGB::WHITE()),
        "help" => print("help <command> || [app <app name>] - Get help for a specific command or app", RGB::WHITE()),
        _ => print("Invalid command. Please specify 'start', 'stop', 'status', 'list', 'led', 'settings', 'help' or 'app <app name>'", RGB::WHITE()),
    }
}

fn settings(args: &[&str], open_threads: &mut Manager) {
    if args.len() == 0 {
        print("No argument provided. Available command:\n\
               reload - Reload configuration settings", RGB::WHITE());
        return;
    } else if args.len() > 1 {
        print("Invalid argument. Only 'reload' is supported.", RGB::WHITE());
        return;
    }

    match args[0] {
        "reload" => open_threads.reload_settings(),
        _ => print("Invalid argument. Please specify 'reload'.", RGB::WHITE()),
    }
}

#[cfg(not(feature = "led"))]
fn led(args: &[&str], open_threads: &mut Manager){
    print("LED's are disabled in this build", RGB::WHITE());
}

#[cfg (feature = "led")]
fn led(args: &[&str], open_threads: &mut Manager){
    if args.len() == 0 {
        print("Available LED commands:", RGB::NOTICE());
        print( "led on [red/green/blue/all] [0-4/all]\n\
        led off [red/green/blue/all] [0-4/all]\n\
        led reset [red/green/blue/all] [0-4/all]\n\
        led color <hex color (0xRRGGBB)> [0-4/all]\n\
        led brightness <0-16> [0-4/all]",
       RGB::WHITE());
        return;
    } else if args.len() > 3 {
        print("To manny arguments.", RGB::WHITE());
        return;
    }
    match args[0] {
        "off" => {
            if args.len() == 3{
                let oper;
                let led_num;
                match args[1] {
                    "red" => oper = LedOption::Red,
                    "green" => oper = LedOption::Green,
                    "blue" => oper = LedOption::Blue,
                    "all" => oper = LedOption::All,
                    _ => {print("Invalid argument. please specify a color and LED\nled off [red/green/blue/all] [1-4/all]", RGB::WHITE());
                        return;},
                }
                match args[2]{
                    "0" => led_num = LedNumber::LED1,
                    "1" => led_num = LedNumber::LED2,
                    "2" => led_num = LedNumber::LED3,
                    "3" => led_num = LedNumber::LED4,
                    "4" => led_num = LedNumber::LED5,
                    "all" => led_num = LedNumber::ALL,
                    _ => {print("Invalid argument. please specify a color and LED\nled off [red/green/blue/all] [1-4/all]", RGB::WHITE());
                        return;},
                }   
                if let Err(_) = open_threads.error_sender.send(ErrorOperation::OffColor(oper, led_num)){
                    print_error("main", "failed to send led command", RGB::CRITICAL_ERROR());
                }
            } else{
                print("Invalid argument. please specify a color and LED\nled off [red/green/blue/all] [1-4/all]", RGB::WHITE());
                return;
            }
        },
        "on" => {
            if args.len() == 3{
                let oper;
                let led_num;
                match args[1] {
                    "red" => oper = LedOption::Red,
                    "green" => oper = LedOption::Green,
                    "blue" => oper = LedOption::Blue,
                    "all" => oper = LedOption::All,
                    _ => {print("Invalid argument. please specify a color and LED\nled on [red/green/blue/all] [1-4/all]", RGB::WHITE());
                        return;},
                }
                match args[2]{
                    "0" => led_num = LedNumber::LED1,
                    "1" => led_num = LedNumber::LED2,
                    "2" => led_num = LedNumber::LED3,
                    "3" => led_num = LedNumber::LED4,
                    "4" => led_num = LedNumber::LED5,
                    "all" => led_num = LedNumber::ALL,
                    _ => {print("Invalid argument. please specify a color and LED\nled on [red/green/blue/all] [1-4/all]", RGB::WHITE());
                        return;},
                }   
                if let Err(_) = open_threads.error_sender.send(ErrorOperation::OnColor(oper, led_num)){
                    print_error("main", "failed to send led command", RGB::CRITICAL_ERROR());
                }
            } else{
                print("Invalid argument. please specify a color and LED\nled on [red/green/blue/all] [1-4/all]", RGB::WHITE());
                return;
            }
        },
        "reset" => {
            if args.len() == 3{
                let oper;
                let led_num;
                match args[1] {
                    "red" => oper = LedOption::Red,
                    "green" => oper = LedOption::Green,
                    "blue" => oper = LedOption::Blue,
                    "all" => oper = LedOption::All,
                    _ => {print("Invalid argument. please specify a color and LED\nled reset [red/green/blue/all] [1-4/all]", RGB::WHITE());
                        return;},
                }
                match args[2]{
                    "0" => led_num = LedNumber::LED1,
                    "1" => led_num = LedNumber::LED2,
                    "2" => led_num = LedNumber::LED3,
                    "3" => led_num = LedNumber::LED4,
                    "4" => led_num = LedNumber::LED5,
                    "all" => led_num = LedNumber::ALL,
                    _ => {print("Invalid argument. please specify a color and LED\nled reset [red/green/blue/all] [1-4/all]", RGB::WHITE());
                        return;},
                } 
                if let Err(_) = open_threads.error_sender.send(ErrorOperation::RestColor(oper, led_num)){
                    print_error("main", "failed to send led command", RGB::CRITICAL_ERROR());
                }
            } else{
                print("Invalid argument. please specify a color and LED\nled reset [red/green/blue/all] [1-4/all]", RGB::WHITE());
                return;
            }
        },
        "color" => {
            if args.len() == 3{
                let color;
                let led_num;
                match args[2] {
                    "0" => led_num = LedNumber::LED1,
                    "1" => led_num = LedNumber::LED2,
                    "2" => led_num = LedNumber::LED3,
                    "3" => led_num = LedNumber::LED4,
                    "4" => led_num = LedNumber::LED5,
                    "all" => led_num = LedNumber::ALL,
                    _ => {print("Invalid argument. please specify a color and LED\nled off (0xrrggbb|| 0XRRGGBB)[1-4/all]", RGB::WHITE());
                        return;},
                }
                if args[1].len() == 8 && args[1].to_lowercase().starts_with("0x") {
                    let trimde  = &args[1][2..];
                    color = RGB::from_hex(u32::from_str_radix(trimde, 16).unwrap_or(0));
                }else{
                    print("Invalid argument. Please specify a color as hex (0xrrggbb|| 0XRRGGBB).", RGB::WHITE());
                    return;
                }
                if let Err(_) = open_threads.error_sender.send(ErrorOperation::ChangeLed(color, false, led_num)){
                    print_error("main", "failed to send led command", RGB::CRITICAL_ERROR());
                }
            } else{
                print("Invalid argument. please specify a color and LED\nled off (0xrrggbb|| 0XRRGGBB)[1-4/all]", RGB::WHITE());
                return;
            }
        },
        "brightness" => {
            if args.len() == 3{
                let new_level;
                let led_num;
                match args[2] {
                    "0" => led_num = LedNumber::LED1,
                    "1" => led_num = LedNumber::LED2,
                    "2" => led_num = LedNumber::LED3,
                    "3" => led_num = LedNumber::LED4,
                    "4" => led_num = LedNumber::LED5,
                    "all" => led_num = LedNumber::ALL,
                    _ => {print("Invalid argument. please specify a brightness and LED\nled off [0-16] [1-4/all]", RGB::WHITE());
                        return;},
                }
                match args[1].parse::<u8>() {
                    Ok(value) => new_level = value,
                    Err(_) => {print("invalit input, it needs to be a number", RGB::WHITE()); return;},
                }
                if let Err(_) = open_threads.error_sender.send(ErrorOperation::CangeBrighness(new_level, led_num)){
                    print_error("main", "failed to send led command", RGB::CRITICAL_ERROR());
                }
            } else{
                print("Invalid argument. Please specify a brightness level and LED [0 - 16].\nled brightness [0-16] [0-4/all]", RGB::WHITE());
                return;
            }
        },
        "help" => {
            print("LED Command Help:\n", RGB::NOTICE());
            print("led on/off/reset [red/green/blue/all] [0-4/all] - Control specific colors\n\
                   led color <hex color (0xRRGGBB)> [0-4/all] - Set LED color\n\
                   led brightness <0-16> [0-4/all] - Set brightness level",
                   RGB::WHITE());
        },
        _ => print("Invalid LED command. Use 'led help' to see available commands.", RGB::WHITE()),
    }
}

//TODO cleanup mach statmenst
//TODO add completer for each command