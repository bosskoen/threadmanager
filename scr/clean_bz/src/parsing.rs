use std::fs;

use library::toml;
use serde::Deserialize;

use crate::types::CleanError;

#[derive(Deserialize)]
pub struct Settings{
    name: String,

    step_rate: usize,
    update_hour: i32, // 0:00 (0am) = 0; 23 (11pm) = 23; 5:30 = 5,5;
    
    pub table_name: String,

    pub user_login_path: String,
    pub owner_login_path: String,

    pub cleaning_profiles: Vec<CleaningProfiles>
}

#[derive(Deserialize)]
pub struct CleaningProfiles{
    full_retention_days: usize,     // keep all rows for this many recent days
    samples_to_keep_per_interval: usize, // keep this many rows per interval per item
    sample_interval_days: i32,      // interval in days (e.g., 1 = daily)
}

impl Settings {
    pub fn get(file_name: &String) -> Result<Self, CleanError> {
        let settings = fs::read_to_string(file_name).map_err(|_| CleanError::FileReadError("settings file".to_string()))?;

        let config = toml::from_str::<Settings>(&settings).map_err(|_| CleanError::ParsingError("failed to parse toml setting".to_string()))?;
        if config.name != super::APP_NAME {
            return Err(CleanError::ParsingError(format!("oped wrong file: expected[ {} ] but got[ {} ]",super::APP_NAME, config.name)));
        }
        Ok(config)
    }
}

#[derive(Deserialize)]
pub struct DataBaseLogin {
    pub user_name: String,
    pub password: String,
    pub host: String,
    pub database_name: String,
}

impl DataBaseLogin {
    pub fn get(file_name: &String) -> Result<Self, CleanError> {
        let settings = fs::read_to_string(file_name).map_err(|_| CleanError::ParsingError("database login file".to_string()))?;

        let config = toml::from_str::<DataBaseLogin>(&settings).map_err(|_| CleanError::ParsingError("failed to parse toml database login".to_string()))?;
        Ok(config)
    }
}