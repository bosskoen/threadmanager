use std::{fmt, fs, sync::{atomic::AtomicBool, Arc, Mutex}, time::SystemTime};

use library::{format_duration, impl_status, toml, DateTime, Local, Status};
use serde::Deserialize;

pub struct Context {
    pub stopflag: Arc<AtomicBool>,
    status: Arc<Mutex<Box<dyn Status>>>,
    settings_path: String,
    pub update_rate: usize,
    pub step_rate: usize,
    pub time_passed: usize,
    last_time_setting_written: SystemTime,
}

impl Context {
    pub fn from(stopflag: Arc<AtomicBool>, status: Arc<Mutex<Box<dyn Status>>>, settings_path: String) -> Result<Self, PluginError> {
        let (settings, last_time_setting_written) = Settings::get(&settings_path)?;

        initialize_status(&status)?;

        Ok(Context { stopflag, status, settings_path, update_rate: settings.update_rate, step_rate: settings.step_rate, time_passed: 0, last_time_setting_written })
    }

    pub fn update_timing(&mut self) -> Result<(), PluginError> {
        let mod_time = fs::metadata(&self.settings_path)
            .map_err(|_| PluginError::FileReadError)?
            .modified()
            .map_err(|_| PluginError::FileReadError)?;

        let duration_since_last_update = mod_time
            .duration_since(self.last_time_setting_written)
            .map_err(|_| PluginError::WTFError("You time traveled, the file was modified in the past!".to_string()))?;

        if duration_since_last_update.as_secs() <= 0 {
            return Ok(());
        }

        let (setting, _) = Settings::get(&self.settings_path)?;
        self.update_rate = setting.update_rate;
        self.step_rate = setting.step_rate;
        self.last_time_setting_written = mod_time;
        Ok(())
    }

    pub fn update_status(&self) -> Result<(), PluginError> {
        match self.status.lock() {
            Ok(mut stat) => {
                let internal_status = if let Some(mut_status) = (**stat).as_any_mut().downcast_mut::<PluginStatus>() {
                    mut_status
                } else {
                    return Err(PluginError::IncorrectStatusType);
                };
                internal_status.times_run += 1;
                internal_status.last_update_time = Local::now();
            },
            Err(_) => {
                return Err(PluginError::LockFailedError);
            },
        }
        Ok(())
    }
}

#[derive(Deserialize)]
pub struct Settings {
    name: String,
    pub step_rate: usize,
    pub update_rate: usize,
}

impl Settings {
    fn get(file_name: &String) -> Result<(Self, SystemTime), PluginError> {
        let settings = fs::read_to_string(file_name).map_err(|_| PluginError::FileReadError)?;
        let last_write = fs::metadata(file_name).map_err(|_| PluginError::FileReadError)?.modified().map_err(|_| PluginError::FileReadError)?;

        let config = toml::from_str::<Settings>(&settings).map_err(|_| PluginError::TOMLReadError)?;
        if config.name != crate::APP_NAME {
            return Err(PluginError::TOMLReadError);
        }
        Ok((config, last_write))
    }
}

#[derive(Debug)]
pub enum PluginError {
    FileReadError,
    TOMLReadError,
    StatusInitializeError,
    ErrorThreadDown(String),
    IncorrectStatusType,
    LockFailedError,
    WTFError(String),
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginError::FileReadError => write!(f, "FILE_READ_ERROR: Couldn't read the settings file."),
            PluginError::TOMLReadError => write!(f, "TOML_READ_ERROR: Error parsing the settings file, it may be malformed or from the wrong application"),
            PluginError::StatusInitializeError => write!(f, "STATUS_INITIALIZE_ERROR: Couldn't get a lock on status while initializing."),
            PluginError::ErrorThreadDown(message) => write!(f, "ERROR_THREAD_DOWN: This error shouldn't be propagated. Couldn't send a message to the error thread, with message {}", message),
            PluginError::IncorrectStatusType => write!(f, "INCORRECT_STATUS_TYPE: Status wasn't of the correct type."),
            PluginError::LockFailedError => write!(f, "LOCK_FAILED_ERROR: Couldn't get a lock on status while updating."),
            PluginError::WTFError(message) => write!(f, "Good job, you did something that should be impossible:\n{}", message),
        }
    }
}

impl std::error::Error for PluginError {}

fn initialize_status(status: &Arc<Mutex<Box<dyn Status>>>) -> Result<(), PluginError> {
    let newstatus = PluginStatus::new();

    if let Ok(mut status) = status.lock() {
        (*status) = Box::new(newstatus);
    } else {
        return Err(PluginError::StatusInitializeError);
    }
    Ok(())
}

struct PluginStatus {
    times_run: usize,
    last_update_time: DateTime<Local>,
    start_time: DateTime<Local>
}

impl PluginStatus {
    fn new() -> Self {
        PluginStatus { times_run: 0, last_update_time: Local::now(), start_time: Local::now() }
    }
}

impl_status! {PluginStatus, |s: &PluginStatus| format!(
    "This is a test and template plugin that ran {} times and started {}.\n\
    Last update was {} and this plugin is running for {}.",
    s.times_run, s.start_time.format("%Y %m-%d; %H:%M:%S"),
    s.last_update_time.format("%Y %m-%d; %H:%M:%S"), format_duration(s.start_time, Local::now())
)}