use std::{
    fmt::{Display}, fs, sync::{atomic::AtomicBool, Arc, Mutex}, time::SystemTime
};

use library::{
    data_base_manager::{DataBaseError, SyncConnection}, error_handeler::Printer, format_duration, DateTime, Local, Status
};

use crate::{
    confurm_db, next_time_hour, parsing::{CleaningProfiles, DataBaseLogin, Settings}
};

pub struct Context {
    pub stopflag: Arc<AtomicBool>,
    status: Arc<Mutex<Box<dyn Status>>>,
    settings_path: String,
    pub update_time: f32,
    pub step_rate: usize,
    last_time_setting_written: SystemTime,

    pub next_run: DateTime<Local>,

    pub db_connection: SyncConnection,
    pub cleaning_profiles: Vec<CleaningProfiles>,
    pub table: String,

    pub printer: Printer
}

impl Context {
    pub fn from(
        stopflag: Arc<AtomicBool>,
        status: Arc<Mutex<Box<dyn Status>>>,
        settings_path: String,
        printer: &Printer
    ) -> Result<Self, CleanError> {
        let (settings, last_time_setting_written) = Settings::get(&settings_path)?;

        initialize_status(&status)?;
        let onwer = DataBaseLogin::get(&settings.owner_login_path)?;
        let user = DataBaseLogin::get(&settings.user_login_path)?;

        confurm_db(
            &settings.table_name,
            &user.user_name,
            SyncConnection::new(
                &onwer.user_name,
                &onwer.password,
                &onwer.host,
                &onwer.database_name,
            )?,
            printer
        )?;
        let db_connection = SyncConnection::new(
                &user.user_name,
                &user.password,
                &user.host,
                &user.database_name,
            )?;
            
        let next_run = next_time_hour(settings.update_hour)?;

        Ok(Context {
            stopflag,
            status,
            settings_path,
            update_time: settings.update_hour,
            step_rate: settings.step_rate,
            last_time_setting_written,
            next_run,
            db_connection,
            cleaning_profiles: settings.cleaning_profiles,
            table: settings.table_name,
            printer: printer.clone()
        })
    }

    pub fn update_status(&self, deleted: u64) ->Result<(), CleanError>{
        if let Ok(mut inner) = self.status.lock(){
            if let Some(status) = (**inner)
            .as_any_mut().downcast_mut::<CleanStatus>(){
                status.time_ran += 1;
                status.rows_deleted += deleted;
            }else {
                return Err(CleanError::StatusError(
                    "downcast to MayorStatus".to_string(),
                ));
            }
        } else {
            return Err(CleanError::StatusError("lock to update".to_string()));
        }
        Ok(())
    }

    pub fn update_timing(&mut self) -> Result<(), CleanError>{
            let last_modified = fs::metadata(&self.settings_path)
            .map_err(|_| CleanError::FileReadError("metadata from setting file".to_string()))?
            .modified()
            .map_err(|_| CleanError::FileReadError("metadata from setting file".to_string()))?;

        if last_modified != self.last_time_setting_written {
            let (settings, _ )= Settings::get(&self.settings_path)?;

            self.last_time_setting_written = last_modified;

            self.step_rate = settings.step_rate;
            self.update_time = settings.update_hour;

            self.cleaning_profiles = settings.cleaning_profiles;


        }
        Ok(())
    }
}

fn initialize_status(status: &Arc<Mutex<Box<dyn Status>>>) -> Result<(), CleanError> {
    let new_status = CleanStatus {
        start_time: Local::now(),
        time_ran: 0,
        rows_deleted: 0,
    };

    if let Ok(mut inner) = status.lock() {
        (*inner) = Box::new(new_status);
    } else {
        return Err(CleanError::StatusError("lock to initiolise".to_string()));
    }

    Ok(())
}

struct CleanStatus {
    // rows cleaned TODO
    start_time: DateTime<Local>,
    time_ran: usize,
    rows_deleted: u64
}

impl Status for CleanStatus {
    fn format(&self) -> String {
        format!(
            "the bazaar cleaning ran {} times and has deleted {} rows\nplugin is running sinds: {}, uptime: {} ",
            self.time_ran,
            self.rows_deleted,
            self.start_time.format("%Y %m-%d; %H:%M:%S"),
            format_duration(self.start_time, Local::now())
        )
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[derive(Debug)]
pub enum CleanError {
    ParsingError(String),
    DataBaseError(DataBaseError),
    StatusError(String),
    FileReadError(String),
    TimeError(String),
    ErrorThreadDown(String),
}

impl From<DataBaseError> for CleanError {
    fn from(value: DataBaseError) -> Self {
        Self::DataBaseError(value)
    }
}

impl std::error::Error for CleanError {}
impl Display for CleanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CleanError::ParsingError(msg) => write!(f, "{msg}"),
            CleanError::DataBaseError(data_base_error) => {
                                write!(f, "Database Error: {data_base_error}")
                            }
            CleanError::StatusError(msg) => write!(f, "Status Error: failed to: {msg}"),
            CleanError::FileReadError(msg) => write!(f, "File Error: failed to read {msg}"),
            CleanError::TimeError(msg) => write!(f, "Time Pars Error: coudn't calulate next cleaning cicel:\n{msg}"),
CleanError::ErrorThreadDown(msg) => write!(f, "Error Thread down:\n{msg}"),
        }
    }
}
