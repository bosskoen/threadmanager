use libloading::{Library, Symbol};
use library::{
    error_handeler::{self, error_catchloop, ErrorOperation, LedOption, Printer, RGB},
    impl_status, toml, ErrorThreadDownError, Status,
};
use serde::Deserialize;
use std::{
    collections::HashMap,
    error::Error,
    fmt, fs, panic,
    sync::{
        atomic::AtomicBool,
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

pub struct IniStatus {}
impl_status!(IniStatus, |_| String::from("initialization status"));

const ERROR_THREAD_DOWN: i32 = 105;
const CRASH_NOTIFIER_DOWN: i32 = 106;
const ERROR_THREAD_RESEVER_DOWN: i32 = 103;
const ERROR_PENICED: i32 = 104;

pub struct Manager {
    map: HashMap<String, ThreadHandel>,
    pub printer: Printer,
    settings: Settings,
    settings_path: String,
    crash_notifier: Sender<String>,
}

impl Drop for Manager {
    fn drop(&mut self) {
        self.printer.print("Stopping all threads", RGB::NOTICE());
        self.stop_all_threads();
        self.printer.print("Stopping error thread", RGB::NOTICE());
        self.stop_error();
        Printer::reset_color();
    }
}

impl Manager {
    pub fn new(
        printer: Printer,
        settings_path: &str,
        crash_notifier: Sender<String>,
    ) -> Result<Self, ManagerError> {
        let settings = Settings::deserialize(settings_path)?;
        Ok(Self {
            map: HashMap::new(),
            printer,
            settings,
            crash_notifier,
            settings_path: settings_path.to_string(),
        })
    }

    pub fn start_new_thread(&mut self, name: String) -> Result<(), ManagerError> {
        let app_setting = if let Some(value) = self.settings.apps.get(&name) {
            value
        } else {
            return Err(ManagerError::AppDoesntExist(name));
        };
        let app_setting_path = app_setting.setting_path.clone();
        let dll_path = app_setting.dll_path.clone();

        if self.map.contains_key(&name) {
            return Err(ManagerError::AppAlreadyRunning);
        }
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_clone = Arc::clone(&stop_flag);
        let status: Arc<Mutex<Box<dyn Status>>> = Arc::new(Mutex::new(Box::new(IniStatus {})));
        let status_clone = Arc::clone(&status);
        let printer_clone = self.printer.clone();
        let name_clone = name.clone();
        let crash_notifier = self.crash_notifier.clone();

        let handle = thread::spawn(move || {
            thread_logic(
                &dll_path,
                name_clone,
                printer_clone,
                stop_flag_clone,
                status_clone,
                app_setting_path,
                crash_notifier,
            );
        });

        self.map
            .insert(name, ThreadHandel::new(handle, stop_flag, status));
        Ok(())
    }

    pub fn stop_thread(&mut self, name: String) -> Result<(), ManagerError> {
        if let Some(thread) = self.map.remove(&name) {
            thread
                .stop_flag
                .store(true, std::sync::atomic::Ordering::Relaxed);
            if let Err(x) = thread.handle.join() {
                self.printer.print_error(
                    "Manager",
                    &format!("{} panicked while closing with error\n{:?}", name, x),
                    RGB::ERROR(),
                );
            }
            Ok(())
        } else {
            if self.settings.apps.contains_key(&name) {
                Err(ManagerError::AppIsntRunning(name))
            } else {
                Err(ManagerError::AppDoesntExist(name))
            }
        }
    } //TOOD stop error thread is een loop

    pub fn is_running(&self, name: &str) -> bool {
        self.map.contains_key(name)
    }

    pub fn stop_all_threads(&mut self) {
        let mut to_keep = Vec::new();
        let mut to_remove = Vec::new();

        for (name, handle) in self.map.drain() {
            if name == "errorThread" || name == error_handeler::light_dimmer_thread::PLUGIN_NAME {
                to_keep.push((name, handle)); // Save to reinsert later
                continue;
            }

            self.printer
                .print(&format!("stopping: {name}"), RGB::NOTICE());
            handle
                .stop_flag
                .store(true, std::sync::atomic::Ordering::Relaxed);
            to_remove.push((name, handle.handle));

            println!();
        }

        // Reinsert the kept entries back into the map
        for (name, handle) in to_keep {
            self.map.insert(name, handle);
        }

        for (name, handle) in to_remove {
            if let Err(x) = handle.join() {
                self.printer.print_error(
                    "Manager",
                    &format!("{} panicked while closing with error\n{:?}", name, x),
                    RGB::ERROR(),
                );
            }
        }
    }

    pub fn help_message(&self, name: String) -> Result<(), ManagerError> {
        if name == "errorThread" {
            #[cfg(feature = "led")]
            self.printer.print("errorThread: Background thread for centralized error handling and LED status control.

- Logs and displays error and status messages from various system components.
- Controls LED indicators to reflect system state (e.g. errors, warnings, operational feedback).
- Reacts to system-wide status updates such as color changes, brightness levels, or LED resets.

Note: This thread runs continuously in the background and cannot be shut down manually.", RGB::WHITE());
            #[cfg(not(feature = "led"))]
            self.printer.print(
                "errorThread: Background thread for centralized error handling.

- Logs and displays error and status messages from various system components.
- Keeps track of system state for diagnostics and user feedback.

Note: This thread runs continuously in the background and cannot be shut down manually.",
                RGB::WHITE(),
            );

            return Ok(());
        } else if name == error_handeler::light_dimmer_thread::PLUGIN_NAME {
            #[cfg(feature = "led")]
            self.printer.print(&format!("{}: Adjusts LED brightness automatically based on time of day. Can be stopped but not recommended.", error_handeler::light_dimmer_thread::PLUGIN_NAME), RGB::DEBUG());
            #[cfg(not(feature = "led"))]
            self.printer.print(
                &format!(
                    "{}:  Disabled because LED support is not enabled.",
                    error_handeler::light_dimmer_thread::PLUGIN_NAME
                ),
                RGB::WHITE(),
            );
            return Ok(());
        } else if let Some(setting) = self.settings.apps.get(&name) {
            self.printer.print(&setting.help_message, RGB::WHITE());
            Ok(())
        } else {
            Err(ManagerError::AppDoesntExist(name))
        }
    }

    pub fn list_threads(&self, mode: Mode) {
        self.printer.print(
            &format!(
                "All {} threads:",
                match mode {
                    Mode::All => "running and stopped",
                    Mode::Running => "running",
                    Mode::Stopped => "stopped",
                }
            ),
            RGB::NOTICE(),
        );
        match mode {
            Mode::All => {
                for (_, setting) in self.settings.apps.iter() {
                    self.printer.print(&setting.name, RGB::WHITE());
                }
                self.printer.print("errorThread", RGB::TRACE());
                #[cfg(feature = "led")]
                self.printer.print(
                    error_handeler::light_dimmer_thread::PLUGIN_NAME,
                    RGB::TRACE(),
                );
            }
            Mode::Running => {
                for (name, _) in self.map.iter() {
                    self.printer.print(&name, RGB::WHITE());
                }
            }
            Mode::Stopped => {
                let running: Vec<&String> = self
                    .settings
                    .apps
                    .keys()
                    .filter(|name| !self.map.contains_key(*name))
                    .collect();
                for name in running {
                    self.printer.print(&name, RGB::WHITE());
                }
            }
        }
    }

    pub fn start_error(&mut self, error_receiver: Receiver<ErrorOperation>) {
        let status: Arc<Mutex<Box<dyn Status>>> = Arc::new(Mutex::new(Box::new(IniStatus {})));
        let status_clone = Arc::clone(&status);
        let printer_clone = self.printer.clone();

        let handle = thread::spawn(move || error_catchloop(error_receiver, printer_clone, status));

        self.map.insert(
            String::from("errorThread"),
            ThreadHandel::new(handle, Arc::new(AtomicBool::new(false)), status_clone),
        );

        #[cfg(feature = "led")]
        self.start_light_dimmer();
    }

    #[cfg(not(feature = "led"))]
    pub fn start_light_dimmer(&mut self) {
        self.printer.print(
            "LED feature is not enabled, light dimmer will not be started",
            RGB::ALERT(),
        );
    }

    #[cfg(feature = "led")]
    pub fn start_light_dimmer(&mut self) {
        let status: Arc<Mutex<Box<dyn Status>>> = Arc::new(Mutex::new(Box::new(IniStatus {})));
        let status_clone = Arc::clone(&status);
        let stopflag = Arc::new(AtomicBool::new(false));
        let stopflag_clone = Arc::clone(&stopflag);
        let printer_clone = self.printer.clone();
        let handle = thread::spawn(move || {
            error_handeler::light_dimmer_thread::start_light_dim(printer_clone, stopflag, status);
        });

        self.map.insert(
            error_handeler::light_dimmer_thread::PLUGIN_NAME.to_string(),
            ThreadHandel::new(handle, stopflag_clone, status_clone),
        );
    }

    pub fn stop_error(&mut self) {
        if let Err(_) = self.printer.send(ErrorOperation::StopErr, "Manager") {
            self.printer.print_error(
                "Manager",
                "ErrorThread's receiver has been dropped too early",
                RGB::ERROR(),
            );
            self.printer.print(
                &format!("exited with exit code {}", ERROR_THREAD_RESEVER_DOWN),
                RGB::WHITE(),
            );
            Printer::close_program();
        }
        if let Some(thread) = self.map.remove(&String::from("errorThread")) {
            if let Err(x) = thread.handle.join() {
                self.printer.print_error(
                    "Manager",
                    &format!("ErrorThread panicked while closing with error\n{:?}", x),
                    RGB::ERROR(),
                );
                self.printer.print(
                    &format!("exited with exit code {}", ERROR_PENICED),
                    RGB::WHITE(),
                );
                Printer::close_program();
            }
        }
        if self
            .map
            .contains_key(&error_handeler::light_dimmer_thread::PLUGIN_NAME.to_string())
        {
            if let Some(thread) = self
                .map
                .remove(&error_handeler::light_dimmer_thread::PLUGIN_NAME.to_string())
            {
                self.printer
                    .print("Stopping light dimmer thread", RGB::NOTICE());
                thread
                    .stop_flag
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                if let Err(x) = thread.handle.join() {
                    self.printer.print_error(
                        "Manager",
                        &format!("ErrorThread panicked while closing with error\n{:?}", x),
                        RGB::ERROR(),
                    );
                }
            }
        }
    }
    pub fn get_status(&self, thread_name: String) -> Result<(), ManagerError> {
        if let Some(handle) = self.map.get(&thread_name) {
            printstatus(&handle.status, &self.printer);
        } else {
            if self.settings.apps.contains_key(&thread_name) {
                return Err(ManagerError::AppIsntRunning(thread_name));
            } else {
                return Err(ManagerError::AppDoesntExist(thread_name));
            }
        }
        Ok(())
    }

    pub fn reload_settings(&mut self) {
        self.settings = match Settings::deserialize(&self.settings_path) {
            Ok(setting) => setting,
            Err(_) => {
                self.printer
                    .print("failed to get new settings", RGB::ERROR());
                return;
            }
        }
    }

    pub fn get_list_all_apps(&self) -> Vec<String> {
        self.settings.apps.keys().cloned().collect()
    }

    pub fn get_list_running_apps(&self) -> Vec<String> {
        self.map.keys().cloned().collect()
    }

    pub fn get_list_stopped_apps(&self) -> Vec<String> {
        self.settings
            .apps
            .keys()
            .filter(|name| !self.map.contains_key(*name))
            .cloned()
            .collect()
    }
}

pub enum Mode {
    All,
    Running,
    Stopped,
}

pub struct ThreadHandel {
    pub handle: JoinHandle<()>,
    pub stop_flag: Arc<AtomicBool>,
    status: Arc<Mutex<Box<dyn Status>>>,
}

impl ThreadHandel {
    pub fn new(
        handle: JoinHandle<()>,
        stop_flag: Arc<AtomicBool>,
        status: Arc<Mutex<Box<dyn Status>>>,
    ) -> Self {
        Self {
            handle,
            stop_flag,
            status,
        }
    }
}

fn printstatus(status: &Arc<Mutex<Box<dyn Status>>>, printer: &Printer) {
    if let Ok(x) = status.lock() {
        printer.print(&(*x).format(), RGB::INFO());
    } else {
        printer.print_error(
            "Manager",
            "Error while printing status: lock failed",
            RGB::ERROR(),
        );
    }
}

fn thread_logic(
    dll_path: &str,
    name: String,
    printer: Printer,
    stop_flag: Arc<AtomicBool>,
    status: Arc<Mutex<Box<dyn Status>>>,
    app_setting_path: String,
    crash_notifier: Sender<String>,
) {
    unsafe {
        if fs::metadata(&dll_path).is_err() {
            report_error(
                &printer,
                &crash_notifier,
                name,
                format!("File not found: \"{dll_path}\""),
            );
            return;
        }

        #[cfg(windows)]
        if !dll_path.ends_with(".dll") {
            report_error(
                &printer,
                &crash_notifier,
                name,
                format!("File is not a dynamic link library: \"{dll_path}\""),
            );
            return;
        }
        #[cfg(unix)]
        if !dll_path.ends_with(".so") {
            report_error(
                &printer,
                &crash_notifier,
                name,
                format!("File is not a shared object: \"{dll_path}\""),
            );
            return;
        }

        if fs::metadata(&app_setting_path).is_err() {
            report_error(
                &printer,
                &crash_notifier,
                name,
                format!("File not found: \"{app_setting_path}\""),
            );
            return;
        }
        let lib = match Library::new(dll_path) {
            Ok(value) => value,
            Err(err) => {
                report_error(
                    &printer,
                    &crash_notifier,
                    name,
                    format!("Error while loading library: \"{err}\""),
                );
                return;
            }
        };
        let start: Symbol<
            fn(
                Printer,
                Arc<AtomicBool>,
                Arc<Mutex<Box<dyn Status>>>,
                String,
            ) -> Result<(), Box<dyn Error>>,
        > = match lib.get(b"start") {
            Ok(value) => value,
            Err(err) => {
                report_error(
                    &printer,
                    &crash_notifier,
                    name,
                    format!("Error while loading start function: {err}"),
                );
                return;
            }
        };
        let printer_clone = printer.clone();
        let status_clone = status.clone();

        match panic::catch_unwind(|| {
            start(printer_clone, stop_flag, status_clone, app_setting_path)
        }) {
            Ok(result) => {
                match result {
                    Ok(_) => {
                        printer.print(
                            &format!("{} has stopped gracefully\nlast status:", &name),
                            RGB::INFO(),
                        );
                        printstatus(&status, &printer);
                    }
                    Err(err) => {
                        if let Some(fatal_error) = err.downcast_ref::<ErrorThreadDownError>() {
                            printer.print_error(
                                "Manager",
                                &format!("{}", fatal_error),
                                RGB::CRITICAL_ERROR(),
                            );
                            printer.print(
                                &format!("exited with exit code {}", ERROR_THREAD_DOWN),
                                RGB::WHITE(),
                            );
                            Printer::close_program();
                        } else {
                            if let Err(_) = printer.send(
                                ErrorOperation::PrintAndChangeLedError(
                                    name.clone(),
                                    format!("Thread stopped with errors {}", err),
                                    RGB::WARNING(),
                                    RGB::RED(),
                                    error_handeler::LedNumber::LED1,
                                ),
                                &name,
                            ) {
                                printer.print_error("Manager", &format!("Error while sending error: {} Thread stopped with errors {}", name, err), RGB::CRITICAL_ERROR());
                                printer.print(
                                    &format!("exited with exit code {}", ERROR_THREAD_DOWN),
                                    RGB::WHITE(),
                                );
                                Printer::close_program();
                            }
                        }
                        send_crash_notifier(&crash_notifier, name.clone(), &printer);
                    }
                }
            }
            Err(error) => {
                if let Err(_) = printer.send(
                    ErrorOperation::PrintAndChangeLedError(
                        name.clone(),
                        format!("Thread panicked unexpectedly: {:?}", error),
                        RGB::CRITICAL_ERROR(),
                        RGB::RED(),
                        error_handeler::LedNumber::LED1,
                    ),
                    &name,
                ) {
                    printer.print_error(
                        "Manager",
                        &format!(
                            "Error while sending error: {} Thread panicked unexpectedly: {:?}",
                            name, error
                        ),
                        RGB::CRITICAL_ERROR(),
                    );
                    printer.print(
                        &format!("exited with exit code {}", ERROR_THREAD_DOWN),
                        RGB::WHITE(),
                    );
                    Printer::close_program();
                }
                if let Err(_) = printer.send(
                    ErrorOperation::OnColor(LedOption::Red, error_handeler::LedNumber::LED1),
                    &name,
                ) {
                    printer.print_error(
                        "Manager",
                        &format!(
                            "Error while sending error: {} Thread panicked unexpectedly: {:?}",
                            name, error
                        ),
                        RGB::CRITICAL_ERROR(),
                    );
                    printer.print(
                        &format!("exited with exit code {}", ERROR_THREAD_DOWN),
                        RGB::WHITE(),
                    );
                    Printer::close_program();
                }
                send_crash_notifier(&crash_notifier, name.clone(), &printer);
            }
        }
        match status.lock() {
            Ok(mut lock) => {
                (*lock) = Box::new(IniStatus {});
            }
            Err(_) => {
                printer.print_error(
                    &name,
                    "Couldn't reset status to initial state",
                    RGB::CRITICAL_ERROR(),
                );
            }
        }
    }
}

fn report_error(
    printer: &Printer,
    crash_notifier: &Sender<String>,
    name: String,
    error_message: String,
) {
    if let Err(_) = printer.send(
        ErrorOperation::PrintError(name.clone(), error_message.clone(), RGB::ERROR()),
        &name,
    ) {
        printer.print_error(
            "Manager",
            &format!("Error while sending error from {name}: {error_message}"),
            RGB::CRITICAL_ERROR(),
        );
        printer.print(
            &format!("exited with exit code {}", ERROR_THREAD_DOWN),
            RGB::WHITE(),
        );
        Printer::close_program();
    }
    send_crash_notifier(crash_notifier, name, printer);
}

fn send_crash_notifier(crash_notifier: &Sender<String>, name: String, printer: &Printer) {
    if let Err(_) = crash_notifier.send(name.clone()) {
        printer.print_error(
            "Manager",
            &format!("Error while notifying main of unexpected crash in {name}"),
            RGB::CRITICAL_ERROR(),
        );
        printer.print(
            &format!("exited with exit code {}", CRASH_NOTIFIER_DOWN),
            RGB::WHITE(),
        );
        Printer::close_program();
    }
}

#[derive(Deserialize)]
struct Settings {
    //TODO logic for downloading plugins (maby feature)
    #[allow(dead_code)]
    update: HashMap<String, String>,

    apps: HashMap<String, AppSetting>,
}

#[derive(Deserialize)]
struct AppSetting {
    name: String,
    dll_path: String,
    setting_path: String,
    help_message: String,
}

impl Settings {
    fn deserialize(setting_path: &str) -> Result<Settings, ManagerError> {
        let text = fs::read_to_string(setting_path).map_err(|_| ManagerError::FileReadError)?;
        let mut sett =
            toml::from_str::<Settings>(&text).map_err(|_| ManagerError::TOMLReadError)?;
        sett.apps = sett
            .apps
            .into_iter()
            .map(|(key, value)| (key.strip_prefix("apps.").unwrap_or(&key).to_string(), value))
            .collect();
        Ok(sett)
    }
}

#[derive(Debug)]
pub enum ManagerError {
    FileReadError,
    TOMLReadError,
    AppDoesntExist(String),
    AppAlreadyRunning,
    AppIsntRunning(String),
}

impl fmt::Display for ManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManagerError::FileReadError => write!(f, "FILE_READ_ERROR: Couldn't read the settings file."),
            ManagerError::TOMLReadError => write!(f, "TOML_READ_ERROR: Error parsing the settings file, it may be malformed or from the wrong application"),
            ManagerError::AppDoesntExist(name) => write!(f, "The application {} doesn't exist.", name),
            ManagerError::AppAlreadyRunning => write!(f, "The application is already running."),
            ManagerError::AppIsntRunning(name) => write!(f, "The application {} isn't running.", name),
        }
    }
}
