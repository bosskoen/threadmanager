use library::{error_handeler::{error_catchloop,ErrorOperation}, impl_status, Status};
use std::{any::Any, collections::HashMap, process::exit, sync::{atomic::AtomicBool, mpsc::{Receiver, Sender}, Arc, Mutex}, thread::{self, JoinHandle}};

pub struct IniStatus{}
impl_status!(IniStatus, |_| String::from("initialisation status"));

pub struct Manager{
    map: HashMap<String, ThreadHandel>,
    error_sender: Sender<ErrorOperation>
}
impl Manager {
    pub fn new(error_sender: Sender<ErrorOperation>)-> Self{
        Self{map: HashMap::new(), error_sender}
    }
    pub fn start_error(&mut self, error_recever: Receiver<ErrorOperation>){
        let status: Arc<Mutex<Box<dyn Status>>>= Arc::new(Mutex::new(Box::new(IniStatus{})));
        let status_clone = Arc::clone(& status);
        
        let handle = thread::spawn(||{
            error_catchloop(error_recever, status);
        });
    
        self.map.insert(String::from("errorThread"),  ThreadHandel::new(handle, AtomicBool::new(false), status_clone));
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
    pub stop_flag:AtomicBool,
    status: Arc<Mutex<Box<dyn Status>>>
}

impl ThreadHandel {
    pub fn new(handel:JoinHandle<()>, stop_flag: AtomicBool, status: Arc<Mutex<Box<dyn Status>>>)-> Self{
        Self{handel,stop_flag ,status}
    }
}

fn printstatus(handle: &ThreadHandel){
    if let Ok(x) = handle.status.lock(){
        println!("{}",(*x).format())
    }
}