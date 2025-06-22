use std::{collections::HashMap, fs};

use library::toml;
use serde::Deserialize;

use crate::PricingError;

#[derive(Deserialize)]
pub struct Settings {
    name: String,
    pub step_rate: usize,
    pub update_rate: usize,
    pub table_name: String,
    pub data_base_path: String,
    pub url: String,
    pub lookup_table_name: String,
}

impl Settings {
    pub fn get(file_name: &String) -> Result<Self, PricingError> {
        let settings = fs::read_to_string(file_name).map_err(|_| PricingError::FileReadError)?;

        let config = toml::from_str::<Settings>(&settings).map_err(|_| PricingError::TOMLReadError)?;
        if config.name != super::APP_NAME {
            return Err(PricingError::TOMLReadError);
        }
        Ok(config)
    }
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct BzTryData {
    success: bool,
    cause: Option<String>,
    lastUpdated: Option<u64>,
    products: Option<HashMap<String, Product>>,
}

pub struct BzData {
    pub success: bool,
    pub last_updated: u64,
    pub products: HashMap<String, Product>,
}

#[derive(Deserialize)]
pub struct Product {
    pub product_id: String,
    pub quick_status: QuickStatus,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
pub struct QuickStatus {
    pub sellPrice: f64,
    pub buyPrice: f64,
    pub sellVolume: usize,
    pub sellMovingWeek: usize,
    pub buyVolume: usize,
    pub buyMovingWeek: usize,
}

impl BzTryData {
    fn get(input: String) -> Result<Self, PricingError> {
        let json = serde_json::from_str::<Self>(&input).map_err(|_| PricingError::JSONReadError)?;
        if !json.success {
            if let Some(value) = json.cause {
                return Err(PricingError::JSONFormatError(value));
            } else {
                return Err(PricingError::JSONFormatError("missing cause".to_string()));
            }
        }
        Ok(json)
    }
}

impl BzData {
    pub fn from_data(input: String) -> Result<Self, PricingError> {
        let json = BzTryData::get(input)?;
        let last_updated = json.lastUpdated.ok_or(PricingError::JSONFormatError("missing lastUpdated field".to_string()))?;
        let products = json.products.ok_or(PricingError::JSONFormatError("missing products field".to_string()))?;
        Ok(BzData {
            success: json.success,
            last_updated,
            products,
        })
    }
}