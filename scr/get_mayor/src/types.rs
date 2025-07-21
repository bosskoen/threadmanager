use std::{
    fmt::Display,
    fs,
    sync::{Arc, Mutex, atomic::AtomicBool},
    time::{Duration, Instant, SystemTime},
};

use humansize::{BINARY, format_size};
use library::{
    DateTime, Local, Status,
    data_base_manager::{DataBaseError, SyncConnection},
    error_handeler::Printer,
    format_duration,
    web_service_adapter::{WebServiceError, get_data_plus_size},
};

use crate::{get_current_mayer, parsing::*, unix_to_sys};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Polling,
    CatchUp,
    Waiting,
}
impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Polling => write!(f, "Polling"),
            Mode::CatchUp => write!(f, "CatchUp"),
            Mode::Waiting => write!(f, "Waiting"),
        }
    }
}
pub struct Context {
    pub accumulated: Duration,
    pub last_loop: Instant,

    pub mode: Mode,

    pub mayor_name: String,

    pub table_name: String,
    pub database_user: SyncConnection,
    pub url: String,

    pub step_rate: usize,
    pub update_rate: usize,
    pub mayor_reletc: usize,
    pub window: usize,

    pub stopflag: Arc<AtomicBool>,
    status: Arc<Mutex<Box<dyn Status>>>,
    settings_path: String,

    last_setting_modifiled: SystemTime,
}

impl Context {
    pub fn from(
        stopflag: Arc<AtomicBool>,
        status: Arc<Mutex<Box<dyn Status>>>,
        settings_path: String,
        printer: &Printer,
    ) -> Result<Self, MayorError> {
        let settings = Settings::get(&settings_path)?;

        let user = DataBaseLogin::get(&settings.user_login_path)?;
        let mut database_user = SyncConnection::new(
            &user.user_name,
            &user.password,
            &user.host,
            &user.database_name,
        )?;

        crate::enshure_database(
            DataBaseLogin::get(&settings.owner_login_path)?,
            &settings.table_name,
            &user.user_name,
            &printer,
        )?;

        let db_option = get_current_mayer(&database_user, &settings.table_name)?;
        let data = get_data_plus_size(&settings.url, 3, Duration::from_secs(3))?;
        let data_used = data.received_bytes + data.sent_bytes;
        let mayor_api_data = MayorData::get(&data.text)?;

        let mayor_name;
        let last_mayor;

        if let Some(db_data) = db_option {
            if db_data.year < mayor_api_data.year {
                //new mayor
                database_user.write_database(
                    &[mayor_api_data.clone()],
                    &settings.table_name,
                    "time, mayor, year",
                )?;
                mayor_name = mayor_api_data.name;
                last_mayor = unix_to_sys(mayor_api_data.time);
            } else {
                mayor_name = mayor_api_data.name;
                last_mayor = unix_to_sys(db_data.time);
            }
        } else {
            database_user.write_database(
                &[mayor_api_data.clone()],
                &settings.table_name,
                "time, mayor, year",
            )?;
            mayor_name = mayor_api_data.name;
            last_mayor = unix_to_sys(mayor_api_data.time);
        }

        start_status(&status, last_mayor.into(), mayor_name.clone(), data_used as u64)?;

        Ok(Self {
            accumulated: Duration::ZERO,
            last_loop: Instant::now(),
            mode: Mode::CatchUp,
            mayor_name,
            table_name: settings.table_name,
            database_user,
            step_rate: settings.step_rate,
            update_rate: settings.update_rate,
            mayor_reletc: settings.mayor_period,
            window: settings.poll_window,
            stopflag,
            status,
            settings_path: settings_path.clone(),
            url: settings.url,
            last_setting_modifiled: fs::metadata(settings_path)
                .map_err(|_| MayorError::FileReadError("metadata from setting file".to_string()))?
                .modified()
                .map_err(|_| MayorError::FileReadError("metadata from setting file".to_string()))?,
        })
    }

