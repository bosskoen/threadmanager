use std::{io::{self, Write}, process::exit, sync::mpsc};

use library::error_handeler::{print_error,print, RGB};
use private_lib::*;
mod private_lib;
mod cli;

fn main() {
    //TODO custor setigns file
    print("CLI Plugin Manager", RGB::CYAN());
    let (error_tx, error_rx) = mpsc::channel();
    let (crach_tx, crach_rx) = mpsc::channel::<String>();
    let mut open_threads = match Manager::new(error_tx, r"E:\project\hypixelPI\git\scr\threadmanager\genaral_setting.toml", crach_tx){
        Ok(threads) => threads,
        Err(err) => {
            print_error("main", &format!("Failed to start threadmaniger: {}", err), RGB::ERROR());
            exit(107);
        }
    };
    open_threads.start_error(error_rx);
    let cli = cli::initialise_cli();

    loop {
        print!("> ");
        if let Err(err) = io::stdout().flush(){
            print_error("main", &format!("Failed to flush stdout: {}", err), RGB::ERROR());
            continue;
        }
        let mut input = String::new();
        if let Err(err) = io::stdin().read_line(&mut input){
            print_error("main", &format!("Failed to read line: {}", err), RGB::ERROR());
            continue;
        }

        crach_rx.try_iter().for_each(|msg| {let _ = open_threads.stop_thread(msg); });

        let input = input.trim();
        let mut args = input.split_whitespace();
        let command = args.next().unwrap_or_default();
        if command == "exit" {
            break;
        }
        if let Some(func) = cli.get(command) {
            func(args.collect::<Vec<&str>>().as_slice(), &mut open_threads);
        } else {
            print("Command not found. Type 'help' for a list of commands.", RGB::WHITE());
        }
    }
}




