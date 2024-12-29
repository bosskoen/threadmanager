use std::{io::{self, Write}, sync::mpsc};

use private_lib::*;
mod private_lib;
mod cli;

fn main() {
    let (error_tx, error_rx) = mpsc::channel();
    let (crach_tx, crach_rx) = mpsc::channel::<String>();
    let mut open_threads = Manager::new(error_tx, r"E:\project\hypixelPI\git\scr\threadmanager\genaral_setting.toml", crach_tx).expect("msg"); //TODO settings and error
    open_threads.start_error(error_rx);
    let cli = cli::initialise_cli();

    loop {
        print!("> ");
        if let Err(err) = io::stdout().flush(){
            println!("Failed to flush stdout: {}", err);
            continue;
        }
        let mut input = String::new();
        if let Err(err) =io::stdin().read_line(&mut input){
            println!("Failed to read line: {}", err);
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
            println!("Command not found. Type 'help' for a list of commands.");
        }
    }
}




