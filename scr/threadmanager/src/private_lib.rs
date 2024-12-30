use libloading::{Library, Symbol};
use library::{error_handeler::{error_catchloop, print_error,print, ErrorOperation, RGB}, impl_status, toml, ErrorThreadDownError, Status};
use std::{collections::HashMap, error::Error, fmt, fs, panic, process::exit, sync::{atomic::AtomicBool, mpsc::{Receiver, Sender}, Arc, Mutex}, thread::{self, JoinHandle}, time::Duration};
use serde::Deserialize;

pub struct IniStatus{}
impl_status!(IniStatus, |_| String::from("initialization status"));

const ERROR_THREAD_DOWN: i32 = 105;
const CRASH_NOTIFIER_DOWN: i32 = 106;

pub struct Manager {
    map: HashMap<String, ThreadHandel>,
    error_sender: Sender<ErrorOperation>,
    settings: Settings,
    crash_notifier: Sender<String>,
}

impl Drop for Manager {
    fn drop(&mut self) {
        self.stop_all_threads();
        self.stop_error();
    }
}

impl Manager {
    pub fn new(error_sender: Sender<ErrorOperation>, settings_path: &str, crash_notifier: Sender<String>) -> Result<Self, ManagerError> {
        let settings = Settings::deserialize(settings_path)?;
        Ok(Self { map: HashMap::new(), error_sender, settings, crash_notifier })
    }

    pub fn start_new_thread(&mut self, name: String) -> Result<(), ManagerError> {
        let sett = if let Some(value) = self.settings.apps.get(&name) {
            value
        } else {
            return Err(ManagerError::AppDoesntExist(name));
        };
        let app_setting_path = sett.setting_path.clone();
        let dll_path = sett.dll_path.clone();

        if self.map.contains_key(&name) {
            return Err(ManagerError::AppAlreadyRunning);
        }
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_clone = Arc::clone(&stop_flag);
        let status: Arc<Mutex<Box<dyn Status>>> = Arc::new(Mutex::new(Box::new(IniStatus {})));
        let status_clone = Arc::clone(&status);
        let sender_clone = self.error_sender.clone();
        let name_clone = name.clone();
        let crash_notifier = self.crash_notifier.clone();

        let handle =
            thread::spawn(move || {
                thread_logic(&dll_path, name_clone, sender_clone, stop_flag_clone, status_clone, app_setting_path, crash_notifier);
            });

        self.map.insert(name, ThreadHandel::new(handle, stop_flag, status));
        Ok(())
    } //TODO als stop error

    pub fn stop_thread(&mut self, name: String) -> Result<(), ManagerError> {
        if let Some(thread) = self.map.remove(&name) {
            thread.stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
            if let Err(x) = thread.handle.join() {
                print_error("Manager", &format!("{} panicked while closing with error\n{:?}", name, x), RGB::ERROR());
            }
            Ok(())
        } else {
            if self.settings.apps.contains_key(&name) {
                Err(ManagerError::AppIsntRunning(name))
            } else {
                Err(ManagerError::AppDoesntExist(name))
            }
        }
    }

    pub fn is_running(&self, name: &str) -> bool {
        self.map.contains_key(name)
    }

    pub fn stop_all_threads(&mut self) {
        for (name, handle) in self.map.drain() {
            if name == "errorThread" {
                continue;
            }
            if let Err(err) = handle.handle.join() {
                print_error("Manager", &format!("Error while stopping {}: {:?}", name, err), RGB::ERROR());
            }
        }
    }

    pub fn help_message(&self, name: String) -> Result<(), ManagerError> {
        if let Some(setting) = self.settings.apps.get(&name) {
            print(&setting.help_message, RGB::WHITE());
            Ok(())
        } else {
            Err(ManagerError::AppDoesntExist(name))
        }
    }

    pub fn list_threads(&self, mode: Mode) {
        print(&format!("All {} threads:", match mode {
            Mode::All => "running and stopped",
            Mode::Running => "running",
            Mode::Stopped => "stopped",
        }), RGB::WHITE());
        match mode {
            Mode::All => {
                for (_, setting) in self.settings.apps.iter() {
                    print(&setting.name, RGB::WHITE());
                }
            },
            Mode::Running => {
                for (name, _) in self.map.iter() {
                    print(&name, RGB::WHITE());
                }
            },
            Mode::Stopped => {
                let running: Vec<&String> = self.settings.apps.keys().filter(|name| !self.map.contains_key(*name)).collect();
                for name in running {
                    print(&name, RGB::WHITE());
                }
            },
        }
    }

