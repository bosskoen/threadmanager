use std::collections::HashMap;

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
        println!("No thread name provided. Please specify a thread name.");
        return;
    }else if args.len() > 1{
        println!("Multiple thread names provided. Please specify only one thread name.");
        return;
    }
    println!("Attempting to start thread: {}", args[0]);
    if let Err(err) = open_threads.start_new_thread(args[0].to_string()){
        println!("{err}");
        return;
    }
    if open_threads.is_running(args[0]){
        println!("Thread {} is running", args[0]);
    }else{
        println!("Thread {} failed to start", args[0]);
    }
}

fn stop_app(args: &[&str],open_threads: &mut Manager){
    if args.len() == 0{
        println!("No thread name provided. Please specify a thread name.");
        return;
    }else if args.len() > 1{
        println!("Multiple thread names provided. Please specify only one thread name.");
        return;
    }
    if args[0] == "all"{
        println!("Attempting to stop all threads");
        open_threads.stop_all_threads();
        println!("All threads stopped");
        return;
    }
    println!("Attempting to stop thread: {}", args[0]);
    if let Err(err) = open_threads.stop_thread(args[0].to_string()){
        println!("{err}");
        return;
    }
    if open_threads.is_running(args[0]){
        println!("Thread {} failed to stop", args[0]);
    }else{
        println!("Thread {} stopped", args[0]);
    }
}

fn get_status(args: &[&str], open_threads: &mut Manager){
    if args.len() == 0{
        println!("No thread name provided. Please specify a thread name.");
        return;
    }else if args.len() > 1{
        println!("Multiple thread names provided. Please specify only one thread name.");
        return;
    }
    if let Err(err) = open_threads.get_status(args[0].to_string()){
        println!("{err}");
    }
}

fn list_apps(args: &[&str], open_threads: &mut Manager){
    if args.len() == 0{
        println!("No argument provided. Please specify 'running', 'stopped', or 'all'.");
        return;
    }else if args.len() > 1{
        println!("Invalid argument. Please specify 'running', 'stopped', or 'all'.");
        return;
    }
    match args[0] {
        "running" => open_threads.list_threads(Mode::Running),
        "stopped" => open_threads.list_threads(Mode::Stopped),
        "all" => open_threads.list_threads(Mode::All),
        _ => println!("Invalid argument. Please specify 'running', 'stopped', or 'all'.")
    }
}

fn help(args: &[&str], open_threads: &mut Manager){
    if args.len() == 0 {
        // General help message for all commands
        println!("Available commands:\n\
            start <thread name> - Start a thread\n\
            stop <thread name> - Stop a thread\n\
            status <thread name> - Get the status of a thread\n\
            list <running/stopped/all> - List all threads\n\
            help <command>||[app <app name>] - Get help for a specific command");
        return;
    } else if args.len() > 1 {
        if args[0] == "app" {
            if args.len() == 2 {
                // Provide help for a specific app
                if let Err(er) = open_threads.help_message(args[1].to_string()) {
                    println!("{er}");
                }
            } else {
                println!("Invalid argument. Please specify 'app <app name>' to get help for a specific app.");
            }
            return;
        } else {
            // Invalid argument case
            println!("Invalid argument. Please specify 'app <app name>' to get help for a specific app.");
            return;
        }
    }
    // Specific help for a command
    match args[0] {
        "start" => println!("start <thread name> - Start a thread with the specified name"),
        "stop" => println!("stop <thread name> - Stop the thread with the specified name"),
        "status" => println!("status <thread name> - Get the status of a running thread"),
        "list" => println!("list <running/stopped/all> - List threads based on their status (running, stopped, or all)"),
        "help" => println!("help <command>||[app <app name>] - Get help for a specific command or app"),
        _ => println!("Invalid command. Please specify 'start', 'stop', 'status', 'list', or 'help'."),
    }
}

//TODO update logic, exit