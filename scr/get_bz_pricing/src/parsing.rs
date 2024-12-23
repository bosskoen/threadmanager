use std::{collections::HashMap, fs, time::SystemTime};

use serde::Deserialize;

use crate::PricingError;


#[derive(Deserialize)]
pub struct Settings{
    name: String,
    pub step_rate: usize,
    pub update_rate: usize,
    pub table_name: String,
    pub data_base_path: String,
    pub url: String,
}

impl Settings{
    pub fn get(file_name: &String) -> Result<(Self, SystemTime),PricingError>{
        let settings = fs::read_to_string(file_name).map_err(|_| PricingError::FileReadError)?;
        let last_wote = fs::metadata(file_name).map_err(|_|PricingError::FileReadError )?.modified().map_err(|_| PricingError::FileReadError)?;

        let config= toml::from_str::<Settings>(&settings).map_err(|_| PricingError::TOMLReadError)?;
        if config.name != "get_bz_pricing"{
            return Err(PricingError::TOMLReadError);
        } 
        return Ok((config, last_wote));
    }
}

#[derive(Deserialize)]
pub struct BzData{
    success: bool,
    cause: Option<String>,
    lastUpdated: Option<u64>,
    products: Option<HashMap<String, Product>>,
}
#[derive(Deserialize)]
pub struct Product{
    product_id: String,
    quick_status: QuickStatus,
}
#[derive(Deserialize)]
pub struct QuickStatus{
    sellPrice: f64,
    buyPrice: f64,
    sellVolume: u64,
    sellMovingWeek: u64,
    buyVolume: u64,
    buyMovingWeek: u64,
}
