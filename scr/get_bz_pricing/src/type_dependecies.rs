use std::{fmt::Display, fs, sync::{atomic::AtomicBool, Arc, Mutex}, time::SystemTime};

use humansize::{format_size, BINARY};
use library::{data_base_manager::{rusqlite, Connection, DataBaseError}, format_duration, impl_status,DateTime, Local, Status};

use crate::parsing::Settings;


pub struct Context {
    pub stopflag: AtomicBool,
    status: Arc<Mutex<Box<dyn Status>>>,
    settings_path: String,
    pub conn: Connection,
    pub data_table_name: String,
    pub update_rate: usize ,
    pub step_rate: usize,
    pub time_passed: usize,
    last_time_setting_written: SystemTime,
    pub url: String,
    pub lookup_table_name: String
}

impl Context {
    pub fn from(stopflag: AtomicBool, status: Arc<Mutex<Box<dyn Status>>>, settings_path: String) -> Result<Self, PricingError>{
        let (settings, last_time_setting_written) = Settings::get(&settings_path)?;
        let data_base_path = settings.data_base_path;
        let data_table_name = settings.table_name;
        let lookup_table_name= settings.lookup_table_name;
        
        let mut conn = Connection::open(data_base_path.clone()).map_err(|_|PricingError::DataBaseConnectionError)?;
        crate::validate_data_base(&mut conn, &data_table_name, &lookup_table_name)?;
        initialise_status(&conn, &data_table_name, &status)?;

        Ok(Context{stopflag, status ,settings_path, conn, data_table_name, update_rate: settings.update_rate, step_rate: settings.step_rate, time_passed: 0, last_time_setting_written, url: settings.url, lookup_table_name})
    }

    pub fn update_timing(&mut self) -> Result<(), PricingError>{
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
        self.update_rate = setting.update_rate;
        self.step_rate = setting.step_rate;
        self.url = setting.url;
        self.last_time_setting_written = mod_time;
        Ok(())
    }

    pub fn update_status(&self, items_tracked: usize, data_used: usize) -> Result<(), PricingError>{
        match self.status.lock(){
            Ok(mut stat) => {let internal_statust  = if let Some(mut_status) = (**stat).as_any_mut().downcast_mut::<PricingStatus>(){
                mut_status
            }else{ 
                return Err(PricingError::IncorectStatusType);
            };
            internal_statust.updates_processed += 1;
            internal_statust.items_being_tracked = items_tracked;
            internal_statust.network_data_used += data_used as u64;
            internal_statust.last_update_time = Local::now();
            },
            Err(_) => { 
                return Err(PricingError::LockFailedError);
            },
        }
        Ok(())
    }

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
    ErrorThreadDown(String),
    FileReadError,
    TOMLReadError,
    JSONReadError,
    JSONFormatError(String),
    WTFError(String),
    NonFatal,
    SQLformatError(String),
    SQLReadWrite(String),
}

impl Display for PricingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PricingError::DataBaseConnectionError => write!(f, "DATA_BASE_CONNECTION_ERROR: Couldn't get a connection to the given data base."),
            PricingError::StatusIntialiseError => write!(f, "STATUS_INTIALISE_ERROR: Couldn't get a lock on status while initialising."),
            PricingError::IncorectStatusType => write!(f, "INCORECT_STATUS_TYPE: Status wasn't of the corect type."),
            PricingError::LockFailedError => write!(f, "LOCK_FAILED_ERROR: Couldn't get a lock on status while updating."),
            PricingError::ErrorThreadDown(messige) => write!(f, "ERROR_THREAD_DOWN: This error shouldend be probegated. Couldn't send a messige to the error thread, with messige {}", messige),
            PricingError::TOMLReadError => write!(f, "TOML_READ_ERROR: Error parsing the settings file, i may be malformed or from the wrong appication"),
            PricingError::FileReadError => write!(f, "FILE_READERROR: Coudn't read the settings file."),
            PricingError::WTFError(messig) => write!(f, "good job you dit somthing that sould be imposible:\n{}", messig),
            PricingError::JSONReadError => write!(f, "JSON_READ_ERROR: failed to parse the json file."),
            PricingError::JSONFormatError(cause) => write!(f, "JSON_FORMAT_ERROR: json file didn't have the expected format:\n{}", cause),
            PricingError::NonFatal => write!(f, "a not fatal error that shouldn't be propegated"),
            PricingError::SQLformatError(messig) => write!(f, "SQL_FORMAT_ERROR: {}", messig),
            PricingError::SQLReadWrite(messig) => write!(f, "SQL_READ_WRITE: error while reading or writing to the database{}", messig),
        }
    }
}

impl From<DataBaseError> for PricingError {
    fn from(value: DataBaseError) -> Self {
       PricingError::SQLReadWrite(format!("{:?}",value))
    }
}
impl From<rusqlite::Error> for PricingError {
    fn from(value: rusqlite::Error) -> Self {
        PricingError::SQLformatError(format!("{:?}", value))
    }
}
impl std::error::Error for PricingError{}

struct PricingStatus{
    updates_processed: usize,
    items_being_tracked: usize,
    network_data_used: u64,
    last_update_time: DateTime<Local>,
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