use libloading::{Library, Symbol};
use library::{error_handeler::{self, error_catchloop, print, print_error, reset_color, ErrorOperation, LedOption, RGB}, impl_status, toml, ErrorThreadDownError, Status};
use std::{collections::HashMap, error::Error, fmt, fs, panic, process::exit, sync::{atomic::AtomicBool, mpsc::{Receiver, Sender}, Arc, Mutex}, thread::{self, JoinHandle}};
use serde::Deserialize;

pub struct IniStatus{}
impl_status!(IniStatus, |_| String::from("initialization status"));

const ERROR_THREAD_DOWN: i32 = 105;
const CRASH_NOTIFIER_DOWN: i32 = 106;
const ERROR_THREAD_RESEVER_DOWN:i32 = 103;
const ERROR_PENICED:i32 = 104;

pub struct Manager {
    map: HashMap<String, ThreadHandel>,
    pub error_sender: Sender<ErrorOperation>,
    settings: Settings,
    settings_path: String,
    crash_notifier: Sender<String>,
}

impl Drop for Manager {
    fn drop(&mut self) {
        self.stop_all_threads();
        self.stop_error();
        reset_color();
    }
}

impl Manager {
    pub fn new(error_sender: Sender<ErrorOperation>, settings_path: &str, crash_notifier: Sender<String>) -> Result<Self, ManagerError> {
        let settings = Settings::deserialize(settings_path)?;
        Ok(Self { map: HashMap::new(), error_sender, settings, crash_notifier, settings_path: settings_path.to_string() })
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
        let sender_clone = self.error_sender.clone();
        let name_clone = name.clone();
        let crash_notifier = self.crash_notifier.clone();

        let handle =
            thread::spawn(move || {
                thread_logic(&dll_path, name_clone, sender_clone, stop_flag_clone, status_clone, app_setting_path, crash_notifier);
            });

        self.map.insert(name, ThreadHandel::new(handle, stop_flag, status));
        Ok(())
    }

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
    } //TOOD stop error thread is een loop

    pub fn is_running(&self, name: &str) -> bool {
        self.map.contains_key(name)
    }

    pub fn stop_all_threads(&mut self) {
        for (name, handle) in self.map.drain() {
            if name == "errorThread" || name == error_handeler::light_dimmer_thread::PLUGIN_NAME {
                continue;
            }
            print(&format!("stopping: {name}"), RGB::NOTICE());
            handle.stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
            if let Err(x) = handle.handle.join() {
                print_error("Manager", &format!("{} panicked while closing with error\n{:?}", name, x), RGB::ERROR());
            }
            println!("");
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
        }), RGB::NOTICE());
        match mode {
            Mode::All => {
                for (_, setting) in self.settings.apps.iter() {
                    print(&setting.name, RGB::WHITE());
                }
                print("errorThread", RGB::TRACE());
                print(error_handeler::light_dimmer_thread::PLUGIN_NAME, RGB::TRACE());
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
            error_catchloop(error_receiver, status) 
        });

        self.map.insert(String::from("errorThread"), ThreadHandel::new(handle, Arc::new(AtomicBool::new(false)), status_clone));

        #[cfg(feature = "led")]
        self.start_light_dimmer();
    }

    #[cfg(not(feature = "led"))]
    pub fn start_light_dimmer(&mut self){
        print("LED feature is not enabled, light dimmer will not be started", RGB::ALERT());
    }

    #[cfg(feature = "led")]
    pub fn start_light_dimmer(&mut self){
        let status :Arc<Mutex<Box<dyn Status>>> = Arc::new(Mutex::new(Box::new(IniStatus {})));
        let status_clone = Arc::clone(&status);
        let stopflag = Arc::new(AtomicBool::new(false));
        let stopflag_clone = Arc::clone(&stopflag);
        let error_sender = self.error_sender.clone();
        let handle = thread::spawn(move || {
            error_handeler::light_dimmer_thread::start_light_dim(error_sender, stopflag, status);
        });

        self.map.insert(error_handeler::light_dimmer_thread::PLUGIN_NAME.to_string(), ThreadHandel::new(handle, stopflag_clone, status_clone));
    }

    pub fn stop_error(&mut self) {
        if let Err(_) = self.error_sender.send(ErrorOperation::StopErr) {
            print_error("Manager", "ErrorThread's receiver has been dropped too early", RGB::ERROR());
            exit(ERROR_THREAD_RESEVER_DOWN);
        }
        if let Some(thread) = self.map.remove(&String::from("errorThread")) {
            if let Err(x) = thread.handle.join() {
                print_error("Manager", &format!("ErrorThread panicked while closing with error\n{:?}", x), RGB::ERROR());
                exit(ERROR_PENICED)
            }
        }
        if self.map.contains_key(&error_handeler::light_dimmer_thread::PLUGIN_NAME.to_string()) {
            if let Some(thread) = self.map.remove(&error_handeler::light_dimmer_thread::PLUGIN_NAME.to_string()) {
                thread.stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                if let Err(x) = thread.handle.join() {
                    print_error("Manager", &format!("ErrorThread panicked while closing with error\n{:?}", x), RGB::ERROR());
                }
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

    pub fn reload_settings(&mut self){
        self.settings = match Settings::deserialize(&self.settings_path) {
            Ok(setting) => setting,
            Err(_) => {
                print("failed to get new settings", RGB::ERROR());
                return;
            },
        }
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
        if fs::metadata(&dll_path).is_err() {
            report_error(&sender_clone, &crash_notifier, name, format!("File not found: \"{dll_path}\""));
            return;
        }

        #[cfg(windows)]
        if !dll_path.ends_with(".dll") {
            report_error(&sender_clone, &crash_notifier, name, format!("File is not a dynamic link library: \"{dll_path}\""));
            return;
        }
        #[cfg(unix)]
        if !dll_path.ends_with(".so") {
            report_error(&sender_clone, &crash_notifier, name, format!("File is not a shared object: \"{dll_path}\""));
            return;
        }

        if fs::metadata(&app_setting_path).is_err(){
            report_error(&sender_clone, &crash_notifier, name, format!("File not found: \"{app_setting_path}\""));
            return;
        }
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
                        print(&format!("{} has stopped gracefully\nlast status:", &name), RGB::INFO());
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
                        send_crash_notifier(&crash_notifier, name.clone());
                    },
                }
            },
            Err(error) => {
                if let Err(_) = sender.send(ErrorOperation::PrintAndChangeLed(name.clone(), format!("Thread panicked unexpectedly: {:?}", error), RGB::CRITICAL_ERROR(), RGB::RED())) {
                    print_error("Manager", &format!("Error while sending error: {} Thread panicked unexpectedly: {:?}", name, error), RGB::CRITICAL_ERROR());
                    exit(ERROR_THREAD_DOWN);
                }
                if let Err(_) = sender.send(ErrorOperation::OnColor(LedOption::Red)) {
                    print_error("Manager", &format!("Error while sending error: {} Thread panicked unexpectedly: {:?}", name, error), RGB::CRITICAL_ERROR());
                    exit(ERROR_THREAD_DOWN);
                }
                send_crash_notifier(&crash_notifier, name.clone());
            },
        }
        match status.lock() {
            Ok(mut lock) => {
                (*lock) = Box::new(IniStatus {});
            },
            Err(_) => {
                print_error(&name, "Couldn't reset status to initial state", RGB::CRITICAL_ERROR());
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
    //TODO logic for downloading plugins
    #[allow(dead_code)]
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
