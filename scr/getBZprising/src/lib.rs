use std::{sync::{atomic::{AtomicBool, Ordering}, mpsc::Sender, Arc, Mutex}, thread, time::{Duration, SystemTime}};
use library::*;
use data_base_manager::Connection;
use error_handeler::{ErrorOperation, RGB};
use humansize::{format_size,BINARY};

const DATA_BASE_CONNECTION_ERROR: usize = 1;
const STATUS_INTIALISE_ERROR: usize = 2;
const INCORECT_STATUS_TYPE: usize = 3;
const LOCK_FAILED_ERROR: usize = 4;

pub fn start(error_handel: Sender<ErrorOperation>, stopflag: AtomicBool, status: Arc<Mutex<Box<dyn Status>>>) -> usize{
    //let mut closure_error_flag = false;
    let data_base_path = String::new(); //TODO all info from JSON setings file
    let table_name = String::new(); //TODO JSON
    let mut update_rate:usize = 0; //TODO JSON
    let mut step_rate:usize = 0; //TODO JSON
    let mut time_passed: usize =0;
    let mut conn = if let Ok( conn) = Connection::open(data_base_path){
        conn
    } else{
        throw_exit_error!(error_handel, PrintAndChangeLed("coudn't get a connection to the given data base",RGB::RED()), DATA_BASE_CONNECTION_ERROR);
    };

    if let Err(code) = initialise_status(&conn, &table_name, status) {
        throw_exit_error!(error_handel, PrintAndChangeLed("coudn't change status to custom type", RGB::RED()), code);
    }

    loop {
        let start_of_loop = SystemTime::now();
        //update update_rate and step_rate
        if stopflag.load(Ordering::Relaxed){
            break;
        }
        if time_passed >= update_rate{
            time_passed = 0;
            //do somthing
        }else{
            time_passed += step_rate;
        }

        let endloop =match start_of_loop.elapsed() {
            Ok(duration) => duration,
            Err(error) => {throw_error!(error_handel, Print(format!("error while getting elepsted time: {}", error.to_string()))); Duration::ZERO},
        };

        if let Some(sleep_duration) = Duration::from_secs(step_rate as u64).checked_sub(endloop) {
            thread::sleep(sleep_duration);
        } else {
            throw_error!(error_handel, Print("loop took to long"));
            time_passed += (endloop.saturating_sub(Duration::from_secs(step_rate as u64))).as_secs() as usize;
        }
        
    }
    return 0;
}

fn initialise_status(conn: &Connection, table_name: &str,status: Arc<Mutex<Box<dyn Status>>>) -> Result<usize, usize>{
    let mut newstatus = PrisingStatus::new();
    if let Ok(timestamp) = conn.query_row_and_then(&format!("SELECT max(time) FROM {}", table_name), [], |row| row.get::<_,i64>(0)){
        newstatus.last_update_time = if let Some(local_time) = DateTime::from_timestamp(timestamp, 0) {
            DateTime::from(local_time)
        } else{ newstatus.last_update_time}
    };

    if let Ok(mut status)= status.lock(){
        (*status) = Box::new(newstatus);
    }else {return Err(STATUS_INTIALISE_ERROR);}

    Ok(0)
}

fn update_status(error_handel: Sender<ErrorOperation> ,status: Arc<Mutex<Box<dyn Status>>>, items_tracked: usize, data_used: u64) -> usize{
    match status.lock(){
        Ok(mut stat) => {let internal_statust  = if let Some(mut_status) = (**stat).as_any_mut().downcast_mut::<PrisingStatus>(){
            mut_status
        }else{ throw_exit_error!(error_handel, PrintAndChangeLed("Prisingstatus was not of type PrisingStatus".to_string(), RGB::RED()), INCORECT_STATUS_TYPE);
        };
        internal_statust.updates_processed += 1;
        internal_statust.items_being_tracked = items_tracked;
        internal_statust.network_data_used += data_used;
        internal_statust.last_update_time = Local::now();
        },
        Err(err) => {throw_exit_error!(error_handel, PrintAndChangeLed("Couldn't get a lock on status", RGB::RED()), LOCK_FAILED_ERROR);},
    }
    0
}

struct PrisingStatus{
    updates_processed: usize,
    items_being_tracked: usize,
    network_data_used: u64,
    last_update_time: DateTime<Local>,
    start_time: DateTime<Local>
}

impl PrisingStatus {
    fn new() ->Self{
        PrisingStatus{updates_processed:0,items_being_tracked:0, network_data_used:0, last_update_time: Local::now() , start_time: Local::now()}
    }
}

impl_status!{PrisingStatus, |s: &PrisingStatus| format!{
    "Pricing tracker processed {} updates.\n\
    Currently tracking {} items.\n\
    Total network data used: {}.\n\
    Last update was at {}.\n\
    Thread started at {} and has been running for {}.",
    s.updates_processed,
    s.items_being_tracked,
    format_size(s.network_data_used, BINARY),
    s.last_update_time.format("%Y %m-%d; %H:%M:%S"),
    s.start_time.format("%Y %m-%d; %H:%M:%S"),
    format_duration(s.start_time, Local::now())
}}

#[cfg(test)]
mod tests {
}
