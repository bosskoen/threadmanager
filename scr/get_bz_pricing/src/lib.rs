use std::{fmt::Display, sync::{atomic::{AtomicBool, Ordering}, mpsc::Sender, Arc, Mutex}, thread, time::{Duration, SystemTime}};
use library::*;
use data_base_manager::Connection;
use error_handeler::ErrorOperation;
use humansize::{format_size,BINARY};

#[derive(Debug)]
#[allow(non_camel_case_types)]
enum PricingError {
    DATA_BASE_CONNECTION_ERROR,
    STATUS_INTIALISE_ERROR,
    INCORECT_STATUS_TYPE,
    LOCK_FAILED_ERROR,
    ERROR_THREAD_DOWN,
}

impl Display for PricingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PricingError::DATA_BASE_CONNECTION_ERROR => write!(f, "DATA_BASE_CONNECTION_ERROR: Couldn't get a connection to the given data base."),
            PricingError::STATUS_INTIALISE_ERROR => write!(f, "STATUS_INTIALISE_ERROR: Couldn't get a lock on status while initialising."),
            PricingError::INCORECT_STATUS_TYPE => write!(f, "INCORECT_STATUS_TYPE: Status wasn't of the corect type."),
            PricingError::LOCK_FAILED_ERROR => write!(f, "LOCK_FAILED_ERROR: Couldn't get a lock on status while updating."),
            PricingError::ERROR_THREAD_DOWN => write!(f, "ERROR_THREAD_DOWN: Couldn't send a messige to the error thread."),
        }
    }
}

impl std::error::Error for PricingError{}

struct Context {
    stopflag: AtomicBool,
    status: Arc<Mutex<Box<dyn Status>>>,
    settings_path: String,
    conn: Connection,
    data_base_path: String,
    table_name: String,
    update_rate: usize ,
    step_rate: usize,
    time_passed: usize,
}

impl Context {
    fn from(stopflag: AtomicBool, status: Arc<Mutex<Box<dyn Status>>>, settings_path: String) -> Result<Self, PricingError>{
        let data_base_path = String::new(); //TODO all info from JSON setings file
        let table_name = String::new(); //TODO JSON
        let update_rate:usize = 0; //TODO JSON
        let step_rate:usize = 0; //TODO JSON

        let conn = if let Ok( conn) = Connection::open(data_base_path.clone()){
            conn
        } else{
            return Err(PricingError::DATA_BASE_CONNECTION_ERROR);
        };

       initialise_status(&conn, &table_name, &status)?;

        Ok(Context{stopflag, status ,settings_path, conn, data_base_path, table_name, update_rate, step_rate, time_passed: 0})
    }

    fn update_status(&self, items_tracked: usize, data_used: u64) -> Result<(), PricingError>{
        match self.status.lock(){
            Ok(mut stat) => {let internal_statust  = if let Some(mut_status) = (**stat).as_any_mut().downcast_mut::<PricingStatus>(){
                mut_status
            }else{ 
                return Err(PricingError::INCORECT_STATUS_TYPE);
            };
            internal_statust.updates_processed += 1;
            internal_statust.items_being_tracked = items_tracked;
            internal_statust.network_data_used += data_used;
            internal_statust.last_update_time = Local::now();
            },
            Err(_) => { 
                return Err(PricingError::LOCK_FAILED_ERROR);
            },
        }
        Ok(())
    }

}

#[no_mangle]
pub fn start(error_handel: Sender<ErrorOperation>, stopflag: AtomicBool, status: Arc<Mutex<Box<dyn Status>>>, settings_path: String) -> Result<(), Box<dyn std::error::Error>>{
    //let mut closure_error_flag = false;
    let mut context = Context::from(stopflag, status, settings_path)?;

    loop {
        let start_of_loop = SystemTime::now();
        //update update_rate and step_rate
        if context.stopflag.load(Ordering::Relaxed){
            break;
        }
        if context.time_passed >= context.update_rate{
            context.time_passed = 0;
            //do somthing
        }else{
            context.time_passed += context.step_rate;
        }

        let endloop =match start_of_loop.elapsed() {
            Ok(duration) => duration,
            Err(error) => {
                if let Err(_) = error_handel.send(ErrorOperation::Print(format!("error while getting elepsted time: {}", error.to_string()))){
                    return Err(Box::new(PricingError::ERROR_THREAD_DOWN));
                }
                Duration::ZERO
            },
        };

        if let Some(sleep_duration) = Duration::from_secs(context.step_rate as u64).checked_sub(endloop) {
            thread::sleep(sleep_duration);
        } else {
            if let Err(_) = error_handel.send(ErrorOperation::Print("loop took to long in price logger".to_string())){
                return Err(Box::new(PricingError::ERROR_THREAD_DOWN));
            }
            context.time_passed += (endloop.saturating_sub(Duration::from_secs(context.step_rate as u64))).as_secs() as usize;
        }
        
    }
    Ok(())
}

fn initialise_status(conn: &Connection, table_name: &str,status: &Arc<Mutex<Box<dyn Status>>>) -> Result<(), PricingError>{
    let mut newstatus = PricingStatus::new();
    if let Ok(timestamp) = conn.query_row_and_then(&format!("SELECT max(time) FROM {}", table_name), [], |row| row.get::<_,i64>(0)){
        newstatus.last_update_time = if let Some(local_time) = DateTime::from_timestamp(timestamp, 0) {
            DateTime::from(local_time)
        } else{ newstatus.last_update_time}
    }

    if let Ok(mut status)= status.lock(){
        (*status) = Box::new(newstatus);
    }else {return Err(PricingError::STATUS_INTIALISE_ERROR);}
    Ok(())
}

struct PricingStatus{
    updates_processed: usize,
    items_being_tracked: usize,
    network_data_used: u64,
    last_update_time: DateTime<Local>,
    start_time: DateTime<Local>
}

impl PricingStatus {
    fn new() ->Self{
        PricingStatus{updates_processed:0,items_being_tracked:0, network_data_used:0, last_update_time: Local::now() , start_time: Local::now()}
    }
}

impl_status!{PricingStatus, |s: &PricingStatus| format!{
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