    pub fn start_error(&mut self, error_receiver: Receiver<ErrorOperation>) {
        let status: Arc<Mutex<Box<dyn Status>>> = Arc::new(Mutex::new(Box::new(IniStatus {})));
        let status_clone = Arc::clone(&status);

        let handle = thread::spawn(move || {
            if let Err(err) = error_catchloop(error_receiver, status) {
                print_error("Manager", &format!("{err}"), RGB::ERROR());
                exit(0) // TODO
            }     //TODO error laten return
        });

        self.map.insert(String::from("errorThread"), ThreadHandel::new(handle, Arc::new(AtomicBool::new(false)), status_clone));
    }
    pub fn stop_error(&mut self) {
        if let Err(_) = self.error_sender.send(ErrorOperation::StopErr) {
            print_error("Manager", "ErrorThread's receiver has been dropped too early", RGB::ERROR());
            exit(103);
        }
        if let Some(thread) = self.map.remove(&String::from("errorThread")) {
            if let Err(x) = thread.handle.join() {
                print_error("Manager", &format!("ErrorThread panicked while closing with error\n{:?}", x), RGB::ERROR());
                exit(104)
            }
        }
    }
    pub fn get_status(&self, thread_name: String) -> Result<(), ManagerError> {
        if let Some(handle) = self.map.get(&thread_name) {
            printstatus(&handle.status);
        } else {
            if self.settings.apps.contains_key(&thread_name) {
                return Err(ManagerError::AppIsntRunning(thread_name));
            } else {
                return Err(ManagerError::AppDoesntExist(thread_name));
            }
        }
        Ok(())
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
    status: Arc<Mutex<Box<dyn Status>>>
}

impl ThreadHandel {
    pub fn new(handle: JoinHandle<()>, stop_flag: Arc<AtomicBool>, status: Arc<Mutex<Box<dyn Status>>>) -> Self {
        Self { handle, stop_flag, status }
    }
}

fn printstatus(status: &Arc<Mutex<Box<dyn Status>>>) {
    if let Ok(x) = status.lock() {
        print(&(*x).format(), RGB::INFO());
    } else {
        print_error("Manager", "Error while printing status: lock failed", RGB::ERROR());
    }
}

fn thread_logic(dll_path: &str, name: String, sender_clone: Sender<ErrorOperation>, stop_flag: Arc<AtomicBool>, status: Arc<Mutex<Box<dyn Status>>>, app_setting_path: String, crash_notifier: Sender<String>) {
    unsafe {
        let lib = match Library::new(dll_path) {
            Ok(value) => value,
            Err(err) => {
                report_error(&sender_clone, &crash_notifier, name, format!("Error while loading library: \"{err}\""));
                return;
            },
        };
        let start: Symbol<fn(Sender<ErrorOperation>, Arc<AtomicBool>, Arc<Mutex<Box<dyn Status>>>, String) -> Result<(), Box<dyn Error>>> = match lib.get(b"start") {
            Ok(value) => value,
            Err(err) => {
                report_error(&sender_clone, &crash_notifier, name, format!("Error while loading start function: {err}"));
                return;
            },
        };
        let sender = sender_clone.clone();
        let status_clone = status.clone();

        match panic::catch_unwind(|| { start(sender_clone, stop_flag, status_clone, app_setting_path) }) {
            Ok(result) => {
                match result {
                    Ok(_) => {
                        print(&format!("{} has stopped gracefully\nlast status:\n", name), RGB::INFO());
                        printstatus(&status);
                    },
                    Err(err) => {
                        if let Some(fatal_error) = err.downcast_ref::<ErrorThreadDownError>() {
                            print_error("Manager", &format!("{}", fatal_error), RGB::CRITICAL_ERROR());
                            exit(ERROR_THREAD_DOWN)
                        } else {
                            if let Err(_) = sender.send(ErrorOperation::PrintAndChangeLed(name.clone(), format!("Thread stopped with errors {}", err), RGB::WARNING(), RGB::RED())) {
                                print_error("Manager", &format!("Error while sending error: {} Thread stopped with errors {}", name, err), RGB::CRITICAL_ERROR());
                                exit(ERROR_THREAD_DOWN);
                            }
                        }
                        send_crash_notifier(&crash_notifier, name);
                    },
                }
            },
            Err(error) => {
                if let Err(_) = sender.send(ErrorOperation::PrintAndBlinkLed(name.clone(), format!("Thread panicked unexpectedly: {:?}", error), RGB::CRITICAL_ERROR(), RGB::RED(), Duration::from_millis(500))) {
                    print_error("Manager", &format!("Error while sending error: {} Thread panicked unexpectedly: {:?}", name, error), RGB::CRITICAL_ERROR());
                    exit(ERROR_THREAD_DOWN);
                }
                send_crash_notifier(&crash_notifier, name);
            },
        }
    }
}

fn report_error(sender: &Sender<ErrorOperation>, crash_notifier: &Sender<String>, name: String, error_message: String) {
    if let Err(_) = sender.send(ErrorOperation::Print(name.clone(), error_message.clone(), RGB::ERROR())) {
        print_error("Manager", &format!("Error while sending error from {name}: {error_message}"), RGB::CRITICAL_ERROR());
        exit(ERROR_THREAD_DOWN);
    }
    send_crash_notifier(crash_notifier, name);
}

fn send_crash_notifier(crash_notifier: &Sender<String>, name: String) {
    if let Err(_) = crash_notifier.send(name.clone()) {
        print_error("Manager", &format!("Error while notifying main of unexpected crash in {name}"), RGB::CRITICAL_ERROR());
        exit(CRASH_NOTIFIER_DOWN);
    }
}

#[derive(Deserialize)]
struct Settings {
    update: HashMap<String, String>,
    apps: HashMap<String, AppSetting>
}

#[derive(Deserialize)]
struct AppSetting {
    name: String,
    dll_path: String,
    setting_path: String,
    help_message: String
}

impl Settings {
    fn deserialize(setting_path: &str) -> Result<Settings, ManagerError> {
        let text = fs::read_to_string(setting_path).map_err(|_| ManagerError::FileReadError)?;
        let mut sett = toml::from_str::<Settings>(&text).map_err(|_| ManagerError::TOMLReadError)?;
        sett.apps = sett.apps.into_iter()
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
