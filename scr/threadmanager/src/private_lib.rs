use libloading::{Library, Symbol};
use library::{error_handeler::{error_catchloop,ErrorOperation, RGB}, impl_status, toml::{self, value}, ErrorThreadDownError, Status};
use std::{collections::HashMap, error::{self, Error}, fmt, fs, panic, path, process::exit, result, sync::{atomic::AtomicBool, mpsc::{Receiver, Sender}, Arc, Mutex}, thread::{self, JoinHandle}, time::Duration};
use serde::Deserialize;

pub struct IniStatus{}
impl_status!(IniStatus, |_| String::from("initialisation status"));

pub struct Manager{
    map: HashMap<String, ThreadHandel>,
    error_sender: Sender<ErrorOperation>,
    settings: Settings,
}
impl Manager {
    pub fn new(error_sender: Sender<ErrorOperation>, settings_path: &str)-> Result<Self, ManagerError>{
        let settings = Settings::deserialize(settings_path)?;
        Ok(Self{map: HashMap::new(), error_sender, settings})
    }

    pub fn start_new_thread(&mut self, name: String) ->Result<(),ManagerError>{
        let sett = if let Some(value) = self.settings.apps.get(&name){
            value
        }else{
            return Err(ManagerError::AppDoesntExist);
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

        //let start = self.load_lib(name, &dll_path)?;
        let handel= 
            thread::spawn(move ||{
            unsafe {
                let lib = match Library::new(dll_path){
                    Ok(value) => value,
                    Err(err) => {
                        if let Err(_) = sender_colen.send(ErrorOperation::Print(name_clone, format!("Error while loading library: {err}"))){
                            //TODO
                        }
                        return;
                    },
                };
                let start: Symbol<fn(Sender<ErrorOperation>, Arc<AtomicBool>, Arc<Mutex<Box<dyn Status>>>, String) -> Result<(), Box<dyn Error>>> = match lib.get(b"start") {
                    Ok(value) => value,
                    Err(err) =>{
                        if let Err(_) = sender_colen.send(ErrorOperation::Print(name_clone, format!("Error while loading startfunction: {err}"))){
                            //TODO
                        }
                        return;
                    },
                };
                let sender = sender_colen.clone();

                match panic::catch_unwind(||{start(sender_colen, stop_flag_clone, status_clone,app_seting_path)}) {
                    Ok(rezult) => {
                        match rezult {
                            Ok(_) => todo!(), //TODO print error stopt mabby status
                            Err(err) => {
                                if let Some(fatal_error) =  err.downcast_ref::<ErrorThreadDownError>(){
                                    //TODO 
                                }else {
                                    //TODO print stopt with error {error} {status}
                                }
                            },
                        }
                    },
                    Err(error) => {
                        if let Err(_) = sender.send(ErrorOperation::PrintAndBlinkLed(name_clone, format!("Thread paniced unexped: {:?}", error), RGB::RED(), Duration::from_millis(500))){
                            //TODO
                        }
                    },
                } 
            }
            });

        self.map.insert(name, ThreadHandel::new(handel, stop_flag, status));
        Ok(()) 
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
    pub fn get_status(&self, thread_name:String){
        if let Some(handle) =self.map.get(&thread_name){
            printstatus(handle);
        }else{
            //TODO chek if app
            // println!("{} is a unknow app", thread_name);
            println!("{} isn't running", thread_name);
        }
    }
}

pub struct ThreadHandel{
    pub handel:JoinHandle<()>, //TODO uitvogelen welke return value ik will
    pub stop_flag:Arc<AtomicBool>,
    status: Arc<Mutex<Box<dyn Status>>>
}

impl ThreadHandel {
    pub fn new(handel:JoinHandle<()>, stop_flag: Arc<AtomicBool>, status: Arc<Mutex<Box<dyn Status>>>)-> Self{
        Self{handel,stop_flag ,status}
    }
}

fn printstatus(handle: &ThreadHandel){
    if let Ok(x) = handle.status.lock(){
        println!("{}",(*x).format())
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
    WTFError(String),
    AppDoesntExist,
    AppAlreadyRunnning,

}
impl  fmt::Display for ManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManagerError::FileReadError => write!(f, "FILE_READERROR: Coudn't read the settings file."),
            ManagerError::TOMLReadError => write!(f, "TOML_READ_ERROR: Error parsing the settings file, i may be malformed or from the wrong appication"),
            ManagerError::WTFError(messig) => write!(f, "good job you dit somthing that sould be imposible:\n{}", messig),
            ManagerError::AppDoesntExist => todo!(),
            ManagerError::AppAlreadyRunnning => todo!(),
        }
    }
}
