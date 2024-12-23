use std::{fmt::Display, fs, sync::{atomic::{AtomicBool, Ordering}, mpsc::Sender, Arc, Mutex}, thread, time::{Duration, SystemTime}};
use humansize::{format_size, BINARY};
use library::*;
use data_base_manager::Connection;
use error_handeler::ErrorOperation;
use parsing::Settings;


mod parsing;

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
    last_time_setting_written: SystemTime,
    url: String,
}

impl Context {
    fn from(stopflag: AtomicBool, status: Arc<Mutex<Box<dyn Status>>>, settings_path: String) -> Result<Self, PricingError>{
        let (settings, last_time_setting_written) = Settings::get(&settings_path)?;
        let data_base_path = settings.data_base_path;
        let table_name = settings.table_name;

        let conn = Connection::open(data_base_path.clone()).map_err(|_|PricingError::DataBaseConnectionError)?;
        initialise_status(&conn, &table_name, &status)?;

        Ok(Context{stopflag, status ,settings_path, conn, data_base_path, table_name, update_rate: settings.update_rate, step_rate: settings.step_rate, time_passed: 0, last_time_setting_written, url: settings.url})
    }

    fn update_timing(&mut self) -> Result<(), PricingError>{
        let mod_time =fs::metadata(&self.settings_path)
            .map_err(|_|PricingError::FileReadError )?
            .modified()
            .map_err(|_| PricingError::FileReadError)?;
        
        let duration_since_last_update = mod_time
            .duration_since(self.last_time_setting_written)
            .map_err(|_| PricingError::WTFError("You time traveled, the file was modified in the past!".to_string()))?;

        if duration_since_last_update.as_secs() <= 0 {
            return Ok(());
        }
        
        let (setting, _) = Settings::get(&self.settings_path)?;
        self.table_name = setting.table_name;
        self.update_rate = setting.update_rate;
        self.step_rate = setting.step_rate;
        self.url = setting.url;
        self.last_time_setting_written = mod_time;
        Ok(())
    }

    fn update_status(&self, items_tracked: usize, data_used: u64) -> Result<(), PricingError>{
        match self.status.lock(){
            Ok(mut stat) => {let internal_statust  = if let Some(mut_status) = (**stat).as_any_mut().downcast_mut::<PricingStatus>(){
                mut_status
            }else{ 
                return Err(PricingError::IncorectStatusType);
            };
            internal_statust.updates_processed += 1;
            internal_statust.items_being_tracked = items_tracked;
            internal_statust.network_data_used += data_used;
            internal_statust.last_update_time = Local::now();
            },
            Err(_) => { 
                return Err(PricingError::LockFailedError);
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
        context.update_timing();
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
                    return Err(Box::new(PricingError::ErrorThreadDown));
                }
                Duration::ZERO
            },
        };

        if let Some(sleep_duration) = Duration::from_secs(context.step_rate as u64).checked_sub(endloop) {
            thread::sleep(sleep_duration);
        } else {
            if let Err(_) = error_handel.send(ErrorOperation::Print("loop took to long in price logger".to_string())){
                return Err(Box::new(PricingError::ErrorThreadDown));
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
    }else {return Err(PricingError::StatusIntialiseError);}
    Ok(())
}


#[derive(Debug)]
pub enum PricingError {
    DataBaseConnectionError,
    StatusIntialiseError,
    IncorectStatusType,
    LockFailedError,
    ErrorThreadDown,
    FileReadError,
    TOMLReadError,
    WTFError(String),
}

impl Display for PricingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PricingError::DataBaseConnectionError => write!(f, "DATA_BASE_CONNECTION_ERROR: Couldn't get a connection to the given data base."),
            PricingError::StatusIntialiseError => write!(f, "STATUS_INTIALISE_ERROR: Couldn't get a lock on status while initialising."),
            PricingError::IncorectStatusType => write!(f, "INCORECT_STATUS_TYPE: Status wasn't of the corect type."),
            PricingError::LockFailedError => write!(f, "LOCK_FAILED_ERROR: Couldn't get a lock on status while updating."),
            PricingError::ErrorThreadDown => write!(f, "ERROR_THREAD_DOWN: Couldn't send a messige to the error thread."),
            PricingError::TOMLReadError => write!(f, "TOML_READ_ERROR: Error parsing the settings file, i may be malformed or from the wrong appication"),
            PricingError::FileReadError => write!(f, "FILE_READERROR: Coudn't read the settings file."),
            PricingError::WTFError(messig) => write!(f, "good job you dit somthing that sould be imposible:\n{}", messig),
        }
    }
}

impl std::error::Error for PricingError{}

pub struct PricingStatus{
    pub updates_processed: usize,
    pub items_being_tracked: usize,
    pub network_data_used: u64,
    pub last_update_time: DateTime<Local>,
    start_time: DateTime<Local>
}

impl PricingStatus {
    pub fn new() ->Self{
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
