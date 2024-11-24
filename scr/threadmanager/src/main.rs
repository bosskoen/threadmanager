use std::{collections::HashMap, sync::{atomic::AtomicBool, mpsc::{Receiver, Sender}, Arc, Mutex}};
use std::sync::mpsc;
use std::thread::{self};
use library::{error_handeler::{error_catchloop, ErrorOperation}, Status};

use private_lib::*;
mod private_lib;

fn main() {
     let mut open_threads:HashMap<String, ThreadHandel> = HashMap::new();
   // let (crach_tx, crach_rx) = mpsc::channel();
    let (error_tx, error_rx) = mpsc::channel();
    start(&mut open_threads, error_rx);

    printstatus(open_threads.get(&String::from("errorThread")).expect("ites"));


    printstatus(open_threads.get(&String::from("errorThread")).expect("ites2"));



    stop(&mut open_threads, error_tx);
}

fn start(open_threads: &mut HashMap<String,ThreadHandel>,error_rx:Receiver<ErrorOperation>){
    let status: Arc<Mutex<Box<dyn Status>>>= Arc::new(Mutex::new(Box::new(IniStatus{})));
    let status_clone = Arc::clone(& status);
    
    let handle = thread::spawn(||{
        error_catchloop(error_rx, status);
    });

    open_threads.insert(String::from("errorThread"),  ThreadHandel::new(handle, AtomicBool::new(false), status_clone));
}

fn stop(open_threads: &mut HashMap<String,ThreadHandel>, error_tx:Sender<ErrorOperation>){


    error_tx.send(ErrorOperation::StopErr);
    if let Some(thread) = open_threads.remove(&String::from("errorThread")){
        thread.handel.join();
    }
}
