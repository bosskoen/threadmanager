use std::{ fmt, fs, sync::{atomic::{AtomicBool, Ordering}, mpsc::Sender, Arc, Mutex}, thread, time::{Duration, SystemTime}};

use error_handeler::ErrorOperation;
use library::*;
use serde::Deserialize;

const APP_NAME: &str = "template_plugin";

struct Context{
    stopflag: AtomicBool,
    status: Arc<Mutex<Box<dyn Status>>>,
    settings_path: String,
    update_rate: usize ,
    step_rate: usize,
    time_passed: usize,
    last_time_setting_written: SystemTime,
}
impl Context {
    fn from(stopflag: AtomicBool, status: Arc<Mutex<Box<dyn Status>>>, settings_path: String) -> Result<Self, PluginError>{
        let (settings, last_time_setting_written) = Settings::get(&settings_path)?;

        initialise_status(&status)?;

        Ok(Context{stopflag, status ,settings_path, update_rate: settings.update_rate, step_rate: settings.step_rate, time_passed: 0, last_time_setting_written})
    }

    fn update_timing(&mut self) -> Result<(), PluginError>{
        let mod_time =fs::metadata(&self.settings_path)
            .map_err(|_|PluginError::FileReadError )?
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

    fn update_status(&self) -> Result<(), PluginError>{
        match self.status.lock(){
            Ok(mut stat) => {let internal_statust  = if let Some(mut_status) = (**stat).as_any_mut().downcast_mut::<PluginStatus>(){
                mut_status
            }else{ 
                return Err(PluginError::IncorectStatusType);
            };
            internal_statust.times_run += 1;
            internal_statust.last_update_time = Local::now();
            },
            Err(_) => { 
                return Err(PluginError::LockFailedError);
            },
        }
        Ok(())
    }
}

#[derive(Deserialize)]
pub struct Settings{
    name: String,
    pub step_rate: usize,
    pub update_rate: usize,
}

impl Settings{
    fn get(file_name: &String) -> Result<(Self, SystemTime),PluginError>{
        let settings = fs::read_to_string(file_name).map_err(|_| PluginError::FileReadError)?;
        let last_wote = fs::metadata(file_name).map_err(|_|PluginError::FileReadError )?.modified().map_err(|_| PluginError::FileReadError)?;

        let config= toml::from_str::<Settings>(&settings).map_err(|_| PluginError::TOMLReadError)?;
        if config.name != APP_NAME{
            return Err(PluginError::TOMLReadError);
        } 
        return Ok((config, last_wote));
    }
}

#[derive(Debug)]
enum PluginError {
    FileReadError,
    TOMLReadError,
    StatusIntialiseError,
    ErrorThreadDown(String),
    IncorectStatusType,
    LockFailedError,
    WTFError(String),
}
impl  fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginError::FileReadError => write!(f, "FILE_READERROR: Coudn't read the settings file."),
            PluginError::TOMLReadError => write!(f, "TOML_READ_ERROR: Error parsing the settings file, i may be malformed or from the wrong appication"),
            PluginError::StatusIntialiseError => write!(f, "STATUS_INTIALISE_ERROR: Couldn't get a lock on status while initialising."),
            PluginError::ErrorThreadDown(messige) => write!(f, "ERROR_THREAD_DOWN: This error shouldend be probegated. Couldn't send a messige to the error thread, with messige {}", messige),
            PluginError::IncorectStatusType => write!(f, "INCORECT_STATUS_TYPE: Status wasn't of the corect type."),
            PluginError::LockFailedError => write!(f, "LOCK_FAILED_ERROR: Couldn't get a lock on status while updating."),
            PluginError::WTFError(messig) => write!(f, "good job you dit somthing that sould be imposible:\n{}", messig),
        }
    }
}
impl std::error::Error for PluginError{}

fn initialise_status(status: &Arc<Mutex<Box<dyn Status>>>) -> Result<(), PluginError>{
    let newstatus = PluginStatus::new();

    if let Ok(mut status)= status.lock(){
        (*status) = Box::new(newstatus);
    }else {return Err(PluginError::StatusIntialiseError);}
    Ok(())
}

struct PluginStatus{
    times_run: usize,
    last_update_time: DateTime<Local>,
    start_time: DateTime<Local>
}

impl PluginStatus {
    fn new() -> Self{
        PluginStatus { times_run: 0, last_update_time: Local::now(), start_time:Local::now()}
    }
}

impl_status!{PluginStatus, |s: &PluginStatus| format!(
    "this is a test and template plugin that ran {} times and stated {}.
    last update was {} and this plugin is running for {}",
    s.times_run, s.start_time.format("%Y %m-%d; %H:%M:%S"),
    s.last_update_time.format("%Y %m-%d; %H:%M:%S"), format_duration(s.start_time, Local::now())
)}

fn test_error_theard(error_handel: &Sender<ErrorOperation>) -> Result<(), PluginError>{
    if let Err(_) = error_handel.send(ErrorOperation::Print(APP_NAME.to_string(),"test print".to_string())){
        return Err(PluginError::ErrorThreadDown("test print".to_string()));
    }
    Ok(())
}

#[no_mangle]
pub fn start(error_handel: Sender<ErrorOperation>, stopflag: AtomicBool, status: Arc<Mutex<Box<dyn Status>>>, settings_path: String) -> Result<(), Box<dyn std::error::Error>>{
    let mut context =Context::from(stopflag, status, settings_path)?;

    test_error_theard(&error_handel).map_err(|err|match err{
        PluginError::ErrorThreadDown(messige) => Box::new(ErrorThreadDownError::new(APP_NAME,&messige)) as Box<dyn std::error::Error>,
        _ => Box::new(err) as Box<dyn std::error::Error>,
    })?;

    loop {
        let start_of_loop = SystemTime::now();
        context.update_timing()?; 
        if context.stopflag.load(Ordering::Relaxed){
            break;
        }
        if context.time_passed >= context.update_rate{
            context.time_passed = 0;
            if let Err(_) = error_handel.send(ErrorOperation::Print(APP_NAME.to_string(),"test plugin action 'print'".to_string())){
                return Err(Box::new(ErrorThreadDownError::new(APP_NAME,"test plugin action 'print'")));
            }
            context.update_status()?
        }else{
            context.time_passed += context.step_rate;
        }

        let endloop =match start_of_loop.elapsed() {
            Ok(duration) => duration,
            Err(error) => {
                if let Err(_) = error_handel.send(ErrorOperation::Print(APP_NAME.to_string(),format!("error while getting elepsted time: {}", error))){
                    return Err(Box::new(ErrorThreadDownError::new(APP_NAME,&format!("error while getting elepsted time: {}", error))));
                }
                Duration::ZERO
            },
        };

        if let Some(sleep_duration) = Duration::from_secs(context.step_rate as u64).checked_sub(endloop) {
            thread::sleep(sleep_duration);
        } else {
            if let Err(_) = error_handel.send(ErrorOperation::Print(APP_NAME.to_string(),"loop took to long in price logger".to_string())){
                return Err(Box::new(ErrorThreadDownError::new(APP_NAME,"loop took to long in price logger")));
            }
            context.time_passed += (endloop.saturating_sub(Duration::from_secs(context.step_rate as u64))).as_secs() as usize;
        }
        
    }
    Ok(())
}
#[cfg(test)]
mod tests {
}
