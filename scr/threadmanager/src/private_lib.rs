use libloading::{Library, Symbol};
use library::{error_handeler::{error_catchloop,ErrorOperation, RGB}, impl_status, toml, ErrorThreadDownError, Status};
use std::{collections::HashMap, error::Error, fmt, fs, panic, process::exit, sync::{atomic::AtomicBool, mpsc::{Receiver, Sender}, Arc, Mutex}, thread::{self, JoinHandle}, time::Duration};
use serde::Deserialize;

pub struct IniStatus{}
impl_status!(IniStatus, |_| String::from("initialisation status"));

const ERROR_THREAD_DOWN: i32 = 105;
const CRACH_NOTIFIER_DOWN: i32 = 106;

pub struct Manager{
    map: HashMap<String, ThreadHandel>,
    error_sender: Sender<ErrorOperation>,
    settings: Settings,
    crach_notifier: Sender<String>,
}

impl Drop for Manager{
    fn drop(&mut self){
        self.stop_all_threads();
        self.stop_error();
    }
}

impl Manager {
    pub fn new(error_sender: Sender<ErrorOperation>, settings_path: &str, crach_notifier: Sender<String>)-> Result<Self, ManagerError>{
        let settings = Settings::deserialize(settings_path)?;
        Ok(Self{map: HashMap::new(), error_sender, settings, crach_notifier})
    }

    pub fn start_new_thread(&mut self, name: String ) ->Result<(),ManagerError>{
        let sett = if let Some(value) = self.settings.apps.get(&name){
            value
        }else{
            return Err(ManagerError::AppDoesntExist(name));
        };
        let app_seting_path= sett.setting_path.clone();
        let dll_path = sett.dll_path.clone();

    	if self.map.contains_key(&name){
            return Err(ManagerError::AppAlreadyRunnning);
        }
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_clone=Arc::clone(&stop_flag);
        let status: Arc<Mutex<Box<dyn Status>>> = Arc::new(Mutex::new(Box::new(IniStatus{})));
        let status_clone = Arc::clone(&status);
        let sender_colen = self.error_sender.clone();
        let name_clone = name.clone();
        let crach_notifier = self.crach_notifier.clone();

        let handel= 
            thread::spawn(move ||{
            thread_logic(&dll_path, name_clone, sender_colen, stop_flag_clone, status_clone, app_seting_path, crach_notifier);
            });

        self.map.insert(name, ThreadHandel::new(handel, stop_flag, status));
        Ok(()) 
    }

    pub fn stop_thread(&mut self, name: String) -> Result<(), ManagerError>{
        if let Some(thread) = self.map.remove(&name){
            thread.stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
            if let Err(x) = thread.handel.join(){
                eprint!("{} paniced while closing with error\n{:?}",name,x);
            }
            Ok(())
        }else{
            if self.settings.apps.contains_key(&name){
                Err(ManagerError::AppIsntRunning(name))
            }else {
                Err(ManagerError::AppDoesntExist(name))
            }
        }
    }

    pub fn is_running(&self, name: &str) -> bool{
        self.map.contains_key(name)
    }

    pub fn stop_all_threads(&mut self){
        for (name, handle) in self.map.drain(){
            if name == "errorThread"{
                continue;
            }
            if let Err(err) = handle.handel.join(){
                eprintln!("Error while stoppint {}: {:?}",name, err);
            }
        }
    }

    pub fn help_message(&self, name: String) -> Result<(), ManagerError>{
        if let Some(setting) = self.settings.apps.get(&name){
            println!("{}", setting.help_message);
            Ok(())
        }else{
            Err(ManagerError::AppDoesntExist(name))
        }
    }

    pub fn list_threads(&self, mode: Mode){ 
        println!("All {} threads:", match mode {
            Mode::All => "running and stopped",
            Mode::Running => "running",
            Mode::Stopped => "stopped",
        });
        match mode {
            Mode::All => {
                for (_, setting) in self.settings.apps.iter(){
                    println!("{}",setting.name);
                }
            },
            Mode::Running => {
                for (name, _) in self.map.iter(){
                    println!("{}", name);
                }
            },
            Mode::Stopped => {
                let running: Vec<&String> = self.settings.apps.keys().filter(|name| !self.map.contains_key(*name)).collect();
                for name in running{
                    println!("{}", name);
                }
            },
        }
    }

    pub fn start_error(&mut self, error_recever: Receiver<ErrorOperation>){
        let status: Arc<Mutex<Box<dyn Status>>>= Arc::new(Mutex::new(Box::new(IniStatus{})));
        let status_clone = Arc::clone(& status);
        
        let handle = thread::spawn( move ||{
            if let Err(err) = error_catchloop(error_recever, status){
                eprintln!("{err}");
                exit(0)// TODO
            }     //TODO error laten returen
        });
    
        self.map.insert(String::from("errorThread"),  ThreadHandel::new(handle, Arc::new(AtomicBool::new(false)), status_clone));
    }
    pub fn stop_error(&mut self){
        if let Err(_) = self.error_sender.send(ErrorOperation::StopErr){
            eprintln!("ErrorThread's resiver hase been dropt to early");
            exit(103);
        }
        if let Some(thread) = self.map.remove(&String::from("errorThread")){
            if let Err(x) = thread.handel.join(){
                eprint!("ErrorThread paniced while closing with error\n{:?}",x);
                exit(104)
            }
        }
    }
    pub fn get_status(&self, thread_name:String) -> Result<(), ManagerError>{
        if let Some(handle) =self.map.get(&thread_name){
            printstatus(&handle.status);
        }else{
            if self.settings.apps.contains_key(&thread_name){
                return Err(ManagerError::AppIsntRunning(thread_name));
            }else{
                return Err(ManagerError::AppDoesntExist(thread_name));
            }
        }
        Ok(())
    }
}