    pub fn update_status(
        &self,
        last_mayor: Option<(DateTime<Local>, String)>,
        data_used: usize,
    ) -> Result<(), MayorError> {
        if let Ok(mut inner) = self.status.lock() {
            let status = if let Some(x) = (**inner).as_any_mut().downcast_mut::<MayorStatus>() {
                x
            } else {
                return Err(MayorError::StatusError(
                    "downcast to MayorStatus".to_string(),
                ));
            };

            status.data_used += data_used as u64;
            status.mode = self.mode;

            if let Some((time, name)) = last_mayor {
                status.mayor_caghes += 1;
                status.last_mayor = time;
                status.mayor_name = name;
            }
        } else {
            return Err(MayorError::StatusError("lock to update".to_string()));
        }

        Ok(())
    }
    pub fn update_timing(&mut self) -> Result<(), MayorError> {
        let last_modified = fs::metadata(&self.settings_path)
            .map_err(|_| MayorError::FileReadError("metadata from setting file".to_string()))?
            .modified()
            .map_err(|_| MayorError::FileReadError("metadata from setting file".to_string()))?;

        if last_modified != self.last_setting_modifiled {
            let settings = Settings::get(&self.settings_path)?;

            self.last_setting_modifiled = last_modified;

            self.url = settings.url;
            self.step_rate = settings.step_rate;
            self.update_rate = settings.update_rate;
            self.mayor_reletc = settings.mayor_period;
            self.window = settings.poll_window;
        }
        Ok(())
    }
}

fn start_status(
    status: &Arc<Mutex<Box<dyn Status>>>,
    last_mayor: DateTime<Local>,
    mayor_name: String,
    data_used: u64
) -> Result<(), MayorError> {
    let new_status = MayorStatus {
        last_mayor,
        mayor_name,
        mayor_caghes: 0,
        mode: Mode::CatchUp,
        data_used,
        start_time: Local::now(),
    };

    if let Ok(mut inner) = status.lock() {
        (*inner) = Box::new(new_status);
    } else {
        return Err(MayorError::StatusError("lock to initiolise".to_string()));
    }
    Ok(())
}

struct MayorStatus {
    last_mayor: DateTime<Local>,
    mayor_name: String,

    mayor_caghes: usize,
    mode: Mode,
    data_used: u64,
    start_time: DateTime<Local>,
}

impl Status for MayorStatus {
    fn format(&self) -> String {
        let now = Local::now();
        format!(
            "Last mayor \"{}\" took offes at {}, {} ago\nmayor tracked: {}\ncurrent mode: {}\ndata used: {}\nplugin is running sinds: {}, uptime: {} ",
            self.mayor_name,
            self.last_mayor.format("%Y %m-%d; %H:%M:%S"),
            int_dur_from(self.last_mayor, now),
            self.mayor_caghes,
            self.mode,
            format_size(self.data_used, BINARY),
            self.start_time.format("%Y %m-%d; %H:%M:%S"),
            format_duration(self.start_time, now)
        )
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

fn int_dur_from(start: DateTime<Local>, end: DateTime<Local>) -> String {
    let duration = end - start;

    let total_days = duration.num_days();
    let hours = duration.num_hours() % 24;
    let minutes = duration.num_minutes() % 60;
    let seconds = duration.num_seconds() % 60;

    format!(
        "{} days, {:02}:{:02}:{:02}",
        total_days, hours, minutes, seconds
    )
}

#[derive(Debug)]
pub enum MayorError {
    ParsingError(String),
    DataBaseError(DataBaseError),
    StatusError(String),
    WebError(WebServiceError),
    FileReadError(String),
}

impl From<DataBaseError> for MayorError {
    fn from(value: DataBaseError) -> Self {
        Self::DataBaseError(value)
    }
}
impl From<WebServiceError> for MayorError {
    fn from(value: WebServiceError) -> Self {
        Self::WebError(value)
    }
}
impl std::error::Error for MayorError {}
impl Display for MayorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MayorError::ParsingError(msg) => write!(f, "{msg}"),
            MayorError::DataBaseError(data_base_error) => {
                write!(f, "Database Error: {data_base_error}")
            }
            MayorError::StatusError(msg) => write!(f, "Status Error: failed to: {msg}"),
            MayorError::WebError(web_service_error) => write!(f, "Web Error: {web_service_error}"),
            MayorError::FileReadError(msg) => write!(f, "File Error: failed to read {msg}"),
        }
    }
}
