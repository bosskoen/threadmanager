use std::sync::mpsc::{self,Receiver};
use library::error_handeler::ErrorOperation;

use private_lib::*;
mod private_lib;

fn main() {
    let (error_tx, error_rx) = mpsc::channel();
    let mut open_threads = Manager::new(error_tx, "settings sting").expect("msg"); //TODO settings and error
 // let (crach_tx, crach_rx) = mpsc::channel(); TODO if a thread stops
    start(&mut open_threads, error_rx);

    stop(&mut open_threads);
}

fn start(open_threads: &mut Manager, error_rx:Receiver<ErrorOperation>){
    open_threads.start_error(error_rx);

}

fn stop(open_threads: &mut Manager){

    open_threads.stop_error();
}
