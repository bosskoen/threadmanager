use std::{
    env,
    process::exit,
    sync::mpsc,
};

use library::error_handeler::RGB;
use private_lib::*;
use library::rustyline::{self, config::Configurer, error::ReadlineError};
mod cli;
mod private_lib;

const FAILED_TO_START_THREADMANIGER: i32 = 107;
const FAILED_TO_START_CLI: i32 = 109;
const MAX_CLI_HISTORY_SIZE: usize = 100;

fn main() {
    let settings_path;
    let arg = env::args().collect::<Vec<String>>();
    if arg.len() == 1 {
        #[cfg(debug_assertions)]
        {
            #[cfg(windows)]
            {
                if env::current_dir().unwrap().ends_with("scr") {
                    settings_path = r"threadmanager\genaral_setting.toml";
                } else {
                    settings_path = r"..\..\scr\threadmanager\genaral_setting.toml";
                }
            }
            #[cfg(unix)]
            {
                if env::current_dir().unwrap().ends_with("scr") {
                    settings_path = r"threadmanager/genaral_setting.toml";
                } else {
                    settings_path = r"../../scr/threadmanager/genaral_setting.toml";
                }
            }
        }
        #[cfg(not(debug_assertions))]
        {
            settings_path = ""; //TODO fille structere uit zoeken
        }
    } else {
        settings_path = &arg[1];
    }

    let mut rl = rustyline::Editor::<(), _>::new().unwrap_or_else(|err| {
        eprint!("\nFailed to create rustyline editor: {}  in main", err);
        exit(FAILED_TO_START_THREADMANIGER);
    }); // TODO test interupts / drop on exit

    rl.set_max_history_size(MAX_CLI_HISTORY_SIZE)
        .unwrap_or_else(|err| {
            eprint!("\nFailed to set max history size: {}  in main", err);
        });
    rl.set_auto_add_history(true);
    let external_printer = rl.create_external_printer().unwrap();

    let (error_tx, error_rx) = mpsc::channel();
    let (crach_tx, crach_rx) = mpsc::channel::<String>();

    let printer = library::error_handeler::Printer::new(external_printer, error_tx);

    //debug
    #[cfg(debug_assertions)]
    {
        printer.print(&format!("{:?}", arg), RGB::DEBUG());
        let current_dir = env::current_dir().unwrap();
        printer.print(
            &format!("Current working directory: {:?}", current_dir),
            RGB::DEBUG(),
        );
        printer.print(settings_path, RGB::DEBUG());
    }


    printer.print("CLI Plugin Manager", RGB::CYAN());

    let mut open_threads = match Manager::new(printer.clone(), settings_path, crach_tx) {
        Ok(threads) => threads,
        Err(err) => {
            printer.print_error(
                "main",
                &format!("Failed to start threadmaniger: {}", err),
                RGB::ERROR(),
            );
            exit(FAILED_TO_START_THREADMANIGER);
        }
    };
    open_threads.start_error(error_rx);


    let cli = cli::initialise_cli();

    loop {

        crach_rx.try_iter().for_each(|msg| {
            let _ = open_threads.stop_thread(msg);
        });

        match rl.readline("> ") {
            Ok(line) => {
                let input = line.trim();

                let mut args = input.split_whitespace();
                let command = args.next().unwrap_or_default();


                if command == "exit" {
                    break;
                }
                if let Some(func) = cli.get(command) {
                    func(args.collect::<Vec<&str>>().as_slice(), &mut open_threads, &printer);
                } else {
                    printer.print(
                        "Command not found. Type 'help' for a list of commands.",
                        RGB::WHITE(),
                    );
                }
            }
            Err(ReadlineError::Interrupted) => {
                printer.print("CTRL-C pressed, exiting.\n", RGB::WHITE());
                break;
            }
            Err(ReadlineError::Eof) => {
                printer.print("CTRL-D pressed, exiting.\n", RGB::WHITE());
                break; ////
            }
            Err(err) => {
                printer.print_error("main", &format!("Error: {:?}", err), RGB::ERROR());
                break;
            }
        }
    }
    drop(open_threads);
}
