use library::Status;
use std::{sync::{atomic::AtomicBool, Arc, Mutex}, thread::JoinHandle};

pub struct IniStatus{}
impl Status for IniStatus {
    fn format(&self) -> String {
        String::from("initialisation status")
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {self}
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

pub fn printstatus(handle: &ThreadHandel){
    if let Ok(x) = handle.status.lock(){
        println!("{}",(*x).format())
    }
}
