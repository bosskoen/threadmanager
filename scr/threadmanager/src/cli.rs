use std::collections::HashMap;

use library::error_handeler::{print, RGB};
use crate::{private_lib::Manager, Mode};

pub fn initialise_cli() -> HashMap<&'static str, Box<dyn Fn(&[&str], &mut Manager)>>{
    let mut cli = HashMap::new();
    cli.insert("start", Box::new(start_app) as Box<dyn Fn(&[&str], &mut Manager)>);
    cli.insert("stop", Box::new(stop_app) as Box<dyn Fn(&[&str], &mut Manager)>);
    cli.insert("status", Box::new(get_status) as Box<dyn Fn(&[&str], &mut Manager)>);
    cli.insert("list", Box::new(list_apps) as Box<dyn Fn(&[&str], &mut Manager)>);
    cli.insert("help", Box::new(help) as Box<dyn Fn(&[&str], &mut Manager)>);

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
    if let Err(err) = open_threads.start_new_thread(args[0].to_string()){
        print(&format!("{err}"), RGB::WHITE());
        return;
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
        print("All threads stopped", RGB::SUCCESS());
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

fn help(args: &[&str], open_threads: &mut Manager){
    if args.len() == 0 {
        // General help message for all commands
        print("Available commands:\n\
            start <thread name> - Start a thread\n\
            stop <thread name> - Stop a thread\n\
            status <thread name> - Get the status of a thread\n\
            list <running/stopped/all> - List all threads\n\
            help <command>||[app <app name>] - Get help for a specific command", RGB::WHITE());
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
        "help" => print("help <command>||[app <app name>] - Get help for a specific command or app", RGB::WHITE()),
        _ => print("Invalid command. Please specify 'start', 'stop', 'status', 'list', 'help' or 'app <app name>'", RGB::WHITE()),
    }
}

//TODO update logic, reload settings