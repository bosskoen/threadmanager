use std::{
    env,
    process::ExitCode,
    sync::{mpsc, Mutex},
};

mod atexit;

use cli::MyCompleter;
use library::error_handeler::{cleanup_static, Printer, RGB};
use library::rustyline::{self, config::Configurer, error::ReadlineError};
use private_lib::*;
mod cli;
mod private_lib;

const FAILED_TO_START_THREADMANIGER: u8 = 107;
const FAILED_TO_START_CLI: u8 = 109;
const FAILED_TO_LOCK_OPEN_THREADS: u8 = 110;

const MAX_CLI_HISTORY_SIZE: usize = 100;

fn main() -> ExitCode {
    let mut atexit = atexit::CleanupRegistry::new();

    atexit.register(|| cleanup_static());

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
            #[cfg(windows)]
            {
                settings_path = r".\settings\genaral_setting.toml";
            }#[cfg(unix)]
            {
                settings_path = r"./settings/genaral_setting.toml";
            } //TODO fille structere uit zoeken
        }
    } else {
        settings_path = &arg[1];
    }

    let mut rl = match rustyline::Editor::<MyCompleter, _>::with_config(
        rustyline::config::Config::builder()
            .bell_style(rustyline::config::BellStyle::None)
            .build(),
    ) {
        Ok(editor) => editor,
        Err(err) => {
            eprint!("\nFailed to create rustyline editor: {}  in main", err);
            return ExitCode::from(FAILED_TO_START_CLI);
        }
    };

    rl.set_max_history_size(MAX_CLI_HISTORY_SIZE)
        .unwrap_or_else(|err| {
            eprint!("\nFailed to set max history size: {}  in main", err);
        });
    rl.set_auto_add_history(true);
    let external_printer = rl.create_external_printer().expect("coudn't get a printer working");

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

    let open_threads = Mutex::new(
        match Manager::new(printer.clone(), settings_path, crach_tx) {
            Ok(threads) => threads,
            Err(err) => {
                printer.print_error(
                    "main",
                    &format!("Failed to start threadmaniger: {}", err),
                    RGB::ERROR(),
                );
                return ExitCode::from(FAILED_TO_START_THREADMANIGER);
            }
        },
    );
    rl.set_helper(Some(cli::MyCompleter::new(&open_threads)));

    match open_threads.lock() {
        Ok(mut threads) => threads.start_error(error_rx),
        Err(err) => {
            printer.print_error(
                "main",
                &format!("Failed to lock open threads: {}", err),
                RGB::ERROR(),
            );
            return ExitCode::from(FAILED_TO_LOCK_OPEN_THREADS);
        }
    }

    let cli = cli::initialise_cli();

    loop {
        {
            let mut thread_data = match open_threads.lock() {
                Ok(data) => data,
                Err(err) => {
                    printer.print_error(
                        "main",
                        &format!("Failed to lock open threads: {}", err),
                        RGB::ERROR(),
                    );
                    return ExitCode::from(FAILED_TO_LOCK_OPEN_THREADS);
                }
            };

            crach_rx.try_iter().for_each(|msg| {
                let _ = thread_data.stop_thread(msg);
            });
        }

        match rl.readline("> ") {
            Ok(line) => {
                let input = line.trim();

                let mut args = input.split_whitespace();
                let command = args.next().unwrap_or_default();

                if command == "exit" {
                    #[cfg(windows)]
                    {
                        if Printer::is_forced_shutdown() {
                            printer.print(
                                "Forced shutdown initiated from other thread, exiting.\n",
                                RGB::ERROR(),
                            );
                        }
                    }
                    break;
                }
                if let Some(func) = cli.get(command) {
                    let mut thread_data = match open_threads.lock() {
                        Ok(data) => data,
                        Err(err) => {
                            printer.print_error(
                                "main",
                                &format!("Failed to lock open threads: {}", err),
                                RGB::ERROR(),
                            );
                            return ExitCode::from(FAILED_TO_LOCK_OPEN_THREADS);
                        }
                    };
                    func(
                        args.collect::<Vec<&str>>().as_slice(),
                        &mut *thread_data,
                        &printer,
                    );
                } else {
                    printer.print(
                        "Command not found. Type 'help' for a list of commands.",
                        RGB::WHITE(),
                    );
                }
            }
            Err(ReadlineError::Interrupted) => {
                #[cfg(unix)]
                {
                    if Printer::is_forced_shutdown() {
                        printer.print(
                            "Forced shutdown initiated from other thread, exiting.\n",
                            RGB::ERROR(),
                        );
                    } else {
                        printer.print("CTRL-C pressed, exiting.\n", RGB::WHITE());
                    }
                }
                #[cfg(windows)]
                {
                    printer.print("CTRL-C pressed, exiting.\n", RGB::WHITE());
                }

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
    ExitCode::SUCCESS
}

//TODO test bz plugin
//TODO set up exe or raspbery
//TODO test led pwm on raspberry

//TODO windows color support ( qof , cargo handles it )