pub enum Mode{
    All,
    Running,
    Stopped,    
}

pub struct ThreadHandel{
    pub handel:JoinHandle<()>,
    pub stop_flag:Arc<AtomicBool>,
    status: Arc<Mutex<Box<dyn Status>>>
}

impl ThreadHandel {
    pub fn new(handel:JoinHandle<()>, stop_flag: Arc<AtomicBool>, status: Arc<Mutex<Box<dyn Status>>>)-> Self{
        Self{handel,stop_flag ,status}
    }
}

fn printstatus(status: &Arc<Mutex<Box<dyn Status>>>){
    if let Ok(x) = status.lock(){
        println!("{}",(*x).format())
    }else {
        eprintln!("Error while printing status: lock failed");
    }
}

fn thread_logic(dll_path: &str, name: String, sender_colen: Sender<ErrorOperation>, stop_flag: Arc<AtomicBool>, status: Arc<Mutex<Box<dyn Status>>>, app_seting_path: String, crach_notifier: Sender<String>){
    unsafe {
        let lib = match Library::new(dll_path){
            Ok(value) => value,
            Err(err) => {
                erreport_error(&sender_colen, &crach_notifier, name, format!("Error while loading library: \"{err}\""));
                return;
            },
        };
        let start: Symbol<fn(Sender<ErrorOperation>, Arc<AtomicBool>, Arc<Mutex<Box<dyn Status>>>, String) -> Result<(), Box<dyn Error>>> = match lib.get(b"start") {
            Ok(value) => value,
            Err(err) =>{
                erreport_error(&sender_colen, &crach_notifier, name, format!("Error while loading start function: {err}"));
                return;
            },
        };
        let sender = sender_colen.clone();
        let status_clone = status.clone();

        match panic::catch_unwind(||{start(sender_colen, stop_flag, status_clone,app_seting_path)}) {
            Ok(rezult) => {
                match rezult {
                    Ok(_) => {
                        println!("{} has stoped grasfuly\nlast status:\n", name);
                        printstatus(&status);
                    },
                    Err(err) => {
                        if let Some(fatal_error) =  err.downcast_ref::<ErrorThreadDownError>(){
                            eprintln!("{}", fatal_error);
                            exit(ERROR_THREAD_DOWN)
                        }else {
                            if let Err(_) = sender.send(ErrorOperation::PrintAndChangeLed(name.clone(), format!("Thread stoped with errors {}", err), RGB::from_hex(0xba8545) ,RGB::RED())){
                                eprintln!("Error while sending error: {} Thread stoped with errors {}", name, err);
                                exit(ERROR_THREAD_DOWN);
                            }
                        }
                        send_crach_notifier(&crach_notifier, name);
                    },
                }
            },
            Err(error) => {
                if let Err(_) = sender.send(ErrorOperation::PrintAndBlinkLed(name.clone(), format!("Thread paniced unexped: {:?}", error), RGB::RED() ,RGB::RED(), Duration::from_millis(500))){
                    eprintln!("Error while sending error: {} Thread paniced unexped: {:?}", name ,error);
                    exit(ERROR_THREAD_DOWN);
                }
                send_crach_notifier(&crach_notifier, name);
            },
        } 
    }
}

fn erreport_error(sender: &Sender<ErrorOperation>, crach_notifier: &Sender<String>, name: String, error_message: String){
    if let Err(_) = sender.send(ErrorOperation::Print(name.clone(), error_message.clone(), RGB::from_hex(0xba8545))){
        eprintln!("Error while sending error from {name}: {error_message}");
        exit(ERROR_THREAD_DOWN);
    }
    send_crach_notifier(crach_notifier, name);
}

fn send_crach_notifier(crach_notifier: &Sender<String>, name: String){
    if let Err(_) = crach_notifier.send(name.clone()){
        eprintln!("Error while notifying main of unexpected crash in {name}");
        exit(CRACH_NOTIFIER_DOWN);
    }
}


#[derive(Deserialize)]
struct Settings{
    update: HashMap<String,String>,
    apps: HashMap<String,Appsetting>
}
#[derive(Deserialize)]
struct Appsetting{
    name: String,
    dll_path:String,
    setting_path: String,
    help_message: String
}

impl Settings {
    fn deserialize(setting_path: &str) -> Result<Settings, ManagerError>{
        let text = fs::read_to_string(setting_path).map_err(|_| ManagerError::FileReadError)?;
        let mut sett = toml::from_str::<Settings>(&text).map_err(|_| ManagerError::TOMLReadError)?;
        sett.apps = sett.apps.into_iter()
            .map(|(key,value)| (key.strip_prefix("apps.").unwrap_or(&key).to_string(), value))
            .collect();
        Ok(sett)
    }
}

#[derive(Debug)]
pub enum ManagerError {
    FileReadError,
    TOMLReadError,
    AppDoesntExist(String),
    AppAlreadyRunnning,
    AppIsntRunning(String),
}
impl  fmt::Display for ManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManagerError::FileReadError => write!(f, "FILE_READERROR: Coudn't read the settings file."),
            ManagerError::TOMLReadError => write!(f, "TOML_READ_ERROR: Error parsing the settings file, i may be malformed or from the wrong appication"),
            ManagerError::AppDoesntExist(name) => write!(f, "The application {} doesn't exist.", name),
            ManagerError::AppAlreadyRunnning => write!(f, "The application is already running."),
            ManagerError::AppIsntRunning(name) => write!(f, "The application {} isn't running.", name),
        }
    }
}
