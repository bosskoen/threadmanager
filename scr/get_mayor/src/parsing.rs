use std::fs;

use library::{data_base_manager::{to_sql_variant, SQLformat}, toml};
use serde::Deserialize;

use crate::types::MayorError;

#[derive(Deserialize)]
pub struct Settings {
    name: String,

    pub step_rate: usize,
    pub update_rate: usize,
    pub mayor_period: usize,
    pub poll_window: usize,

    pub table_name: String,

    pub user_login_path: String,
    pub owner_login_path: String,

    pub url: String,
}

#[derive(Deserialize)]
pub struct DataBaseLogin {
    pub user_name: String,
    pub password: String,
    pub host: String,
    pub database_name: String,
}

impl DataBaseLogin {
    pub fn get(file_name: &String) -> Result<Self, MayorError> {
        let settings = fs::read_to_string(file_name).map_err(|_| MayorError::ParsingError("database login file".to_string()))?;

        let config = toml::from_str::<DataBaseLogin>(&settings).map_err(|_| MayorError::ParsingError("failed to parse toml database login".to_string()))?;
        Ok(config)
    }
}

impl Settings {
    pub fn get(file_name: &String) -> Result<Self, MayorError> {
        let settings = fs::read_to_string(file_name).map_err(|_| MayorError::FileReadError("settings file".to_string()))?;

        let config = toml::from_str::<Settings>(&settings).map_err(|_| MayorError::ParsingError("failed to parse toml setting".to_string()))?;
        if config.name != super::APP_NAME {
            return Err(MayorError::ParsingError(format!("oped wrong file: expected[ {} ] but got[ {} ]",super::APP_NAME, config.name)));
        }
        Ok(config)
    }
}
#[derive(Clone)]
pub struct MayorData{
    pub time: i64,
    pub name: String,
}
impl MayorData {
    pub fn get(json: &String)-> Result<Self, MayorError>{
        #[derive(Deserialize)]
        #[allow(non_camel_case_types)]
        struct name{ name: String}

        #[derive(Deserialize)]
        #[allow(non_camel_case_types, non_snake_case)]
        struct inner{lastUpdated: i64, mayor: name}

        let x = serde_json::from_str::<inner>(&json).map_err(|_| MayorError::ParsingError("failed to parse json".to_string()))?;
        Ok(Self { time: x.lastUpdated, name: x.mayor.name })
    }
}

impl<'c> SQLformat<'c> for MayorData{
    fn sqlformat(&'c self) -> Vec<library::data_base_manager::ToSql<'c>> {
        vec![to_sql_variant(&self.time), to_sql_variant(&self.name)]
    }
}